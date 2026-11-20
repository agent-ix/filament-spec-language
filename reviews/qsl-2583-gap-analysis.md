---
id: SR-2026111919340478491681420
title: "Gap analysis — filament-spec-language Decimal checked types"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; Cargo.toml, Cargo.lock, repository matrix, changed Decimal implementation/tests, tests/fixtures/decimal-value-types.semantic-ir.json; FR-056-AC-43, FR-056-AC-63, FR-056-AC-66, FR-056-AC-67, FR-056-AC-68, FR-056-AC-71, FR-056-AC-72"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/spec
---

## Summary

The PR implements the Decimal reader, domain propagation, and exercised cases for native Decimal parameters/results, scalar Decimal, alias narrowing, clauses, snapshots, and malformed policies. The computed matrix shows FR-056-AC-63 remains covered only by an ignored test, whose native-alias cases omit the Decimal `Amount` scenario required by that criterion.

## Verdict

**FAIL** — FR-056-AC-63 has no runnable tagged test; the matrix also reports broad pre-existing repository trace gaps.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-056-AC-63 is tagged only by an ignored test, and that test omits the required Decimal `Amount` alias case | FR-056-AC-63; fsl-semantics/tests/it/model_intake.rs::a_value_type_bound_to_a_data_native_by_target_is_that_native | correct-requirement-no-evidence |

## Coverage

Plan completion: not assessed.

Reconciliation: `quoin matrix --repo /home/peter/dev/worktrees/qsl-2583 --json` timed out after 30 seconds (exit 124, no result). The documented static fallback `quire matrix --scope /home/peter/dev/worktrees/qsl-2583 --format json` completed; no run evidence was read. Rollup from that computed matrix: 4,408 tagged, 1,232 untagged, 239 tagged-by-ignored-test, 104 method-without-symbol, and 13 retired criteria. This is a repository-wide baseline; individual unrelated rows were not enumerated in this PR-scoped artifact. The affected AC-63 binder is the ignored `model_intake.rs::a_value_type_bound_to_a_data_native_by_target_is_that_native` at line 3534. AC-43 and AC-66/67/68/71/72 have tagged binders, including the new Decimal cases.

Reverse-gap and stub review covered the changed source and test paths. Decimal parameter/result policy intake, scalar and alias walks, range narrowing, checked-domain propagation, and corresponding tests map to FR-056. No additional unowned behavior or hollow implementation was identified. The new alias walk has an unbounded recursive chain; that resource-bound issue is separately reported by code review. `quire coverage` did not return JSON within the command's 30-second execution window, and emitted existing `symbol-id-collision` diagnostics to stderr, so untracked-symbol diagnostics and the coverage census remain incomplete. No source/test execution was performed; the assigned lead owns the combined focused build.

## Scope Examined

- `spec/functional/FR-056-admit-domain-package-model-declarations.md`: AC-43, AC-63, AC-66, AC-67, AC-68, AC-71, AC-72 and the Decimal value-type rule.
- All nine changed paths in PR #3138, with reverse-gap and stub inspection focused on the changed behavior.
- Subsequent PR delta through `a6def083ab4b2322e6712ecb5c5f16a907ab01c8`: inspected the fixture's explicit required scalar return shape (`[1, 1]`, nonnullable); it is consistent with other operation-result fixtures and does not alter trace bindings or Decimal acceptance.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `amount_decimal_native_alias_has_its_policy_and_checks_equality` is an active test tagged to FR-056-AC-63. It checks the named Amount alias's policy-derived field type and evaluates `self.amount = self.amount`; the original ignored test no longer stands as the only AC-63 binder. |
