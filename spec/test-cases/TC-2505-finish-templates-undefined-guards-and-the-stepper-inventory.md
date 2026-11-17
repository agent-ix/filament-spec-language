---
id: TC-2505
title: "Finish, compensation templates and undefined initial guards are stepped, and the stepper inventory has no unsupported construct"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-882
    type: verifies
---
# TC-2505: Finish, compensation templates and undefined initial guards are stepped, and the stepper inventory has no unsupported construct

## Description

Verify that `finish` and compensation templates are stepped, that an undefined value at an instance's first state is a refutation, and that the construct corpus explores. Scope: FR-882-AC-1, FR-882-AC-2, FR-882-AC-7, FR-882-AC-8.

## Test Procedure

1. Explore ADR-027 §7.1's `Pay` under `on each` with triggers 0 and 1.
2. Check deadlock freedom of a protocol whose `choice` guard divides by a capture equal to 0, under `origin` and under `each`.
3. Explore the one-protocol-per-construct corpus of FR-218.
4. Check `Flow` with `check Gate { protocol Other }` where `Other`'s terminal obligation holds from its initial states and where it is refuted, and with two different model states reached before `Gate`; then `Flow` with `while { protocol Flow }`; then a `send` constraint and a capture holding `protocol Other`.

## Expected Results

- Step 1: `finish` steps are reached; one compensation thread per activation; no unstepped control class.
- Step 2: `UndefinedEvaluation` at the `choice`, `refuted`; the prefix is empty under `origin` and `activate` alone under `each`.
- Step 3: every protocol settles; the only refusals are `UnenumerableRecord` for the text-field binder and `WeakMemory` for the unimplemented model.
- Step 4: proved, then refuted at `Gate`, the same at both reached states; the self-reference settles `refuted` with `UndefinedEvaluation` at the guard and terminates; the `send` and capture read the `check`'s value.
