---
id: TC-3083
title: "A reserved-word operation parameter is read by its qualified name"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-104
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-104-AC-18
    type: verifies
---
# TC-3083: A reserved-word operation parameter is read by its qualified name

## Description

Verify that a `pre` or `post` body reads an operation parameter through the
qualified name `<clause operation>::p`, that this is the same parameter
reference as the bare name, and that the qualified form refuses for an unknown
parameter, another operation's parameter, and an invariant. Scope:
FR-104-AC-18.

## Test Procedure

1. Over FR-104-AC-18's model (`Meter` with `adjust(by: P, step: P)` and
   `reset(by: P)`), check `pre A using v on M::Meter::adjust
   { M::Meter::adjust::by >= 0.00 }` and the same body as a `post`.
2. Check `pre S1 using v on M::Meter::adjust { M::Meter::adjust::step >= 0.00 }`
   and `pre S2 using v on M::Meter::adjust { step >= 0.00 }` and compare their
   node ids.
3. In a `post` on `adjust`, check `pre(M::Meter::adjust::by) >= 0.00`.
4. In a `pre` on `adjust`, check `M::Meter::adjust::nope >= 0.00` and
   `M::Meter::reset::by >= 0.00`; check `M::Meter::adjust::by >= 0.00` in
   `invariant I using v on M::Meter at current`.
5. Check `pre B using v on M::Meter::adjust { by >= 0.00 }`.
6. Through `fsl compile`, compile the operations-parameters source corpus case
   `decimal-keyword-parameter.native` (a `pre` on `M::Meter::adjust` of the
   `test/decimal` model) with its body written `M::Meter::adjust::by >= 0.00`,
   and keep a corpus case with the bare body `by >= 0.00`.

Tag the tests `#[trace("TC-3083", "FR-104-AC-18")]`.

## Expected Results

- Step 1: both clauses check; the qualified name is `adjust`'s parameter `by`,
  typed `P`, and `0.00` is typed `P`.
- Step 2: both check and lower to one node id.
- Step 3: `wrong_snapshot`/`forbidden-pre-read` at the `pre`.
- Step 4: each refuses `missing_declaration`/`missing-name` at the name.
- Step 5: `invalid_syntax`/`unexpected-token` at `by`, stage S1.
- Step 6: the qualified case compiles, exit 0, last stage S4; the bare case
  exits 20 at S1 with `invalid_syntax`/`unexpected-token`.
