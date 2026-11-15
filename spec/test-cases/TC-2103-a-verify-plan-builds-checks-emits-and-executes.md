---
id: TC-2103
title: "A verify plan builds, checks, emits and executes"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-753
    type: verifies
---
# TC-2103: A verify plan builds, checks, emits and executes

## Description

Verify the `verify` declaration through S2, S3, S4 and the coordinator. Scope: FR-753-AC-1 to FR-753-AC-4. S2 tests live in `fsl-forms/tests/it/synthesis_verify_forms.rs`; S3 tests in `tests/it/synthesis_verify_check.rs`.

## Test Procedure

1. Build FSpec FR-192's `Release` plan at S2.
2. Check it and each mutation FSpec FR-192-AC-9 and AC-10 list at S3.
3. Compile the unit with and without `Release`; emit the plan document.
4. Execute `Release` with `safe` settling `refuted`.

## Expected Results

- Step 1: one `VerificationPlan` form, three steps with names, methods, bounds and `depends`, each with its span.
- Step 2: each refusal with its cause and span, the cycle as `invalid_package`/`definition-cycle`.
- Step 3: equal package identities; a document with three nodes and one `requires` edge.
- Step 4: `tested` runs, `timely` is `blocked` naming `safe`, one disposition per node.
