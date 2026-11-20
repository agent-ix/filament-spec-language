---
id: TC-2160
title: "Null refuses at check with its source span"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-763
    type: verifies
---
# TC-2160: Null refuses at check with its source span

## Description

The checker reports the type error for a null expression that S2 admits.

Scope: FR-763-AC-6.

## Test Procedure

Parse, select and check a function whose body is `null`, with no model
imports, and serialize its check outcome. Test:
`tests/it/forms_refusal_cause.rs`
`a_null_expression_is_refused_at_check_with_its_locus`.

Tag the tests `#[trace("TC-2160", "FR-763-AC-6")]`.

## Expected Results

- S2 succeeds. Check returns exactly one `ill_typed`/`type-mismatch`
  diagnostic whose locus is the exact four-byte span of `null` and whose
  message names `null`, `none` and `absent` (FR-986).
- Category `refusal`, exit 20.
