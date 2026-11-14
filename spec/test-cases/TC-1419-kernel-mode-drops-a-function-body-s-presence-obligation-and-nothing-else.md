---
id: TC-1419
title: "Kernel mode drops a function body's presence obligation and nothing else"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-495
    type: verifies
---
# TC-1419: Kernel mode drops a function body's presence obligation and nothing else

## Description

Verify that `CheckMode::Kernel` records no presence obligation for a function
body, and that it leaves a state clause and a termination measure checked as
under `CheckMode::Linked`. Scope: FR-495-AC-10, FR-495-AC-11 and FR-495-AC-12.

## Test Procedure

1. Check `p` and `g` of FR-495-AC-10 under `CheckMode::Linked`, then under
   `CheckMode::Kernel`, through `PackageDeclarations::check`.
2. Over the ConfigVersion package (FR-103-AC-1), check the invariant
   `NoGuard` and the guarded `ParentOrder` of FR-495-AC-11 under each mode.
3. Check `w` and each of the two `f` functions of FR-495-AC-12 under each
   mode.

Tag the tests `#[trace("TC-1419", "<AC id>")]`.

## Expected Results

- Step 1: `p` is refused `undefined_expression`/`unproved-presence` under
  `Linked` and checks under `Kernel`; `g` checks under both.
- Step 2: `NoGuard` is refused `undefined_expression`/`unproved-presence`
  under both modes, one refusal each; `ParentOrder` checks under both.
- Step 3: `w` is refused `unproved-presence` under both modes; `f(x)` is
  refused `unproved-decrease`/`missing-measure` at the body root under both;
  `f(x) + 1` is refused `unproved-range` under `Linked` and
  `unproved-decrease`/`missing-measure` at the first operand's location
  under `Kernel`.
