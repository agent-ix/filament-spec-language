---
id: SR-2026111612172400000036
title: "Scoped gap analysis of filament-spec-language PR #2913"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #2913 diff origin/main...c8c9216dc: FR-569-AC-16, FR-569-AC-11, FR-569-AC-8 and their tagged tests"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-569", type: references}
  - {target: "ix://agent-ix/filament-spec-language/TC-1408", type: references}
---
# Scoped gap analysis of PR #2913

## Summary

Ticket: QSL-1880. Exact head `c8c9216dc4fac45c5000f8c889e2ea3333b98bd8`. Quoin has no `gap-analysis` command in the installed version, so this is a manual scoped AC-to-test check over the ACs the PR adds or retags. `quire matrix --scope . --format json` produced no output within a 300 s bound, so trace tags were found by grepping `#[trace(...)]` attributes. Plan completion: not assessed.

## Verdict

PASS with one medium gap. FR-569-AC-16 has a running tagged test for own-vs-inherited, redefinition, the query flag and the `operation_effects` correspondence; its bytes round-trip with non-empty `operations` is in tests ignored on QSL-2347, which the brief accepts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The only running FR-569-AC-16 test uses an ad-hoc unit (`Order { size, bump }`, `Sub { size }`, `Shape`) rather than fixture S, so the AC's parameterised operations (`Order::count`, `Order::scaled(n)`, `Sub::scaled(m)` redefinition) and "D's emission has none" are not asserted by any test. | fsl-package/src/emit/tests/source_objects.rs:491-541 |

## Coverage

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-569-AC-16 | examined | Over fixture S, the `operations` of the source-object rows are: `Order` `[{identity: "Order::size", query: true}, {identity: "Order::count", query: true}, {identity: "Order::scaled", query: true}, {identity: "Order::bump", query: false}]` in source order; `Sub` `[{identity: "Sub::size", query: true}, {identity: "Sub::scaled", query: true}]` [...] D's emission has none. |
| FR-569-AC-11 | examined | S4 [...] emits the unit's source objects per FR-995 and FR-996 and does not refuse the unit for declaring an `object`. |
| FR-569-AC-8 | examined | The emission over S carries one source-object row per type [...] written to `identities.json` `source_objects` and read back equal. |

- Tagged tests for FR-569-AC-16: `source_object_rows_carry_their_own_operations_and_query_flag` (runs), `source_object_operations_roundtrip_through_qc30_intake` (ignored, QSL-2347), `public_source_object_rows_constructor_serializes_own_operations` (ignored, QSL-2347).
- The reader-rejection clause of AC-16 is untested; recorded as FND-001 of the code-review artifact SR-2026111612172400000035, not duplicated here.

## Dispositions

Round 1, verified against fix commit `f1111b5cb`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Fixture S part fixed: `source_object_rows_carry_their_own_operations_and_query_flag` now emits TC-1408 fixture S's object block verbatim and asserts Order size/count/scaled/bump, Sub size/scaled, Shape []. Still missing: AC-16's "D's emission has none" is asserted by no test (one emit of the `orders_model` fixture D with `source_object_operations().is_empty()` closes it). |

The coder flagged that the file-level `S` constant in `source_objects.rs` names `measure`/`tally` where TC-1408 fixture S says `size`/`count`. Acceptable for this PR, not a finding against it: the new AC-16 test uses the TC text, and the drift comes from #2826 (fc5ada4e8, QSL-2340) on main, outside this diff. It is worth a separate ticket to realign the constant with TC-1408 or the reverse.

Round 2, verified against fix commit `bcf04ba96` (head `bcf04ba96905dc75ceb75b4d6657aecd4591f1ca`). `fixture_d_emission_has_no_source_object_operations` passes (fsl-package `source_objects`: 16 passed, 8 ignored on QSL-2347); clippy on fsl-package clean.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | New test `fixture_d_emission_has_no_source_object_operations` emits fixture D (`model M = "test/orders"`, no source object) and asserts `selected_source_object_rows()` and `source_object_operations()` are empty. |
