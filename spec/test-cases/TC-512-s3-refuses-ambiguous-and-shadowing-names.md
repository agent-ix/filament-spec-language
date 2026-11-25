---
id: TC-512
title: "S3 refuses ambiguous, shadowing, wrong-kind and wrong-channel names, in builder order"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-113
    type: verifies
---
# TC-512: S3 refuses ambiguous, shadowing, wrong-kind and wrong-channel names, in builder order

## Description

Verify the `ambiguous_declaration`/`ambiguous-name` refusals for two
declarations of one name in one scope and for a shadowing binder, and the
refusal order.

Scope: FR-113-AC-4 to FR-113-AC-10.

## Test Procedure

Use TC-510's `RecoveryFlow` fixture.

1. Add a second `effect Applied of Tried ...` in `Main`. Check it; then swap
   the two `Applied` declarations and check again.
2. Inside `Undo`, add `capture forward: Boolean = saved;` (the template binds
   `forward`). Check it. Separately, rename `attempt Tried`'s record binder
   to `M`. Check it. Separately, add a second protocol to the unit whose
   `attempt` record binder is also `attempted`. Check it.
3. Combine `effect Applied of Absent` with the shadowing `forward` capture.
   Check it twice.
4. Check four variants, one defect each: `effect Applied of Recovered`;
   `compensate Undo for Main::Tried`; an `await Wait after Main ...` in
   `Main`; channels `C` and `D` from `R` to `R` with `send S via C ...` and
   `receive Got via D of S ...`. Then change the receive to `via C`.
5. **FR-113-AC-8:** Use the TC-380 compensation fixture with
   `activate first (f: M::Failure)` and its later
   `event Fail by w as (f: M::Failure) { true };` declaration. Check it.
6. **FR-113-AC-9:** Add `capture f: M::Failure = f;` inside that activation
   and check the variant.
7. **FR-113-AC-10:** In a fresh TC-510 `RecoveryFlow` fixture, insert
   `sequence Inner { event Recovered2 by R for Undo as (tried: Boolean) { true }; }`
   in `Main` after `attempt Tried by R on M::Actor::op contracts [] as
   (tried: Boolean) { true };`. Check it.

Tag the tests `#[trace("TC-512", "FR-113-AC-n")]`.

## Expected Results

- Step 1: `ambiguous_declaration`/`ambiguous-name` at both `Applied`
  declarations and at the `Main::Applied` anchor, naming both in source
  order; after the swap, the same refusals with the two loci in the new
  source order.
- Step 2: `ambiguous_declaration`/`ambiguous-name` at the capture naming the
  bound parameter `forward`; at the binder `M` naming the model alias; the
  second protocol checks.
- Step 3: both refusals, ordered by builder state then source position, no
  checked protocol node, and equal results on both runs.
- Step 4: `ill_typed`/`type-mismatch` at the anchor,
  naming `effect-of` and `event`; `compensate-for` and `attempt`;
  `await-after` and `sequence`; then channels `C` and `D`. With `via C` the
  protocol checks.
- Step 5: the TC-380 trigger and later event binder reuse checks without
  `ambiguous_declaration`.
- Step 6: the added capture refuses `ambiguous_declaration`/`ambiguous-name`
  at the capture naming the active trigger binder.
- Step 7: the inner event refuses `ambiguous_declaration`/`ambiguous-name`
  naming the outer attempt binder, which remains active in the nested scope.
