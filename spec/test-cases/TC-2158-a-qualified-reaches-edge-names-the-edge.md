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

Run the `parse` operation over a state clause `reaches(self, self, Config::ConfigVersion::parent)`. Test: `tests/it/forms_refusal_cause.rs` `a_qualified_reaches_edge_names_the_edge`.

Tag the tests `#[trace("TC-2158", "FR-763-AC-4")]`.

## Expected Results

- `construct` `qualified-reaches-edge`, `text` `Config::ConfigVersion::parent`, locus the edge's span.
