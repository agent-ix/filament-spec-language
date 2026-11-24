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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | renumber now refuses (SourceMapInvariant) when a walked owner that visits a multiply-used node is not in `function_identity_at`. An FR-322 owner-join function (derived from an operation body or precondition clause) that shares a node with a declared function would make a valid package refuse. No test covers an owner-join owner. | fsl-package/src/emit/value_validity_requirements/ordinals.rs:160-170 |
| FND-011 | low | No test covers a state-clause (Use::Other) use of a shared node that the checker numbers before the function uses, which the "uses no walk reaches follow the visits" rule exists for. | fsl-package/src/emit/value_validity_requirements/ordinals.rs:258-268 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | union_nodes an_omitted_owner_leaves_a_shared_application_with_the_written_owners_record asserts occurrence 0 for b, a single expression occurrence and b's region. Without the renumbering, b keeps checker ordinal 1 and the test fails. fixture_s_regions_and_rows_follow_the_owner_walk_numbering covers the regions and the term rows. The missing state-clause case is FND-011. |
| FND-002 | fixed | An unwritten claim is now an OmittedNode with cause NoClauseNode (once), and the record names it. rows() refuses SourceMapInvariant unless the claim is in that omission set. The written-node set is built in one pass. The new test is an_unwritten_presence_claim_is_an_omitted_node_its_record_names. |
| FND-003 | fixed | Uses are keyed by declaration index and paired in source-path order. There is no leftover fill: a visit with no use refuses for an application. Literal-guard pruning can still pair a visit with a pruned use, but that now refuses rather than mis-numbering. |
| FND-004 | fixed | Mapping is Vec<(u64, Option<u64>)>. ordinal() returns None for a dropped use. rows() and append refuse unless the owner is omitted. relabel_term_data drops the rows of a dropped use. |
| FND-005 | fixed | carry_rows calls ordinals::rewalk when owner_order changes, then relabels nodes, the source map, rows (relabel_rows) and term rows in finish/emit_timed_models. Only one monitor test reaches this path. |
| FND-006 | fixed | frame_chain now carries the region of the guard's occurrence, and expected.node is computed with cited() from the new frame id. |
| FND-007 | fixed | Use::Body(usize) and Use::Measure(usize) carry the ExpressionOwner declaration index. |
| FND-008 | still-open | The linear scan, the per-visit String clone and the undrained table.visited are fixed, and renumber charges its visits to the meter. The frames::derive, rederive and rewalk walks are still not charged to the emit meter (coder caveat c). rederive and rewalk are new in this PR. |
| FND-009 | fixed | rustfmt --check reports 0 diffs in all five files at 1fecd1c037. The single_match patterns are gone. Clippy was not run. |
