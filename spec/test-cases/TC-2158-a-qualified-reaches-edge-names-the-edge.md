---
id: TC-2158
title: "A qualified reaches edge names the edge"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-763
    type: verifies
---
# TC-2158: A qualified reaches edge names the edge

## Description

A qualified reaches edge names the edge.

Scope: FR-763-AC-4.

## Test Procedure

Run the `parse` operation over a state clause `reaches(self, self, Config::ConfigVersion::parent)`. Test: `tests/it/forms_refusal_cause.rs` `a_qualified_reaches_edge_builds_without_a_forms_refusal`.

Build the same clause through S2 and assert the complete edge spelling and
span, and both `SelfRef` operands. Test:
`fsl-forms/tests/it/state_model_forms.rs`
`a_qualified_edge_name_builds_a_reaches_form`.

Tag the tests `#[trace("TC-2158", "FR-763-AC-4")]`.

## Expected Results

- Successful parse (exit 0), no diagnostic. The form retains
  `Config::ConfigVersion::parent` and its complete span (FR-102-AC-3).
