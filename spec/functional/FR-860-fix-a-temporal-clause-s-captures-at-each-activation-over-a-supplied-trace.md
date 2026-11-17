---
id: FR-860
title: "Fix a temporal clause's captures at each activation over a supplied trace"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-337
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-330
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-524
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-820
    type: references
  - target: ix://agent-ix/filament-specification/FR-093
    type: implements
---
# FR-860: Fix a temporal clause's captures at each activation over a supplied trace

## Description

When a temporal clause declares captures (`capture n: T = e;`, FR-337
"Captures at S3"), the spine's temporal evaluator SHALL evaluate them over a
supplied trace and let the clause's `holds` conditions read them. At each
activation the evaluator fixes every capture once, from the observation at
the activation's position, and every `holds` condition of that activation
reads the fixed value at every later position it visits. This holds for every
entry that runs a temporal clause over a trace: FR-330's `run_clause` and
trace-subject route, FR-283's `monitor`, FR-643's `MonitorSession` and
FR-820's `ClauseState`, over every trace kind FR-641 carries.

A capture holds a value the clause compares later observations against,
such as the speed when a braking rule activated. Without it the rule
re-reads the current value at each position and states a different claim.

### Decision: a trace run reads captures; it does not refuse them

FR-337 "Capture observation" has the model checker refuse a clause whose
conditions read a capture (`ModelCheckRefusal::CaptureRead`), because a
letter of the product automaton is shared by the concurrent activations of
an `on each` clause whose capture vectors differ. A trace evaluator has no
such letter. It evaluates each activation separately, at its own position,
with its own environment, so each activation's captures are a fixed
binding of that one evaluation and the meaning is exactly FSpec FR-093's:
immutable and evaluated once at the activation anchor. The run path
therefore evaluates such a clause and returns its verdict. A capture fault
is reported as the clause's verdict at the activation that faulted, located
at the capture, like an undefined or refused `holds` condition (FR-327),
and never as an internal fault.

## Inputs

- The checked clause's captures in source order, each with its name,
  declared type, typed initializer and evaluation slot count (FR-337
  "Captures at S3"), read from the checked package.
- The trace, the `over` binding and the members FR-330 and FR-641 give each
  trace kind.

## Outputs

FR-330's `ClauseRunReport`, unchanged in shape. A capture fault is reported
as an `Undefined` violation or a refused condition, both of which the report
already carries.

## Behavior

### Fixing the captures

- For each activation FR-327 evaluates (position 0 for `on origin`, and each
  activating position for `on each`, after its `when` guard reads true),
  the evaluator SHALL evaluate the clause's captures in source order, once,
  over the observation at the activation's position. The `over` parameter
  is bound as FR-330 binds it at that position (the `over` object of the
  snapshot, or the record sample), the trigger binder `q` is bound to the
  same reference for `on each`, and each earlier capture is bound to its
  value. Where the guard is false the activation does not exist and no
  capture is evaluated.
- Each `holds` condition of the activation SHALL read a capture at its
  FR-337 slot as the value fixed at the activation, at every position the
  activation's formula visits, past positions and positions after the
  activation included. A later position SHALL NOT re-evaluate a capture.
  A capture of one activation SHALL NOT be read by another activation.
- The evaluator SHALL charge each capture it evaluates as a `holds`
  condition's evaluation is charged (FR-327), so a work budget bounds it.

### Capture faults

- A capture whose evaluation is undefined (a read at a key that holds no
  object of the required type, `absent-key`, or a kernel undefined value
  such as `division-by-zero`) SHALL make the activation's result
  `Undefined` at the activation's position, with the `UndefinedEvaluation`
  cause and the capture initializer's locus. FR-330 reports it as a
  `violation`, exit 10, with the counterexample FR-330 gives an `Undefined`
  result. It is never a default value and never a false capture.
- A capture whose evaluation is refused (a kernel refusal such as
  `integer_out_of_domain` when the initializer's value is coerced to the
  declared type) SHALL make the activation's result the refused condition
  of FR-327 "Refused condition" at the activation's position, with the
  refusal record and the capture initializer's locus. FR-330 reports it at
  stage `evaluate`, category `refusal`, exit 20.
- A capture evaluation that stops on an exhausted budget SHALL make the run
  `Incomplete` (exit 22), as an atom's evaluation does.
- A capture fault is ranked against the clause's other activations exactly
  as FR-327 ranks an undefined atom or a refused condition at the same
  position. An activation whose capture faults evaluates no `holds`
  condition.
- No capture reading SHALL be reported as an internal fault: a `holds`
  condition that reads a capture slot the evaluator has not bound is a
  defect of the evaluator, not an outcome of the run.

### Every trace kind and entry

- Over a `Lasso` or `Model` trace, the evaluator SHALL fix the captures of
  each activation from the observation at its position in the
  prefix-then-loop order of FR-329. A capture is a function of its
  activation position's observation, so the activations of two loop
  repetitions at the same loop position carry equal captures, and FR-329's
  exact reading of the lasso is unchanged.
- Over `Progressed`, `Timestamped` and `Timed` traces and over record
  samples, the evaluator SHALL fix the captures from the activation
  position's admitted observation exactly as above.
- The descent to the decisive atom (FR-330 "The decisive atom") SHALL
  evaluate an atom that reads a capture with the capture values of the
  activation being descended.
- FR-820's `ClauseState` SHALL keep each open activation's capture values
  as part of that activation's carried state, write them in `to_bytes` and
  read them back in `from_bytes`, and SHALL NOT evaluate a capture again
  after the activation is opened, on a later `advance` or after a restore.
  A capture value is not a position: it is not counted in `retained()` and
  it is dropped when its activation settles. FR-643's `MonitorSession` and
  its checkpoint inherit this, so a resumed session reports what the
  uninterrupted session would.

## Acceptance Criteria

The criteria use the unit with `record Reading { level: Int[0, 9]; stopped: Boolean; }`
and samples of `level` and `stopped`.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-860-AC-1 | Through `command::run`, the event-position false-extension clause `Fast` (`on origin { capture startFast: Boolean = r.level > 3; eventually[0,3] holds(r.stopped or not startFast) }`) over finite samples with levels 5, 1, 1, 1 and `stopped` false at each reports `violated` at position 0, exit 10, because `startFast` keeps the value `true` it took at position 0; the same clause with `not (r.level > 3)` in place of `not startFast` reports `holds` over the same samples, and `Fast` over levels 1, 5, 5, 5 reports `holds`. No run exits 30. | Test (TC-2440) |
| FR-860-AC-2 | Through `command::run`, the infinite-trace clause `Origin` (`on origin { capture startFast: Boolean = r.level > 3; eventually[0,3] holds(r.stopped or not startFast) }`) over a `lasso` with an empty prefix and the loop samples level 5 with `stopped` false, then level 1 with `stopped` false, reports `violated` at position 0, while the same clause with `not (r.level > 3)` in place of `not startFast` reports `holds`; over the loop level 5 `stopped` false, then level 5 `stopped` true, `Origin` reports `holds`. | Test (TC-2440) |
| FR-860-AC-3 | Through `command::run`, the event-position clause `Each` (`on each (q: Reading) { capture start: Int[0, 9] = q.level; always[0,0] holds(r.level = start) }`) over finite samples with levels 0, 1, 2 reports `holds`: each activation reads its own `start`, never another activation's. | Test (TC-2440) |
| FR-860-AC-4 | Through `command::run`, the event-position clause `Div` (`on origin { capture m: Integer = 3 mod (2 - r.level); always[0,1] holds(m >= 0) }`) over levels 2, 0 reports `undefined` at position 0 with category `violation`, exit 10, and cause `division-by-zero` located at the capture's initializer; over levels 0, 2 it reports `holds`, the capture being evaluated at position 0 only; the clause `on each (q: Reading)` with the same body over levels 0, 2 reports `undefined` at position 1. With `type Zero = Int[0, 0];` and `capture z: Zero = count<Zero>(m in ints(sequence[r.level]): m > 0);` read by `always[0,1] holds(z = 0)`, a sample with level 1 at position 0 reports stage `evaluate`, category `refusal`, exit 20, with `integer_out_of_domain`/`outside-domain` located at the capture's initializer. None of these runs exits 30 or reports `runtime_invariant`. | Test (TC-2440) |
| FR-860-AC-5 | Through `command::run` with a `native-monitor/1` request, `Fast` over the records levels 5, 1, 1, 1 with `stopped` false stays unsettled at records 0 to 2 and settles `violation` at record 3 with `settlement` `{step: "4", trigger: "decided"}` among its members; its disposition after each record equals the `native-run/1` run of `Fast` over the samples received so far, the capture fixed at record 0 throughout. | Test (TC-2441) |
| FR-860-AC-6 | A `ClauseState` of `Fast` written by `to_bytes` after the sample at position 1 and read back by `from_bytes` reports, after the samples at positions 2 and 3, the reports of the uninterrupted state, `violation` at position 0 after position 3, without evaluating the capture again; `retained()` does not count the capture. | Test (TC-2441) |

## Dependencies

- [FR-337](FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md):
  the checked captures, their slots and their reading by the model checker.
- [FR-327](FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md),
  [FR-330](FR-330-run-a-temporal-clause-through-the-spine.md): the
  activations, the ranking of undefined and refused results and the report.
- [FR-820](FR-820-advance-a-temporal-clause-one-position-at-a-time-from-carried-state.md),
  [FR-643](FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md):
  the carried state and the monitor session.
- [FR-524](FR-524-create-one-temporal-instance-per-trigger-or-origin-with-immutable-captures-and-name-every-capture-fault.md):
  the evidence entry, whose caller supplies capture facts instead of a
  trace.
- FSpec FR-093: a capture is immutable and evaluated once at the
  activation anchor.
