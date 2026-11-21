---
id: SR-2026112021332500000000002
title: "gap-analysis of PR 3046 changes since the 2026-11-19 draft review"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3046 head b216703b5c5a5f678440ac1e4c28fb518ddbe461; FR-091-AC-77 (QSL-2610), FR-106-AC-53..61/66/68/69 rebase interplay, admit_population_values refusal path; fsl-semantics/src/check/type_form.rs, fsl-semantics/src/model/observation/document.rs, fsl-semantics/tests/it/{native_scalar_equality,native_snapshot_values}.rs, fsl-conformance/cases/type-operators/native-equality-accepted.native"
review_set: subset
---

## Summary

Ticket: QSL-1068, with QSL-2610 in the same PR. Planless gap analysis of what changed since the 2026-11-19 draft review: the QSL-2610 commit, the refusal path commit, and the rebase onto main's Float admission. Plan completion: not assessed. `quire matrix` (quire 0.52.0, engine 0.62.2) was run read-only over the main checkout `/Users/peter/dev/qsl-2631-code`, not the PR head. The PR's own trace tags were checked by reading the diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

FR-091-AC-77: every clause has a tagged test. The four functions assemble with no domain package and check, with one `filament.op.bytes.eq` and one `filament.op.json_object.eq` (`bytes_and_json_object_name_their_native_types`). Parameter types resolve through `native_scalar_named`, and the aliases resolve to `ValueType::Bytes`/`JsonObject`. Bytes against JsonObject gives `type-mismatch` (`bytes_against_json_object_refuses_type_mismatch`). `record Bytes` gives `ambiguous-name` naming the use, the record span and the native candidate with no span (`a_declared_bytes_record_makes_the_native_name_ambiguous`). "Compiles through S4 with exit 0" is carried by the conformance case `native-equality-accepted.native` with `equal2`/`equal3` added (`matrix_native_equality`). The type_form.rs change has an owner (FR-091 type-name resolution, lines 862-869).

FR-106-AC-53..61, 66, 68, 69: after the rebase, Float membership and the Float payload come from main (QSL-2176). This PR's Float tests (`native_collection_float_bits_preserve_zero_sign_and_nan_payload`, `native_collection_frame_identity_uses_decimal_value_and_float_bits`) still exercise them, including a Set of Option of Float. The refusal-path commit has no requirement of its own. Its behaviour is covered as a code defect in the code-review artifact (SR-2026112021332500000000001 FND-001), not as an untraced change.

Matrix: on main, the FR-106 row's extracted criteria still end at AC-61. The cause is a blank line at `spec/functional/FR-106-admit-snapshots-and-invocations.md:1309`, which splits the acceptance table so that AC-62..69 fall outside it. That file is not in this PR's diff. See the disposition of the earlier review's FND-001.

## Verdict

Clean for the delta. Every changed behaviour has an owning requirement and a tagged test.
