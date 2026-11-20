---
id: TC-3117
title: "A u64 seed is a decimal string and unsafe integers are refused"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-3117: A u64 seed is a decimal string and unsafe integers are refused

## Description

Verify FR-286 "Numbers": `seed` is FSpec FR-331's `U64String`, the shared
encoder refuses integers outside the IEEE 754 safe range, request settings
copied into the outcome are bounded, and the FR-517 handoff files keep
their own form. Scope: FR-286-AC-32.

## Test Procedure

1. Encode a `prove` item built by `OutcomeItem::from_state_graph` from a
   record with seed 18446744073709551615, validate it against
   `OUTCOME_SCHEMA_1` and read the seed back; repeat with seed 7.
2. Encode `{"n": 9007199254740992}` and `{"n": 9007199254740991}` with
   `fsl_foundation::canonical_json::encode`.
3. Submit a state-graph analyze request whose `witness_samples` is
   9007199254740992.
4. Write FR-517-AC-5's handoff tree and read `selection.json`.

## Expected Results

- Step 1: `"seed":"18446744073709551615"`, valid, and the same `u64` read
  back; `"seed":"7"` for seed 7.
- Step 2: the first refuses `CanonicalJsonError::UnsafeInteger` at `/n`;
  the second encodes.
- Step 3: `invalid-request` naming `witness_samples`.
- Step 4: two-space indentation and FR-517's member order.

Test: `fsl-command/tests/it/outcome_schema.rs::a_u64_seed_is_a_decimal_string_and_unsafe_integers_are_refused`.
