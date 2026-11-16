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
  - target: ix://agent-ix/filament-spec-language/FR-404
    type: references
---
# TC-2270: A remainder record and its replay keep the exact interval

## Description

Verify that an integer `mod`, `div` or `rem` node keeps its exact interval as
its checked type in every position, that its lowered application is typed
`Integer` with exactly one narrow under a bounded consumer (over the
application, or over the binder read that consumes it), that its requirement
record's result bound, type node and conversion context follow FR-062 for each
kind of position, and that the request item and the replay read that one
bound. Scope: FR-495-AC-21, FR-495-AC-22 and FR-062-AC-16.

## Test Procedure

1. Under `CheckMode::Linked`, check FR-495-AC-21's `mod` unit (no law header)
   and its `div`/`rem` unit (Euclidean law header). For each function that
   holds an integer division, read the checked division node's type, the
   lowered application's `result_type`, every narrow over the application or
   over a read of its binder, and the division record's result bound
   `ValueType`, type node, conversion context and guards. Read the `+`
   records of `derived` and `derivedb`.
2. Check FR-062's RR-2 and RR-6 units and read each record's result bound
   `ValueType` and type node.
3. Build the `fsl-route` request items from the records of step 1 and compare
   each item's result bound node and conversion context with its record.
4. For each function of step 1, call `run_with_replay_context` under
   `CallerLimits::default()` with `a = 100`, `b = 12` (`ifb`: `a = 9`,
   `c = 4`), build `PluginReplayContext::request` for the division record's
   member with the same assignment and the plugin `acme`, replay it under the
   `CallerLimits`' `replay` limits and settle the terminal record.

Test: `tests/it/modulo_claim_bounds.rs`
`modulo_claim_bound_context_and_replay_agree_for_each_consumer`, tagged
`#[trace("TC-2270", "FR-495-AC-21", "FR-495-AC-22", "FR-062-AC-16")]`.

## Expected Results

- Step 1: every `mod` node has type `Int[0, 11]` (`ifb`'s `Int[0, 9]`), the
  `div` nodes `Int[-100, 100]` and the `rem` nodes `Int[0, 11]`, including
  under `Integer` results and parameters, the comparison, the `let`s and the
  `+`; every lowered division application has the builtin `Integer` node as
  `result_type`. `equal`, `wider`, `arg`, `ifb`, `qb` and `rb` have exactly
  one narrow directly over the application and bounds `Int[0, 11]`,
  `Int[0, 20]`, `Int[0, 20]`, `Int[0, 9]`, `Int[-100, 100]` and
  `Int[-11, 11]` with the narrow's `type_argument` node and `expression`
  occurrence; `ifb`'s record has the guard `c != 0` true. `letb` has exactly
  one narrow, over the read of `r`, holding no record, and its `mod` record is
  `Int[0, 11]` with the graph's `Int[0, 11]` node and no context. `integer`,
  `argw`, `cmp`, `bound`, `derived`, `derivedb`, `qi` and `ri` have no narrow
  over the application or a read of it, and bounds `Int[0, 11]` (`qi`:
  `Int[-100, 100]`) with the graph's type node of that interval and no
  context. `derived`'s `+` record is `Integer` with no context; `derivedb`'s
  is `Int[1, 12]` with its narrow's occurrence.
- Steps 1 and 2: each bound's `ValueType` and type node name one type; RR-2's
  node is its narrow's `Int[0, 10]` node and RR-6's the builtin `Integer`
  node.
- Step 3: each item equals its record in result bound node and conversion
  context.
- Step 4: each request's `occurrence_key` is the member's; each replay is
  `inconclusive` with cause `Verdicts`, proved `violation`, replayed
  `success`, evaluated value `true`; none is `NoValue`. Each terminal record
  is `inconclusive` with cause `replay-parity`, naming `acme`.
