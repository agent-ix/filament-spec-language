---
id: SR-2026111612172400000033
title: "Code and Rust review of the FSL main baseline gate repair (filament-spec-language PR 2885)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@282db8836beddb453c46e24e7b71b80bf9827fe6; git diff origin/main...HEAD (PR 2885, branch qsl-2392-baseline, 163 files); focus: fsl-package/src/emit/tests/{protocol_graph,owner_model}.rs, fsl-semantics/src/check/lowering.rs, fsl-semantics/src/check/refusal.rs, fsl-forms/src/dispatch.rs, fsl-package/src/{checked,checked_v2,reader_protocol_projection,bare_slot}.rs, fsl-conformance/src/run.rs, fsl-analyze/src/{analyze.rs,protocol_analysis/conformance.rs}, fsl-command/src/command{.rs,/items.rs,/wire.rs,/output/types.rs}, fsl-package/tests/source_object_identity_rows_public.rs, fsl-eval/tests/it/collection_algebra.rs"
review_set: subset
date: 2026-11-18
---

## Summary

Ticket: QSL-2392. PR: filament-spec-language#2885, head 282db8836. Method:
`/code-review` with the `/rust-review` lane, over `git diff origin/main...HEAD`
(merge base 886989583). Reviewer-only; no edits to the branch.

The diff is a mechanical gate repair: a rustfmt pass (edition 2021 import and
pattern layout over about 150 files), clippy fixes that box large `Err` and
enum payloads, explicit match arms in place of wildcards, small iterator and
struct-init refactors, a `quire-extraction` cfg on `Stage::QuireContext`, an
intra-doc link fix and a `#![cfg(feature = "std")]` on one integration test.

## Verdict

**PASS with two low findings; mergeable.** No behaviour change found.

- `lower_node` wildcard replacements (lowering.rs:4880, 4905, 4946, 5654): each
  `_` arm is now an explicit `ValueType` variant list. The compiler proves the
  match exhaustive with no wildcard, and clippy `-D warnings` (which includes
  `unreachable_patterns`) proves no listed variant duplicates an earlier arm, so
  each list is exactly the old wildcard's remainder. Same results, refusals and
  causes.
- Boxing: every boxed payload is on a type that derives no `Serialize`
  (`CheckCause`, `FormsCause`, `LinkRefusal`, `ImportViewRefusal`,
  `fsl_conformance::Report`, `ConformanceCounterexample`, `EngineReport`,
  `WalkFailure`, `ItemExecution`, the `check_outcome_inner` error tuple). JSON
  output is unchanged. The Rust signatures of these public enums do change, but
  every crate is `publish = false` and no repository under /home/peter/dev
  outside this one depends on them.
- `Stage::QuireContext` is built only by `RunCause::InvalidQuireContext`, which
  is already `#[cfg(feature = "quire-extraction")]`. With the feature on,
  `"quire-context"` serializes as before. Clippy over fsl-command with
  `--features quire-extraction --all-targets -D warnings` passes.
- `FileSelection.file` carries a documented `#[allow(dead_code)]`; the struct is
  `deny_unknown_fields`, so the field is still validated.
- fsl-package NoOwnerModel: the final diff does not contain the
  NoOwnerModel-to-ChannelCarriesBounded swap from commit dce455d5a. Merge
  bacfb87a5 took main's versions, and protocol_graph.rs and owner_model.rs
  differ from main only by rustfmt. Main's versions match FR-383-AC-27 and
  FR-880-AC-7: a non-object `over` emits with no `owner_model`, and the
  `no_owner` pair test asserts both protocols are `Emitted`. No regression is
  hidden. See FND-001 for the PR body.
- Tests: across every changed `.rs` file, the counts of `#[test]`, `#[ignore`,
  `#[trace` and `expect_err` are the same as on main. The only assert change is
  in collection_algebra.rs:1286-1287, where `assert_eq!(x, true/false)` becomes
  `assert!(x)` / `assert!(!x)`, which is the same check. `#[test]`
  `recording_assessment_maps_to_the_shared_conformance_output` moved below the
  production items in conformance.rs, with its assertions unchanged. The new
  std gate keeps `source_object_identity_rows_public` in the default (`std`)
  build. No test was deleted, weakened or newly ignored.

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` (toolchain 1.98) | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo clippy -p fsl-command --all-targets --features quire-extraction -- -D warnings` | pass |
| fsl-package lib `protocol_graph`/`owner_model` | 23 pass, 6 ignored (as on main) |
| fsl-package `conformance_c14` against filament-specification origin/main d155012a | pass. Against the local stale checkout, 35 commits behind, it fails as the PR body says. |
| fsl-package `a_non_object_over_is_written --ignored` | fails at protocol_graph.rs:2023; the ignore (QSL-2220) is still warranted |
| fsl-eval `map_source`; fsl-analyze lib `conformance`; fsl-command it (run_requirement_items, prepared_command, integer_division, compile_limits); fsl-package `source_object_identity_rows_public` | pass |
| fsl-semantics lib `lowering` | 127 pass, 1 fail. `equality_refuses_a_map_and_reads_a_bounded_quantity_as_its_quantity` fails the same way at main 886989583: #2882 gave Map structural equality and left the test stale. The PR does not touch `Lowering::equality`. This is pre-existing and not caused by this PR; it needs its own ticket. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The PR body says `a_non_object_over_is_written` was un-ignored and the NoOwnerModel tests were rewritten, but after merge bacfb87a5 HEAD carries main's versions. The test is still `#[ignore = "Blocked by QSL-2220..."]`, so FR-880-AC-7 / TC-2502 coverage stays ignored. The PR does not change that, and the description should not be used to close it. | fsl-package/src/emit/tests/protocol_graph.rs:2010-2014 |
| FND-002 | low | 11 new `#[allow(clippy::too_many_arguments)]` (not six) are added with no reason comment, on every_behavior_result, evaluate_body_value, evaluate_body, assemble_with_mode, check_sources, event, read_pending_access, access, of_rows_with_source_rows, check_with_profile_catalog_and_inputs and compose_in_mode. The repo already has 111 such allows, so this follows convention. The suppressions are still undocumented. | fsl-command/src/command.rs:2291; fsl-analyze/src/protocol_analysis/conformance.rs:741 |
