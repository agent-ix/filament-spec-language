---
id: TC-440
title: "FSL's extent agrees with IR's requires-bound"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-097
    type: verifies
---
# TC-440: FSL's extent agrees with IR's requires-bound

## Description

Verify that FSL's extent classification and IR's `requires-bound` agree
over the v2 wire FSL emits. Scope: FR-097-AC-6.

## Test Procedure

1. Check and emit a package of records: `Bag` (unbounded sequence),
   `Counter` (`Integer`), `Box` (all bounded), `Outer` (names `Bag`),
   `Holder` (names `Box`), `Ints` (bounded sequence of `Integer`), `Flags`
   (bounded sequence of `Boolean`), `RangedTree` (recursive) and `Mixed`
   (`Integer` beside `Int[0,9]`).
2. Read it with IR's v2 reader and lower each record
   with `require_bounds`.
3. Classify each record with `classify_extent`.
4. Check and emit `function f using v(x: Int[0, 9], n: Integer): Integer
   pure { (x + 1) + n }`. Read its requirement records (FR-062-AC-13) and,
   with IR's v2 reader, apply IR's `requires-bound`
   predicate to each record's application node. Then check and emit
   FR-062's RR-7 (`let t = x + 1 in t * 2`) and `function k using v(x:
   Int[0, 9], n: Integer): Integer pure { if n = 0 then x + 1 else 0 }`,
   and read their records.

## Expected Results

- IR returns `RequiresBound` exactly when FSL's extent is `Unbounded`, and
  IR's first unbounded node is the form FSL names; for a recursive type it
  is a member of the recursion group of the root FSL names `Recursive`.
- Step 4: IR answers `requires-bound` for the outer `+` node, whose record
  is `Unbounded` at `n`, and not for the inner `+` node, whose record is
  `Bounded`. For RR-7's `*` record (rooted through `t`'s bound value) and
  `k`'s `+` record (rooted at `n` through the guard), IR's predicate is not
  asserted; the records' extents are asserted: `Bounded` for the `*`, and
  `Unbounded` with one `Integer` domain at `n` for `k`'s `+`.
