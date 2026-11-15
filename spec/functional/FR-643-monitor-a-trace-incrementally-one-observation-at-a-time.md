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
stamp order. Its status after every step equals FR-283's batch `monitor` over the
trace observed so far (FSpec FR-114), and a result once settled never changes.
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
`retained_positions` (setting `monitor.retained_positions`), the most positions
the session keeps at once.

## Outputs

- `MonitorStep`: for each clause, in `clauses` order, its `ClauseStatus`
  (`Settled(ClauseRunReport)` or `Pending(ClauseRunReport)`), and the names of the
  clauses whose status changed at this step.
- `MonitorOutcome`: FR-283's outcome over the whole trace, with the final ledger.
- `MonitorState`: the checkpoint, bytes in the closed JSON form
  `filament.monitor-state/1`: the package identity, the open request's clause
  selections, kind, binding and lower boundary, the number of positions seen,
  the retained positions (each with its identity, stamp and admitted value), each
  clause's settled report or open obligations, and the ledger in
  `filament.progress-ledger/1` form (FR-641).

## Behavior

- **Batch equality.** After the `k`-th call of `step` or `assert_progress`, each
  clause's report SHALL equal the report FR-283's `monitor` gives for that clause
  over the trace of the first `k` observations, of the session's kind, with the
  latest progress assertion, the surrounding execution `open`, and the ledger the
  session was opened with. `finish(Closed)` SHALL equal `monitor` over the closed
  trace, and `finish(Open)` over the open one.
- **Finality.** Once a clause's status is `Settled`, every later step SHALL report
  the same report for it, and the session SHALL stop evaluating that clause.
- **Retained state.** The session SHALL retain only the positions that some
  clause can still read: for a clause under a bounded profile, the positions
  within its past horizon of the newest position and from the oldest open
  activation; for an infinite-trace or timed clause still pending, every position
  since its activation. When retaining one more position would exceed
  `retained_positions`, that clause SHALL report `incomplete` with the limit
  record `{limit: "node count", value, setting: "monitor.retained_positions"}`,
  and the other clauses SHALL continue.
- **Order.** An observation whose stamp is below the previous one (for `Timed`) or
  at or below the ledger's watermark for its binding (for `Timestamped` and
  `Progressed`) SHALL refuse with FR-502's contradiction or FR-645's
  `timed-stamps-decrease`, and the session SHALL be unchanged, so the caller may
  continue with a correct observation.
- **Atomic steps.** A refused step, a cancelled step and an admission failure SHALL
  leave the session exactly as before the call.
- **Checkpoint.** `resume` from `checkpoint()` SHALL give a session whose every
  later step reports what the original session would have reported. `resume`
  SHALL refuse, at stage `admit`, `invalid_runtime_input`/`invalid-value` with
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
| FR-643-AC-5 | A `Timestamped` observation stamped at or below the retained watermark refuses with `contradicted_dimension` `Watermark` and leaves the session unchanged, after which a correct observation is accepted; with `retained_positions` 2, an infinite-trace clause still pending at the third observation reports `incomplete` naming `monitor.retained_positions`, and a bounded clause in the same session settles normally. | Test (TC-1667) |

## Dependencies

- [FR-283](FR-283-monitor-a-supplied-trace-offline.md): the batch monitor the
  session equals.
- [FR-330](FR-330-run-a-temporal-clause-through-the-spine.md),
  [FR-502](FR-502-reject-a-regressing-watermark-and-a-conflicting-completeness-revision-on-the-spine.md),
  [FR-641](FR-641-carry-every-temporal-trace-kind-and-the-progress-ledger-on-the-run-request.md),
  [FR-642](FR-642-run-a-timed-clause-over-a-recorded-timed-trace.md).
- FSpec FR-114 (batch and incremental agree), FR-094 (restart needs restored
  state).
