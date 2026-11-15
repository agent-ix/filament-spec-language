---
id: TC-2178
title: "A format document carries the formatted text"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-2178: A format document carries the formatted text

## Description

A format document carries the formatted text.

Scope: FR-286-AC-16.

## Test Procedure

Format `tests/fixtures/spine-compile.native`, serialize `OutcomeDocument::from_format`, then format its `text` again. Test: `fsl-replay/tests/it/format_outcome_facade.rs` `format_json_result_carries_text`. The existing `a_driver_serializes_format_success_at_s2_without_text_payload` asserts `result` `null` and changes with this criterion.

Tag the tests `#[trace("TC-2178", "FR-286-AC-16")]`.

## Expected Results

- `result` is `{"kind": "formatted", "text": T}`, `T` equal to `fsl_cst::format`'s string; exit 0.
- Formatting `T` gives `T`.
