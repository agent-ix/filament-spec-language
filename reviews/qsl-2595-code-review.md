---
id: SR-2026112005160200000001
title: "QSL-2595 batch code-review source-route diagnostics"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2595, QSL-2594, QSL-2658, QSL-2609; fsl-command/src/command.rs, fsl-cst/src/lexer.rs, fsl-cst/src/lib.rs, fsl-forms/src/dispatch.rs, fsl-forms/src/hyper_clause.rs, fsl-forms/src/probabilistic.rs, fsl-forms/src/temporal_clause.rs, fsl-forms/src/value.rs, fsl-replay/src/spine.rs, fsl-semantics/tests/it/temporal_clock_names.rs, tests/it/compile_command.rs, tests/it/forms_refusal_cause.rs, tests/it/refinement_remaining_source_routes.rs, tests/it/refinement_source_routes.rs"
review_set: subset
---

## Summary

Reviewed head 0b3edd5afe56895cd96eed79a7446c1d67276d16 on qsl-batch-rh-2594-2595-2658-2609, diff origin/main...HEAD, PR filament-spec-language#3142. Tickets: QSL-2595 (primary), QSL-2594, QSL-2658, QSL-2609. Rust lane (rust-review, rust-style) folded in. Read-only; no builds.

## Verdict

**MERGEABLE** with one medium and three low findings. The S2 hyper-condition refusal is gone and every holds condition (temporal clause, probabilistic event) reaches an S3 typer that refuses Indexed/StepLabel/Stutter unconditionally with OutsideHyperClause (typing.rs:1505) or, for a hybrid claim, HybridCause::HyperExpression (also unsupported_construct/expression-form). The removed helpers `unrepresented`/`source_text` had no other caller. `declared_language` skips comments and whitespace, compares the decoded literal like `declared_edition`, and returns None for anything that is not `language <text>`, so it cannot misfire on a valid header. The Population arm matches its siblings. The QSL-2609 oracle change follows FSpec FR-322 "Operand index segments" (node-held ids carry holder-relative operand slots). No Python, shims, pin changes or new ignores; four ignores removed; merge-tree against origin/main is clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new two-trace case pins the refusal locus to the whole condition `v.versionNumber @ a = v.versionNumber @ b`, while FR-612 Behavior locates the refusal at the first indexed read in source order (`v.versionNumber @ a`); the test enshrines a locus that differs by shape, contrary to the arch ruling's "no split by shape". Fix the locus or record a spec ruling, then assert `v.versionNumber @ a`. | fsl-semantics/tests/it/temporal_clock_names.rs:211-214 |
| FND-002 | low | `FormsCause::UnrepresentedConstruct` has no producer left after `unrepresented` was deleted; its definition and match arms are dead. Delete the variant and its arms (pre-release, no compat). | fsl-forms/src/dispatch.rs:479, fsl-forms/src/dispatch.rs:850, fsl-replay/src/spine.rs:972, fsl-replay/src/outcome.rs:2030 |
| FND-003 | low | The un-ignored QSL-2595 corpus test asserts exit, code and cause but not the locus `c.value @ a` or the hyper message that the ticket's acceptance names. Add both assertions. | tests/it/refinement_source_routes.rs:160-171 |
| FND-004 | low | `require_complete_edition` now also refuses a foreign language; its name no longer says what it does. Rename (for example `require_native_header`). | fsl-command/src/command.rs:2632-2634 |

## Coverage

Checked every holds-condition consumer at S3 (assemble.rs, hybrid/claim.rs:631, probabilistic/claim.rs:562/1359/1608, typing.rs:1505/1645). Fairness and where clauses never went through the removed S2 check, so they are unchanged. Deleted test `every_construct_message_begins_with_the_construct` exercised only the hyper-condition construct (two conditions, short and 200-byte); no other construct loses coverage. Pre-existing failures: `clause_text_parses_as_a_unit_clause_body` compares spans between two wrappers whose prefixes differ by 3 bytes (`pre Clause using p` in clause_text.rs:35 vs `pre Own using v` in the test), which is exactly 124 vs 121, and neither file nor the lexer recognizer is touched; `unparenthesized_record_opens_arm_list_and_fails_there` is an S1 diagnostic test untouched by the diff; the values corpus and LIFECYCLE_UNIT hold no `@`, no hyper form and no non-native language, so neither root corpus failure can come from this diff.

## Validation limits

No builds or test runs (owner instruction). Coder-reported focused runs taken as given. FND-001 root cause (S2 node span versus checker location) not traced to a line; the observable locus is what the coder's passing test asserts.

## Examined units

```yaml
scope:
  - {id: S2/holds-condition, path: fsl-forms/src/temporal_clause.rs, role: examined}
  - {id: S3/OutsideHyperClause, path: fsl-semantics/src/check/check/typing.rs, role: examined}
  - {id: declared_language, path: fsl-cst/src/lexer.rs, role: examined}
  - {id: require_complete_edition, path: fsl-command/src/command.rs, role: examined}
  - {id: RowClass::Population, path: fsl-replay/src/spine.rs, role: examined}
  - {id: resolve_operand_slots-oracle, path: tests/it/refinement_remaining_source_routes.rs, role: examined}
```
