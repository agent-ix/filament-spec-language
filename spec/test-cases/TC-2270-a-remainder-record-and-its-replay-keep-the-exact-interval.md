---
id: TC-2270
title: "A remainder record and its replay keep the exact interval"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-062
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-098
    type: references
---
# TC-2270: A remainder record and its replay keep the exact interval

## Description

Verify that a `mod` node keeps its exact interval as its checked type in every
position, that its lowered application is typed `Integer` with a narrow only
under a bounded consumer, that its requirement record's result bound,
type node and conversion context follow FR-062 for each kind of position, and
that the request item and the replay read that one bound. Scope:
FR-495-AC-21 and FR-495-AC-22.

## Test Procedure

1. Under `CheckMode::Linked` with no law header, check FR-495-AC-21's unit.
   For each function that holds a `mod`, read the checked `mod` node's type,
   the lowered `filament.op.integer.mod` application's `result_type`, the
   narrow (if any) directly over it, and the `mod` record's result bound
   `ValueType`, type node and conversion context. Read the `+` records of
   `derived` and `derivedb`.
2. Build the `fsl-route` request items from the records and compare each
   item's result bound node and conversion context with its record.
3. For each function that holds a `mod`, run it with `a = 100`, `b = 12`, build
   the plugin counterexample request at the `mod` record's occurrence with
   that assignment, replay it and settle the terminal record.

Test: `tests/it/modulo_claim_bounds.rs`
`modulo_claim_bound_context_and_replay_agree_for_each_consumer`, tagged
`#[trace("TC-2270", "FR-495-AC-21", "FR-495-AC-22", "FR-062-AC-13")]`.

## Expected Results

- Step 1: every checked `mod` node has type `Int[0, 11]`, including under the
  `Integer` result of `integer`, the `Integer` parameter of `argw`, the
  comparison of `cmp`, the `let` of `bound` and the `+` of `derived` and
  `derivedb`; every lowered `mod` application has the builtin `Integer` node
  as `result_type`. `equal`, `wider` and `arg` have one narrow over the
  application and bounds `Int[0, 11]`, `Int[0, 20]` and `Int[0, 20]` with the
  narrow's `type_argument` node and `expression` occurrence. The other six
  have no narrow over the application and bound `Int[0, 11]` with the
  graph's `Int[0, 11]` type node and no conversion context. `derived`'s `+`
  record is `Integer` with no context; `derivedb`'s is `Int[1, 12]` with its
  narrow's occurrence. Each bound's `ValueType` and type node name one type.
- Step 2: each item equals its record in result bound node and conversion
  context.
- Step 3: each replay is `inconclusive` with cause `Verdicts`, proved
  `violation`, replayed `success`, evaluated value `true`; none is `NoValue`.
  Each terminal record is `inconclusive` with cause `replay-parity`, naming the
  plugin.
