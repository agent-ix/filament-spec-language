---
id: SR-967175026302197768900474181941935084856
title: "Gap analysis — filament-spec-language QSL-3292 red-main tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3907 diff (fsl-analyze build.rs and three test files) against FR-196-AC-13, FR-203-AC-14, FR-282-AC-6, FR-339-AC-2, FR-1042-AC-18"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Ticket: QSL-3292. A planless gap analysis of PR #3907 at `f168d5ef4c9442fcaca04fc0710e4a8946112785`, limited to the criteria the diff touches. `quire matrix --scope . --format json` ran on the worktree; `quoin matrix` evidence was not used, because no test runs were possible. FR-196-AC-13, FR-203-AC-14, FR-282-AC-6 and FR-339-AC-2 are tagged. FR-1042-AC-18, which the PR cites to justify keeping `ProbBuild::Partial` in production, is untagged. The production `max_states` behaviour has no test of its own, while the FR-196 stop tests check a test-only wrapper.

## Verdict

**CONDITIONAL**: two medium findings and no new untagged criterion introduced by the PR. The FR-282-AC-6 wire non-conformance is a high finding, recorded in the code-review SR (FND-001 there).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-1042-AC-18 (partial-product `lower` document, `exploration: Partial`) has no tagged test. The PR's case that production keeps `Partial` correctly rests on an AC that nothing traces. | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:276; fsl-analyze/src/exact_probabilistic/build.rs:612-630 |
| FND-002 | medium | Coverage inflation. The FR-203-AC-14 and TC-631 FR-196 stop bindings in `exact_probabilistic.rs` run `build_product`, which has no production caller. The production builder's `max_states` path (`build_retaining_monitor` → `Partial`) is not what those tags verify. | fsl-analyze/src/exact_probabilistic/build.rs:45-57; fsl-analyze/tests/it/exact_probabilistic.rs:3545,3973,4010,4243 |

## Coverage

- Plan completion: not assessed
- FR-196-AC-13: tagged (3 binders, including `an_instance_undefined_at_an_initial_state_ends_at_position_zero`).
- FR-203-AC-14: tagged (9 binders: 7 in `digital_clocks.rs`, 2 in `exact_probabilistic.rs`).
- FR-282-AC-6 and FR-339-AC-2: tagged (`an_altered_certificate_is_rejected_at_the_rule_the_alteration_breaks`, `scc.rs` `tests::a_tampered_partition_is_rejected_at_its_locus`). The changed `swapping_counter_components_…` test carries only `TC-526`.
- FR-1042-AC-18: untagged.
- Reverse gap: the changes add no unowned behaviour. Every edit is traced to FR-196 or FR-203.
- Stubs: none found in the diff.
- Semantic review: not run as a separate opt-in pass. The intent judgements are in the code-review SR.
- Tests: not run. The memory gate refused the cargo run (24 GB < 30 GB).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-3331. Production does not meet FR-1042-AC-18: document construction enumerates the whole stopped graph under `max_states`. The fix is soundness-sensitive independent-checker work that this red-main PR should not take on, and the ticket names the cause, the fix and a draft test. The tag is correctly withheld until a test passes. |
| FND-002 | fixed | 1d6f3dc94d: the FR-196 and FR-203-AC-14 stop tests run `build_product`, which now returns the production builder's result (`Partial`), and they assert the prefix and its completion. `a_partial_chain_that_settles_nothing_stops_at_max_states` (fsl-probability-check) covers the production settlement. |
