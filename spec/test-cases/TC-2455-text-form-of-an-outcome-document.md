---
id: TC-2455
title: "The text form of an outcome document names its operation, category, exit, stage, causes, items and result"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: verifies
---
# TC-2455: The text form of an outcome document names its operation, category, exit, stage, causes, items and result

## Description

Verify `render_outcome`'s text form of FR-286 outcome documents. Scope: FR-865-AC-1, FR-865-AC-2, FR-865-AC-5.

## Test Procedure

1. Render as text a `check` document at S3, category refusal, with one `ill_typed` diagnostic carrying cause `type-mismatch`, and a `package` document at S4, category success.
2. Render as text FR-286-AC-5's three `execute` documents, FR-286-AC-11's monitor document and FR-286-AC-9's indeterminate document.
3. Render step 1's refusal with a `RenderSources` holding its source under path `src/rule.fsl`, and a compile refusal through the outcome's exposed admitted sources.

## Expected Results

- Step 1: the header `check: refusal (exit 20) at S3`, FR-298's diagnostic line, then `  cause: type-mismatch`; the package document is the one line `package: success (exit 0) at S4`.
- Step 2: the `result:` lines FR-865-AC-2 states, the item line with its clause and the `undefined-evaluation (division-by-zero)` cause line, and `(exit 11)` in the indeterminate header.
- Step 3: the diagnostic line begins `src/rule.fsl:`; the compile refusal names the program source's path.

Test: `fsl-inspect/src/render.rs` (tests module).
