---
id: TC-2661
title: "An invariant item is certified and its counterexample replays"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-951
    type: verifies
---
# TC-2661: An invariant item is certified and its counterexample replays

## Description

Verify that EN-1 decides an `invariant` state clause over a model subject
with a certificate under the invariant key-context root, and that its
finite-prefix counterexample replays. Scope: FR-951-AC-1, FR-951-AC-2,
FR-951-AC-3, FR-951-AC-4.

## Test Procedure

1. Analyze `AtMostThree` over FR-625-AC-4's Counter request and read the item's certificate.
2. Analyze `AtMostTwo` and read its counterexample and replay.
3. Replay the truncated, looped and deadlock-kind payloads, and check the tampered certificates.
4. Check the possibly-zero and literal-divisor `mod` invariants under `CheckMode::Linked`.

Test: `fsl-analyze/tests/it/invariant_item.rs`.

## Expected Results

- Step 1: `proved`, `exhaustive`, `en-1`, `certified`; root `["invariant",N,"claim",O]`, binding `[]`.
- Step 2: `refuted`, `en-1`; prefix ends at `value = 3`, empty loop; replay reproduced.
- Step 3: `ReplayParity`; two `invalid-value` refusals; `BadState`; foreign-root refusal.
- Step 4: `unproved-nonzero` at check; the literal-divisor clause is `proved`.
