---
id: FR-645
title: "Refuse a malformed or mismatched trace with a reason, a message and a locus"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-330
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-642
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: references
---
# FR-645: Refuse a malformed or mismatched trace with a reason, a message and a locus

## Description

Every refusal of a supplied trace's shape (an empty finite trace, an empty lasso
loop, a trace kind the clause's profile does not read, disordered timed stamps)
SHALL say which rule failed, in a message a person can act on, and where: the
trace member of the request and the clause whose profile decided it. Today these
cases share `invalid_runtime_input`/`invalid-value` with no message, so a user
cannot tell an empty recording from a timed clause run over untimed snapshots.
The catalog code and cause stay `invalid_runtime_input`/`invalid-value`; this
requirement adds the fields that tell the cases apart.

## Use case

An operator runs a timed rule over a log of snapshots that carry no time stamps.
The refusal says `clause "Ack" uses profile filament.temporal.timed/v1, which
reads a timed trace; the request supplies a finite trace`, points at
`selection.temporal.trace.finite` and at the clause declaration, and the operator
adds stamps instead of guessing.

## Inputs

A trace input refused by FR-330, FR-642 or FR-643 for its shape or its fit with
the clause's profile.

## Outputs

The refusal record (FR-109's `RefusalRecord` at stage `admit`, FR-100's `refused`
outcome at stage `evaluate`) with code and cause `invalid_runtime_input`/
`invalid-value`, plus:

- `fields.reason`: one token from the table below;
- `fields.profile`: the clause's profile identity;
- `fields.expected` and `fields.supplied` for `trace-kind-mismatch`: the trace
  kinds the profile reads, as an array of FR-641 member names in the table's
  order, and the kind supplied;
- `fields.position` where one position is at fault;
- `fields.request_pointer`: when the trace came from a request file, the member
  path from `clause` to the faulty trace member (for example
  `selection.temporal.trace.lasso.loop`); absent for an in-process caller;
- `message`: the text in the table, with the placeholders filled;
- `locus`: the source region of the clause's declaration.

| `reason` | Case | Message |
|----------|------|---------|
| `empty-finite-trace` | a `finite`, `progressed` or finite `timed` trace with no position | `a <kind> trace needs at least one position` |
| `empty-lasso-loop` | a `lasso`, `model` or timed lasso with an empty loop | `a lasso needs at least one loop position` |
| `trace-kind-mismatch` | a trace kind the clause's profile does not read | `clause "<name>" uses profile <profile>, which reads a <expected> trace; the request supplies a <supplied> trace` |
| `timed-stamps-decrease` | a timed stamp below the one before it | `stamp <s> at position <i> is below the stamp <t> before it` |
| `timed-period-too-short` | a timed lasso period not positive or below the loop span | `the loop period <p> must be positive and at least the loop span <d>` |
| `timed-member-misplaced` | `through` on a lasso or `period` on a finite timed trace | `<member> is not a member of a <finite or lasso> timed trace` |
| `timed-through-before-last-stamp` | `through` below the last stamp | `through <x> is below the last stamp <t>` |
| `monitor-state-mismatch` | FR-643 `resume` against another package or request | `the monitor state was taken for another package or request` |

The profile reads these kinds (`expected`): event-position and fixed-sample
false-extension `["finite", "progressed"]`; infinite-trace `["finite", "lasso",
"model"]`; timestamped-event finite-window `["timestamped"]`; timed `["timed"]`.

## Behavior

- Each refusal above SHALL carry its `reason`, `profile`, `message` and `locus`,
  and `request_pointer` when the trace came from a request file. It SHALL keep its
  existing stage: a kind mismatch and every timed-shape check at stage `admit`
  before any position is admitted; an empty finite trace and an empty lasso loop
  at stage `evaluate`, where FR-330 already reports them.
- FR-267 SHALL write `fields` and `message` in the result document's refusal
  record, and FR-286's outcome document SHALL carry them unchanged.
- Code that decides from a refusal SHALL read `reason`, never `message`; the
  message is for people and may be reworded without changing `reason`.
- A request carrying a ledger with a trace that carries no progress keeps
  FR-502's refusal and adds `reason` `ledger-without-progress` and the message
  `a progress ledger applies only to progressed or timestamped traces`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-645-AC-1 | Through `command::run`, FR-330-AC-3's `Lasso` with an empty loop reports stage `evaluate`, `refusal`, `invalid_runtime_input`/`invalid-value`, `reason` `empty-lasso-loop`, `request_pointer` `selection.temporal.trace.lasso.loop`, a non-empty `message` and the locus of `Reaches`'s declaration. | Test (TC-1670) |
| FR-645-AC-2 | A `finite` array of zero keys reports `reason` `empty-finite-trace`, `request_pointer` `selection.temporal.trace.finite`, a message and the clause locus. | Test (TC-1670) |
| FR-645-AC-3 | `Bounded` (event-position) over a `lasso` reports stage `admit`, `reason` `trace-kind-mismatch`, `profile` the event-position identity, `expected` `["finite", "progressed"]`, `supplied` `lasso`, the table's message and the clause locus; a timestamped-event clause over `finite` reports `expected` `["timestamped"]`, `supplied` `finite`. | Test (TC-1671) |
| FR-645-AC-4 | A timed clause over `finite` snapshots reports FR-642-AC-5's refusal with the message `clause "<name>" uses profile filament.temporal.timed/v1, which reads a timed trace; the request supplies a finite trace`. | Test (TC-1671) |
| FR-645-AC-5 | The same refusals built in process carry equal records except that `request_pointer` is absent. | Test (TC-1671) |

## Dependencies

- [FR-109](FR-109-run-a-state-clause-through-the-spine.md),
  [FR-330](FR-330-run-a-temporal-clause-through-the-spine.md): the refusal
  records and stages.
- [FR-267](FR-267-write-run-results-as-native-run-result-2.md): the result
  document that writes them.
