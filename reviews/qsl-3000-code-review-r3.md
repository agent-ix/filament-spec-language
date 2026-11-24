---
id: SR-3632002
title: "code-review of PR 3632 commits be9c7b1958, 9a22e065ed, 6d278ad4fc (owner-walk ordinals, unwritten operation-contract claims)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; git diff cdcd828ced..6d278ad4fc: fsl-package/src/emit.rs, fsl-package/src/emit/monitor_plan.rs, fsl-package/src/emit/value_validity_requirements.rs, fsl-package/src/emit/value_validity_requirements/{frames,ordinals,tests}.rs"
review_set: subset
---

## Summary

Ticket: QSL-3000. PR filament-spec-language#3632, head 6d278ad4fc6a0be8a0a4e833cc5b1e17ce3d0df1. Only the three commits after cdcd828ced were reviewed. Earlier rounds are SR-3632001 and Linear comment 748baf6c. The diff was read from the existing worktree, read-only. No cargo build or test was run. The only command run on the code was `rustfmt --check` on single files.

The reference is FSpec origin/main 709d7e092: FR-322 "Expression occurrence ordinals", FR-825 "Shared path-condition frames" and "Emission", and FR-825-AC-11. The reader is contract-ir 729a6641, `derive_package_path_conditions` and `walk_owner_expression` in requirement_records.rs:2042-2530.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new test records_of_a_shared_application_follow_the_owner_walk passes on the old numbering, because the checker already numbers alpha 0 and beta 1. No test fails without the renumbering, the source-map region move, or relabel_term_data. | fsl-package/src/emit/value_validity_requirements/tests.rs:487-522 |
| FND-002 | high | be9c7b1958 silently drops an operation-contract record and its row when no written node has the claim's id. FR-825 Emission says one row SHALL be written for every record and none omitted. The OmittedNode names a claim that is not itself in `omitted`. In rows(), the new `continue` replaces the fail-closed `ok_or(SourceMapInvariant)` for any missing operation record. There is no test. | fsl-package/src/emit/value_validity_requirements.rs:346-351,831-837 |
| FND-003 | medium | The fill rule is a guess. If a visit has no located use in its owner (an owner missing from owner_names, such as an FR-322 owner-join function, or an exhausted pool), it takes the earliest unmatched checker ordinal, which can be a state-clause use. The pairing inside an owner is FIFO by checker ordinal, which assumes the lowering order equals the walk order with no uses skipped. The reader's literal-guard pruning breaks that assumption. Either way the moved source-map region names a different read. | fsl-package/src/emit/value_validity_requirements/ordinals.rs:170-199 |
| FND-004 | medium | A dropped use (from an omitted owner) has no mapping entry. Renumbering::ordinal and relabel_term_data therefore leave it at its old ordinal, so a checked-term row for that use keeps a stale ordinal that can collide with a renumbered one or point past the occurrence list. | fsl-package/src/emit/value_validity_requirements/ordinals.rs:45-55,282-296 |
| FND-005 | medium | carry_rows re-derives the frame table after a rekey, but it does not re-run the occurrence renumbering. Owners with no declaration entry are ordered by node id, so a rekey that changes those ids changes the walk order. Node occurrences and the records' `occurrence` keep the pre-rekey numbering, and rederive then refuses a valid package. | fsl-package/src/emit/value_validity_requirements.rs:1119-1183 |
| FND-006 | low | assert_rows_follow_the_rekey now compares only (guard, holds) chains. It no longer asserts guard occurrence ordinals, and it copies `expected.node` from the output. The reorder is justified by FR-825 canonical order, and the reader still admits final_package. The occurrence and the node id derived from the new frame id should still be asserted. | fsl-package/src/emit/monitor_plan.rs:1826-1871 |
| FND-007 | low | Use is keyed by the function name only, and the `index` of ExpressionOwner::Body/Measure is ignored, so two owners with the same name share one pool. | fsl-package/src/emit/value_validity_requirements/ordinals.rs:90-96,137-141 |
| FND-008 | low | Allocation and work. `nodes.iter().any` runs once per operation-contract record, so the cost is records × nodes. visits() clones the owner name String once per visit. `table.visited` grows on every visit in derive() and rederive() and is never drained. Up to three package walks are not charged to the emit meter. The walks are iterative, so a 100000-deep body cannot overflow the stack. | fsl-package/src/emit/value_validity_requirements.rs:351; frames.rs:219,361-400 |
| FND-009 | medium | The new code is not rustfmt-clean (ordinals.rs relabel_term_data sorts; new drift in frames.rs, value_validity_requirements.rs and monitor_plan.rs), and it has clippy single_match / for_kv_map patterns, so `make` fmt/clippy -D warnings fails. | fsl-package/src/emit/value_validity_requirements/ordinals.rs:184-189,215,248-258,322-347 |

## Verdict

NOT MERGEABLE.

Verified correct:
- owner_order matches the reader's owner order exactly: declaration source-map position (source, start, end), then node id, for functions with a body root.
- visits() matches the reader's numbering walk. It reuses Walk::owner: body before measure, derive=true pruning on the body and none on the measure, one counter per node kept across owners, the `active` back-edge rule, and Leave after the subtree.
- canonical() and rederive() order the table by owner node id, then by first touch. The frame guard occurrence is the next unconsumed ordinal, matching FR-825.
- Uses of an omitted owner are dropped only from nodes that a kept owner or context also uses, which matches a walk that never visits an unwritten owner.
- The 6d278ad4fc gate is sound for the multi-owner case. Single-owner nodes under literal-guard pruning were already a gap before these commits.
- The walks use an explicit stack with no recursion.
