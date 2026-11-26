---
id: FR-1052
title: "Settle a proof that is not certified inconclusive when the request requires certified proofs"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-282
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-160
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-250
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-290
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-314
    type: references
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-520
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-301
    type: depends_on
---
# FR-1052: Settle a proof that is not certified inconclusive when the request requires certified proofs

## Description

Every `proved` result carries one certification (ADR-018 PC-1, FSpec FR-331
"Certification"): `certified` when a core checker accepted its certificate or
the core's own prove path produced it, `uncertified` when a first-party engine
or solver outside the core proved it and no core checker re-checked it, and
`trusted` when a third-party plugin proved it. By default all three settle
`proved` and the label tells the reader which kind of proof it is.

A request MAY instead require certified proofs, on every route that can
settle `proved`. Under that policy a `proved` result whose certification is not `certified` SHALL
settle `inconclusive`, so a reader who must act only on checked proofs never
receives an unchecked `proved`. The policy is off by default; with it off,
every result and every label is as it is without this requirement.

The policy is `CertificationPolicy`:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CertificationPolicy {
    /// Every certification settles `proved` (ADR-018 PC-1).
    #[default]
    AdmitAny,
    /// Only a `certified` proof settles `proved`.
    RequireCertified,
}
```

The type is `fsl-replay`'s, the one definition every route takes. Each route
selects it as follows; an absent member is `false`:

| Route | Selected by |
| --- | --- |
| `analyze` over a request | The `certification` member of FR-281's `AnalyzeRequest`, of the document route's `AnalyzeRun` and of `PackageRun` (FR-850) |
| `analyze_items` and `provider::run` | Their `CertificationPolicy` argument |
| The verification planner's `settle` and `settle_transfer` (FR-529) | Their `CertificationPolicy` argument |
| A verification plan run (FR-753) | The `certification` member of `PlanInputs` |
| `native-analyze/1` (FR-620) | The request member `require_certified`, a JSON boolean |
| `native-run/1`: a `call`'s claim items (FR-960), a checked-package run (FR-1039), `refinement_transfer` (FR-1030) and `verification_plan` (FR-1038) | The request member `require_certified`, a JSON boolean |
| A monitor request's provider-routed `items` (FR-644) | The request member `require_certified`, a JSON boolean read only beside `items` |

The driver's commands that send these requests select it with the flag
`--require-certified`, which sets the member (FSpec FR-301). An assurance
profile turns it on (see "Assurance profile" below).

## Rationale

A proof with no core check rests on an engine or a solver outside the
qualified core. Keeping such proofs `proved` by default keeps every
capability; the label already says what stands behind the proof. A user who
needs every proof checked, for example in a defence or safety case, needs a
run in which an unchecked proof cannot be read as a proof at all. Moving such
a result to `inconclusive` removes no capability: the same request without
the policy still gives the labelled proof.

### Assurance profile

No assurance profile that a request can name is defined in FSL or in FSpec
today; the quoin `AssuranceProfile` artifact selects a repository's review
requirements and is not an input of `analyze`. Selecting one is not specified
here. When an assurance profile is defined, it SHALL set
`CertificationPolicy::RequireCertified`; until then the policy is off unless a
request or caller selects it.

## Inputs

- FR-281's `AnalyzeRequest`, with its `certification` member.
- The terminal record each item settles to under FR-281's settlement map,
  after every certificate check, replay and coverage check that map runs.

## Outputs

Each item's terminal record, changed only as "Behavior" states.

## Behavior

- Under `AdmitAny`, `analyze` SHALL settle every item exactly as it does
  without this requirement: each result, basis, cause, label and record
  member unchanged.
- Under `RequireCertified`, `analyze` SHALL settle an item whose terminal
  value would be `Proved{basis, certification}` with `certification`
  `Uncertified` or `Trusted` as `inconclusive`, basis `unsettled`, O-16
  category inconclusive, with the FSpec FR-331 cause
  `proof-not-certified{basis, certification}`, which records the proof's
  `basis` object and its `certification`. The record SHALL carry no `proved` result and no `certification`
  member.
- Under `RequireCertified`, a `Proved{basis, certification: Certified}`
  record and every record that is not `proved` (`refuted`, `inconclusive`,
  `incomplete`, `unsupported`, `declined`, `invalid-request`, `failed`,
  `measured`) SHALL be unchanged.
- The policy SHALL apply after the item's own settlement and SHALL NOT
  change which engine or provider runs, its limits, its certificate check or
  its replay. It SHALL apply to every route that can settle `proved`: FSL's
  in-process engines, linked providers, process providers (FR-635) and
  plugin results (FR-290).
- The paths the policy covers are, at least, every route ADR-018 PC-1 labels
  `uncertified` or `trusted`:

  | Proof | Certification by default | Defined by |
  | --- | --- | --- |
  | SMT `BoundedComplete` or `Inductive` with no certificate, or with one whose proof uses a rule outside the checked Alethe set | `uncertified` | ADR-018 PC-6, FR-314 |
  | Symmetry- or partial-order-reduced proof; proof under a state constraint | `uncertified` | ADR-021 RV-1, FR-160 |
  | Hybrid `bounded-solver` or `inductive-solver` proof with no certificate, or from a method with no certificate form | `uncertified` (a plugin's `trusted`) | FR-250 |
  | Refinement liveness half, which no core checker covers (dense and discrete) | `uncertified` | ADR-020 CT-4 |
  | State-graph, hyper and refinement simulation proofs whose checker FR-282 lists as "none yet", and protocol-analysis proofs, which have no core checker | `uncertified` | FR-282, ADR-018 PC-1 |
  | A global-conformance proof with an obligation the closure check does not cover: a `exactly-once-premise` channel or a `requires temporal` requirement, and every proof over a recording | `uncertified` | FR-896, FR-338 |
  | A finite-domain value-validity proof evaluated exhaustively, whose method has no core certificate checker | `uncertified` | FSpec FR-483 |
  | A linked provider's `proved` | `uncertified` | FR-635 |
  | A process provider's or plugin's `proved` | `trusted` | FR-290 |

  A rejected certificate already settles `inconclusive`,
  `CertificateRejected{rule, at}` (ADR-018 PC-2, FR-751), and a closed
  search whose certificate no core check read already settles `incomplete`,
  `CertificateUnchecked` (FR-182), with or without the policy.
- Every route of "Description"'s table SHALL apply the same rule to each
  result it would write `proved`. A `refinement_transfer` request SHALL
  analyze its premises under the policy, and its transfer result, and a
  planner guarantee or composition result, whose fold of premise
  certifications is `uncertified` or `trusted` SHALL settle `inconclusive`
  with cause `proof-not-certified{certification}`, with no `certification`
  member and no conclusion record. A planner result has no proof `basis`
  object, so that cause carries none (FSpec FR-492).
- A supplied transfer premise that is `proved`, `uncertified` or `trusted` is
  folded as FSpec FR-820 folds it, so the transfer result it would certify no
  better than that settles `inconclusive`.
- A global-conformance item whose record is not a `certified` proof SHALL
  write each FR-990 label that is `proved` as `inconclusive`: a `proved`
  `global` member becomes label `inconclusive`, basis `unsettled`, `complete`
  `false` and cause `proof-not-certified`, and so does each `proved` label of
  `prefixes` and `projections`. Its obligation verdicts and closure record are
  the evidence the engine computed and stay. An item whose record is a
  `certified` proof is unchanged.
- FR-621's `aggregate` SHALL read the settled record: a required item the
  policy settles `inconclusive` is a `not-proved` gap with `result`
  `inconclusive`.
- The `native-analyze/1` reader SHALL refuse a `require_certified` member
  that is not a JSON boolean as any member of the wrong JSON type is
  refused (FR-620 "Reading"): the whole request at the request stage, with
  `invalid-request`, exit 20, naming `require_certified`, and no item run.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1052-AC-1 | Over FR-751-AC-1's `Braking` without `t`, the `inductive-solver` entry with no certificate settles `proved`, `Uncertified` with `require_certified` absent and with it `false`; with `require_certified` `true` it settles `inconclusive`, basis `unsettled`, with `proof-not-certified` recording a basis of type `inductive-solver` and certification `uncertified`, and the record carries no `certification` member. The same entry with FR-751-AC-1's accepted certificate settles `proved`, `Certified` with the member absent and with it `true`. | Test (TC-3240) |
| FR-1052-AC-2 | A linked provider's `proved` for a routed value-validity item settles `proved`, `Uncertified` by default and `inconclusive` with `proof-not-certified` recording the same basis and certification `uncertified` under `RequireCertified`. | Test (TC-3240) |
| FR-1052-AC-3 | A `native-analyze/1` item that proves FR-620's Counter claim under a state constraint every state satisfies settles `proved`, `uncertified` by default and `inconclusive` with `proof-not-certified` recording its `basis` and `uncertified` with `require_certified` `true`. | Test (TC-3240) |
| FR-1052-AC-4 | FR-290-AC-1's process-provider `proved` result settles `proved`, `trusted` by default and `inconclusive` with `proof-not-certified` recording certification `trusted` under `require_certified` `true`. | Test (TC-3240) |
| FR-1052-AC-5 | Under `require_certified` `true`, FR-620's Counter `Bounded` item, whose EN-1 closure certificate the core checks, settles `proved`, `Certified`, as without the member; an item that settles `refuted`, one that settles `incomplete` `LimitReached{max_states}` (FR-620-AC-7) and one that settles `invalid-request` (FR-620-AC-4) settle identically with and without the policy. | Test (TC-3240) |
| FR-1052-AC-6 | A `native-analyze/1` request whose only required item is AC-1's uncertified entry has `aggregate` `{"status": "complete", "gaps": []}` by default and, under `require_certified` `true`, `{"status": "incomplete", "gaps": [{"request_index": 0, "reason": "not-proved", "result": "inconclusive"}]}`. | Test (TC-3240) |
| FR-1052-AC-7 | A `native-analyze/1` request with `"require_certified": "yes"`, and one with `"require_certified": 1`, each refuse the whole request at the request stage with `invalid-request`, exit 20, naming `require_certified`, and run no item. | Test (TC-3240) |
| FR-1052-AC-8 | `analyze` and `provider::run` over AC-2's request and AC-4's request settle the item under `CertificationPolicy::RequireCertified` as AC-2 and AC-4 state, and under `AdmitAny` settle it `proved`. | Test (TC-3240) |
| FR-1052-AC-9 | A global-conformance item over a recording with a closed closure record, whose record is `proved`, `uncertified` and whose `global` member is `proved`, settles under `RequireCertified` `inconclusive` and writes `global` with label `inconclusive`, basis `unsettled`, `complete` `false` and cause `proof-not-certified`, with the same closure record and obligation verdicts. The same claim over a protocol system whose channels are `at-most-once` and that declares no `requires temporal`, whose record is `proved`, `certified`, is unchanged. | Test (TC-3240) |
| FR-1052-AC-10 | A `native-run/1` call whose claim item settles `proved`, `uncertified` by default settles it `inconclusive` with `proof-not-certified` under `require_certified` `true`. | Test (TC-3240) |
| FR-1052-AC-11 | A `refinement_transfer` request whose transfer settles `proved` with a supplied `uncertified` premise settles it `inconclusive` with `proof-not-certified` `uncertified` under `require_certified` `true`. | Test (TC-3240) |
| FR-1052-AC-12 | The planner's `settle` over a composition whose fold is `trusted` settles `proved` under `AdmitAny`, and under `RequireCertified` `inconclusive` with `proof-not-certified` `trusted`, no `certification` and no conclusion; a `certified` fold is unchanged. | Test (TC-3240) |
| FR-1052-AC-13 | A `verification_plan` run whose step settles `proved`, `uncertified` by default settles that step `inconclusive` under `require_certified` `true`. | Test (TC-3240) |
| FR-1052-AC-14 | A monitor request's provider-routed item that settles `proved`, `uncertified` or `trusted` by default settles `inconclusive` under `require_certified` `true`; `require_certified` without `items` is refused. | Test (TC-3240) |
| FR-1052-AC-15 | `analyze_items` over an item whose engine proves it `uncertified` settles it `proved` under `AdmitAny` and `inconclusive` with `proof-not-certified` under `RequireCertified`. | Test (TC-3240) |
| FR-1052-AC-16 | A global-conformance claim over a protocol with an `exactly-once-premise` channel and every obligation satisfied settles `proved`, `uncertified`. | Test (TC-3240) |

## Dependencies

- FSpec FR-331 "Certification" and its `inconclusive` cause table: the cause
  `proof-not-certified{basis, certification}`. FSpec owns the cause
  vocabulary and the result schema.
- FSpec FR-520 "Document members": the `native-analyze/1` member
  `require_certified`.
- FSpec FR-301: the `--require-certified` flag, which the driver implements
  (FR-287); FSL holds no command line.
- An assurance profile that sets the policy, which neither FSL nor FSpec
  defines yet.
- FR-281's `AnalyzeRequest`, FR-620's reader, FR-621's `aggregate`.
