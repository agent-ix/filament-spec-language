---
id: TC-3198
title: "The text of a compile success says what was produced and where it was written"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: verifies
---
# TC-3198: The text of a compile success says what was produced and where it was written

## Description

Verify the three-line text of a successful compile. Scope: FR-865-AC-11 and FR-865-AC-12.

## Test Procedure

1. Compile FR-810-AC-1's request carrying `output` `{"package": "spine.package.json"}` and call `render_text` on the outcome.
2. Compile the same request without `output` and without `RequestOverrides::output` and call `render_text`.

## Expected Results

- Step 1: exactly `package: success (exit 0) at stage packaging`, `produced: checked package (<n> bytes)` with `<n>` the length of `package_bytes()`, and `written to: spine.package.json`.
- Step 2: the same first two lines and `nothing written: no --output option or output member was given`; no `written to:` text.

Test: `fsl-command/tests/it` (compile outcome text).
