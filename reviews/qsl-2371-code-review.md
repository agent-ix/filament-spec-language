---
id: SR-537
title: "QSL-2371 rational range owner reuse code review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2371; fsl-semantics/src/check/check.rs; fsl-semantics/src/check/facts.rs; fsl-semantics/tests/it/rational_value_types.rs; checker_rulings.rs; FR-700; FSpec FR-146"
review_set: subset
---

## Summary

Reviewed frozen 6d9ba0551384ccc2b867fa5e6ce4cdefc2101342 against origin/main, including the Rust lane. No production defect found. The former rational_result_of arithmetic implementation is removed; both callers use filament-exact's RationalDomain::result_of.

## Verdict

PASS for bounded code inspection. The parent-owned final scoped library Clippy receipt passes at frozen head 6d9ba05513 (clippy-final.log, exit 0, Finished in 44.09s). No aggregate, workspace, or whole-crate gate was run or inferred.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Examined both arithmetic inference and definedness paths, literal admission/normalization, unchanged nonzero refusal, owner implementation at existing b26a8e4 dependency, and the five named test bodies. No spec/plan change, CI weakening, new public API, unsafe, wire contract, or Tokio behavior. No applicable AssuranceProfile found in spec. No spec-review method applies.

The literal adapter maps an already normalized Rational value to point IntegerIntervals. It does not reimplement arithmetic, mutate static types, refine stable paths, or bypass literal membership. The expect follows Rational's positive normalized denominator invariant. Searches for rational_result_of and rational literal domain helpers found no remaining duplicate arithmetic or duplicate helper. Both positive-square and positive-addition precision come from the owner dependency; no manifest/pin change is needed.

Final quotient receipt: focused-quotient-green.log, Cargo exit 0, 1 passed/1827 filtered; covers four admissions and eight numerator/denominator narrowing refusals. Four FR-700 tests passed in focused-before-oracle-fix.log against f143b6d3f6; production diff f143b6d3f6..6d9ba05513 is empty. The corrected inferred-let oracle distinguishes IllTyped/type-mismatch from direct UndefinedExpression/unproved-range. Existing integration warnings and check.rs formatter drift are unchanged, per preserved baseline comparison. git diff origin/main --check passes. No duplicate cargo work was launched.

## Examined units

- FR-700-AC-1, role: examined, excerpt: "`function f using v(a: Rational[1, 9; 1, 9], b: Rational[1, 9; 1, 9]): Rational[1, 81; 1, 81] pure { a * b }` and the same with body `a * a` each check with no refusal."
- FR-700-AC-2, role: examined, excerpt: "The same functions with result type `Rational[1, 80; 1, 81]`, and with result type `Rational[1, 81; 1, 80]`, each refuse `undefined_expression`/`unproved-range` at the product."
- FR-700-AC-3, role: examined, excerpt: "`function g using v(a: Rational[1, 9; 1, 9], b: Rational[1, 9; 1, 9]): Rational[1, 162; 1, 81] pure { a + b }` checks (raw numerator `[2, 162]` over `[1, 81]`), and with result type `Rational[1, 161; 1, 81]` it refuses `undefined_expression`/`unproved-range`."
- FSpec-FR-146 literal rule, role: examined, excerpt: "A literal `rational(n,d)` has the points of its normalized `n/d`; a stable path has its declared `[n1,n2]` and `[d1,d2]`;"
- FSpec-FR-146-AC-9, role: context_only, excerpt: "Rational `+`, `-`, `*`, `/` and unary `-` range proofs use exactly the numerator and denominator interval rules"

Bindings checked: squaring_a_rational_checks_in_the_derived_range → FR-700-AC-1; a_rational_product_refuses_when_its_derived_range_does_not_fit → FR-700-AC-2; rational_products_and_sums_meet_their_normalized_intervals → FR-700-AC-1/3; narrower_rational_results_refuse_unproved_range → FR-700-AC-2/3; positive_rational_division_by_one_keeps_its_derived_bounds → FSpec-FR-146-AC-9. All inspected bindings are correct, with the last exercising only the rational range slice of the compound external criterion.

Final runtime readiness: all five named behavior tests have passing receipts, and the final local scoped Clippy command passed at 6d9ba05513: cargo clippy --locked -p fsl-semantics --features test-support --lib -j1 --no-deps -- -D warnings, shared single cargo slot, debug disabled, incremental disabled. The earlier remote request was cancelled before START acknowledgement; no duplicate lint run was inferred. The availability disposition in SR-538 is deferred under parent scope authority; repository-wide assurance remains CONDITIONAL/INCOMPLETE. Parent retains merge and artifact custody.
