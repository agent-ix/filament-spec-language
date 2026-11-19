---
id: TC-2872
title: "Target results over an export enter as trusted evidence"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1022
    type: verifies
---
# TC-2872: Target results over an export enter as trusted evidence

## Description

Verify proof intake, counterexample replay and the refusals of evidence intake. Scope: FR-1022-AC-1 to AC-3.

## Test Procedure

1. Submit a Lean proof of `Ob_Low`, with and without `lean` present.
2. Submit a TLC trace of a seeded violation, then an edited trace.
3. Submit evidence for an `unrepresented` record and for a stale package key.

## Expected Results

- Step 1: `proved`, `trusted`, never `certified`; `tool-unavailable`, exit 21.
- Step 2: `refuted` after replay; nothing for the edited trace.
- Step 3: `invalid-value` at each member.
