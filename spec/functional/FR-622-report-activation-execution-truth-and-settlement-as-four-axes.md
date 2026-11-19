---
id: FR-622
title: "Report activation, execution, truth and settlement as four independent outcome axes"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-516
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: traces_to
  - target: ix://agent-ix/filament-specification/FR-521
    type: depends_on
---
# FR-622: Report activation, execution, truth and settlement as four independent outcome axes

## Description

FSL SHALL report the activation state, the execution disposition, the truth
value and the settlement of a temporal outcome as four separate members of
one `axes` object, beside the outcome's single O-16 category, in every
clause-run result document (FR-267), every `monitor` item and every analyze
item that has an activation scope (FR-621).

### Decision: the four axes go into both the run and the analyze outcome

The temporal engine already derives the four values separately (FR-516's
`activation`, `execution`, `truth` and `settlement`). One category
collapses distinctions a consumer acts on differently: an obligation that
never activated and one that held, an execution that failed and a claim
that was false. Reporting them only in analyze would leave `run` and
`monitor` consumers with the collapsed view for the same clause, so every
outcome that has the four values carries them. The category stays the
outcome's one category and its exit code (FR-285); the axes never change
it.

A function run's `spine-run-result/1` (FR-100) and an analyze item with no
activation scope (`value-validity`, `operation-contract`, `schedulability`,
`probabilistic-satisfaction` and `simulation` items) carry no `axes`
member: they have no obligation that activates, so the absence of the
member is the statement that the axes do not apply.

## Inputs

A clause-run report (FR-109), a monitor item, or an analyze record (FR-281).

## Outputs

An `axes` object `{"activation", "execution", "truth", "settlement"}` on the
document or item.

## Behavior

- **Spellings.** `activation` is one of `inactive`, `unknown`, `active` and
  `unavailable`, and is never `null`. `execution` is `completed`, `failed`
  or `null` when no execution ran. `truth` is the string `true`, `false` or `pending`, or
  `null`. `settlement` is `closed-scope`, `decisive-witness`,
  `decisive-counterexample`, `unsettled`, `unavailable` or `null`. These are
  FSpec FR-521's spellings.
- **Clause runs and monitor items.** On a clause-run document `axes` is a top-level member.
  The four members are exactly the
  FR-516 result's four fields of the assessed instance; they are copied, not
  recomputed from the category.
- **Analyze items.** An analyze item's axes follow from its terminal record:

| Record | `activation` | `execution` | `truth` | `settlement` |
| --- | --- | --- | --- | --- |
| `proved` | `active` | `completed` | `true` | `closed-scope` |
| `refuted` | `active` | `completed` | `false` | `decisive-counterexample` |
| `inconclusive` | `active` | `completed` | `null` | `unsettled` |
| `incomplete` | `active` | `failed` | `null` | `unsettled` |
| `unsupported`, `declined`, `failed`, `invalid-request` | `unavailable` | `null` | `null` | `unavailable` |

  An exhaustive search closes the scope of every explored behaviour, so a
  proof's settlement is `closed-scope`; a refutation rests on the replayed
  counterexample.
- **Independence.** Changing one input that decides one axis SHALL change
  only that axis: closing or opening a trace's scope changes `settlement`
  and leaves `truth`, `execution` and the category as they were when the
  truth value is unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-622-AC-1 | A clause-run result document carries `axes` whose four members equal the FR-516 result's `activation`, `execution`, `truth` and `settlement` of the assessed instance, for an active true, an active false, an inactive and an unavailable result. | Test (TC-1582) |
| FR-622-AC-2 | A function run's `spine-run-result/1` document carries no `axes` member. | Test (TC-1582) |
| FR-622-AC-3 | Analyze items settling `proved`, `refuted`, `incomplete` and `unsupported` carry the `axes` the table gives; a `schedulability` item carries no `axes`. | Test (TC-1582) |
| FR-622-AC-4 | Two clause runs that differ only in the trace's scope closure have equal `truth`, `execution` and category and different `settlement`. | Test (TC-1582) |

## Dependencies

- FR-516 owns the four values; FSpec FR-521 owns their wire spelling.
