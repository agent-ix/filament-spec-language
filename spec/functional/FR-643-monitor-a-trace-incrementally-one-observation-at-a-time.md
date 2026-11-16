---
id: FR-643
title: "Monitor a trace incrementally, one observation at a time, with carried state"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-283
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-330
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-820
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-502
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-642
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: references
  - target: ix://agent-ix/filament-specification/FR-114
    type: implements
  - target: ix://agent-ix/filament-specification/FR-094
    type: depends_on
---
# FR-643: Monitor a trace incrementally, one observation at a time, with carried state

## Description

`fsl-analyze` SHALL provide a stepping monitor, `MonitorSession`, that takes the
observations of one trace one at a time and reports, after each, the status of
every selected temporal clause. Between steps it carries the state the next step
needs: the positions still readable by an open activation or a past operator, each
clause's open obligations and settled result, the progress ledger (FR-502) and the
stamp order. Each clause's status after every step agrees with FR-283's batch
`monitor` over the trace observed so far (FSpec FR-114), in the sense "Batch
equality" states, and a result once settled never changes.
The session can be checkpointed to bytes and resumed, so a monitor restarted
after a crash continues from its state instead of starting a fresh complete
stream (FSpec FR-094-AC-6).

## Use case

A service streams state snapshots from a running rover. The operator opens a
session over the rover's three safety clauses and feeds each snapshot as it
arrives. The `NoCollision` clause settles `violation` the moment a decisive
snapshot arrives; the deadline clause stays `pending` until its window passes;
the session's checkpoint is written every minute, and after a restart the session
resumes from the last checkpoint with the same verdicts.

## Inputs

```rust
impl MonitorSession {
    pub fn open(
        package: &CheckedPackage,
        request: &MonitorOpen<'_>,
        limits: &MonitorLimits,
        cancel: &Cancel,
    ) -> Result<MonitorSession, MonitorRefusal>;
    pub fn step(&mut self, observation: MonitorObservation<'_>, cancel: &Cancel)
        -> Result<MonitorStep, MonitorRefusal>;
    pub fn assert_progress(&mut self, progress: ProgressAssertion, cancel: &Cancel)
        -> Result<MonitorStep, MonitorRefusal>;
    pub fn finish(self, end: TraceEnd, cancel: &Cancel) -> Result<MonitorOutcome, MonitorRefusal>;
    pub fn checkpoint(&self) -> MonitorState;
    pub fn resume(
        package: &CheckedPackage,
        state: &MonitorState,
        request: &MonitorOpen<'_>,
        limits: &MonitorLimits,
        cancel: &Cancel,
    ) -> Result<MonitorSession, MonitorRefusal>;
}

pub struct MonitorOpen<'a> {
    pub clauses: Vec<MonitorClause>,
    pub kind: MonitorTraceKind,
    pub lower_boundary: ProgressLowerBoundary,
    pub binding: Option<TraceBinding>,
    pub cover: Option<TimestampedCover>,
    pub ledger: Option<ProgressLedger>,
    pub packages: &'a PackageInput,
    pub sources: &'a [Source],
    pub temporal_selections: &'a [TemporalClockSelection],
}

pub enum MonitorTraceKind { Finite, Progressed, Timestamped, Timed }

pub struct MonitorObservation<'a> {
    pub position: MonitorPosition<'a>,
    pub stamp: Option<MonitorStamp>,
    pub order_key: Option<TimestampedOrderKey>,
}

pub enum MonitorPosition<'a> { Snapshot(&'a [u8]), Sample(&'a Value) }
pub enum MonitorStamp { Tick(u64), Exact(ExactRational) }
pub enum TraceEnd { Closed, Open }
```

`clauses`, `packages`, `sources`, `temporal_selections` and `limits` are FR-283's.
`kind` names the trace kind the observations form (FR-641); `binding`, `cover`
and `lower_boundary` are the members FR-641 gives that kind. `MonitorLimits` adds
`retained_positions` (setting `monitor.retained_positions`), the most
observation values one clause's evaluation state holds at once (FR-820
`retained()`): a bound on the session's memory, not a count of positions seen.

## Outputs

- `MonitorStep`: for each clause, in `clauses` order, its `ClauseStatus`
  (`Settled(ClauseRunReport)` or `Pending(ClauseRunReport)`), and the names of the
  clauses whose status changed at this step.
- `MonitorOutcome`: FR-283's outcome over the whole trace, with the final ledger.
- `MonitorState`: the checkpoint, bytes in the closed JSON form
  `filament.monitor-state/1`: the package identity, the open request's clause
  selections, kind, binding and lower boundary, the number of positions seen,
  the identities and stamps of the positions a counterexample can still cite,
  each settled clause's report and each unsettled clause's FR-820 `ClauseState`
  bytes (which hold the admitted values it retains), and the ledger in
  `filament.progress-ledger/1` form (FR-641).

## Behavior

- **Batch equality.** Write `B(k)` for the report FR-283's `monitor` gives a
  clause over the trace of the first `k` observations, of the session's kind,
  with the latest progress assertion, the surrounding execution `open`, and
  the ledger the session was opened with. After the `k`-th call of `step` or
  `assert_progress`:
  - a clause still `Pending` SHALL report exactly `B(k)`;
  - a clause that settled at call `s <= k` SHALL report exactly `B(s)`, the
    report over the prefix that decided it, and `B(k)` SHALL have the same
    category and truth as `B(s)`.
  A settled verdict holds for every extension of its deciding prefix (see
  "Settling"), so its category and truth are the same over every longer
  prefix; its other members are not. The batch counterexample of a bad
  prefix lists every position of the prefix it was run over (FSpec FR-364),
  and an `on each` clause's batch run may report an earlier activation that
  a later position also violates. The session therefore keeps the report of
  the deciding prefix, which names what decided the clause and never
  changes, and the batch agreement for a settled clause is on its category
  and truth over each longer prefix and on the whole report over its
  deciding prefix.
- **Finality.** Once a clause's status is `Settled`, every later step SHALL
  report the same report for it, and the session SHALL stop evaluating that
  clause.
- **Settling.** One rule holds for every profile: a clause SHALL settle at the
  first call after which its verdict over the observations seen holds for
  every extension of the trace, and at no other call. Each profile reads that
  rule through its own evaluator:
  - infinite-trace: a bad prefix (`violation`, or an `undefined` that is a
    violation) or a good prefix (an eventuality satisfied), which FR-327
    reads as decided whatever follows;
  - event-position and fixed-sample (closed-trace) profiles: the verdict the
    evaluator returns when it reads the positions seen as an open-scope
    prefix (FR-504 "Open scope"), definite only when every continuation
    preserves it, with basis `decisive-counterexample` or
    `decisive-witness`. The closed reading of a short prefix is not final: a
    bounded `eventually` whose window runs past the newest position is
    violated under false extension and may be satisfied by the next
    observation, so the clause stays `Pending` while `B(k)` reports that
    closed verdict;
  - finite-window and timed profiles: a verdict FR-501 or FR-642 reports
    with a decisive basis, or one whose every activation's window the
    asserted progress has closed.
  A clause that never meets the rule settles at `finish`.
- **Trace end.** `finish(end)` SHALL return, for each clause not yet settled,
  `B(n)` over all `n` observations, and for each settled clause its settled
  report. `end` closes the stream, not the trace: the closure of the trace
  is fixed by the trace itself, so `Closed` and `Open` give the same outcome
  for every kind. A `Finite` trace has one reading per profile, closed by
  false extension under event-position and fixed-sample and a prefix under
  infinite-trace (FR-327, FR-283 "Finite traces"); a `Progressed` or
  `Timestamped` trace is closed exactly as its last progress assertion's
  `scope` and `execution` say, which FR-502 does not let a later record
  revise; a `Timed` trace by FR-642's `through`. A caller that wants a
  closed progressed trace asserts progress with scope `closed` before the
  end.
- **Retained state.** `retained_positions` SHALL bound the memory of the
  session: each clause's evaluation is FR-820's carried `ClauseState`,
  advanced once per observation, and the session SHALL hold an observation's
  admitted value only while some unsettled clause's state holds it (FR-820
  "Summarized positions"). A position no unsettled clause reads by value is
  dropped; its identity is kept only where a counterexample can still cite
  it. When advancing a clause would make its `retained()` exceed
  `retained_positions`, that clause SHALL report `incomplete` with the limit
  record `{limit: "node count", value, setting: "monitor.retained_positions"}`,
  and the other clauses SHALL continue.
- **Order.** If an observation's stamp is below the previous one (for `Timed`)
  or at or below the ledger's watermark for its binding (for `Timestamped` and
  `Progressed`), then the session SHALL refuse it with FR-502's contradiction or
  FR-645's `timed-stamps-decrease` and SHALL keep its state unchanged; the
  caller can then continue with a correct observation.
- **Atomic steps.** A refused step, a cancelled step and an admission failure SHALL
  leave the session exactly as before the call.
- **Checkpoint.** The session SHALL resume from `checkpoint()` to a session
  whose every later step reports what the original session would have
  reported. `resume` SHALL refuse, at stage `admit`, `invalid_runtime_input`/`invalid-value` with
  `reason` `monitor-state-mismatch`, a state whose package identity, clause
  selections, kind or binding differ from the given package and request, and a
  state document that is not a closed `filament.monitor-state/1`.
- `open` SHALL refuse a clause whose profile does not read `kind` with FR-645's
  `trace-kind-mismatch`, before any observation.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-643-AC-1 | For FR-330-AC-1's `Bounded`, AC-2's `Reaches` and AC-12's `always holds(c.value <= 1)` over snapshots `c.value` 0, 1, 2, 1, 0 fed one at a time, the reports after each step equal FR-283's `monitor` over the prefix of that length, clause by clause, and `finish(Closed)` equals `monitor` over the closed trace. | Test (TC-1666) |
| FR-643-AC-2 | A `Timestamped` session over FR-641-AC-1's three observations with progress asserted after each reports, after the last, FR-501-AC-4's outcome; the ledger it returns equals the one the batch run returns. | Test (TC-1666) |
| FR-643-AC-3 | Once `always holds(c.value <= 1)` settles `violation` at the step of `c.value` 2, the later steps report the same report for it; a clause still pending changes status only at the step that decides it, and each step names exactly the clauses that changed. | Test (TC-1666) |
| FR-643-AC-4 | A checkpoint taken after step 2 and resumed gives, for steps 3 to 5, the reports of the uninterrupted session; resuming under another package, or with another clause list, refuses `monitor-state-mismatch`. | Test (TC-1667) |
| FR-643-AC-5 | A `Timestamped` observation stamped at or below the retained watermark refuses with `contradicted_dimension` `Watermark` and leaves the session unchanged, after which a correct observation is accepted; with `retained_positions` 2, the event-position clause `Recent` (`always[0,9] count where holds(r.level = 2) over past[0, 3] min_count 0 >= 0`), whose window holds three positions at the third observation, reports `incomplete` naming `monitor.retained_positions`, while the infinite-trace `Reaches` and `Never` in the same session, which retain no value (FR-820), stay `Pending` over levels 0, 1, 0. | Test (TC-1667) |
| FR-643-AC-6 | Over samples with levels 0, 1, 2, 1, the infinite-trace `Never` (`always holds(r.level < 2)`) settles `violation` at step 3; at step 4 its report equals `B(3)`, whose counterexample lists three samples, while `B(4)` has the same category `violation` and truth `false` and a counterexample of four samples; `Reaches` settles `success` at step 3 and reports `B(3)` at step 4. | Test (TC-2321) |
| FR-643-AC-7 | The event-position clause `E` (`eventually[0,2] holds(r.level = 2)`) over levels 0, 1 stays `Pending` at steps 1 and 2 while `B(1)` and `B(2)` report `violation` (the closed reading), and settles `success` with basis `decisive-witness` at step 3 on level 2; over levels 0, 1, 1 it settles `violation` with basis `decisive-counterexample` at step 3, when the window `[0, 2]` is wholly observed. | Test (TC-2321) |
| FR-643-AC-8 | For a `Finite` session over FR-643-AC-1's snapshots, `finish(Closed)` and `finish(Open)` return equal outcomes; for a `Progressed` session whose last assertion has scope `open`, `finish(Closed)` returns the open-scope outcome, equal to `finish(Open)`. | Test (TC-2322) |
| FR-643-AC-9 | A `Finite` session over 1,000 samples whose `level` cycles 0 to 8, with `retained_positions` 4, keeps the infinite-trace `Below` (`always holds(r.level < 9)`) and `Nine` (`eventually holds(r.level = 9)`) `Pending` to the end with no `incomplete`, and `Recent` (window of four positions) settles `success` at step 10 without `incomplete`, holding at most four values until then; the checkpoint after the last sample holds no admitted value. | Test (TC-2322) |

## Dependencies

- [FR-283](FR-283-monitor-a-supplied-trace-offline.md): the batch monitor the
  session equals.
- [FR-820](FR-820-advance-a-temporal-clause-one-position-at-a-time-from-carried-state.md):
  the carried clause state each step advances.
- [FR-330](FR-330-run-a-temporal-clause-through-the-spine.md),
  [FR-502](FR-502-reject-a-regressing-watermark-and-a-conflicting-completeness-revision-on-the-spine.md),
  [FR-641](FR-641-carry-every-temporal-trace-kind-and-the-progress-ledger-on-the-run-request.md),
  [FR-642](FR-642-run-a-timed-clause-over-a-recorded-timed-trace.md).
- FSpec FR-114 (batch and incremental agree), FR-094 (restart needs restored
  state).
