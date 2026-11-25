---
id: TC-3083
title: "Operation parameters and quantifier binders resolve by lexical scope"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-104
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-104-AC-18
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-016-AC-11
    type: verifies
---
# TC-3083: Operation parameters and quantifier binders resolve by lexical scope

## Description

Verify qualified operation-parameter reads, bare-name resolution to the
innermost quantifier binder, and the distinct identities of a same-named
quantifier binder and qualified operation parameter. Also verify the accepted
function-parameter shadowing case. Scope: FR-104-AC-18 and FR-016-AC-11.

## Test Procedure

1. Over FR-104-AC-18's model (`Meter` with `adjust(by: P, step: P,
   steps: Sequence<P>[0,3])` and `reset(by: P)`), check `pre A using v on M::Meter::adjust
   { M::Meter::adjust::by >= 0.00 }` and the same body as a `post`.
2. Check `pre S1 using v on M::Meter::adjust { M::Meter::adjust::step >= 0.00 }`
   and `pre S2 using v on M::Meter::adjust { step >= 0.00 }` and compare their
   node ids.
3. In a `post` on `adjust`, check `pre(M::Meter::adjust::by) >= 0.00`.
4. In a `pre` on `adjust`, check `M::Meter::adjust::nope >= 0.00` and
   `M::Meter::reset::by >= 0.00`; check `M::Meter::adjust::by >= 0.00` in
   `invariant I using v on M::Meter at current`.
5. Check `pre B using v on M::Meter::adjust { by >= 0.00 }`.
6. In a `pre` on `adjust`, check `let step = M::Meter::adjust::by in M::Meter::adjust::step >= step`.
7. With `Gauge` a subtype of `Meter` inheriting `adjust`: check
   `pre G1 using v on M::Gauge::adjust { M::Gauge::adjust::by >= 0.00 }`;
   check `M::Meter::adjust::by >= 0.00` in a `pre` on `M::Gauge::adjust` and
   `M::Gauge::adjust::by >= 0.00` in a `pre` on `M::Meter::adjust`.
8. Check `M::Meter::adjust::by >= 0.00` in
   `constraint K using v on M::Meter at current`.
9. In a unit whose source declares object type `Dial` with operation
   `turn(by: P)`, check `pre T using v on Dial::turn { Dial::turn::by >= 0.00 }`.
10. Through `fsl compile`, compile the source corpus case
    `tests/corpus/operations/parameters/decimal-keyword-parameter.native`
    (a `pre` on `M::Meter::adjust` of the `test/decimal` model) with its body
    written `M::Meter::adjust::by >= 0.00`, and a sibling case with the bare
    body `by >= 0.00`. The corpus case is added by the code change that
    implements this resolution; until then it is staged as
    `tests/corpus/operations/parameters/pending/decimal-keyword-parameter.native`
    on branch `codex/qsl-2041-parameters`.
11. In both a `pre` and a `post` on `M::Meter::adjust`, check
    `forall(step in steps: step >= M::Meter::adjust::step)` with
    `steps: Sequence<P>[0,3]` in `adjust`'s parameters; compare the node
    identities for the bare quantifier-binder read and qualified parameter
    read in the predicate.
12. Check the function
    `function f using v(x: Integer, s: Sequence<Integer>[0,3]): Boolean pure { forall(x in s: x = 0) }`.

Tag the tests `#[trace("TC-3083", "FR-104-AC-18", "FR-016-AC-11")]`.

## Expected Results

- Step 1: both clauses check; the qualified name is `adjust`'s parameter `by`,
  typed `P`, and `0.00` is typed `P`.
- Step 2: both check and lower to one node id.
- Step 3: `wrong_snapshot`/`forbidden-pre-read` at the `pre`.
- Step 4: each refuses `missing_declaration`/`missing-name` at the name.
- Step 5: `invalid_syntax`/`unexpected-token` at `by`, stage S1.
- Step 6: checks; the qualified name lowers to `adjust`'s `step`
  `value`/`parameter` node and the bare `step` to the `let` binder.
- Step 7: `G1` checks; the other two each refuse
  `missing_declaration`/`missing-name` at the name.
- Step 8: `missing_declaration`/`missing-name` at the name.
- Step 9: checks, the name read as `turn`'s parameter `by`.
- Step 10: the qualified case compiles, exit 0, last stage S4; the bare case
  exits 20 at S1 with `invalid_syntax`/`unexpected-token`.
- Step 11: both clauses check; bare `step` resolves to the quantifier binder,
  `M::Meter::adjust::step` resolves to the operation parameter, and the two
  reads have distinct identities.
- Step 12: the function checks; bare `x` in the quantifier predicate resolves
  to the quantifier binder, shadowing the same-named function parameter.
