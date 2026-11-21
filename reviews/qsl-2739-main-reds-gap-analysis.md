---
id: SR-2026112117074738624701
title: "Gap analysis (planless, targeted) — QSL-2739 main reds"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3348; frozen 79aa75be18b07aab639f8078caa3d4c674826ac5 over 660a550b40b068df5337155009e009299654f3b1; FR-1044 (AC-10 and Current system behavior), FR-620-AC-5, FR-620-AC-10, FR-850-AC-5, FR-850-AC-7, FR-940 sealed view, FR-489 production registry; fsl-analyze/src/analyze/document/package_run.rs; fsl-analyze/tests/it/analyze_composition_cli.rs; fsl-analyze/tests/it/analyze_request_wire.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: references
---

## Summary

Ticket: QSL-2739. This is a planless gap analysis targeted at the PR diff. The matrix was
computed with `quire matrix --scope . --format tsv` (quire 0.52.0, engine 0.62.2). The
changed criteria stay tagged. The product change has an owning requirement, and there are
no stubs or hollow oracles. One pre-existing, low-severity coverage-inflation observation
on FR-1044-AC-10 is recorded. It is not caused by this PR, and this PR does not need to fix it.

## Verdict

**CONDITIONAL**: one low, pre-existing finding. The PR's own traceability, reverse-gap and
stub checks are clean.

- **Matrix (computed).** FR-620-AC-5 is `tagged` by `analyze_request_wire.rs:103`.
  FR-620-AC-10 is `tagged` by `analyze_request_wire.rs:318,472,513`. FR-850-AC-5 is `tagged`
  by `analyze_package_selection.rs:219,601`. FR-850-AC-7 is `tagged` by `analyze_package_selection.rs:452,601`.
  FR-1044-AC-10 is `tagged` by `analyze_composition_cli.rs:866`. The diff removes no tags.
- **Oracles versus criteria.** For FR-620-AC-5, the test's "one item per checked claim in declaration
  order followed by the derived item" is met: `CanInc, Inc, Bounded, BelowThree`, then
  deadlock-freedom at index 4. Index 4 pins the exact claim count. For FR-620-AC-10, "`BelowThree`
  before `Bounded` … in that order, then the derived item" is met by the relative order,
  with the authored contract claims first per FR-620's source-order rule (lines 125-129).
  FR-489 line 55 routes `pre`/`post` to `contract-checker`, so `CanInc`/`Inc` are real checked
  claims, not stray library items. The `Early` exclusion is retained.
- **Reverse gap.** The `package_run.rs` change is owned by FR-1044 "Current system behavior"
  (the AC-10 package-handoff bullet), FR-940's sealed view (the predicate reads only
  `view.package().resolved_graph()`) and FR-940-AC-6's typed pending-family refusal. It is
  not unowned behaviour.
- **FR-850 claim-name rule.** No name is read from a node body, region or `NodeRef` path. The
  `unknown-claim` fallback is unchanged for packages with no composition (FR-850-AC-5/AC-7
  fixtures have none). The `unnamed-claim` exception exists only in FR-1044's documented
  temporary current behaviour.
- **Stubs and hollow evidence.** None. The new and modified current-behaviour tests assert exit,
  category, empty items, null `last_stage`, the single diagnostic's code, cause and
  family-bearing message, all of which the renderer actually produces. The Local control can fail on
  over-refusal.
- **Untracked tests.** `the_package_default_items_refuse_a_composition`,
  `a_package_item_naming_a_composition_is_refused` and
  `a_package_item_naming_a_local_clause_beside_a_composition_settles` carry no trace tag. They back
  non-normative current-behaviour prose, so tagging them FR-1044-AC-10 would be misleading.
  They are correctly untagged and this is not a finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-1044-AC-10 computes `tagged` from `a_composition_is_a_default_item`, which covers only the source half; the package-handoff half is backed only by the `#[ignore]`d `the_package_route_settles_the_same_items`, so the matrix overstates AC-10 coverage (pre-existing from #3287, not introduced by this diff) | fsl-analyze/tests/it/analyze_composition_cli.rs:864 |

FND-001: this does not need a fix in this PR. It is acceptable to defer it to QSL-2739's
package-consumer lane, which owns AC-10 parity. It is recorded so the `tagged` status is not
read as AC-10 being delivered.

## Coverage

- Requirements examined: FR-1044 (AC-10 and Current system behavior), FR-620-AC-5, FR-620-AC-10,
  FR-850-AC-5, FR-850-AC-7, FR-850 "Building the analysis input", FR-940 sealed view and AC-6,
  FR-489 production registry line 55.
- Plan completion: not assessed
- Semantic review: skipped as a separate fan-out. The targeted intent↔test↔code agreement above was
  done inline for the changed oracles only.
- Runtime: none. Test outcomes are pending Draco qualification and no result is claimed.

## Dispositions

Round 1, checked at PR #3348 head `b1414504a4404010a2e96fa9226fcddbe160ab3a`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `#[trace("FR-1044-AC-10")]` was removed from `a_composition_is_a_default_item`. The test body, the AC-10 text and the `#[ignore]`d parity test are unchanged. FR-1044-AC-10 now computes as untagged until the QSL-2739 package owner delivers the full criterion. That is the honest state, not a new finding. |
