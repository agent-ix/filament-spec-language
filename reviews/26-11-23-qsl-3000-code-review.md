---
id: SR-3632001
title: "Code review (rust-review lane) - QSL-3000 FSL pins, ClockReference arms and emit stack, PR 3632"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; Cargo.toml; Cargo.lock; fsl-package/src/emit/**; fsl-eval/src/**; fsl-replay/src/**; fsl-analyze/**; fsl-command/src/**; fsl-semantics/src/**; spec/functional/FR-062-implement-checked-family-contract.md; spec/test-cases/TC-3161-linear-guard-frame-emission.md"
review_set: subset
---

## Summary

Ticket: QSL-3000. PR 3632 head 5e2e22d4ef9a42ed803ddb7b71b1f04d09f045a2 (61 ahead, 1 behind main). Diff read via gh; no cargo run.

The recursive-owner guard in `frames.rs` `Walk::owner` is correct: `active` is checked before `visits` is incremented, `Leave` is pushed (below the body `Term`) on every successful insert, so a diamond is walked in full each time and a back-edge is skipped without counting a visit. This is the same rule as contract-ir 729a6641 `walk_owner_expression`. The pins are clean: one `filament-contract-model` copy at 729a6641, one `filament-observation` at 6e05241, `filament-mltl` at c04de7a (its own manifest pins the same observation rev), and no stale source fragment. The `fsl-replay` `test_clock_reference` restore is faithful: blob c12abe56 equals bab5b7fd4c's, and the `mod` line is back in `lib.rs`. HostAllocationFailed is wired to a refusal everywhere, with no `unreachable!` or `todo!`.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | high | Unit test still builds `ValueType::Reference(clock_reference_target())`, but `clock_reference_as_leaf` now rewrites only `ValueType::ClockReference`. The leaf rewrite is a no-op and `assert!(!Typer::holds_references(&leafed))` fails. fsl-semantics lib tests are outside the control set. | fsl-semantics/src/check/check.rs:6832 |
| FND-002 | high | `clock_place_reference` requires the node type `ValueType::Reference(clock target)`, but the checker now types `Reference<Clock>` as `ValueType::ClockReference`. Every `ref(place)` evaluation halts with `place-reference-result-type-expected`. Non-ignored integration test `source_generates_clock_references_from_ordinary_admitted_owners` exercises this path and is not in the control set. | fsl-eval/src/value/expression/evaluate.rs:1264 |
| FND-003 | medium | `claim_text` is imported but never used in the module, so `cargo clippy --workspace --all-targets -D warnings` (Makefile:61) fails on unused_imports | fsl-package/src/emit/value_validity_requirements.rs:20 |
| FND-004 | medium | Lines rustfmt would rewrite: the 114-char use list, the 106-char match arm and a hand-wrapped let-else. `cargo fmt --all -- --check` (Makefile:60) fails. | fsl-package/src/emit/value_validity_requirements.rs:20; fsl-replay/src/spine/run_value.rs:423; fsl-analyze/tests/it/analysis_view.rs:188 |
| FND-005 | medium | No test asserts that a refinement or step-relation claim writes no operation-contract row or record node (FSpec #826 decision). `relation_owned` is also not applied in `record_names()`, so the omission closure still counts these unwritten records. | fsl-package/src/emit/value_validity_requirements.rs:482-495,544,775 |
| FND-006 | medium | The rewritten operation-contract package tests assert only the item count and the absence of refusal text, with no category, record or source-route equality. The docs cite FR-940-AC-6 / FSpec FR-825-AC-6, but the attribute still traces FR-940-AC-13, which is the value-validity frame AC. | fsl-analyze/tests/it/analyze_package_provider.rs:241-275,311-326 |
| FND-007 | low | The TC-416 test doc says pinned contract-ir admission still re-walks without bound, which is stale at 729a6641. The name says "and_admission" but the test only emits (admission runs through corpus rows). Without the guard the test aborts on OOM rather than tripping its 256 MiB assert. | fsl-package/src/emit/tests/admission_corpus.rs:4224-4243 |
| FND-008 | low | FR-062-AC-17 now carries a measurement log ("measured 15,875,705 ... plus about 25% headroom") and loosens the byte bound from 8 MiB to 20 MB. This is build-log text inside a requirement. | spec/functional/FR-062-implement-checked-family-contract.md:412 |
| FND-009 | low | The PR body is stale: it says the emitter is unchanged and that a 16-error compile blocker remains | PR 3632 description |

## Verdict

Not mergeable: FND-001 and FND-002 are probable red tests outside the control set, and FND-003/FND-004 fail the Makefile lint gate. Correct as reviewed: the frames recursion guard (diamond-safe, Leave always pushed, same rule as contract-ir), the Cargo pins and lock (one copy each), the HostAllocationFailed arms in fsl-replay, fsl-analyze, fsl-command and fsl-package (each a refusal, with unit tests in fsl-replay and fsl-package), the fsl-eval ClockReference arms (explicit, no wildcard) and the restored fsl-replay test module. The operation-contract body branch in the stratified-grammar corpus test checks the seven FR-825 member names in order. Methods not run: gap-analysis and spec-review sub-analyses. The spec delta is two files, and its single issue is FND-008.
