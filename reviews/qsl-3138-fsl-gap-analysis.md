---
id: SR-2026112561111362
title: "gap-analysis of QSL-3138 FSL owner-operand clause key"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3920 at 1dce281b; FR-383-AC-1, FR-383-AC-24, FR-383-AC-26, FR-383-AC-27"
review_set: subset
---

## Summary

Ticket: QSL-3138. Gap analysis scoped to the PR diff: each AC the change claims, traced to the test tagged with it. Plan completion: not assessed. The criterion-to-test check was done by hand and labeled as such. `quire matrix` was not run on this branch: read-only rule, and no build. Spec text is FR-383 at the PR base (includes FSL #3913).

## Examined scope

- FR-383-AC-1 (examined): "the clause has `owner_model` ... and `node_id.path` equal to its declaration path followed by one operand segment resolving to that target". `a_clause_names_the_object_type_its_over_input_references` asserts the literal `["Flow", {"operand": k}]` and that the row entry is the owner's graph index.
- FR-383-AC-26 (examined): "retargeting one changes its clause key". `clauses_over_different_object_types_have_distinct_owner_edges` asserts that the keys differ.
- FR-383-AC-27 (examined): a `Boolean`/`Integer` path equals the declaration path, and an owner-bearing path is the declaration path plus one operand. The stale retarget refuses `InvalidPackage`, `Some(StaleNodeKey)`, at `/semantic_graph/nodes/{i}/owner_model`, with the clause as locus. All of these are literal expectations in `protocol_owner_edges_follow_object_types_reached_through_subjects`, which is tagged TC-1063 and FR-383-AC-27. It can pass only after the pin moves (see SR-2026112521829029 FND-002).
- FR-383-AC-24 (examined): the new group test covers the sole-declared-member case; the cyclic predicate test covers the case with several declared members.

## Verdict

Coverage is close. One test-intent gap remains on the admitting half of AC-27.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The admitting half of FR-383-AC-27 ("A package whose edge, dependency and final path operand all resolve to the new target admits") is tested with a fresh emission over `M::Other`. It is not tested by retargeting the edge, the dependency and the row entry at slot `k` together on the `M::Node` package, which is the change FSpec FR-322-AC-538 describes. The fresh package also changes the `over` parameter's type and the clause body, so it does not isolate agreement of the owner key. A reader that admitted any package whose three resolve together, and refused only the mutated one, would pass either way. The minimal mutation would also catch a reader that compares more than the operand. Fix: on the stale package, also set `operand_rows[clause][k]` to Other's index and assert `AdmittedV2`. | fsl-package/tests/checked_term_protocol_rows.rs:398-411 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 94c36876: the stale package gets its row entry at slot `k`, and every index-form clause id the document holds, moved to Other's index. `assert_owner_operand_names(&moved, "Other")` then asserts `AdmittedV2`. That is the minimal three-way agreement FR-322-AC-538 describes; the fresh `M::Other` emission stays as a second check. |
