---
id: TC-2954
title: "A reference literal formats with # touching its neighbours"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-003
    type: verifies
---
# TC-2954: A reference literal formats with # touching its neighbours

## Description

Verify the canonical style of a reference literal. Scope: FR-003-AC-16.

Catches a formatter that spaces `#` as a binary operator.

## Test Procedure

Format a unit whose clause body is
`self = M::Server#"s1" or self = V::Server#"s2"`, then format the output.

## Expected Results

- The output holds `M::Server#"s1"` and `V::Server#"s2"`.
- The second pass gives identical bytes, and the output's significant token
  sequence equals the input's.
