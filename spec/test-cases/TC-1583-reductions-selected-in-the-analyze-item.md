---
id: TC-1583
title: "Reductions selected in the analyze item keep the verdict or refuse typed"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-623
    type: verifies
---
# TC-1583: Reductions selected in the analyze item keep the verdict or refuse typed

## Description

Verify FR-623's reduction selection, its refusals and its proof basis. Scope: FR-623-AC-1 to FR-623-AC-5.

## Test Procedure

1. Over the symmetric fixture, analyze a holding and a refuted clause with no reduction, `symmetry` `"declared"`, `partial_order` true, and both.
2. Compare the admitted requests of `"declared"` and the explicit one-class declaration.
3. Analyze a bounded MLTL clause with `partial_order` true beside a second item.
4. Analyze with an unknown `constraint` name, and `"declared"` over a unit with no symmetric population.
5. Read the proof basis of the symmetry-reduced proof.

## Expected Results

- Step 1: one verdict per clause across the selections.
- Step 2: equal requests.
- Step 3: `inconclusive` `reduction-not-preserving` naming `partial-order`, exit 22; the second item settles as alone.
- Step 4: `invalid_runtime_input`/`invalid-value` naming the name; `declined` `invalid-input`.
- Step 5: `reduced` listing `symmetry`, certification `uncertified`.
- Assertions compare the outcome document's typed members and the exit code, never message text.
