---
id: SR-1871
title: "code review of QSL-3339 absent field after deref (FSL PR #3937)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@bd193e0cb4b95cb5122ef4cf1117788edc340bf8; fsl-semantics/src/check/state_model.rs, fsl-eval/tests/it/inherited_attributes.rs, fsl-eval/tests/it/state_model_family.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-363
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-300
    type: reviews
---
# SR-1871: code review of QSL-3339 absent field after deref (FSL PR #3937)

## Summary

Ticket: QSL-3339. PR agent-ix/filament-spec-language#3937 at bd193e0cb4, base origin/main
9c0156bcf1. Method: code-review with the rust-review lane folded in. Cargo was not run (brief).

The production change deletes the `if *derefed { return Err(mismatch(location)); }` early return
that #3866 (fa7df0494b, QSL-3236) added in the `(None, [])` arm of the StateModel `Form::Attribute`
check. `git diff fa7df0494b^ HEAD -- fsl-semantics/src/check/state_model.rs` is comment-only: the
arm is back to its pre-#3866 code. An absent field read after `deref` gets
`StateModel(UnknownField { owner, field, span })` again, or the FR-056-AC-43 untyped cause when
the field is declared with no checked type.

What #3866 was for: QSL-3236 saw three fsl-eval tests that compared the inner cause with
`IllTyped(IllTypedCause::TypeMismatch)` and fail on `UnknownField`. It changed the code to match
the tests, citing FR-056-AC-9 and FR-300-AC-2. Neither criterion fixes the Rust variant. FR-056-AC-9
says only "refuse `ill_typed` at check". FR-300-AC-2 asks for `FamilyKind::StateModel`, the
`state-model` catalog prefix and the FSpec code and cause. `UnknownField` maps to
`Code::IllTyped` / `Some("type-mismatch")` (fsl-semantics/src/check/refusal.rs:3992, 4095), so
every requirement #3866 cited holds on both variants. FR-363's "no candidate" bullet is the one
requirement that does pick between them. It SHALL keep the owner, the field spelling and the token
span, "with model-field refusals keeping their `StateModel` attribution and
`ill_typed`/`type-mismatch` pair (FR-300)". FR-363 wins because it is the only binding rule on the
cause payload, and it is consistent with FR-300 and FR-056-AC-9, not in conflict with them. #3866
also cut out a live consumer: `fsl-replay/src/spine.rs:1609-1612` enriches the CLI message for
`StateModel(UnknownField)` with "`owner` has no field `field`".

Other reads after `deref` that change refusal: a field declared with no checked type (bare native
`Text`/`Rational`/`Decimal`) now gets `untyped_cause` (`ill_typed`/`operator-ineligible`) and no
longer gets `type-mismatch`. FR-056 line 257 says "every use of it refuses
`ill_typed`/`operator-ineligible`", so this is correct. It is also the pre-#3866 behaviour. A
relationship end read through `deref` (`deref(self).order`, FR-363-AC-5) now carries
`UnknownField` and keeps the `ill_typed`/`type-mismatch` pair that
`relationship_navigation::n12_*` asserts. A history field name still takes the FR-138
`missing-name` arm ahead of this one. A declared field with no field reads (`(Some(_), [])`) is
unaffected.

Changed assertions: both are stronger than before. `state_model_family` compares the whole
`UnknownField { owner: "A", field: "missing", span }` with the span computed from the compiled
source, and keeps the catalog pair, prefix and locus checks. `inherited_attributes` checks
code, cause, variant, field spelling and `span: None`, which is correct for a programmatic
`Expression` with no source. It does not check the owner (`A`/`B`); that is acceptable because
variant_fields covers owner retention.

Repo search for other asserters of the bare variant: no other test compares a deref absent-field
refusal with `IllTyped(TypeMismatch)`. `relationship_navigation` (n08, n12), the
`state_model_family::five_negative_*` row and `tests/it/compile_command.rs:724` assert only the
code/cause pair and the span, and both variants give the same pair. `region.rs` gives
`UnknownField` no span override, so the compile_command locus is unchanged. The test the ticket
reports red, `variant_fields::unknown_record_and_model_fields_name_their_owner_and_keep_type_mismatches`,
asserts the restored form exactly.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No test reads a field declared with no checked type through `deref` (`deref(r).memo` over FR-056-AC-43's `Note`). That is the path #3866 silently moved from `operator-ineligible` to `type-mismatch`, and this PR moves it back, with nothing pinning either. FR-056-AC-43's test (`model_intake::bare_native_text_rational_and_decimal_model_members_are_admitted`) covers only `self.memo` forms. Add one row asserting `ill_typed`/`operator-ineligible` naming `Note` and `Field.typeRef` for `deref(r).memo`, in this PR or as a follow-up. | fsl-semantics/src/check/state_model.rs:299-308, spec/functional/FR-056-admit-domain-package-model-declarations.md:257-260 |

## Verdict

Mergeable. The change correctly restores the FR-363 retention behaviour, and no requirement that
#3866 cited is weakened. The edited assertions are stronger and grounded in the spec. FND-001 is a
low coverage gap and does not block the merge. The spec gaps are in SR-6441.
