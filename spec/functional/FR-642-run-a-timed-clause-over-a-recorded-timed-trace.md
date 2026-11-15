---
id: FR-642
title: "Check and run a timed clause over a recorded timed trace"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-233
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-330
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-645
    type: references
  - target: ix://agent-ix/filament-specification/FR-541
    type: implements
  - target: ix://agent-ix/filament-specification/FR-416
    type: depends_on
---
# FR-642: Check and run a timed clause over a recorded timed trace

## Description

A clause under `filament.temporal.timed/v1` SHALL be checkable and runnable over
a recorded timed trace: a sequence of positions each with an exact rational time
stamp. Over a timed model subject the clause binds `model-time` (FR-233) and the
trace supplies the stamps the snapshots do not hold. Over a declared record trace
subject the clause's clock name is a role that the request's `clocks` member
binds to a `timed` configuration naming the unit (FR-640). The evaluator reads
the clause pointwise with FSpec FR-416's meaning and settles it by FSpec FR-541:
exactly over a timed lasso, and three-valued over a finite prefix.

## Use case

An operator has a recorded log of a controller's acknowledgements, stamped in
milliseconds, and the rule `always (holds(r.req) implies eventually[0 ms, 200 ms]
holds(r.ack))`. They run it over the log as a `timed` trace. A request with no
acknowledgement inside 200 ms of the stamps is reported as a violation at that
position; a request near the end of the log, whose 200 ms are not yet covered,
is reported pending unless the log's `through` stamp says no event happened
before the window closed.

## Inputs

FR-641's `timed` trace, `{"prefix": [Q, ...], "loop": [Q, ...], "period": X,
"through": X}`:

- `Q`: `{"position": P, "stamp": X}`, `P` an FR-641 position, `X` an exact
  non-negative rational written as a canonical string: a decimal integer (`"5"`)
  or a reduced fraction with a positive denominator (`"3/2"`);
- `prefix`: required, non-empty when `loop` is absent;
- `loop`: optional; present means a timed lasso, whose `period` is required;
- `period`: the time one loop repetition adds, a positive rational; repetition
  `k >= 1` of loop position `j` has stamp `τ_j + k·period`;
- `through`: optional on a finite trace and absent on a lasso: the stamp up to
  which the recording is complete, so no position with stamp at most `through`
  exists beyond the supplied ones.

In process the same trace is `TraceInput::Timed { prefix: Vec<TimedPosition>,
repeat: Vec<TimedPosition>, period: Option<ExactRational>, through:
Option<ExactRational> }`, and FR-283's `monitor` takes it as a `Typed` trace.

## Outputs

FR-109's `ClauseRunReport` with FSpec FR-541's verdict: `success`, `violation`
(with the failing position and its stamp), `pending` (category `pending`, basis `unsettled`), an `Undefined` violation, or a refusal.

## Behavior

### Checking

- Over a declared record trace subject, the checker SHALL admit a clause under
  the timed profile whose clock name is not `model-steps` or `model-time`, and
  SHALL record its clock name as a clock role to be bound by a `timed`
  configuration (FR-640). It no longer refuses such a clause as a timed trace
  subject that is not checked yet.
- Over a record trace subject, `clock "model-time"` SHALL refuse
  `invalid_timed_model`/`model-time-on-trace-subject` at the clock argument: a
  record has no model time source.
- Over a model subject, the FR-233 rules are unchanged.
- The clause's intervals SHALL be read in the binding's unit: the model's time
  unit, or the `timed` configuration's `unit`.

### Running

- The entry SHALL evaluate a timed clause over `TraceInput::Timed` exactly once
  with FSpec FR-416's pointwise meaning, reading each interval as a difference of
  stamps, and SHALL settle it by FSpec FR-541.
- The entry SHALL admit each snapshot position by FR-106 (or each sample by the
  trace-subject route's type check) before evaluating.
- The entry SHALL refuse at stage `admit`, `invalid_runtime_input`/`invalid-value`,
  with FR-645's reason, message and locus, a timed trace whose stamps decrease
  (`timed-stamps-decrease`), a lasso with an empty loop (`empty-lasso-loop`), a
  finite timed trace with an empty prefix (`empty-finite-trace`), a lasso whose
  `period` is not positive or is shorter than the loop's own span
  `τ_last − τ_first` (`timed-period-too-short`), a lasso carrying `through` or a
  finite trace carrying `period` (`timed-member-misplaced`), and a `through`
  below the last stamp (`timed-through-before-last-stamp`).
- A timed clause over any trace kind other than `timed`, and a `timed` trace for
  a clause under any other profile, SHALL refuse with FR-645's
  `trace-kind-mismatch`. In particular a timed clause over a model subject run
  over `finite` or `lasso` snapshots refuses at stage `admit` with `profile`
  `filament.temporal.timed/v1`, `expected` `["timed"]` and `supplied` the kind
  given: snapshots carry no time stamp, and the evaluator does not substitute
  positions for time.
- A timed lasso SHALL be evaluated exactly: its value at a position of
  repetition `k` equals its value at the same loop position of repetition
  `k − 1` once `k·period` exceeds the clause's largest finite interval bound
  plus the loop span, so the evaluator reads finitely many repetitions.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-642-AC-1 | A unit with `record Reading { req: Boolean; ack: Boolean; }` and `temporal Ack using timed over (r: Reading) clock "plant" on origin { always (holds(r.req) implies eventually[0, 200] holds(r.ack)) }` with `clocks` `{"plant": {"kind": "timed", "unit": "ms"}}` compiles; the same unit with `clock "model-time"` refuses `invalid_timed_model`/`model-time-on-trace-subject` at the clock argument. | Test (TC-1664) |
| FR-642-AC-2 | `Ack` over a finite `timed` trace of samples `req` at 0, `ack` at 150 reports `pending`; with `ack` at 250 and `through` 250 it reports `violation` at position 0 (the origin activation); with `ack` at 150, a second `req` at 300 and `through` 600 it reports `violation` at position 0 with the request at stamp 300 as the unanswered one; with `ack` at 150 and `through` 150 it still reports `pending`, because the unbounded `always` reads positions the recording does not hold yet. | Test (TC-1664) |
| FR-642-AC-3 | `Ack` over a timed lasso with prefix `req` at 0, loop `ack` at 100 then `req` at 120, `period` 200, reports `success`; with `period` 400 (the next `ack` 380 after the loop `req`) it reports `violation`. | Test (TC-1664) |
| FR-642-AC-4 | `eventually[0, 1/2] holds(p)` over stamps `"0"`, `"1/3"` with `p` true only at the second position reports `success` and the interval is compared exactly, not in floating point. | Test (TC-1664) |
| FR-642-AC-5 | A timed clause over a model subject run over `finite` snapshots refuses at stage `admit`, `invalid_runtime_input`/`invalid-value`, `reason` `trace-kind-mismatch`, `profile` `filament.temporal.timed/v1`, `expected` `["timed"]`, `supplied` `finite`, with FR-645's message; stamps 0, 5, 3 refuse `timed-stamps-decrease` naming position 2; a lasso with `period` 0 refuses `timed-period-too-short`. | Test (TC-1665) |

## Dependencies

- [FR-233](FR-233-bind-a-model-claim-to-model-time-or-model-steps.md): the
  model-subject bindings.
- [FR-640](FR-640-select-clock-configurations-in-compile-and-run-requests.md):
  the `timed` clock configuration;
  [FR-641](FR-641-carry-every-temporal-trace-kind-and-the-progress-ledger-on-the-run-request.md):
  the `timed` trace on the wire.
- FSpec FR-416 (pointwise timed meaning), FR-541 (settling over a timed trace).
