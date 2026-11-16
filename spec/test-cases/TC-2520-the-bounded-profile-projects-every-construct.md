---
id: TC-2520
title: "The bounded profile projects every construct and matches the fragment"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-890
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-575
    type: verifies
---
# TC-2520: The bounded profile projects every construct and matches the fragment

## Description

Verify that the bounded profile projects every construct and matches the fragment. Scope: FR-890-AC-1, FR-890-AC-7, FR-575-AC-5.

Catches a projector that still answers a PA-21 slug under `bounded`, or whose `bounded` result drifts from `bounded-static` on the fragment.

## Test Procedure

1. Run each FR-575-AC-1 protocol under profile `bounded` onto every role; run a fragment-only protocol under both profiles; run the `String`-binder and unimplemented-memory-model units under `bounded`.

## Expected Results

- Step 1: Each construct projects to `proved` with a total correspondence table and composed traces included in the global ones; the fragment protocol's two results differ only in the key's profile; the two limit units yield `evaluator-unsupported` with `UnenumerableRecord` and `WeakMemory`.
