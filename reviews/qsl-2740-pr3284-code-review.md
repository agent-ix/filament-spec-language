---
id: SR-202611210022442313144108504
title: "Code review — filament-spec-language QSL-2740 source population aggregates"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-eval/src/value/expression/{causes.rs,evaluate.rs}, tests/it/{main.rs,run_value_spellings.rs,workflow_population_aggregates.rs}; FR-046, FR-391, FR-106"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-046", type: references }
---

## Summary

Reviewed all five changed files at frozen head `c3d5c8ccf661a4e2ee970fe03edc897820922ef7`, including the Rust lane and the source-to-test behavior for explicit population aggregates. The new completeness guard duplicates the existing named-population completeness rule, leaving two production implementations that can diverge. The workflow tests also do not exercise the duplicate-receipt versus distinct-business-effect case required by the external recovery criterion they reference.

## Verdict

**FAIL** — the duplicated completeness rule is a P0 finding; a required external workflow edge also lacks a test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Source-population completeness is enforced in a second implementation instead of sharing the existing named-population check, so future changes can make the two paths return different refusal behavior. | fsl-eval/src/value/expression/evaluate.rs:2409,4465 |
| FND-002 | medium | The workflow aggregate cases never model a duplicate delivery receipt separately from a distinct business effect, so they cannot verify that duplicate receipts do not add another effect to the recovery aggregate. | tests/it/workflow_population_aggregates.rs:216-226 |

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| `fsl-eval/src/value/expression/causes.rs` | examined | `NamedPopulationIncomplete` refusal code and fields; documentation now includes source population queries. |
| `fsl-eval/src/value/expression/evaluate.rs` | examined | `require_complete_population` checks the selected population view; `AllInstances` invokes it; `named_members` separately checks the same completeness map and constructs the same refusal. |
| `tests/it/main.rs` | examined | Registers the workflow aggregate integration test module. |
| `tests/it/run_value_spellings.rs` | examined | Exercises complete and incomplete explicit population views with source `sum` and `count`. |
| `tests/it/workflow_population_aggregates.rs` | examined | Exercises grouped workflow `sum` behavior, incomplete refund refusal and contradictory totals. |
| FR-046-AC-6 | examined | “Missing input, unknown required membership and missing completeness produce no successful total; an independently supplied complete counterpart evaluates without reconstructing missing members or consulting an ambient store.” |
| FR-391 | context_only | Population bindings and their evaluation over admitted observations. |
| FR-106 | context_only | Snapshot admission and population completeness facts supplied to evaluation. |
| FSpec-FR-049-AC-8 | examined | A recovery aggregate retains finite view, snapshot/window membership and completeness dependencies; missing members or duplicate business effects cannot be erased to make the aggregate pass. |
| FSpec-TC-173 | context_only | A duplicate receipt does not duplicate an effect; a distinct effect remains an aggregate member and may violate the recovery target. |

## Review Notes

The new `require_complete_population` helper at lines 2409–2426 repeats the completeness lookup, `listed` calculation, `NamedPopulationIncomplete` construction, and refusal path already present in `named_members` at lines 4459–4477. The selected observation is resolved differently by the two callers, but the shared completeness rule can take that already-selected observation and population key. Keep the selected-view lookup at each call site and centralize the completeness decision/refusal construction.

The `source_aggregates_group_reservations_and_duplicate_refunds_by_order` fixture has refund keys `r1`, `r2`, and `r3` with order and amount fields only. It does not represent a repeated delivery receipt or a separate business-effect identity. TC-173 requires both a duplicate receipt and a distinct second effect; the current test can pass even if repeat receipts are counted as extra effects.

The supplied focused runner evidence is for the exact frozen head: `workflow_population_aggregates::` 3 passed and `run_value_spellings::` 12 passed, 0 failures, 3 existing ignored. No local Cargo gate was run.
