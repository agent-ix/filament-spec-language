---
id: TC-1591
title: "Hyperproperty items report k-trace counterexamples and certificates"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-631
    type: verifies
---
# TC-1591: Hyperproperty items report k-trace counterexamples and certificates

## Description

Verify FR-631's request item and the emission of object-free hyper clauses. Scope: FR-631-AC-1 to FR-631-AC-4.

## Test Procedure

1. Analyze the vault non-interference clause for the resetting and the leaking vault.
2. Repeat the proof with copy-swap `symmetry`.
3. Analyze a forall-exists clause with `max_witness_set` 1.
4. Emit and analyze a hyper `temporal` clause with no object parameter.

## Expected Results

- Step 1: `proved`; `refuted` with a two-trace counterexample replayed copy by copy.
- Step 2: `reduced`, `certified`.
- Step 3: `incomplete` `LimitReached{max_witness_set}`.
- Step 4: a hyperproperty node is emitted and the item settles `proved` or `refuted`.
- Assertions compare the outcome document's typed members and the exit code, never message text.
