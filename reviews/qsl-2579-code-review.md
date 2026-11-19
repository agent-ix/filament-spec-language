---
id: SR-2026111921594100000001
title: "code-review of QSL-2579 duplicate terminal consumers (FSL #3108, codegen #871)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language PR #3108 at 7851747c (Cargo.toml, Cargo.lock, fsl-package/src/checked_v2/terminal.rs, fsl-package/src/emit/tests/terminal.rs, fsl-command/src/checked_handoff/corpus.rs, tests/it/checked_handoff.rs); agent-ix/filament-contract-codegen PR #871 at ec040156 (Cargo.toml, Cargo.lock, src/routed/capability.rs, src/routed/contract_package_tests.rs, tests/it/capability_settlement.rs, spec/routed/functional/FR-019-capability-settlement.md, spec/routed/matrix/TC-030-capability-settlement.md); context: contract-ir e576f749 TerminalRow, FSpec FR-476-AC-5/6, FSL FR-124-AC-11"
review_set: subset
---

## Summary

Code review (rust-style and rust-review lane) plus a manual AC-to-test gap check of the two QSL-2579 pin-bump PRs. Ticket: QSL-2579. Review is read-only from the diffs and the ticket worktrees at the PR heads; nothing was built or run. Confidence "high" below means traced by reading or by `git merge-tree`, not reproduced by a test run.

FSL #3108 pins filament-contract-model to contract-ir e576f749, makes `read_terminals` walk `CheckedPackageV2::terminal_rows()` (IR order: model node key, then terminal node key), adds `ReadTerminal::ordinal` (IR's `TerminalRow.ordinal`, u32), changes `read_terminal` to `(package, model, ordinal)`, and drops `terminal_duplicate: None` from the two `CheckedPackageRefusal` literals. Codegen #871 drops `terminal_duplicate` from `ContractPackageRefusalProjection`, its projection function, both tests and FR-019/AC-54/TC-030, and pins contract-ir e576f749 and FSL to #3108's head.

## Examined scope

- `read_terminals`: one entry per IR terminal row, in row order, ordinal copied from the row; matches contract-ir `TerminalRow` docs ("zero-based position among the rows of `model`, in terminal node key order") and FSpec FR-476-AC-6. Correct by reading. No test covers more than one row on a model (FND-005). Scan is O(rows x nodes) (FND-006). (examined)
- `read_terminal(model, ordinal)`: lookup by the (model, ordinal) key; None when absent. Correct. (examined)
- Removed `pub(crate) read_nodes` in terminal.rs: no remaining caller (the other `read_nodes` in protocol_occurrence.rs, inventory_ceilings.rs and intake.rs are unrelated functions). (examined)
- Leftover readers of the removed field: none on either PR head outside `reviews/` history and the absence asserts (FND-008). Outside these PRs, filament-driver still constructs it (see Notes). (examined)
- Compatibility shim: none. Python: none. FSL pin lives only in Cargo.toml/Cargo.lock. Codegen's `src/core/profile.rs` mirrors the pins for emitted oracle manifests and was not updated (FND-001). (examined)
- Codegen projection removal and spec edits: consistent with IR #1076 (CheckedTerminalDuplicate and the field deleted). (examined)
- Merge order and mergeability: FSL #3108 conflicts with FSL main (FND-004); codegen pins an FSL PR-head commit (FND-002). (examined)
- Reported reds: codegen lib test `genuine_fsl_function_claim_uses_admitted_package_identity_and_reached_keys` (FND-003), fsl-semantics clippy `result_large_err`, codegen `src/kani` clippy `dead_code`. (examined)

## Verdict

Not mergeable, either PR. FSL #3108: logic is correct and matches the IR row semantics, but it conflicts with FSL main on the contract-model pin (FND-004, rebase and keep e576f749, which descends from main's dd390f51) and lacks a test of several terminals on one model node, the case the ticket exists for (FND-005). Codegen #871: the dependency-source constants in `src/core/profile.rs` still name contract-ir c7b62038 and FSL 8877b38b, so `manifest_sources_match_this_crates_manifest` fails and emitted oracle crates name a different contract-model than codegen (FND-001); the FSL pin is a PR-head commit a squash merge drops (FND-002); and the lib suite is red (FND-003). Order: merge #3108 first, then re-pin codegen Cargo.toml, Cargo.lock and profile.rs to FSL's merged main rev.

Reds not caused by the diff: the failing codegen test also fails at FSL 5fae529b7 (#3108's merge base) with the old IR pin, and the diff touches only the terminal reader and the refusal projection, which that test does not reach; it is FSL lowering drift from 8877b38b..5fae529b7. It needs its own ticket, which blocks #871 (arch files it). The fsl-semantics `result_large_err` red is unrelated: the diff touches no fsl-semantics code and the IR bump only removes a field, which shrinks the refusal type. The codegen `src/kani` `dead_code` red is unrelated: the diff touches no `src/kani` file and nothing there reads the refusal projection.

Notes outside the diff: (1) filament-driver `src/refusal.rs` constructs `terminal_duplicate: None` at lines 1504, 1648, 1722, 1761 and 1787 and pins contract-model c7b62038, FSL 8877b38b and codegen a11a66ab; QSL-2184's paired-pin rule keeps it on codegen's FSL rev, so it breaks at the same bump and the ticket missed it. (2) Three ordinals now sit side by side: `ReadTerminal.ordinal` (per model node, terminal node key order), `BareTerminal.ordinal` (per model identity, row order) and `PackageTerminal.ordinal` on the source route (per identity, alias order). FSL FR-124-AC-11 requires the terminal key to be the ordinal in terminal node key order and to be unchanged when the placements swap; the alias-order source route looks like it breaks that, and no test is tagged FR-124-AC-11. Pre-existing, so not a finding here; worth an arch check.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | codegen `src/core/profile.rs` still spells `MODEL_DEPENDENCY_SOURCE` at contract-ir c7b62038 and `FSL_REPLAY_DEPENDENCY_SOURCE`/`FSL_PACKAGE_DEPENDENCY_SOURCE` at FSL 8877b38b, while Cargo.toml moved to e576f749 and 7851747c. `manifest_sources_match_this_crates_manifest` fails, and generated function/equality oracle crates name a contract-model and FSL different from codegen's, so a scratch crate seeded with codegen's lock resolves a second filament-contract-model. Every earlier pin bump (#870, #869, #868) updated these constants. The coder's one-failing-test report omits this | src/core/profile.rs:27, src/core/profile.rs:38, src/core/profile.rs:40 |
| FND-002 | high | codegen pins fsl-replay/fsl-package (Cargo.toml and all eight FSL Cargo.lock entries) to 7851747c, #3108's PR-head commit. A squash merge of #3108 leaves that commit off FSL main, so this pin is not mergeable as is. Merge #3108 first, then re-pin Cargo.toml, Cargo.lock and the profile.rs constants to FSL's merged main rev | Cargo.toml:43-44 |
| FND-003 | medium | The codegen lib test `genuine_fsl_function_claim_uses_admitted_package_identity_and_reached_keys` is red at the PR head (assert_ne that the V1 accounting node id differs from the V2 request). Not caused by the diff: it fails at FSL 5fae529b7 with the old IR pin, and FSL drift 8877b38b..5fae529b7 (10 commits) is the cause. #871 must bump FSL to take IR e576f749, so it cannot merge green until this is fixed. Needs its own ticket that blocks #871 | src/routed/contract_package_tests.rs:3049-3190 |
| FND-004 | medium | FSL #3108 conflicts with FSL main in Cargo.toml and Cargo.lock: main moved filament-contract-model to dd390f51 after #3108's base 5fae529b7. Rebase and keep e576f749 or later (e576f749 descends from dd390f51) | Cargo.toml:30 |
| FND-005 | medium | No test exercises several terminal rows on one model node, the case QSL-2579 and FSpec FR-476-AC-5/6 exist for. The changed test asserts only a single row with `ordinal == 0` and `read_terminal(.., 0)`. Add a case with two terminals on one model node (the two-aliases-of-one-identity fixture in fsl-analyze/tests/it/terminal_two_models.rs fits) asserting two entries with the same `model`, ordinals 0 and 1 in terminal node key order, and `read_terminal(model, 1)` returning the second declaration | fsl-package/src/emit/tests/terminal.rs:222-268 |
| FND-006 | low | `read_terminals` filters every node and recomputes its structural key for each row, so it is O(rows x nodes) key lookups. Match `node.node_id == row.node` directly, or index terminal nodes once | fsl-package/src/checked_v2/terminal.rs:139-155 |
| FND-007 | low | FR-019 adds "Contract IR admits several terminal nodes per model node and no longer refuses a duplicate". "No longer" is changelog wording. The spec should state the current system (the projection's fields), not the change | spec/routed/functional/FR-019-capability-settlement.md:445-446 |
| FND-008 | low | The new `assert!(... .get("terminal_duplicate").is_none())` lines guard a member that no type can produce now that IR deleted the field. They protect nothing beyond the field list asserted above them, and they keep a dead name in the tests | src/routed/contract_package_tests.rs:522, tests/it/capability_settlement.rs:115 |
