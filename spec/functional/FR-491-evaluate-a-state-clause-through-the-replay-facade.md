---
id: FR-491
title: "Evaluate a state clause at an observation through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-060
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-952
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: traces_to
---
# FR-491: Evaluate a state clause at an observation through the replay facade

## Description

A backend that checks a state clause's truth at one observation, for example
CG's precondition oracle, must not name `fsl_replay::spine`: ADR-011 FB-05
lets CG depend on the layer-6 `replay` facade only, and T-12 rule (a)
(FR-060 T12-A) refuses any CG reference to `spine`. `spine::run_clause`
(FR-109) is the evaluation, but it returns a report built from spine types,
and it is not reachable from the crate root.

This requirement adds the replay-owned entry `fsl_replay::evaluate_clause`
beside `replay_state_clause` (FR-122). It runs FR-109's `Clause` selection
(FR-106 admission, then FR-107 evaluation) and returns one typed result.

## Inputs

- A `ClauseEvaluationRequest`: the unit (source identity, display path and
  bytes), the domain package documents it selects, its dependency input, the
  authored state clause's declared name, the spine, observation and accounting limits
  and one `ClauseObservation`.
- A `ClauseObservation` is of the clause's own kind (FR-106 check 2):
  `Current` (snapshot document bytes, anchor, self object) for an invariant
  or constraint, `PreCall` (pre snapshot document bytes, self object,
  parameter values) for a precondition.

## Outputs

`ClauseEvaluation`:

- `Completed(bool)`: the clause evaluated; `true` when it holds at the
  observation, `false` when it does not.
- `Undefined { reason }`: the clause has no value at the observation.
- `Refused(ClauseRefusal)`: the request was refused before a value was
  computed: `EmptySource`, `UnreadableObservation`, `WrongObservation` (an
  observation of the wrong kind for the clause), `MissingClause`, or
  `Coded` (stage, category, catalog code and cause) for a compile, admission
  or evaluation refusal, including a refusal of the evaluation's own inputs; or `Fault` for a broken invariant.
- `Incomplete(ClauseIncomplete)`: `Accounting` (the exhausted counter), `Stopped` (a resource stop or cancel
  of the evaluation's inputs) or `Observation` (evidence the clause reads is
  missing at admission, or the snapshot exceeds its byte limit).

## Behavior

- The entry SHALL select the clause by name and the observation by its kind,
  admit it by FR-106 and evaluate it by FR-107, as FR-109's `Clause`
  selection does, and SHALL NOT restate either.
- The entry SHALL NOT evaluate a precondition at a current observation or an
  invariant at a pre-call observation; the mismatch is `Refused`.
- No input, type or result of the entry SHALL record or compare a digest,
  hash, revision, seal, lock or pin. The request supplies document bytes, not
  document labels; the result carries no identity of a package, source or
  document.
- The root SHALL export the entry and its types so a consumer does not name
  `fsl_replay::spine`.

## Acceptance Criteria

Runtime division-by-zero coverage uses the function-body boundary of
[FR-495](./FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md):
a genuine helper body is checked under `CheckMode::Kernel`, and the declared
invariant that calls it remains `Linked`
([FR-952](./FR-952-record-each-function-body-s-check-mode-and-replay-under-it.md)). The existing
internal Kernel-preparation and shared clause-run/disposition path exercises
that case; the public source facade retains default Linked compilation and
no mode input. Its ordinary public-entry fixtures contain no unproved
zero-divisor body, and the direct unproved-source refusal is checked
separately.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-491-AC-1 | Over the `example/config-version` unit, the invariant `ParentOrder` at a `Current` snapshot where the parent's version is below the child's completes `true`, and where it is not completes `false`. | Test (TC-1274) |
| FR-491-AC-2 | The precondition `ParentPresent` at a `PreCall` snapshot completes `true` for an object with a parent and `false` for one without. | Test (TC-1274) |
| FR-491-AC-3 | `ParentOrder` at a `PreCall` observation and `ParentPresent` at a `Current` observation each return `Refused(WrongObservation)`. | Test (TC-1274) |
| FR-491-AC-4 | The public source facade refuses an invariant with the inline body `10 mod self.versionNumber >= 0`, and a helper body containing that unproved division compiled through the same public Linked entry, as `undefined_expression`/`unproved-nonzero` before evaluation. With `function q using v(x: Integer): Integer pure { 10 mod x }` genuinely prepared under `CheckMode::Kernel` and the Linked invariant body `q(self.versionNumber) >= 0`, the shared checked clause-run/disposition path returns `Undefined`, reason `division-by-zero`, at an object whose version is 0 and completes `true` at version 1. A unit test of the disposition mapping also covers `Undefined` and `Completed`; the separate public-entry controls remain those of AC-1 to AC-3. | Test (TC-1274) |
| FR-491-AC-5 | A clause name the unit does not declare returns `Refused(MissingClause)`; bytes that are not a snapshot document return `Refused(UnreadableObservation)`; accounting `work_units = 0` returns `Incomplete(Accounting)`. | Test (TC-1274) |
| FR-491-AC-6 | A consumer reaches the entry and every type of its request and result through the `fsl_replay` root alone: a test that names only `fsl_replay` root items compiles and runs the entry. | Test (TC-1274) |
| FR-491-AC-7 | A refusal of the evaluation's own inputs (a dangling reference, a value of the wrong kind) returns `Refused(Coded)` with its catalog code at stage `admit`; one that is a resource stop or cancel returns `Incomplete(Stopped)`. These arise only from S6a's supplied-value, storage and unsupported-helper stops, which the facade has no limit knob to provoke, so this criterion is verified on the disposition mapping (unit test), not end to end. | Test (TC-1274) |
| FR-491-AC-8 | A snapshot longer than the request's `observation_limits.document_bytes` returns `Incomplete(Observation)` carrying FR-255 Behavior 1's `stage_limit_exceeded` record (`kind` `input bytes`, `bound` the ceiling, `actual` the snapshot's byte count, `setting` `observation.input_bytes`, the one shape FR-312 gives this limit at every entry point) before it is read; at the limit it evaluates. | Test (TC-1274, TC-2982) |

## Dependencies

FR-106, FR-107, FR-109, FR-122 (the facade pattern), FR-060 (T12), ADR-011.
