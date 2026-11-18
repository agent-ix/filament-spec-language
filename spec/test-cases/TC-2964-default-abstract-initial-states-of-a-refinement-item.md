---
id: TC-2964
title: "A refinement item with no abstract subject starts from every admissible completion"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-630
    type: verifies
---
# TC-2964: A refinement item with no abstract subject starts from every admissible completion

## Description

Verify that a refinement item whose request carries no `abstract_subject`
takes as its abstract initial states every completion of the mapping's image
of each concrete initial state, its unfixed fields ranging over their types
under the abstract invariants and unit bounds; that the enumeration is
charged to one `max_candidates` total across all initial states; and that an
empty completion set is a refutation at position 0. Scope: FR-630-AC-5.

## Test Procedure

1. Over the Counter fixture, analyze the refinement `Hidden` (no field fixed,
   `inc` to `stutter`) over the snapshot `a` at 0 with no `abstract_subject`.
2. Repeat with `temporal.limits.max_candidates` 3.
3. Analyze `Hidden` over the snapshots `a` at 0 and `a` at 1 with
   `max_candidates` 4, then 8.
4. Analyze the refinement `Shifted` (`value = self.value + 3`) over the
   snapshot `a` at 1.

Test: `fsl-analyze/tests/it/analyze_family_cli.rs`.

## Expected Results

- Step 1: `proved`; the initial candidate set holds `value` 0, 1, 2 and 3.
- Step 2: `incomplete`, `resource-incomplete`, `limit-reached` naming
  `max_candidates`.
- Step 3: `incomplete` naming `max_candidates`, then `proved`.
- Step 4: `refuted` with `InitialNotAbstract{initial: 0}` at position 0 and a
  one-state counterexample that replays; never `declined`.
- Assertions compare the outcome document's typed members, never message text.
