---
id: SR-3594002
title: "Integrity review of FR-204-AC-18 and TC-3204 (filament-spec-language PR 3594)"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language@505fbf2d161f07d040d647541f69434cbb4c94dc; spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md (FR-204-AC-18, Worked example); spec/test-cases/TC-3204-given-delays-race-under-unequal-weights.md"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-3204
    type: references
---

# Integrity review of FR-204-AC-18 and TC-3204 (PR 3594)

## Summary

Ticket: QSL-2884. This review checks completeness, consistency, atomicity and
traceability for the new AC row, the worked-example section and TC-3204.

Examined, clean:

- **IDs.** FR-204-AC-18 continues the AC sequence after AC-17. TC-3204 comes
  after TC-3203. Neither id is used on origin/main or on any other remote
  branch.
- **TC-3204 convention.** It follows TC-2176: frontmatter `id`, `title`,
  `type: TC` and a `verifies` relationship to FR-204; the sections
  Description, `Scope: FR-204-AC-18.`, Test Procedure (naming
  `fsl-analyze/tests/it/digital_clocks.rs`
  `given_delays_race_under_unequal_weights`) and Expected Results; and the
  trace tag `#[trace("TC-3204", "FR-204-AC-18")]`. The AC's Verification cell
  reads `Test (TC-3204)`.
- **Validation.** `quire validate --scope .` on both files exits 0 with no
  output.
- **Self-sufficiency.** The AC row states every checked value itself: the four
  edge masses, the edge label, the absent actions, the four claims with their
  thresholds and exact values, and the two rejected certificates' `b` values
  with their rule and locus. It names the model `Trio` and inlines its
  delays and weights. It relies on the section in the same file only for the
  guards, invariant and effects, as other FR-204 ACs rely on a named model
  (`Retx`).
- **Consistency.** Every number in the worked example matches the AC (see
  SR-3594001).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | FR-204-AC-18 bundles three behaviours in one row: the race-edge structure of the initial state, the four exact verdicts, and the two certificate rejections. FR-204's existing rows (AC-6, AC-10) are just as bundled, and every conjunct is pinned to an exact value, so this does not block. A failing test would still not say which behaviour broke. Fix, optional: split into AC-18 (edges), AC-19 (verdicts) and AC-20 (certificates), all under TC-3204. | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:253 | wrong-requirement |
| FND-002 | low | AC-18 says the initial state has "four race edges" and excludes only `FreeDelays` and `WorkloadResolve`. It does not exclude a spurious `UnitDelay` or `EnterIdleDelay` action. FR-204 says a delay no distribution gives is the scheduler's action, so an all-given state has none. A product that also offered `UnitDelay` there would satisfy the row as written. Fix: write "exactly four race edges and no other action (no `UnitDelay`, `FreeDelays` or `WorkloadResolve`)". | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:253 | wrong-requirement |

## Verdict

The structure is sound and the conventions are followed. The two low findings
tighten atomicity and the exclusivity of the action set. Neither blocks the
merge on its own.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-003 | low | TC-3204's Expected Results was not updated with AC-18's fix. It still reads "Four race edges at the initial state … no `FreeDelays`, no `WorkloadResolve`". AC-18 now requires "exactly four race edges and no other action (no `UnitDelay`, …)", and TC-3205 already reads "Exactly three edges, no other action". Fix: change it to "Exactly four race edges and no other action (no `UnitDelay`, `FreeDelays` or `WorkloadResolve`)". | spec/test-cases/TC-3204-given-delays-race-under-unequal-weights.md:25 | correct-requirement-no-evidence |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | AC-18 is not split. That is acceptable: FR-204's existing rows (AC-6, AC-10) are bundled the same way, every conjunct is pinned to an exact value, and the finding was marked optional. AC-19 and AC-20 follow the same pattern. |
| FND-002 | fixed | b84df6c5: AC-18 reads "exactly four race edges and no other action (no `UnitDelay`, `FreeDelays` or `WorkloadResolve`)". TC-3204 lags behind this text, recorded as FND-003. |
| FND-003 | fixed | 36c0aec2: TC-3204's Expected Results now reads "Exactly four race edges and no other action (no `UnitDelay`, `FreeDelays` or `WorkloadResolve`) at the initial state". |
