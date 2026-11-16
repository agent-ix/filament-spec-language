---
id: TC-2502
title: "Terminal over under each, non-object over inputs and every repeat form are written"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-880
    type: verifies
---
# TC-2502: Terminal over under each, non-object over inputs and every repeat form are written

## Description

Verify the remaining FR-880 forms and the S3 refusal of a pre-test decision that reads its body. Scope: FR-880-AC-6, FR-880-AC-7, FR-880-AC-8, FR-880-AC-9.

## Test Procedure

1. Compile `terminal when { present(deref(input).trail) }` under `on each (t: Tick)`.
2. Compile protocols over `Int[0, 3]`, `Boolean`, `Parts::Handoff`, `Option<Integer>`, `Sequence<Integer>`, `Option<M::Order>` and `Sequence<M::Order>`.
3. Compile the post-test `Loop` reading `got.ok`, a `repeat` stepping only through a nested `repeat`, and a post-test `repeat` with a stepless body cycle.
4. Check pre-test `repeat`s (guard `true`) whose `visible`, `invariant` or `variant` reads `got`.

## Expected Results

- Step 1: compiles; the condition reads the `over` binder.
- Step 2: the first five have no `owner_model`, the last two name `M::Order`; all are admitted.
- Step 3: the first has `test` `post`, the second compiles, both are admitted; the third refuses at S3 by the progress rule.
- Step 4: `missing_declaration`/`missing-name` at the read, at S3.
