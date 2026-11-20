---
id: TC-3038
title: "A protocol capture read needs no listing"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-381
    type: verifies
---
# TC-3038: A protocol capture read needs no listing

## Description

Verify that S3 never refuses a read of a protocol capture
`wrong_snapshot`/`wrong-observation`, that a guard reading the `over` input or
the activation binder in place of a capture keeps its own refusal, and that
FR-381's read classification is the only refusal of a capture read. Scope:
FR-218-AC-48 and FR-381-AC-16.

## Test Procedure

1. Check TC-1057's `Refunds` with `profile S = "filament.state.graph/v1";`, a
   second role `role Clerk on M::Node;`, and at the end of `run sequence Main`
   `choice ByService by Service visible () { case pos when { order >= 0 }
   check A using S { true }; case neg when { not (order >= 0) } check B using
   S { true }; }` and `choice ByClerk by Clerk visible () { case pos when {
   order >= 0 } check C using S { true }; case neg when { not (order >= 0) }
   check D using S { true }; }`.
2. Change `ByService`'s `pos` guard to `view.total >= 0`.
3. Change `ByClerk`'s `pos` guard to `opened.ready`.
4. Change the protocol capture to `capture order: M::Total = later;` followed
   by `capture later: M::Total = view.total;`; then to
   `capture order: M::Total = order;`; then to
   `capture order: M::Total = target;`.

## Expected Results

- Step 1: the unit checks; both guards read `order` at a decision of each
  role with an empty `visible` list, and no `wrong_snapshot` refusal occurs.
- Step 2: `ill_typed`/`operator-ineligible` with span `view.total`, naming
  `ByService` and slot `guard`.
- Step 3: `wrong_snapshot`/`wrong-observation` with span `opened.ready`,
  `node` `ByClerk`, `owner` `Clerk`, `read` `opened.ready`, `reason`
  `ineligible-kind`.
- Step 4: `missing_declaration`/`missing-name` with `stage`
  `protocol-capture`: at `later` with `read` `forward`; at `order` in its own
  initializer with `read` `forward`; at `target` with `owner` the protocol and
  `read` `foreign`; none is `wrong_snapshot`.
