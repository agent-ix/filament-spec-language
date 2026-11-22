---
id: SR-765
title: "Code review — QSL-2811 root evaluator dependency edge"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; Cargo.toml, src/population_fixtures.rs, examples/population_v2_fixtures.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-100", type: references }
---

## Summary

Reviewed PR #3493 at frozen head `6fcf58aedf05eb51dbaf64b49af316de48feb5e5` for QSL-2811. Removing the optional normal dependency is correctly paired with passing the existing layer-3 `RewardEvaluator` port through the fixture API; unit and example callers supply the real evaluator through the retained dev dependency.

## Verdict

**PASS** — no scoped code, Rust API, or test-intent findings. The change preserves actual fixture checking and the dependency boundary in FR-100-AC-7; no scanner or assertion changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Scope and evidence

- `Cargo.toml`: examined normal, build, and dev dependency tables and the `population-fixtures` feature.
- `src/population_fixtures.rs`: examined the public helpers, package-check call, error flow, unit-test caller updates, and deterministic output assertions.
- `examples/population_v2_fixtures.rs`: examined the external caller's concrete evaluator.
- FR-100-AC-7: examined; the root has no normal or build `fsl-eval` dependency, while dev-only use is allowed.
- ADR-011 X-8 and §7.1: examined; they state the same dependency boundary and test-only exception.
- TC-390 `no_crate_below_layer_three_depends_on_the_check_core`: examined; its root dependency-table checks remain unchanged.
- Titan evidence supplied by arch: both compile-only commands, strict layering test (2/2 with sibling), ten population-fixture tests, workspace check, and population-fixtures feature check passed. I ran no build or tests.
- `git diff --check` and scoped `rustfmt --check` were reported green by the PR author; no workflow files changed.
