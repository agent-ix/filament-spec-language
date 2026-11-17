---
id: TC-2834
title: "A reference literal checks, stays pending in a barred slot and lowers"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1000
    type: verifies
---
# TC-2834: A reference literal checks, stays pending in a barred slot and lowers

## Description

Verify S3 typing and refusals of a reference literal, that S3 leaves a
barred-slot occurrence pending rather than refusing it, and S4 lowering.
Scope: FR-1000-AC-2 and FR-1000-AC-3.

Catches a checker that decides fixed or created-object without a subject.

## Test Procedure

1. Over TC-1076's `Counter` unit, check FR-1000-AC-2's invariant `R`, then
   the same with `M::Nothing#"a"`, `M::Level#"a"` and `M::Counter#""`.
2. Check a protocol whose `choice` guard compares the received `got.target` to `M::Counter#"z"`.
3. Emit `R`, and a unit with a second alias of the package and two
   occurrences of the literal.

## Expected Results

- Step 1: `R` checks with the literal typed `Reference<M::Counter>`; then
  `missing_declaration`/`missing-name` at `Nothing`, `ill_typed`/
  `type-mismatch` at `M::Level` (expected `object type`), and
  `invalid_package`/`invalid-value` at `""`.
- Step 2: no refusal.
- Step 3: one `literal` term, `value_kind` `reference`, `value` `61`, shared
  by both occurrences.
