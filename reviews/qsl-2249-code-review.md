---
id: SR-2026111518064800000004
title: "code-review review of PR2726 QSL-2249 tests and fixture comment"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-package/src/emit/tests.rs, fsl-package/src/emit/tests/collection_if.rs, fsl-semantics/src/check/union_fixture.rs, fsl-semantics/tests/it/state_clauses.rs, fsl-semantics/tests/it/variant_fields.rs; PR2726 b3a8fd5effb5d01a1082c0f86a1a069424d87876"
review_set: subset
---

## Summary

Ticket: QSL-2249. Code-review with the rust-review lane over the four new tagged tests and the union_fixture.rs comment correction. No production code changed (union_fixture is `#[cfg(any(test, feature = "test-support"))]`), so no panic/unsafe/integer-conversion surface; quoin gap-analysis was skipped for that reason and a manual AC-to-test check is recorded here. Trace attributes are plain `#[trace(...)]` in the order each TC prescribes. Tests were not re-run by the reviewer (a Kani job held the machine; one cargo job at a time); coder reports make ci green.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-093-AC-65's clause "the unguarded `let q = value(o) in q` refuses undefined_expression/unproved-presence (AC-15)" is asserted by no test: the tagged test checks only the guarded form, TC-1423 omits it, and FR-093-AC-15's tests cover `if a then value(o) else 0`, not the let form. | fsl-package/src/emit/tests/collection_if.rs:148-178 |
| FND-002 | low | The AC-12 test checks the missing-`Empty` arm with `any(cause == "unproved-exhaustiveness")`: it does not assert the code `undefined_expression` the AC names (TC-1009 says assertions compare typed codes), and it tolerates extra refusals. | fsl-semantics/tests/it/variant_fields.rs:306-312 |
| FND-003 | low | The AC-64 operand case asserts only that the forall's collection operand is an `if` of arity 3, not that it is "over `c`, `s` and `t`" as the AC states. | fsl-package/src/emit/tests/collection_if.rs:136-142 |
| FND-004 | low | The stale claim the PR corrects in union_fixture.rs survives in the module's declaration comment: fsl-semantics/src/check/mod.rs:124-125 still reads "while no checked package can hold a union (FR-319)". | fsl-semantics/src/check/mod.rs:124-125 |

## Scope and evidence

- FR-360-AC-12 / TC-1009 step 5 test (`a_payload_variant_parameter_is_a_union_scrutinee_and_a_nullary_one_is_not`): first clause asserts zero refusals (check passes); third asserts exactly one refusal, `IllTyped(TypeMismatch)`, at path `[0]` (scrutinee). Second clause is FND-002. Not tautological.
- FR-093-AC-64 / TC-1422 test: forall body asserted `integer.le` over a 3-argument `control.if`; exists body asserted `control.if`; invariant over `self.history` asserted `control.if` body; each read back `Verified` via the `emitted` helper with `omitted == []`. Operand case is FND-003.
- FR-093-AC-65 / TC-1423 test: if-node count 1, condition `option.present`, then-branch `control.let` binding `option.value`, `Verified`. Unguarded clause is FND-001.
- FR-104-AC-17 / TC-459 step 8 test: asserts each of the four clauses refuses exactly `[(IllTyped, type-mismatch)]` and `exists(x in p: x > 0)` checks. Faithful to the AC as written; the AC itself is the subject of SR-2026111518064800000005 FND-001 (contradicts FR-103).
- union_fixture.rs comment: "A checked package can hold a source union (FR-319, FR-320)" is accurate (source unions lower and emit; fsl-package/src/emit/tests/union_nodes.rs, source_union_rows.rs). FND-004 is the leftover copy.

## Verdict

Tests are real and mostly assert every AC clause; FND-001 is a genuinely untested AC clause, the rest are nits. Code-side mergeability is gated by the spec-review findings.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The parameter-presence fix changes every presence-optional operation parameter's type (T to Option<T>) for all downstream consumers (S4 emission of parameter nodes, IR v2 read-back, fsl-eval invocation binding, fsl-replay argument decoding, CG), yet the only new tests are S3 check tests; no test emits and reads back, or replays, a clause over an optional parameter. CG's own fixture (filament-contract-codegen tests/state_frame_support/native_twin.rs:981-1000, `[0, 1]` optional) will now see Option of a set on its next FSL bump. | fsl-semantics/src/check/assemble.rs:2907-2912 |
| FND-006 | low | Operation-redefinition conformance compares parameter records' value_type and multiplicity only (fsl-semantics/src/model/conformance.rs:355-395), so a redefining operation can now turn a required parameter T into Option<T> with no refusal. Field redefinition ignores presence too, so this follows the existing field rule, but the gap is newly reachable for parameters. | fsl-semantics/src/model/conformance.rs:355-395 |
| FND-007 | low | The FR-104-AC-17 `[0, 1]` assertion (`value(p) > 0` refuses type-mismatch) also passed before the fix (a bare bounded set refuses the same way), so it does not show the "Option of a collection" clause; `present(p)` at `[0, 1]` checking would. | fsl-semantics/tests/it/state_clauses.rs:5901-5904 |
| FND-008 | low | TC-1423 step 2 expects the refusal "at `value(o)`", but `an_unguarded_let_of_an_option_value_refuses_unproved_presence` asserts no location. | fsl-package/src/emit/tests/collection_if.rs:171-199 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | New test an_unguarded_let_of_an_option_value_refuses_unproved_presence asserts exactly [(UndefinedExpression, unproved-presence)] for the unguarded let; FR-093-AC-65 now spells the `unguarded` function and TC-1423 has step 2. |
| FND-002 | fixed | The missing-arm assertion now compares the full (code, cause) list to exactly [(UndefinedExpression, unproved-exhaustiveness)], and the nullary assertion also checks Code::IllTyped. |
| FND-003 | fixed | The operand test now asserts the three if arguments reference c, s and t. |
| FND-004 | fixed | fsl-semantics/src/check/mod.rs:124-125 now reads "hand-built rather than compiled from a checked package". |
