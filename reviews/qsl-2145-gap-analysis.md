---
id: SR-2026111612172400000045
title: "Gap analysis of QSL-2145 partial delivery, FR-944-AC-1 and FR-861-AC-4 (filament-spec-language PR 2936)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@2d7c94ff5d32b0c582758827aee1bfc0d957a74d; git diff origin/main...HEAD (PR 2936); FR-944 (Partial values and interval order; Completeness; Owner requirement: completeness revision; AC-1..AC-4, TC-2624), FR-861 (tracked exception QSL-2327; AC-4, TC-2442)"
review_set: subset
date: 2026-11-18
---

## Summary

Ticket: QSL-2145, a partial delivery. This is a manual AC-to-test check over
the diff's tagged tests and code. No computed matrix was used. Plan
completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new test `the_complete_through_rule_accepts_and_refuses_as_fr_944_states` is tagged FR-861-AC-4 but covers only the AC's agreement-case clause. The AC's source-scan clause ("a source scan finds it as the only FSL item comparing two completeness assertions and fails on any second one") has no test. The still-ignored `fsl_eval_calls_the_observation_owner_for_its_four_authority_rule_sets` checks only that the manifest and the module paths are named, and scans for no second comparison item. With a passing test carrying the AC-4 tag, the computed matrix will show AC-4 as backed | fsl-eval/tests/it/observation_owner.rs:22-57,69-71 |
| FND-002 | low | The `<=` boundary of the exception rule ("a position at or before the boundary that `covered` does not hold") is tested only strictly inside it (stamp 80 < 100). No case puts an uncovered position at stamp == through (refuse) or after it (accept), so a mutation of `<=` to `<` passes the TC-2442 test | fsl-eval/src/completeness.rs:45-48; fsl-eval/tests/it/observation_owner.rs:101-113 |

## Scope and evidence

Examined (clean unless listed above):
- FR-944-AC-1 (TC-2624): un-ignored, and covers the stated grid (readings 0..6,
  bounds 0..6, Exact and Counter{1}, every pair). Its independence after the
  switch is weak (SR-2026111612172400000044 FND-001).
- FR-944 "Partial values and interval order" bullets 1-3: the code matches the
  mapping for Counter/Exact and the `definitely_before`/`within_bound` read rule.
  The domain identities depart from the text (SR-...44 FND-004). The Bounded arm
  goes beyond the text and is untested (SR-...44 FND-002).
- FR-944 bullet 4 (Kleene to `PossibilitySet`, `partial::derive`/`read`): not
  delivered. It is declared out of scope in the PR body.
- FR-944 "Completeness" and the owner requirement: `completeness::revise` is
  not in v0.1.1 (verified). The QSL-2327 exception function is in place, its
  doc cites QSL-2327, and the ledger reports the `Completeness` contradiction
  exactly when the function refuses.
- FR-944-AC-2 / AC-3 / AC-4: not delivered (admission, activation, repair, and
  the owner's revise). This matches the PR body's "Not delivered" list.
- FR-861-AC-4 agreement cases: accepts 100/140 after 100 and refuses 60,
  Incomplete and the uncovered stamp 80, each case asserted by name. It also
  covers a retained Incomplete.
- FR-861 "Every existing temporal test ... SHALL pass": the focused runs are
  listed in SR-...44's Verdict.
- QSL-2145 ticket ACs (T-064 agreement tests in monitor_authority/, no second
  copy, existing temporal tests unchanged, disagreement fails): only partly
  met. The ticket correctly stays open.

## Verdict

The delivered slice is backed: FR-944-AC-1 runs (with weak independence) and
FR-861-AC-4's agreement cases are asserted. FND-001 should be fixed by
scoping the tag or adding the source scan before the AC reads as covered.
FND-002 is a cheap pair of boundary cases.
