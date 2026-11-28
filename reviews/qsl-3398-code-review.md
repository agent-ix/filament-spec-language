---
id: SR-9476
title: "code review of QSL-3398 twin dispatch table test (FSL PR #3963)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@25173079b60e0031e1fe0c86e295ed0cf856a7de; fsl-semantics/tests/it/source_objects.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: reviews
---
# SR-9476: code review of QSL-3398 twin dispatch table test (FSL PR #3963)

## Summary

Ticket: QSL-3398. PR agent-ix/filament-spec-language#3963 at 25173079b6. Method: code-review with
the rust-review lane folded in. Only test code changed
(`the_source_and_model_twins_link_equal_tables_and_check_equal_types` and its helpers `declared`,
`tables`, `project`; `tables_receivers` removed). Cargo was not run (brief); the author's logs at
this head show fsl-semantics `source_objects::` 57 passed, fsl-eval `source_objects::` 12 passed,
fsl-package `emit::tests::source_objects::` 39 passed, clippy fsl-semantics `-D warnings` exit 0,
workspace check exit 0. `QSL-3398-baseline-twin.log` shows the twin test failing on the base.

The new test is not tautological: it asserts literal expected function names per route (S names
`ix://P/Order/size.precondition.effective`, D names `ix://P/Order/size.precondition`), compares the
bodies of every named function across routes, checks `C` (1) and `B` (7), checks that S's own
effective function for `Order.size` has `Qs`'s body and equals its one contributing clause, and that
`Sub.size`'s is `Qs2 or Qs` with `Qs2` first. The authority rewrite in `declared` is hard-coded to
`ix://tc-1408/` and `ix://test/orders/`; a miss fails the literal assertions, so it cannot pass
silently. Trace tag `FR-569-AC-5`, `TC-1408` is correct. No panic or unsafe surface beyond test
`expect`s.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `tables` sorts each table's entries by receiver name (`entries.sort()`), so the test no longer compares candidate order between S and D, which the old index-based comparison did and FR-494 makes part of the table (the reader preserves candidate array order). Only function indices needed stripping. Keep entries in table order (sorting the clause lists is fine). | fsl-semantics/tests/it/source_objects.rs:800-833 |

## Verdict

Mergeable on code grounds. The rewrite does what ruling c9e092a7 item 2 asks: compare by what each
function declares, not by index, with the one S/D difference stated. FND-001 is a low coverage loss.

## Dispositions

Round 1 reviewed head dd73650661eebaa7142ca31dedc717e95d524e1d. The production change in 205ba46972
(`clause_declaration_span`, `clause_declaration_region`, `RecordedLocation::Declaration` in
`fsl-package/src/emit.rs`) was read: it touches only `declaration` occurrences of source clause
functions whose path is a body or `effective_precondition`; model-route clause functions,
authored preconditions and check refusal loci (`region`, `declaration_region`) are unchanged. The
narrowed `fixture_s_regions_and_rows_follow_the_owner_walk_numbering` keeps containment for every
other function and asserts, with a non-zero count, that an effective precondition's uses lie
outside its new declaration region; containment cannot hold there because the body is read from
the `pre` clauses (FR-096), so the narrowing is correct and not a weakening of any other case.
Red attribution checked: the four fsl-package and one fsl-semantics failures have identical panic
sites and messages on 550adf30da in the same worktree; QSL-3300's deep chain fails the same
assertion with peak and retained equal and total 3,561,251 against 3,561,144 (+107 bytes, against
a 375,000 ceiling already exceeded on main); the 39 `it_monitor_protocol` and one
`fsl-probability-check` failures match by name and message (`SourceMapInvariant`; the
`ExactProbMaxStates` limit) another run's logs on later main 3e150e7552, which descends from
550adf30da and does not contain this PR.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `tables` no longer sorts entries; it keeps table order and sorts only each candidate's clause list. |
