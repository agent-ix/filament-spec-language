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
type Failure = StageFailure<MonitorRefusal>;

impl<'a> MonitorSession<'a> {
    pub fn open(
        package: &'a CheckedPackage,
        request: MonitorOpen<'a>,
        limits: &MonitorLimits,
        cancel: &Cancel,
    ) -> Result<MonitorSession<'a>, Failure>;
    pub fn step(&mut self, observation: MonitorObservation<'_>, cancel: &Cancel)
        -> Result<MonitorStep, Failure>;
    pub fn assert_progress(&mut self, progress: ProgressAssertion, cancel: &Cancel)
        -> Result<MonitorStep, Failure>;
    pub fn assert_through(&mut self, through: ExactRational, cancel: &Cancel)
        -> Result<MonitorStep, Failure>;
    pub fn finish(self, cancel: &Cancel) -> Result<MonitorOutcome, Failure>;
    pub fn checkpoint(&self) -> Result<MonitorState, Failure>;
    pub fn resume(
        package: &'a CheckedPackage,
        state: &MonitorState,
        request: MonitorOpen<'a>,
        limits: &MonitorLimits,
        cancel: &Cancel,
    ) -> Result<MonitorSession<'a>, Failure>;
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

pub enum MonitorPosition<'a> { Snapshot(&'a [u8]), Sample(&'a str) }
pub enum MonitorStamp { Tick(u64), Exact(ExactRational) }
```

`clauses`, `packages`, `sources`, `temporal_selections` and `limits` are FR-283's.
`kind` names the trace kind the observations form (FR-641); `binding`, `cover`
and `lower_boundary` are the members FR-641 gives that kind. `MonitorLimits` adds
`retained_positions` (setting `monitor.retained_positions`), the most
observation values one clause's evaluation state holds at once (FR-820
`retained()`): a bound on the admitted values the session holds, not a
count of positions seen. It also adds `retained_identities` (setting
`monitor.retained_identities`, published default 1,048,576), the most
position identities one unsettled clause keeps for the counterexample it may
still report (FR-820 `cited()`). Together the two bound every part of the
session's state that grows with the trace. `assert_through` is the `Timed`
kind's completeness assertion: FR-642's `through`, the stamp up to which the
recording is complete.

### Decision: the accepted shape of the public API

Five points of the session's public API follow FSL's existing monitor and
spine APIs rather than a first sketch, and are accepted as its contract:

- **Errors are `StageFailure<MonitorRefusal>`.** Every entry returns FR-283's
  `StageFailure`, so a refusal (`Refused(MonitorRefusal)`), a cancellation
  and an internal fault are told apart exactly as the batch `monitor`
  reports them, and a caller handles both with one match.
- **`MonitorPosition::Sample` is canonical text.** A record sample is passed
  as its canonical JSON object text (JCS), not a parsed `Value`. The text is
  the sample's canonical value, which the session admits without
  re-serializing. It is not the position's identity: a record-sample
  position is identified by its index in the trace (FR-330), so two equal
  samples at different indices are distinct positions, each cited by its
  own index. To refuse a changed sample at a covered index (FR-502), each
  clause's carried state keeps a content fingerprint of every covered
  sample (FR-993). That fingerprint is integrity data, not identity
  (ADR-013 §2): it is written in the ledger's covered entries and the
  checkpoint, never in a report's verdict members. `step` always assigns the
  next index; a re-delivered sample reaches a covered index through a later
  request's trace checked against its input ledger (FR-502).
- **Reports are shared, not copied.** The session's `ClauseStatus` holds its
  FR-283 `MonitorRecord` and that record's `ClauseRunReport` behind an
  `Arc`: a settled report is returned at every later step
  unchanged (Finality), so each step shares it rather than cloning a
  counterexample that may list many positions.
- **`checkpoint` returns `Result<MonitorState, Failure>`.** Writing the
  state serializes the ledger, the binding and each clause state, and that
  serialization is the only step that can fail. The fault is returned as
  FR-283's internal-fault arm of `StageFailure` rather than a state that
  lacks a member; no refusal and no cancellation is possible, because
  `checkpoint` reads no input and does no evaluation.
- **`open` and `resume` take the request by value.** The session keeps the
  request for its life (clause selections, kind, binding, sources), so it
  owns `MonitorOpen<'a>`, borrowing only the package and the inputs the
  request itself borrows, for the lifetime `'a` of the session.

## Outputs

- `MonitorStep`: for each clause, in `clauses` order, its name and its
  `ClauseStatus`, `Settled(MonitorRecord, Settlement)` or
  `Pending(MonitorRecord)`, and the names of the clauses whose status changed
  at this step. The `MonitorRecord` (FR-283) holds the clause's report, which
  is `B(k)` or `B(s)` ("Batch equality"). The `Settlement` records how the
  clause settled ("Settlement" below). A status changes when the clause
  passes from `Pending` to `Settled`, or when its report's category or
  disposition changes.
- `MonitorOutcome`: FR-283's outcome over the whole trace, with the final
  ledger when the kind carries progress, and each settled clause's
  `Settlement`.
- `MonitorState`: the checkpoint, bytes in the closed JSON form
  `filament.monitor-state/1`: the package identity, the open request's clause
  selections, kind, binding and lower boundary, the number of positions seen,
  the identities and stamps of the positions a counterexample can still cite,
  each settled clause's report and each unsettled clause's FR-820 `ClauseState`
  bytes (which hold the admitted values it retains and the FR-993 content
  fingerprints of its covered samples), and the ledger in
  `filament.progress-ledger/1` form (FR-641).

## Behavior

- **Batch equality.** Write `B(k)` for the report FR-283's `monitor` gives a
  clause over the trace of the first `k` observations, of the session's kind,
  with the latest progress assertion (its `scope` and its `execution`, which
  is `open` when no assertion has been made), the latest `through` for a
  `Timed` session, and the ledger the session was opened with. After the
  `k`-th counted call ("Counted calls") of `step`, `assert_progress` or `assert_through`:
  - a clause still `Pending` SHALL report exactly `B(k)`;
  - a clause that settled at call `s <= k`, other than on a limit, SHALL
    report exactly `B(s)`, the batch report over the prefix that decided it,
    and `B(k)` SHALL have the same category and truth as `B(s)`; the reading
    that decided it is its `Settlement` ("Settlement"), never part of its
    report. A clause settled on a limit reports the limit report
    ("Settlement").
  - "Exactly" here and "member for member" in FR-820 cover every verdict
    member of the report: category, truth, disposition, counterexample or
    witness, settlement and positions. They do not cover `usage`, which
    reports the session's own incremental work, not a batch run's.
- **Usage.** Each clause report's `usage` SHALL be a closed object with
  exactly two members, `cumulative` and `step`, each one FR-109
  `ClauseRunUsage` (the admission work and the evaluation meter's charges,
  separately), the usage FR-283's batch report carries:
  `cumulative` the work the session charged that clause since it opened,
  `step` the work charged in the latest counted call. A settled clause
  charges nothing more, so its `step` is zero after the call that settled
  it. Meter limits SHALL be charged per advance: each counted call checks
  its own `step` against the limit, which bounds each step's work, and
  `retained_positions` stays the memory bound. A clause that exhausts a
  per-step limit settles `incomplete` with `trigger` `limit`, exempt as
  "Settlement" states.
  A settled verdict is sound (see "Settling"), so its category and truth are
  the same over every longer prefix; its other members are not. The batch
  counterexample of a bad prefix lists every position of the prefix it was
  run over (FSpec FR-364), and an `on each` clause's batch run may report an
  earlier activation that a later position also violates. The session
  therefore keeps the report of the deciding prefix, which names what
  decided the clause and never changes, and the batch agreement for a
  settled clause is on its category and truth over each longer prefix and on
  the whole report over its deciding prefix.
- **Counted calls.** The session SHALL count, from 1, each accepted call of
  `step`, `assert_progress` or `assert_through`. A refused call (a malformed
  or out-of-order observation, a contradiction, an admission failure) and a
  cancelled call leave the session unchanged ("Atomic steps") and are not
  counted. "The `k`-th call" in "Batch equality" and `step` in a
  `Settlement` mean the `k`-th counted call, and a clause settled at
  `finish` has `step` equal to the number of counted calls made before
  `finish`.
- **Settlement.** The session SHALL report, for a clause settled with
  `trigger` `decided` or `finish`, exactly `B(s)`, the batch report over its
  deciding prefix, with no member `B(s)` lacks: for example no
  `finite_window` and no ledger on a `Finite` trace, and the basis `B(s)`
  carries. A clause settled with `trigger` `limit` is exempt from that rule
  and from the category-and-truth agreement of "Batch equality": its report
  is the `incomplete` limit report "Retained state" gives, which no batch
  run produces. The session SHALL report the reading that decided the
  clause beside its report as its `Settlement`, so the report stays
  comparable with the batch run and the reason it is final is still shown.
  `Settlement` is the closed record:
  - `step`: `s`, the counted call at which the clause settled ("Counted
    calls"), a decimal string;
  - `trigger`: `decided` (its profile's evaluator decided it, "Settling"),
    `limit` (it settled `incomplete` on `retained_positions` or
    `retained_identities`, "Retained state") or `finish` (it settled at
    `finish`);
  - `basis`: for `decided`, the FSpec FR-243 basis of the deciding reading:
    the basis of FR-820's `decided` report for an event-position or
    fixed-sample clause (`decisive-counterexample` or `decisive-witness`),
    of FR-501's report for a finite-window clause, and of `B(s)` itself for
    an infinite-trace or timed clause; absent for `limit` and `finish`;
  - `horizon`: for `decided`, the one seen horizon of the deciding reading,
    by profile and kind:
    - event-position, over a `Finite` or `Progressed` trace, and
      fixed-sample over a `Finite` trace: `S = n - 1`, the index of the
      last position seen (FR-820 `decided`, FR-504);
    - fixed-sample over a `Progressed` trace: FR-504's seen horizon `S`,
      `min(W, through)` under `Complete { through }` and `n - 1` under
      `Incomplete`;
    - finite-window (`Timestamped`): the asserted inclusive watermark `W`
      (FR-501);
    - infinite-trace: `n - 1`;
    - timed: the `through` when `assert_through` settled the clause, and
      otherwise the newest stamp;
    written as a decimal string, except a timed horizon, which is an FSpec
    FR-642 exact-rational string; absent for `limit` and `finish`;
  - `position`: for `decided`, a decimal string read from the deciding
    reading: for a `violation` (an `Undefined` that is a violation
    included), its outcome's `where`, the failing activation's position;
    for a `success`, the least position of its FR-501 decision support; and
    `null` for a reading with no `where` and no decision support; absent
    for `limit` and `finish`;
  - `through`: for a timed clause settled by `assert_through`, that value as
    an FSpec FR-642 exact-rational string; absent otherwise.
- **Finality.** Once a clause's status is `Settled`, every later step SHALL
  report the same report for it, and the session SHALL stop evaluating that
  clause.
- **Settling.** The session SHALL settle a clause at the first call at which
  its profile's evaluator returns a decided verdict over the observations
  seen, and SHALL NOT settle it at any other call before `finish`:
  - infinite-trace: a bad prefix of a formula in FR-328's safety fragment
    (`violation`), or an `Undefined` that is a violation. A formula outside
    the fragment, an eventuality among them, is never decided by a finite
    prefix: FR-328 reports it `pending` whatever the prefix shows, so
    `Reaches` (`eventually holds(r.level = 2)`) stays `Pending` after it
    sees level 2 (FR-330-AC-2);
  - event-position and fixed-sample (closed-trace) profiles: FR-820's
    `decided` verdict, FR-504's open-scope reading of the positions seen, when
    it is definite. Its seen horizon is `S = n - 1` over a `Finite` trace and
    FR-504's `S` over a `Progressed` trace: `n - 1` for an event-position
    clause, and for a fixed-sample clause `min(W, through)` under
    `Complete { through }` and `n - 1` under `Incomplete`, so a watermark
    alone, with its silent samples, can decide it (FR-504-AC-1). It is
    definite (basis `decisive-counterexample`
    or `decisive-witness`). The closed reading `B(k)` of a short prefix is
    not decided: a bounded `eventually` whose window runs past the newest
    position is violated under false extension and may be satisfied by the
    next observation, so the clause stays `Pending` while `B(k)` reports that
    closed verdict;
  - finite-window: a verdict FR-501 reports with a decisive basis; and, when
    the retained progress scope is `Closed`, a verdict whose every
    activation's window the asserted progress has established: FR-502
    forbids reopening a closed scope, and the session refuses every
    observation after it ("Order"), so no later activation can open;
  - timed: a `violation` whose deciding window ends below the newest stamp,
    or at or below the latest `through`. Stamps are monotone and a
    decreasing stamp is refused, so once a stamp above a window's end is
    accepted no later observation can fall inside that window, and the
    window is complete without `through`; this is FR-642's reading of a
    window covered by the recording's stamps (FR-642 "Use case"), with
    `through` covering a window the stamps do not reach. A `success` of a
    timed clause settles only at `finish`, because a later timed position
    can open an activation the recording has not yet covered; an unbounded
    timed `always` stays `pending` even then (FR-642-AC-2).
  Each of these is sound: a decided verdict holds for every extension of
  the trace, so settling never reports a verdict the batch run over a longer
  prefix contradicts. The rule is not complete: a clause whose verdict is
  already fixed but which its evaluator does not detect as decided, such as
  a window atom that is true over every trace, stays `Pending`.
- **Settling at finish.** The session SHALL settle, at `finish`, every
  clause that has not settled before it.
- **Through.** The session SHALL accept `assert_through(X)` on a `Timed`
  session only, and SHALL keep `through` monotone: an `X` below the newest stamp is refused
  `timed-through-before-last-stamp` (FR-642), and one below a `through` already
  asserted is refused `timed-stamps-decrease` (FR-645), each leaving the
  session unchanged. When `through` `X` has been asserted, the session SHALL
  refuse a `Timed` observation stamped at or below `X` with `timed-stamps-decrease`,
  reading `X` as the stamp before it, and SHALL leave the session unchanged:
  `through` asserts that no further position exists up to `X` (FR-642). On any other kind it is refused `invalid-request`
  naming `through`.
- **Trace end.** `finish()` SHALL return, for each clause not yet settled,
  `B(n)` over all `n` observations, and for each settled clause its settled
  report. It ends the session and adds no closure of its own, because the
  closure of the trace is fixed by the trace: a `Finite` trace has one
  reading per profile, closed by false extension under event-position and
  fixed-sample and a prefix under infinite-trace (FR-327, FR-283 "Finite
  traces"); a `Progressed` or `Timestamped` trace is closed exactly as its
  latest progress assertion's `scope` and `execution` say, which FR-502 does
  not let a later record revise; a `Timed` trace is complete up to its
  latest `through` (FR-642). A caller that wants a closed progressed trace
  asserts progress with scope `closed` before finishing.
- **Retained state.** The two limits SHALL bound the memory of the session.
  Each clause's evaluation is FR-820's carried `ClauseState`, advanced once
  per call, and the session SHALL hold an observation's admitted value only
  while some unsettled clause's state holds it (FR-820 "Summarized
  positions"), and a position's identity only while some unsettled clause
  may still cite it (FR-820 "Identities"). A position no unsettled clause
  reads or cites is dropped. When advancing a clause would make its
  `retained()` exceed `retained_positions`, that clause SHALL settle
  `incomplete` with the limit record `{limit: "node count", value, setting:
  "monitor.retained_positions"}`; when it would make its `cited()` exceed
  `retained_identities`, that clause SHALL settle `incomplete` with
  `{limit: "node count", value, setting: "monitor.retained_identities"}`.
  In either case the session SHALL drop that clause's `ClauseState`, so
  neither count grows past its limit, and the other clauses SHALL continue. A pending infinite-trace `always` over
  an unending stream therefore stops `incomplete` at `retained_identities`
  positions instead of growing without bound; a caller who needs a longer
  run raises the setting (FR-255).
- **Order.** If an observation's stamp is below the previous one (for `Timed`)
  or at or below the ledger's watermark for its binding (for `Timestamped` and
  `Progressed`), then the session SHALL refuse it with FR-502's contradiction or
  FR-645's `timed-stamps-decrease` and SHALL keep its state unchanged; the
  caller can then continue with a correct observation. If the retained
  progress scope is `Closed`, then the session SHALL refuse every further
  observation with FR-502's contradiction on `Closure` and SHALL keep its
  state unchanged: a closed scope says the trace holds no further position.
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
| FR-643-AC-1 | For FR-330-AC-1's `Bounded`, AC-2's `Reaches` and AC-12's `always holds(c.value <= 1)` over snapshots `c.value` 0, 1, 2, 1, 0 fed one at a time in a `Finite` session, the report after step `k` equals `B(k)` for each clause still `Pending` and `B(s)` for a clause settled at step `s` (`Bounded` settles `violation` at step 2, `always holds(c.value <= 1)` at step 3, `Reaches` never), with `B(k)` of the same category and truth for each settled clause, and `finish()` returns `B(5)` for `Reaches` and the settled reports for the others. | Test (TC-1666) |
| FR-643-AC-2 | A `Timestamped` session over FR-641-AC-1's three observations with progress asserted after each reports, after the last, FR-501-AC-4's outcome; the ledger it returns equals the one the batch run returns. | Test (TC-1666) |
| FR-643-AC-3 | Once `always holds(c.value <= 1)` settles `violation` at the step of `c.value` 2, the later steps report the same report for it; a clause passes from `Pending` to `Settled` only at the step that decides it, a `Pending` clause whose report changes is named as changed with status still `Pending`, and each step names exactly the clauses whose status or report changed. | Test (TC-1666) |
| FR-643-AC-4 | A checkpoint taken after step 2 and resumed gives, for steps 3 to 5, the reports of the uninterrupted session; resuming under another package, or with another clause list, refuses `monitor-state-mismatch`. | Test (TC-1667) |
| FR-643-AC-5 | A `Timestamped` observation stamped at or below the retained watermark refuses with `contradicted_dimension` `Watermark` and leaves the session unchanged, after which a correct observation is accepted; after a progress assertion with scope `closed` is retained, a further observation is refused with `contradicted_dimension` `Closure` and leaves the session unchanged. In a separate `Finite` session with `retained_positions` 2 over levels 0, 1, 0, the event-position clause `Recent` (`always[0,9] count where holds(r.level = 2) over past[0, 3] min_count 0 <= 1`), whose window holds three positions at the third observation, settles `incomplete` naming `monitor.retained_positions` at the third observation, with `Settlement` `{step: "3", trigger: "limit"}` and no `basis`, `horizon` or `position`, while the infinite-trace `Reaches` and `Never`, which retain no value (FR-820), stay `Pending`. | Test (TC-1667) |
| FR-643-AC-6 | In a `Finite` session over samples with levels 0, 1, 2, 1, the infinite-trace `Never` (`always holds(r.level < 2)`) settles `violation` at step 3; at step 4 its report equals `B(3)`, whose counterexample lists three samples, while `B(4)` has the same category `violation` and truth `false` and a counterexample of four samples. `Reaches` (`eventually holds(r.level = 2)`) stays `Pending` at every step, reporting `B(k)` (`pending`, no truth) at step `k`, and `finish` returns `B(4)` for it. | Test (TC-2321) |
| FR-643-AC-7 | In a `Finite` session, the event-position clause `E` (`eventually[0,2] holds(r.level = 2)`) over levels 0, 1 stays `Pending` at steps 1 and 2 while `B(1)` and `B(2)` report `violation` (the closed reading), and settles at step 3 on level 2 with report exactly `B(3)` (`success`, basis `closed-scope`, no `finite_window` and no ledger) and `Settlement` `{step: "3", trigger: "decided", basis: "decisive-witness", horizon: "2", position: "2"}`; over levels 0, 1, 1 it settles at step 3, when the window `[0, 2]` is wholly observed, with report exactly `B(3)` (`violation`, basis `closed-scope`) and `Settlement` `{step: "3", trigger: "decided", basis: "decisive-counterexample", horizon: "2", position: "0"}`. A `finish` over levels 0, 1 settles it with `Settlement` `{step: "2", trigger: "finish"}` and report `B(2)`. | Test (TC-2321) |
| FR-643-AC-8 | `finish()` of a `Finite` session over FR-643-AC-1's snapshots equals FR-283's `monitor` over that trace; a `Progressed` session whose latest assertion has scope `open` and execution `closed` returns, for each unsettled clause, `B(n)` under scope `open` and execution `closed`; `MonitorSession::finish` takes no closure argument. | Test (TC-2322) |
| FR-643-AC-9 | A `Finite` session over 1,000 samples whose `level` cycles 0 to 8, with `retained_positions` 4, keeps the infinite-trace `Below` (`always holds(r.level < 9)`) and `Nine` (`eventually holds(r.level = 9)`) `Pending` to the end with no `incomplete`; `Recent` (at most one 2 in any four consecutive samples, which holds here) settles `success` at step 10, its state holding `min(k, 4)` values after step `k` until then; over levels 2, 0, 2 `Recent` settles `violation` at step 3. The checkpoint after the last sample holds no admitted value. | Test (TC-2322) |
| FR-643-AC-10 | In a `Timed` session of FR-642's `Ack` (`always (holds(r.req) implies eventually[0 ms, 200 ms] holds(r.ack))`): with `req` at 0 and `ack` at 250, `Ack` settles `violation` at position 0 at the `ack` step, the stamp 250 having passed the window `[0, 200]`; with `req` at 0 and no further observation it stays `Pending` until `assert_through(250)`, which settles `violation` at position 0 with `Settlement` `{step: "2", trigger: "decided", basis: "closed-scope", horizon: "250", position: "0", through: "250"}`; with `req` at 0, `ack` at 150 and `assert_through(300)` it stays `Pending`, and `finish` returns `pending` for it, the unbounded `always` reading positions the recording does not hold (FR-642-AC-2). The bounded `Bound` (`always[0 ms, 300 ms] (holds(r.req) implies eventually[0 ms, 200 ms] holds(r.ack))`) over `req` at 0, `ack` at 150 and `assert_through(500)` stays `Pending` until `finish`, which returns `success`. `assert_through(100)` after a stamp 150 refuses `timed-through-before-last-stamp`; `assert_through(250)` after `assert_through(300)` refuses `timed-stamps-decrease`; after `assert_through(300)`, an observation stamped 260, and one stamped 300, each refuse `timed-stamps-decrease`; each refusal leaves the session unchanged, and `assert_through` on a `Finite` session refuses `invalid-request` naming `through`. | Test (TC-2324) |
| FR-643-AC-11 | A `Finite` session with `retained_identities` 3 over levels 0, 0, 2, 0: `Never` settles `violation` at step 3 citing three positions, and at step 4 `Below` settles `incomplete` naming `monitor.retained_identities`, its state dropped, while `Never` keeps its settled report; a further observation leaves `Below`'s report unchanged. | Test (TC-2324) |
| FR-643-AC-12 | In a `Finite` session over `always holds(c.value <= 1)` and `c.value` 0, 1, 1, each report's `usage` has exactly the members `cumulative` and `step`, each one FR-109 `ClauseRunUsage` (admission work and evaluation meter charges); after each counted call `step` is that call's charge and `cumulative` is the sum of the `step` values so far; every other member equals `B(k)`. With a per-step work limit one unit below the second call's `step`, the clause settles `incomplete` with `trigger` `limit` at the second call while a first call within the limit is unaffected. | Test (TC-2628) |

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
