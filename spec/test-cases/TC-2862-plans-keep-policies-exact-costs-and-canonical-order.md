---
id: TC-2862
title: "Plans keep policies, exact costs and the canonical order"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1012
    type: verifies
---
# TC-2862: Plans keep policies, exact costs and the canonical order

## Description

Verify non-safety policies, deterministic outcomes and exact decimal costs. Scope: FR-1012-AC-1 to AC-3.

## Test Procedure

1. Plan under `eventually holds(c.versionNumber = 3)`.
2. Run one request twice.
3. Minimize a decimal per-step cost.

## Expected Results

- Step 1: not refused; every plan visits version 3.
- Step 2: byte-identical documents.
- Step 3: the exact run value, `potential` certificate.
