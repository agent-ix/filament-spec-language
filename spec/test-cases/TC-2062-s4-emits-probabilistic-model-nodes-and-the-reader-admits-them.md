---
id: TC-2062
title: "S4 emits probabilistic model nodes and the reader admits them"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-185 FR-186 FR-740
    type: verifies
---
# TC-2062: S4 emits probabilistic model nodes and the reader admits them

## Description

Verify that S4 emits random parameters, workloads, rewards and delays as v2 `probabilistic` nodes, that the reader rebuilds the checked records, and that any S4 refusal about them carries the declaration's locus.

Scope: FR-185-AC-7, FR-185-AC-12, FR-186-AC-5 (emission), FR-740-AC-1 to FR-740-AC-4.

## Test Procedure

1. Compile `Service` through S4; list its `probabilistic` nodes, occurrences and bodies; read the package back.
2. Over the read package, call `weighted_steps` at FR-187-AC-1's state under `Steady`.
3. Compile a `time dense` unit with a `discrete` delay; a workload with `Server::reset: 0`; `Steady` with its weight lines reordered.
4. Emit a claim `under Steady` with `Service` and a claim followed by a reward declaration; read the package with the strict reader.
5. Force an S4 refusal concerning a random parameter (for example a value family S4 cannot write) and read its diagnostic.

Tag the tests `#[trace("TC-2062", "FR-740-AC-n")]` (and `FR-185-AC-7`, `FR-185-AC-12`, `FR-186-AC-5`).

## Expected Results

- Step 1: the nodes FR-740-AC-1 lists; records equal to S3's.
- Step 2: FR-187-AC-1's six steps and probabilities.
- Step 3: one `delay_distribution` node admitted; weight 0 emitted; byte-identical output.
- Step 4: scheduler resolved to the `Steady` node; claim and reward nodes both emitted; package admitted.
- Step 5: locus member holds the declaration's source reference and label span, never `null`; message names the declaration in words.
