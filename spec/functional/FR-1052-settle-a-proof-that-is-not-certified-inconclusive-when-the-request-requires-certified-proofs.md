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

An `analyze` request MAY instead require certified proofs. Under
that policy a `proved` result whose certification is not `certified` SHALL
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

It is the `certification` member of FR-281's `AnalyzeRequest`, of the document
route's `AnalyzeRun` and the argument of `provider::run`. A
`native-analyze/1` request (FR-620) selects it with the request member
`require_certified`, a JSON boolean, absent meaning `false`; the driver's
commands that send that request select it with the flag
`--require-certified`, which sets that member (FSpec FR-301). An assurance
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
`CertificationPolicy::RequireCertified`; until then the policy is selected
only by the request member or the flag.

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
  | State-graph, hyper and refinement simulation proofs whose checker FR-282 lists as "none yet", and protocol-analysis and conformance proofs, which have no core checker | `uncertified` | FR-282, ADR-018 PC-1 |
  | A finite-domain value-validity proof evaluated exhaustively, whose method has no core certificate checker | `uncertified` | FSpec FR-483 |
  | A linked provider's `proved` | `uncertified` | FR-635 |
  | A process provider's or plugin's `proved` | `trusted` | FR-290 |

  A rejected certificate already settles `inconclusive`,
  `CertificateRejected{rule, at}` (ADR-018 PC-2, FR-751), and a closed
  search whose certificate no core check read already settles `incomplete`,
  `CertificateUnchecked` (FR-182, FR-204), with or without the policy.
- A global-conformance item the policy settles `inconclusive` SHALL write its
  FR-990 `global` member with label `inconclusive`, basis `unsettled`,
  `complete` `false` and cause `proof-not-certified`, and each `prefixes` and
  `projections` label that is `proved` as `inconclusive`. Its obligation
  verdicts and closure record are the evidence the engine computed and stay.
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
| FR-1052-AC-2 | FR-314-AC-3's bounded certificate whose proof uses `:rule hole` and is not shown wrong settles `proved`, `BoundedComplete`, `Uncertified` by default and `inconclusive` with `proof-not-certified` under `require_certified` `true`; FR-314-AC-11's `CasRefinesCounter` records with no `Base` record settle `Proved{Inductive{depth: 1}, Uncertified}` by default and `inconclusive` with `proof-not-certified` under the policy. | Test (TC-3240) |
| FR-1052-AC-3 | FR-160-AC-1's symmetry-reduced `fair weak each` instance and FR-160-AC-2's `Wide` constrained proof settle `proved`, `Uncertified` by default and `inconclusive` with `proof-not-certified` under `require_certified` `true`, each cause recording its `reduced` or `exhaustive` basis. | Test (TC-3240) |
| FR-1052-AC-4 | FR-290-AC-1's process-provider `proved` result settles `proved`, `trusted` by default and `inconclusive` with `proof-not-certified` recording certification `trusted` under `require_certified` `true`. | Test (TC-3240) |
| FR-1052-AC-5 | Under `require_certified` `true`, FR-620's Counter `Bounded` item, whose EN-1 closure certificate the core checks, settles `proved`, `Certified`, as without the member; an item that settles `refuted`, one that settles `incomplete` `LimitReached{max_states}` (FR-620-AC-7) and one that settles `invalid-request` (FR-620-AC-4) settle identically with and without the policy. | Test (TC-3240) |
| FR-1052-AC-6 | A `native-analyze/1` request whose only required item is AC-1's uncertified entry has `aggregate` `{"status": "complete", "gaps": []}` by default and, under `require_certified` `true`, `{"status": "incomplete", "gaps": [{"request_index": 0, "reason": "not-proved", "result": "inconclusive"}]}`. | Test (TC-3240) |
| FR-1052-AC-7 | A `native-analyze/1` request with `"require_certified": "yes"`, and one with `"require_certified": 1`, each refuse the whole request at the request stage with `invalid-request`, exit 20, naming `require_certified`, and run no item. | Test (TC-3240) |
| FR-1052-AC-8 | `analyze` over AC-1's `AnalyzeRequest`, and `provider::run` over the same items, settle the item as AC-1 does under `CertificationPolicy::RequireCertified`, and under `AdmitAny` settle it `proved`, `Uncertified`. | Test (TC-3240) |

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
