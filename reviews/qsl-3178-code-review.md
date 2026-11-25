---
id: SR-967175026302197768900474181941935084826
title: "Code and Rust review of QSL-3178 AC-9 analysis tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@a23fb6fda3b5b15496914618795389f7f4d9cd3a; git diff origin/main...HEAD; fsl-analyze/tests/it/analyze.rs; fsl-analyze/tests/it/claim_family.rs; fsl-analyze/tests/it/graph_world.rs; fsl-analyze/tests/it/provider_test_support.rs; FR-281-AC-9; FR-281-AC-13; QSL-3178"
review_set: subset
date: 2026-11-25
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: reviews
---

## Summary

Ticket: QSL-3178. Final disposition review is against head `a23fb6fda3b5b15496914618795389f7f4d9cd3a` and the requested `origin/main...HEAD` scope. The additions exercise both FR-281-AC-9 clauses and the request-level FR-281-AC-13 handoff cases, including the required `family: state-model` payload on each state-clause refusal record. The duplicate provider test doubles have been consolidated into `provider_test_support.rs`.

## Scope examined

- FR-281-AC-9, clause 1: “AC-3's request returns its three records in `request_index` order 0, 1, 2.” Bound by `three_items_give_three_records_the_first_unsupported_naming_its_kind`, which asserts `[0, 1, 2]` at `fsl-analyze/tests/it/analyze.rs:156-159`.
- FR-281-AC-9, clause 2: “The same request whose binding's `universes` name a population the package does not declare returns `StageFailure::Refused(AnalyzeRefusal::Binding)` holding FR-120's `invalid_runtime_input`/`wrong-role-mapping`, no record, and no engine runs.” Bound by `analyze_refuses_an_undeclared_population_before_running_items`, which asserts the refusal and both catalog fields and observes zero provider calls at `fsl-analyze/tests/it/analyze.rs:205-214, 279-293`.
- `fsl-analyze/tests/it/claim_family.rs` and `fsl-analyze/tests/it/graph_world.rs`: inspected for behavior changes; only import ordering and formatting changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Several changed import lists and the `CONFIG_CONTRACT` assignment are not rustfmt-clean; focused `rustfmt --check` reports differences in all three changed files. The repository-wide format gate also reports numerous pre-existing differences outside this diff. | fsl-analyze/tests/it/analyze.rs:17; fsl-analyze/tests/it/claim_family.rs:13; fsl-analyze/tests/it/graph_world.rs:579 |

## Verdict

**PASS** — FND-001 and FND-002 are fixed. The scoped tests use public provider APIs, assert the relevant literal output, and share one counting provider fixture. No production behavior changed.

## Verification

- `quire matrix --scope . --format json`: completed; FR-281-AC-9 is `tagged` with both changed test functions as binders.
- `quoin matrix --repo . --json`: unavailable; native runtime refused the matrix request as exceeding the accepted size. No run evidence was read.
- `cargo fmt --all -- --check`: failed on extensive formatting differences throughout the existing workspace. The focused rustfmt check on `analyze.rs`, `claim_family.rs`, and `provider_test_support.rs` passes; `main.rs` retains unrelated pre-existing module-order drift and has only the added helper registration in this fix.
- `git diff --check origin/main...HEAD`: passed.
- Coder reported both focused tests passing on the merged head. An independent filtered run was canceled before completion while compiling dependencies; no independent test result is claimed.


## Additional scope examined (disposition pass 1)

- FR-281-AC-13, full request-level routing cases: the updated descriptor now advertises only `value-validity`/`unbounded`; the test checks wrong-family precedence over unknown backend, inconsistent manifest, and ambiguous disposition, plus the temporal routing results and no provider invocation.
- FR-281-AC-22: its matrix entry remains untagged; no test or production changes in this diff address its integer proof-bound behavior.
- Updated trace matrix: AC-9 and AC-13 are `tagged`; AC-22 is `untagged` with no binders. The four focused test filters and focused formatting/diff checks are coder-reported; I independently verified the static matrix and formatting/diff checks, but did not run Cargo tests.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The request-level AC-13 test checks the state-clause records' result, diagnostic code/cause, and absent backend, but never asserts the required diagnostic payload `family: state-model` on those records. The neighboring `analyze_items` test checks a similar payload, but does not exercise the `AnalyzeRequest` handoff cases whose output AC-13 specifies. | fsl-analyze/tests/it/claim_family.rs:797-815 |

### FND-002 disposition

Fixed on final head `a23fb6fda3b5b15496914618795389f7f4d9cd3a`. The request-level test now asserts on each of the three state-clause records:

```rust
assert_eq!(
    record
        .disposition_detail()
        .expect("state-clause refusal includes its family detail")["family"],
    "state-model"
);
```

Evidence: `fsl-analyze/tests/it/claim_family.rs:797-815`.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Focused rustfmt check on changed test files passes; pre-existing module-order drift in `main.rs` is outside this review finding. |
| FND-002 | fixed | The request-level test asserts the literal family payload for each state-clause record. |

### Disposition details

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      use std::sync::atomic::{AtomicUsize, Ordering};
      use std::sync::Arc;
      pub(crate) const CONFIG_CONTRACT: &str =
          "post Advance using v on Config::ConfigVersion::attemptUpdate { \
           (pre(self.versionNumber) < 2 and self.versionNumber = pre(self.versionNumber) + 1) or \
           (pre(self.versionNumber) = 2 and self.versionNumber = 0) }\n";
```

### Round 1 evidence

- FND-001: focused rustfmt checks pass on the changed test files; `git diff --check origin/main...HEAD` passes.
- FND-002: closed by the literal request-level payload assertion documented above.
