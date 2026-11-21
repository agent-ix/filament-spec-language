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

## New findings (disposition pass 2)

Head `d758e24d25e4b9f5f2ae8960755914a9a18438a3`. This is a manual acceptance-criteria-to-tests check, not a full gap audit: the reviewer did not load the `quoin:gap-analysis` skill, and no matrix, coverage census or test run was performed on this head. Examined FR-056-AC-63, AC-66, AC-67, AC-68, AC-71 and AC-72 against `fsl-semantics/tests/it/decimal_value_types.rs`, the `intake.rs` unit tests and `tests/it/source_corpus.rs`.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | medium | No active test covers the FR-056-AC-63 Cash row. That row is a `target`-bound Decimal value type with no policy, admitted, with `self.cash = self.cash` refusing `ill_typed`/`operator-ineligible` (TC-3031 step 2). The only binder for it was replaced by an alias case with the opposite outcome (code-review FND-003). Either AC-63 and TC-3031 are amended to match the FCD contract, or the row is tested | FR-056-AC-63; fsl-semantics/tests/it/decimal_value_types.rs:298-334 | correct-requirement-no-evidence |
| FND-003 | medium | The FR-056-AC-72 row "a value type with `target` native Decimal carrying a policy" has no test that tells it apart: the bound test passes on the construct's `target` schema finding whatever the policy is (code-review FND-004) | FR-056-AC-72; fsl-semantics/tests/it/decimal_value_types.rs:577-593 | weak-oracle |
| FND-004 | low | FR-056-AC-71 requires every refusal to name the domain and the supplied coefficient and scale. Only one of the seven refusals is checked for those items | FR-056-AC-71; fsl-semantics/tests/it/decimal_value_types.rs:503-524 | weak-oracle |

AC-66, AC-67 and AC-68 have active binders on this head: `decimal_members_have_the_golden_policy_types`, `bounds_on_every_node_of_the_walk_narrow` and `decimal_members_check_in_clauses`, all un-ignored. AC-72's other rows are covered by `decimal_policy_defects_refuse_once`, which asserts exactly one refusal per row. The corpus test `corpus_decimal_model_and_native_parameter_cases` now stages the extension's types and runs `decimal.native`, which is expected to succeed.
