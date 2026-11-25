---
id: SR-3112001
title: "code-review and rust-review of QSL-3112 FR-563 parity runner (filament-spec-language PR 3800)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; Cargo.toml; Cargo.lock; Makefile; fsl-analyze/Cargo.toml; fsl-conformance/**; fsl-jit/**; fsl-eval/src/execution.rs; fsl-eval/src/execution/coverage.rs; fsl-eval/src/value/**; fsl-replay/src/spine/run_value.rs; fsl-semantics/src/check/{claims,ir,references}.rs; xtask/**"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-563
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-567
    type: references
---

# Code review of QSL-3112 FR-563 parity runner (filament-spec-language PR 3800)

## Summary

Ticket: QSL-3112. Head d60f04a225b7cb8fc04cee8874413db37ca5c6ff; merge base equals origin/main 1df75ab47f, so `git diff origin/main...HEAD` (43 files) is the full PR. Rust lane (rust-review) folded into this file.

Earlier review: no recorded review names a head. The ticket's two reviewer comments cover codegen PR #905 only; reviews/ has no QSL-3112 artifact; the PR body says "M1/L2 resolved, no new findings" without a SHA or artifact. Commit e811f11b49 ("Fix parity review coverage gaps") implies a review of 312f22d3c5 and a disposition on e811f11b49, but nothing records it, so the whole PR diff was reviewed.

Static review only: under the lead's memory and cargo-slot orders no cargo build, check or test was run (see Verdict for the exact commands not run).

Pins: filament-eval/filament-checked 7fc24dcb, filament-contract-model 244513ea, filament-contract-runtime e09091f6 (each equals its repository's main head), and filament-contract-codegen a57e2cc7 (PR #905 head, 6 commits ahead of codegen main) all resolve on GitHub. Cargo.lock resolves one source per changed shared crate. The only duplicated git crates (quire-canonical, quire-canonical-derive, quire-rs, agent-ix-semantic-schema) are duplicated identically on main. 8250655f49 is an ancestor of the head. No test or assertion from main was lost in the merges; main's `a_failing_matrix_case_fails_the_gate_by_name` was moved and keeps all its assertions with a parity override.

TC-444: the contract-model bump dc9b220 -> 244513e changes the checked-package v2 reader (dispatch, body_location_read, budgeted_read, contract_package, operations, protocol_body) but not requirement_records.rs, which raises InvalidValue at /path_condition_frames. Moving from SourceMapInvariant on main to InvalidPackage/InvalidValue on 8250655f49 therefore looks like the reader now getting past an earlier failure to the same frame validator. On that evidence the pin changes make nothing newly red. It is a changed symptom of an existing red test. No test was run to prove this.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Merged-in main test `a_passing_run_names_the_three_parts_and_qualifies` (no test-support) runs `qualify::run` with no override, which now always calls the real JIT `parity_part`. `parity_report_part` returns only Passed or Failed, never NoBackend, so its assertion `parity == "no-backend"` cannot hold: the test is red. Its doc comment still says the FR-563 runner "is not in this repository yet". The rename in 0ca5fafab6 kept both copies, but main's behaviour is the one this PR removed; no-backend is now covered by `no_linked_backend_is_a_passing_no_backend_part`. | xtask/tests/qualify_parts.rs:613-632; xtask/src/qualify.rs:155-180 |
| FND-002 | medium | Core semantics change outside FR-563: `NodeKind::BytesLength` becomes a scalar application (`=> true`), reversing QSL-3043/#3715, which deliberately classified Bytes length as not a scalar application. Every Bytes `length` site now pushes a value claim (claims.rs:875-908) into emitted packages. No test covers it, no spec asks for it, and fsl-jit does not translate Bytes at all. It is a leftover of the jit branch's own Bytes lowering, which e811f11b49 only half removed. | fsl-semantics/src/check/claims.rs:498; fsl-semantics/src/check/references.rs:82-83 |
| FND-003 | medium | fsl-analyze dev-dep pins codegen a57e2cc7, the head of open PR #905. filament-contract-codegen disallows merge commits (squash and rebase only), so a57e2cc7 will never be on codegen main and FSL main would pin an orphan PR commit (main's previous pin 6e139bed was a codegen main merge commit). Merge #905 first, re-pin #3800 to the landed codegen commit, then merge #3800. 8250655f49 stays reachable through #3800's merge commit. | fsl-analyze/Cargo.toml:66; Cargo.lock |
| FND-004 | medium | The generated-unit loop `while admitted < inputs.count` has no discard bound. If checking refuses every draw (a checker or profile regression), `make parity`, and through it `make ci`, spins forever instead of failing. Discards are routine: AC-1 expects 1,263 per 10,000. | fsl-conformance/src/parity.rs:402-418 |
| FND-005 | medium | coverage-gap is decided on the single construct the backend names (`construct_within_coverage`), not on whether all of the entry's constructs lie in coverage as FR-563 states. A backend that names any out-of-coverage construct for a fully covered entry escapes the gap (a gate escape), and a partly uncovered entry whose named construct is covered is reported as a gap. `entry_within_coverage` exists but is unused. | fsl-conformance/src/parity.rs:629-650; fsl-eval/src/execution/coverage.rs:86-138 |
| FND-006 | medium | FR-563-AC-1 ("passes the gate over the corpus and the default generated units") is tested only over an empty synthetic corpus. The pinned complete-v1 fixture has one tool/format vector and no runtime/run vector, so the corpus half of `make parity` compares nothing and no test runs a delegating backend over a selected corpus vector. | fsl-conformance/tests/it/parity.rs:734-752 |
| FND-007 | low | `corpus_bytes_argument_is_decoded_for_selected_runtime_vector` is tagged FR-563-AC-1, but it runs an all-unsupported backend (compared 0, so an EmptyComparison finding, exit 1). It asserts the opposite of AC-1's no-finding, compared>0, exit 0. | fsl-conformance/tests/it/parity.rs:925-949 |
| FND-008 | low | Mid-run refusals beyond FR-563's list: ReferencePrepareFailed, ComparisonIncomplete, and any selected corpus vector that fails to compile, admit or decode (all mapped to UnreadableCorpus) stop the run after units have run. FR-563 says a refusal stops the run before any unit runs, and lists only empty-scope, unreadable corpus and unknown option. | fsl-conformance/src/parity.rs:243-259; fsl-conformance/src/parity.rs:507-518; fsl-conformance/src/parity.rs:1177-1208 |

## Verdict

Not mergeable as is. FND-001 is a red test brought in by the merges. FND-002 is an unrelated core semantics change to revert or justify with a test. FND-003 must be settled in the merge order with #905. FND-004 to FND-006 should be fixed in this PR. FND-007 and FND-008 are small.

The rest was reviewed with no defect found: FR-563 runner structure and ordering (reference faults first, nondeterminism before difference, record_finding cap and total), the test-support gating of `run_parity_with_reference` (compile_fail doctest plus feature control), the xtask parity CLI (unknown option exits 2 through Code::Qualification), the Makefile `ci: parity` wiring, and the fsl-jit unsafe boundary (exec.rs: signature and arity checked before transmute, Machine dropped last, unsafe denied elsewhere).

Not run (cargo forbidden by the lead's orders), exact commands:
- `cargo test -p xtask --test qualify_parts -- --exact a_passing_run_names_the_three_parts_and_qualifies` (expect FAIL, FND-001)
- `cargo test -p xtask --features test-support --test qualify_parts parity_backed`
- `cargo test -p fsl-package --lib bytes_operations_emit_and_pass_strict_checked_package_v2_admission` and `cargo test -p fsl-semantics --test it bytes_length_and_concat_lower_to_native_operations` (FND-002)
- `cargo test --test it supplied_actual_refinement_internal_list_replays_a_value` on head and on origin/main (TC-444)
- `cargo test -p fsl-conformance --features test-support --test it parity` and `cargo test -p fsl-conformance --doc`
