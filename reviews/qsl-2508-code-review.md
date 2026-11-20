---
id: SR-2026111612172400000052
title: "Code review — verification plan S3 declaration checking"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-forms/src/{lib.rs,syntax.rs,synthesis_verify.rs}, fsl-replay/src/{spine.rs,spine/refinement.rs}, fsl-semantics/src/check/{assemble.rs,check.rs,mod.rs,refusal.rs}, tests/it/{synthesis_verify_check.rs,formatter_identity.rs,main.rs,unbounded_declarations.rs,union_nodes.rs}; FR-753-AC-2"
review_set: subset
---

## Summary

Reviewed QSL-2508's Rust implementation and focused integration tests for S3 verification-plan declaration checking against FR-753-AC-2 and its FSpec FR-192 mutations. The refusal catalog cases are exercised, but the acceptance criterion's span and cycle-path guarantees are not asserted by the active tests.

Disposition pass 2 also reviewed the rebased QSL-2252 RewardEvaluator adapter changes: the shared zero evaluator is passed only at existing no-reward integration fixtures to satisfy the new S3 API parameter.

## Verdict

**CONDITIONAL** — one medium test-evidence gap remains for required refusal locations and cycle-edge retention.

## Scope

- `FR-753-AC-1` (context only): S2 plan form and span construction; parser/form tests are outside this PR's S3 slice.
- `FR-753-AC-2` (examined): S3 positive plan plus refusal cases, causes, spans and cycle edges.
- `FR-753-AC-3` through `FR-753-AC-5` (context only): package identity, emission and execution are later slices and not changed here.
- FSpec `FR-192-AC-9` and `FR-192-AC-10` (examined): exact mutation cases and expected causes/locations.
- QSL-2252 RewardEvaluator adapters (examined in disposition pass 2): four root integration files add the shared test-only zero evaluator to existing fixtures; those fixture sources contain no reward declarations.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Active S3 tests check refusal catalog codes but never assert the required source spans or cycle-edge path | tests/it/synthesis_verify_check.rs:123 |
| FND-002 | low | New Rust test assertions are not formatted by the repository's pinned rustfmt | tests/it/synthesis_verify_check.rs:80 |

The cycle test at lines 108–130 asserts only `invalid_package/definition-cycle`. Duplicate, missing claim/domain/dependency, zero-bound, and inapplicable-method cases likewise compare only `CatalogCode`. FR-753-AC-2 requires the causes and spans stated by FSpec FR-192, and FR-192-AC-10 specifically requires the cycle edges `a -> b -> a`. An attempted edge assertion was added to the ignored synthesis test at lines 67–79, where the fixture yields an unbounded-synthesis refusal rather than a cycle; that assertion cannot verify the S3 cycle behavior and would fail if the ignored test were enabled.

## Rust Review

The new checker is a bounded, iterative DFS for dependency cycles, and no panic or unbounded recursion path was found in the reviewed cycle/name logic. The test suite was inspected for assertion intent and criterion binding. The assertion defect above is the only review finding. Workspace-wide clippy, deny and aggregate tests were not run because the batch instructions require targeted checks only; no CI workflow was changed.

## Test Evidence

- Matrix binding: FR-753-AC-2 is statically tagged by the four active tests `a_plan_dependency_cycle_refuses_definition_cycle`, `an_acyclic_plan_checks`, `plan_step_name_and_method_refusals`, and `plan_reference_bound_and_method_applicability_refusals`.
- Focused test run: `cargo test --locked -p filament-spec-language --test it synthesis_verify_check::` via `locked-build.sh` with a 16 GiB address-space cap, debuginfo off and incremental off — 4 passed, 0 failed, 3 ignored, 849 filtered out.
- Formatting: workspace `cargo fmt --all -- --check` failed with formatting deltas across many files; a changed-file `rustfmt --check` also identified newly added assertion formatting in `tests/it/synthesis_verify_check.rs` (lines 80-91 and 197-224). No files were modified during review.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The active cycle test now asserts the ordered `a -> b -> a` edges. The active refusal tests assert exact spans for duplicate names, unknown claim/method/domain/dependency names, zero bounds, and inapplicable methods; the inapplicable-method cause now stores `step.method.span`. |
| FND-002 | accepted-no-change | The PR's added test assertions still differ from pinned rustfmt output. This is a low-severity style issue, and the direct project instruction says minor review findings do not block; accepting it without change for this merge. |

## Disposition pass 2 review note

At head `e687e202aa25aaef7ad3988943b4b04953cc0dcf`, the AC-2 span assertions and cycle path are present. The QSL-2252 harness adaptation in `tests/it/{formatter_identity.rs,unbounded_declarations.rs,union_nodes.rs}` passes the shared `ZeroRewardEvaluator` only to fixtures without reward declarations; `tests/it/main.rs` imports that existing test-only helper. This is a minimal API adaptation and adds no new finding. The focused `synthesis_verify_check::` filter was reported passing 4/4 with 3 ignored.
