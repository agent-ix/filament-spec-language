---
id: TC-2153
title: "The diagnostic list is cut at s1.diagnostics"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-762
    type: verifies
---
# TC-2153: The diagnostic list is cut at s1.diagnostics

## Description

The diagnostic list is cut at s1.diagnostics.

Scope: FR-762-AC-6.

## Test Procedure

Parse a unit of 150 `record` declarations, each missing a field type, under the default limits. Test: `fsl-cst/tests/it/complete_cst.rs` `the_diagnostic_list_is_cut_at_its_bound`.

Tag the tests `#[trace("TC-2153", "FR-762-AC-6")]`.

## Expected Results

- 101 diagnostics: 100 `unexpected-token`, then one `invalid_syntax`/`diagnostic-list-cut` at the 101st record's offending token whose message names `s1.diagnostics` and 100.
- No diagnostic is `stage_limit_exceeded`.
