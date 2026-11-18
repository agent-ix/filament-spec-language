---
id: TC-2951
title: "A function claim's select refusal carries its payload in the outcome document"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-867
    type: verifies
---
# TC-2951: A function claim's select refusal carries its payload in the outcome document

## Description

Verify that the `filament-outcome/1` `monitor` document of a `function` claim run refused at stage `select` carries the refusal's `diagnostic` with its FR-272 payload. Scope: FR-867-AC-4.

## Test Procedure

1. Build `OutcomeDocument::from_clause_runs` over the FR-867-AC-1 run selecting `seven` over FR-108's ConfigVersion snapshot.
2. Build it over the runs selecting `boundedValue`, `nope` and `yes`.

## Expected Results

- Step 1: category `refusal`; one item with `clause` `seven`, `result` `declined`, `cause` `{"kind": "type-mismatch", "code": "ill_typed"}` and `diagnostic` `{"code": "ill_typed", "cause": "type-mismatch", "detail": {"function": "seven", "expected": "Boolean", "actual": "Integer"}}`.
- Step 2: `boundedValue` gives `detail.actual` `Int[0, 3]`; `nope` gives `diagnostic` `{"code": "missing_declaration", "cause": "missing-name", "detail": {"name": "nope"}}`; `yes` gives an item `tested` with `cause` null and no `diagnostic`.

Test: `tests/it/function_claim_selection.rs`.
