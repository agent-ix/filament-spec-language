---
id: TC-2534
title: "A hyperproperty over a finite trace set checks and its defects refuse"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-898
    type: verifies
---
# TC-2534: A hyperproperty over a finite trace set checks and its defects refuse

## Description

Verify that a hyperproperty over a finite trace set checks and its defects refuse. Scope: FR-898-AC-1, FR-898-AC-2.

Catches the blanket finite-domain refusal.

## Test Procedure

1. Compile `Det` over `Obs bounded 3`; compile each refusal variant.

## Expected Results

- Step 1: `Det` writes a hyperproperty node with `Obs`, 3 and the prefix; each variant refuses at its span with its code; none refuses `FiniteDomain`.
