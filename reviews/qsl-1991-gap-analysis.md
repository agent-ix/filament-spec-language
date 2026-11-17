---
id: SR-2026111518064800000005
title: "Gap analysis — QSL-1991 zero workload weights"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #2729 workload-check implementation and tests; FR-185-AC-3; FR-185-AC-10; FR-187-AC-5"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-185", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/FR-187", type: reviews }
---

## Summary

Ticket: QSL-1991. Planless, scoped gap analysis of the workload-weight change and the runtime criterion it enables. Quoin's computed matrix covered 510 requirements; FR-185-AC-10 is tagged by the two new non-ignored tests, while FR-187-AC-5 is tagged only by an ignored test. The latter test is explicitly ignored as not implemented, and the runtime code still mishandles zero weights. Plan completion was not assessed. Semantic review was skipped because it was not requested. No applicable AssuranceProfile was found.

## Verdict

**FAIL** — FR-187-AC-5 has no active test evidence and its current runtime implementation does not satisfy the zero-weight behavior.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | high | FR-187-AC-5 is covered only by an ignored test, and the current evaluator does not implement the zero-weight and all-zero fallback behavior now reachable from accepted workloads. | fsl-eval/src/simulation/model/probabilistic.rs:884-908; fsl-eval/src/simulation/model/probabilistic/tests.rs:541-551; FR-187-AC-5 |

## Coverage

- Matrix: `quoin matrix --repo /home/peter/dev/worktrees/qsl-1991-workload-weights --json` exited 0. FR-185-AC-10 is statically tagged by `workload_zero_weight_follows_spec` and `workload_omitted_operation_follows_spec`; FR-187-AC-5 is tagged only by ignored `a_weight_zero_operation_is_chosen_only_when_no_positive_one_is_enabled`. The matrix reports no run evidence for either criterion.
- Reverse-gap review: scoped to the changed S3 checker and its newly admitted workload values; the downstream evaluator is required to consume those values under FR-187-AC-5. It currently sums all enabled weights and divides by the sum, so zero-weight choices are retained and all-zero enabled choices cannot be normalized.
- Manual acceptance check: FR-185-AC-3 remains backed by named unknown-operation and negative-weight tests; FR-185-AC-10 has non-ignored tests for zero recording and omitted-operation refusal. The required runtime behavior is not active or implemented.
- Plan completion: not assessed.
- Semantic review: skipped.
- No test suites or aggregate Rust gates were run by this reviewer; focused results were supplied by the dispatching lead.

## Scope examined

- `fsl-semantics/src/check/probabilistic/model.rs:740-765, 895-915, 1240-1275`
- `fsl-semantics/tests/it/probabilistic_declarations.rs:3216-3310`
- `fsl-eval/src/simulation/model/probabilistic.rs:884-908`
- `fsl-eval/src/simulation/model/probabilistic/tests.rs:541-555`
- FR-185-AC-3, FR-185-AC-10, and FR-187-AC-5

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-187-AC-5 now has an active test covering zero-weight omission, the reset-only fallback, and all-zero equivalence; the evaluator implements those cases. |

After excerpt:

```rust
#[trace("TC-2064", "FR-187-AC-5")]
#[trace("FSpec-TC-1561", "FSpec-FR-406-AC-7")]
#[test]
fn zero_weight_workloads_follow_enabled_operation_weights() {
```

```rust
let has_positive_weight = operations.values().any(|(weight, _)| !weight.is_zero());
if has_positive_weight && weight.is_zero() { continue; }
let effective_weight = if has_positive_weight { weight.clone() } else { Rational::from_integer(Integer::one()) };
```
