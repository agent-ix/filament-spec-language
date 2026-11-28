---
id: SR-6441
title: "gap analysis of QSL-3339 absent field after deref (FSL PR #3937)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@bd193e0cb4b95cb5122ef4cf1117788edc340bf8; spec/functional/FR-363-type-navigation-over-a-relationship-end.md, spec/functional/FR-300-check-model-forms-through-the-state-model-family.md, spec/functional/FR-056-admit-domain-package-model-declarations.md, fsl-semantics/src/check/state_model.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-363
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-300
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-056
    type: reviews
---
# SR-6441: gap analysis of QSL-3339 absent field after deref (FSL PR #3937)

## Summary

Ticket: QSL-3339. This run is planless and scoped to the criteria that own the changed arm:
FR-363-AC-2, FR-363-AC-5, FR-300-AC-2, FR-056-AC-9 and FR-056-AC-43.

`quire matrix --scope . --format json` was computed at bd193e0cb4. All five criteria are
`tagged`:

- FR-363-AC-2: relationship_navigation n08 and far-end role.
- FR-363-AC-5: n12 and n13.
- FR-300-AC-2: inherited_attributes x2 and state_model_family x3.
- FR-056-AC-9: compile_command:724 and others.
- FR-056-AC-43: model_intake:3484.

The PR adds no code without an owning requirement, and no stubs. Semantic review was not opted
into and was skipped. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-363's retention sentence has no acceptance criterion: "When a record or model field is absent, the refusal SHALL retain the declared owner's qualified type name, the authored field spelling and the exact authored field-token span, with model-field refusals keeping their `StateModel` attribution and `ill_typed`/`type-mismatch` pair". FR-363-AC-2 tests only the code/cause pair. The one test that asserts retention, `variant_fields::unknown_record_and_model_fields_name_their_owner_and_keep_type_mismatches`, has no `#[trace]` (doc comment "QSL-2009" only). The matrix therefore could not show that #3866 removed specified behaviour: its traced filters stayed green while this untraced test went red (QSL-3339). Needs a ticket: add an FR-363 AC row for retention (record and model, after `deref` and through `r.name`) and trace the variant_fields test to it. | spec/functional/FR-363-type-navigation-over-a-relationship-end.md:80-83, fsl-semantics/tests/it/variant_fields.rs:950-952 |
| FND-002 | medium | FR-363 asks for the owner's "qualified type name". The StateModel arm builds the owner from the last `/` segment of the effective node (`Account`), while the record arm gives `Config::Settings`. variant_fields pins the unqualified `"Account"`, and state_model_family pins `"A"`. This predates the PR and was not introduced here. Spec and code disagree on what qualified means for a model object type: the alias-qualified `Config::Account`, or the package node. Needs a ticket to pick one and align the code, the tests and FR-363's wording. It does not block this PR. | fsl-semantics/src/check/state_model.rs:310-319, fsl-semantics/tests/it/variant_fields.rs:993-994, spec/functional/FR-363-type-navigation-over-a-relationship-end.md:80-81 |

## Verdict

CONDITIONAL. Both findings are pre-existing spec and trace gaps that the PR exposes, not defects
it introduces. Neither blocks the merge. Both should be filed as follow-up tickets, because
FND-001 is the root cause of the #3866 / QSL-3339 flip-flop and will allow it again.

## Coverage

- Criteria examined: 5. Untagged: 0.
- Reverse gaps: none in the diff.
- Stubs and coverage inflation: none.
- Semantic review: skipped (not opted in).
- Plan completion: not assessed
