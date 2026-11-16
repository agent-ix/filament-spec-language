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

impl ClauseState {
    pub fn open(clause: &TemporalClauseNode, kind: TraceKind, binding: Option<&TraceBinding>)
        -> Result<ClauseState, ClauseRefusal>;
    pub fn advance(&mut self, position: AdmittedPosition<'_>, progress: Option<&ProgressAssertion>,
        cancel: &Cancel) -> Result<ClauseRunReport, ClauseRefusal>;
    pub fn report(&self, end: TraceClosure) -> ClauseRunReport;
    pub fn retained(&self) -> u64;
    pub fn to_bytes(&self) -> Vec<u8>;
    pub fn from_bytes(clause: &TemporalClauseNode, bytes: &[u8]) -> Result<ClauseState, ClauseRefusal>;
}
```

- The clause, the trace kind and binding the batch request would carry
  (FR-330, FR-641).
- Each position, admitted once by the caller (FR-106, FR-641), with its
  absolute trace position, stamp and order key.

## Outputs

- After each `advance`, the clause's `ClauseRunReport` over the positions
  seen, under the reading FR-327 gives a trace of that kind ending there.
- `retained()`: the number of admitted positions the state holds by value.
- `to_bytes()`: the state's bytes, read back by `from_bytes` for FR-643's
  checkpoint.

## Behavior

- **Equality.** For every prefix, the report `advance` returns after its
  last position SHALL equal FR-330's report for the same clause over that
  prefix as one trace of the same kind, members, counterexample and
  decisive record included.
- **Summarized positions.** The state SHALL hold a position by value only
  while the clause can still read that position's value: a position inside
  the window of a window aggregate of an open activation, or inside the
  horizon of a bounded operator whose residual obligation still reads it.
  The state SHALL carry the future obligations of open activations as
  residual obligations and past operators as running values, holding no
  position for either.
- **Identities.** A counterexample and a decisive record cite positions by
  identity and index (FR-330, FSpec FR-364), not by value. The state SHALL
  keep the identity of each position a reported counterexample can cite,
  and SHALL NOT count identities in `retained()`.
- **No partial state.** A refused or cancelled `advance` SHALL leave the
  state as it was before the call.
- **Bytes.** `from_bytes(to_bytes())` SHALL give a state whose later
  advances return what the original's would.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-820-AC-1 | For FR-330-AC-1's `Bounded`, FR-330-AC-2's `Reaches` and `always holds(c.value <= 1)` over `c.value` 0, 1, 2, 1, 0, the report after each `advance` equals FR-330's report over the prefix of that length, member for member. | Test (TC-2320) |
| FR-820-AC-2 | Over 1,000 sample positions whose `level` cycles 0 to 8, `retained()` stays 0 for the infinite-trace `always holds(r.level < 9)` and for `eventually holds(r.level = 9)`, and stays at most 4 for `always[0,999] count where holds(r.level = 2) over past[0, 3] min_count 0 >= 0`; the reports still equal FR-330's over each prefix. | Test (TC-2320) |
| FR-820-AC-3 | A state written by `to_bytes` after position 2 and read by `from_bytes` gives, for positions 3 and 4, the reports of the uninterrupted state; a cancelled `advance` leaves `to_bytes()` unchanged. | Test (TC-2320) |

## Dependencies

- [FR-330](FR-330-run-a-temporal-clause-through-the-spine.md): the batch
  report this form equals; [FR-327](FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md):
  the readings of a trace; [FR-194](FR-194-evaluate-window-aggregates-in-monitors.md):
  window aggregates.
- [FR-643](FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md):
  the session built on it.
- FSpec FR-114: batch and incremental evaluation agree.
