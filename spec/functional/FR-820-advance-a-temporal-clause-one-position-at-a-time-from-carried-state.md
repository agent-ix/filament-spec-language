---
id: FR-820
title: "Advance a temporal clause one position at a time from carried evaluation state"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-330
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-194
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: references
  - target: ix://agent-ix/filament-specification/FR-114
    type: implements
---
# FR-820: Advance a temporal clause one position at a time from carried evaluation state

## Description

The spine's temporal evaluator (FR-330) SHALL provide an incremental form: a
carried `ClauseState` for one clause and one trace, and an operation that
advances it by one position and returns the clause's report over every
position seen so far. The state summarizes the positions already read, so
the caller does not hand the evaluator the whole prefix again and does not
keep positions the clause can no longer read. FR-643's `MonitorSession` is
built on it, and its `retained_positions` limit is a bound on the memory
this state holds.

### Decision: carried state, not re-evaluation of a stored prefix

A monitor that re-evaluates the whole prefix at every step needs every
position it has seen, so its memory and its per-step work grow with the
trace, and a monitor of an unending stream eventually fails. A temporal
formula's future obligations and its past operators are summarized by a
fixed amount of state per open activation (formula progression for the
future, one running value per past operator), so only the positions a
clause still reads by value need to be held: the positions inside a window
aggregate's window (FR-194) and inside a bounded operator's horizon whose
values the residual obligation cannot fold. The report the incremental
form returns is required to equal the batch evaluation over the same
prefix, so carrying state changes the cost, never the verdict (FSpec
FR-114).

## Inputs

```rust
pub struct ClauseState { /* opaque, serializable */ }

pub struct ClauseOpen<'a> {
    pub kind: TraceKind,
    pub lower_boundary: ProgressLowerBoundary,
    pub binding: Option<TraceBinding>,
    pub cover: Option<TimestampedCover>,
    pub ledger: Option<&'a ProgressLedger>, // re-delivery is checked against its fingerprints (FR-993)
    pub sources: &'a [Source],
    pub temporal_selections: &'a [TemporalClockSelection],
}

pub struct Advance {
    pub report: ClauseRunReport,
    pub decided: Option<ClauseRunReport>,
}

impl ClauseState {
    pub fn open(package: &CheckedPackage, clause: &TemporalClauseNode, request: &ClauseOpen<'_>)
        -> Result<ClauseState, ClauseRefusal>;
    pub fn advance(&mut self, position: AdmittedPosition<'_>, cancel: &Cancel)
        -> Result<Advance, ClauseRefusal>;
    pub fn assert_progress(&mut self, progress: &ProgressAssertion, cancel: &Cancel)
        -> Result<Advance, ClauseRefusal>;
    pub fn assert_through(&mut self, through: &ExactRational, cancel: &Cancel)
        -> Result<Advance, ClauseRefusal>;
    pub fn report(&self) -> ClauseRunReport;
    pub fn retained(&self) -> u64;
    pub fn cited(&self) -> u64;
    pub fn to_bytes(&self) -> Vec<u8>;
    pub fn from_bytes(package: &CheckedPackage, clause: &TemporalClauseNode, bytes: &[u8])
        -> Result<ClauseState, ClauseRefusal>;
}
```

- The clause and the members FR-641 gives the trace kind, exactly as the
  batch request would carry them (FR-330, FR-641, FR-283): the lower
  boundary, binding, cover, ledger, sources and temporal clock selections,
  so the report can be the batch report (a `HistoryCutoff` boundary's
  `MissingHistory`, FR-504-AC-6, and a refused condition's locus rendered
  from `sources`, FR-283, included).
- Each position, admitted once by the caller (FR-106, FR-641), with its
  absolute trace position, stamp and order key; each progress assertion of
  a `Progressed` or `Timestamped` trace; each `through` of a `Timed` trace.

## Outputs

- After each `advance`, `assert_progress` or `assert_through`, an `Advance`:
  - `report`: the clause's `ClauseRunReport` over the positions seen with the
    latest progress and `through`, under the reading FR-327 gives a trace of
    that kind ending there (the closed reading for an event-position or
    fixed-sample clause over a `Finite` trace);
  - `decided`: for an event-position or fixed-sample clause, FR-504's
    open-scope verdict over the positions seen, with seen horizon
    `S = n - 1` over a `Finite` trace and FR-504's `S` over a `Progressed`
    trace (`n - 1` for event-position; for fixed-sample `min(W, through)`
    under `Complete { through }`, reading the silent samples up to it, and
    `n - 1` under `Incomplete`), present exactly when it is definite (basis
    `decisive-counterexample` or `decisive-witness`); a `Finite` trace needs
    no binding for it. For the other profiles, `report` itself when the
    profile's evaluator reports it decided (FR-643 "Settling"), and absent
    otherwise.
- `report()`: the latest `report`, with no further position.
- `retained()`: the number of admitted positions the state holds by value.
- `cited()`: the number of position identities the state keeps for the
  counterexample it may still report.
- `to_bytes()`: the state's bytes, read back by `from_bytes` for FR-643's
  checkpoint.

## Behavior

- **Equality.** For every prefix, the state SHALL return, after the
  prefix's last call, a `report` equal to FR-330's report for the same clause over that prefix as one
  trace of the same kind with the same members and latest assertions,
  counterexample and decisive record included, and the same
  ledger, covered fingerprints included (FSpec FR-114).
- **Progress-only steps.** `assert_progress` SHALL advance the state with no
  position: a watermark alone can settle a deadline (FR-504-AC-1,
  FR-501-AC-3) and adds the fixed-sample silent samples a window reads
  (FR-504 "Silent samples"), each read as FR-504 reads it.
- **Summarized positions.** The state SHALL hold a position by value only
  while the clause can still read that position's value: a position inside
  the window of a window aggregate of an open activation, or inside the
  horizon of a bounded operator whose residual obligation still reads it.
  The state SHALL carry the future obligations of open activations as
  residual obligations and past operators as running values, holding no
  position for either.
- **Identities.** A counterexample and a decisive record cite positions by
  identity and index (FR-330, FSpec FR-364), not by value. The state SHALL
  keep the identity of each position a counterexample it may still report
  can cite, count them in `cited()` and not in `retained()`, and drop them
  once no report can cite them (when the clause settles with its report
  already built, or when no open activation's counterexample reaches them).
  A record sample is cited by its index, and no report or ledger spells a
  sample's text or value.
- **Content fingerprints.** The state SHALL record and compare each covered
  record sample's content fingerprint (FR-993) in its ledger entries exactly
  as batch evaluation does (FR-502): a position at an index already covered
  in `ClauseOpen.ledger` or in the state's own ledger, with a different
  fingerprint, is refused `Completeness`. The fingerprint is integrity data,
  not an identity (ADR-013 §2); it is never in a report's verdict members,
  and it is counted in the ledger's `progress.covered_positions` bound.
- **No partial state.** A refused or cancelled call SHALL leave the state as
  it was before the call.
- **Bytes.** `from_bytes(to_bytes())` SHALL give a state whose later calls
  return what the original's would. `from_bytes` restores the content
  fingerprints, so a restored state still refuses a changed sample at a
  covered index.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-820-AC-1 | For FR-330-AC-1's `Bounded`, FR-330-AC-2's `Reaches` and `always holds(c.value <= 1)` over `c.value` 0, 1, 2, 1, 0, the report after each `advance` equals FR-330's report over the prefix of that length, member for member, for every verdict member (category, truth, disposition, counterexample or witness, settlement and positions); `usage` is the session's own work, as FR-643 "Usage" states, and is not compared. | Test (TC-2320) |
| FR-820-AC-2 | Over 1,000 sample positions whose `level` cycles 0 to 8, `retained()` stays 0 for the infinite-trace `always holds(r.level < 9)` and for `eventually holds(r.level = 9)`, and is exactly `min(k, 4)` after position `k` for `always[0,999] count where holds(r.level = 2) over past[0, 3] min_count 0 <= 1` while its activation is open; the reports still equal FR-330's over each prefix, the window clause's `success`; over levels 2, 0, 2 the window clause's report is `violation` at position 2, equal to FR-330's. | Test (TC-2320) |
| FR-820-AC-3 | A state written by `to_bytes` after position 2 and read by `from_bytes` gives, for positions 3 and 4, the reports of the uninterrupted state; a cancelled `advance` leaves `to_bytes()` unchanged. | Test (TC-2320) |
| FR-820-AC-4 | For `E` (`eventually[0,2] holds(r.level = 2)`, event-position) over a `Finite` trace, `advance` over levels 0, 1 returns `report` `violation` (the closed reading) and no `decided`, and over level 2 next returns `decided` `success` with basis `decisive-witness`; a fixed-sample `Progressed` state after `assert_progress` with watermark 3 and no new position returns FR-504-AC-1's `Completed(false)` report; a `HistoryCutoff` lower boundary given to `open` gives FR-504-AC-6's `MissingHistory` report. | Test (TC-2320) |

## Dependencies

- [FR-330](FR-330-run-a-temporal-clause-through-the-spine.md): the batch
  report this form equals; [FR-327](FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md):
  the readings of a trace; [FR-194](FR-194-evaluate-window-aggregates-in-monitors.md):
  window aggregates.
- [FR-643](FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md):
  the session built on it.
- FSpec FR-114: batch and incremental evaluation agree.
