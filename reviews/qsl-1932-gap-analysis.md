---
id: SR-2026111414542500000004
title: "Gap analysis of QSL-1932 shared evaluator drivers (PR #2710)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1932; PR #2710 head 97f32eea06926042177612ec7d81fb1b9b838570 vs origin/main bb909513989a; qsl-eval/src/value/{collections.rs,composite.rs}; qsl-eval/src/value/expression/evaluate.rs; source PRs #2599, #2601, #2595"
review_set: subset
---
# Gap analysis: QSL-1932 shared evaluator drivers

## Summary

Ticket: QSL-1932. Planless gap analysis; plan completion: not assessed. The
PR changes no spec and adds no acceptance criterion: it moves interpreter
code (FR-107 charges, FR-139 queries, `only` and `seq`, FR-145 exact sums,
FR-359 field reads) into `value::composite` and `value::collections` and has
the evaluator call it. The criteria those paths serve keep their existing
tagged tests, which run unchanged against the moved code; no new untagged
criterion and no production code without an owning requirement was found.

Ticket goal check, item by item: composite values (delivered, identical to
#2599's `composite.rs`); collection iteration driver (delivered, identical to
#2601's `collections.rs` except `Sequence` fields narrowed to `pub(crate)`);
scan naming of untranslated constructs (not delivered). The scan naming
lives in the `qsl-eval/src/execution.rs` (`BackendCoverage::lists_type`) and
`qsl-eval/src/execution/scan.rs` (`form_name`, `operation_of`) hunks of
#2599 and #2601. Neither `BackendCoverage` nor `execution/scan.rs` exists on
main; both come from #2595 (QSL-602), so they cannot land here. Omitting
them is correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The ticket's third deliverable (scan naming of untranslated constructs, the #2599/#2601 `execution.rs` and `execution/scan.rs` hunks) is not in this PR, and no Linear ticket records that those hunks must be carried with the QSL-602 landing; QSL-1933 (landing 3/4) does not cover them and no landing 4/4 ticket was found | qsl-eval/src/execution.rs; qsl-eval/src/execution/scan.rs |

## Verdict

The PR is mergeable as a partial slice of QSL-1932 provided FND-001 is
resolved on the tracker, not in code: either narrow QSL-1932's goal to the
two delivered items and record the scan-naming hunks (the
`form_name`/`operation_of` collection names and operator names, and the
collection-element check in `BackendCoverage::lists_type`) on the ticket
that lands `BackendCoverage` (QSL-602 / the landing 4/4 ticket), or keep
QSL-1932 open for them until that lands.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1, tracker: lead scope note on QSL-1932 (comment dd0fb018) narrows QSL-1932 to the composite and collection drivers, and the carry-over note on QSL-602 (comment e92c2f25) records that the #2599/#2601 `execution.rs`/`execution/scan.rs` hunks land with QSL-602's `BackendCoverage` landing. |
