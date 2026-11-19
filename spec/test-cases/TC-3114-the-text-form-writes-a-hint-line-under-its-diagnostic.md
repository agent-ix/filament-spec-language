---
id: TC-3114
title: "The text form writes a hint line under its diagnostic"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: verifies
---
# TC-3114: The text form writes a hint line under its diagnostic

## Description

Verify that `render_outcome`'s text form writes a diagnostic's `hint` on
its own line after its cause and label lines, and no hint line for a
diagnostic without one. Scope: FR-865-AC-8.

## Test Procedure

1. Render as text FR-286-AC-28's `check` refusal and its `io-error`
   document.
2. Render as text FR-286-AC-15's blank-label document.
3. Render as text FR-286-AC-23's `output-failure` document.

## Expected Results

- Step 1: the `check` text has the diagnostic line, then
  `  cause: type-mismatch`, then `  hint: <text>` with `<text>` the
  diagnostic's `hint` byte for byte; the `io-error` text has its diagnostic
  line followed by `  hint: <text>`.
- Step 2: `  cause: blank-label`, then `  label: <label>`, and no line
  beginning `  hint:`.
- Step 3: no line begins `  hint:`.

Test: `fsl-inspect/src/render.rs` (tests module).
