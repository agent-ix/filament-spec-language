---
id: TC-1421
title: "Kernel mode drops the rational-range, rational divisor and nonempty-reduction obligations"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-495
    type: verifies
---
# TC-1421: Kernel mode drops the rational-range, rational divisor and nonempty-reduction obligations

## Description

Verify that `CheckMode::Kernel` records no rational-range, rational `/` divisor
nonzero or nonempty-reduction obligation for a function body, while
`CheckMode::Linked` refuses each (FR-495). Each fixture is refused under
`Linked` and checks under `Kernel`, so a Kernel check that still recorded the
obligation fails it. Scope: FR-495-AC-14, FR-495-AC-15 and FR-495-AC-16.

## Test Procedure

1. Check `r` of FR-495-AC-14 through `PackageDeclarations::check` under
   `CheckMode::Linked`, then under `CheckMode::Kernel`.
2. Check `q` of FR-495-AC-15 under each mode.
3. Check `m` of FR-495-AC-16 with `q: Sequence<S>[0, 2]` under each mode, then
   the same function with `q: Sequence<S>[1, 2]` under each mode.

Tag the tests `#[trace("TC-1421", "<AC id>")]`.

## Expected Results

- Step 1: `r` is refused `undefined_expression`/`unproved-range` at the `+`
  under `Linked` (cause `Obligation::RationalRange`) and checks under `Kernel`.
- Step 2: `q` is refused `undefined_expression`/`unproved-nonzero` at the `/`
  under `Linked` (cause `Obligation::Nonzero`) and checks under `Kernel`.
- Step 3: the `[0, 2]` form is refused `undefined_expression`/`unproved-range`
  at the `reduce` under `Linked` (cause `Obligation::NonemptyReduction`) and
  checks under `Kernel`; the `[1, 2]` form checks under both.
