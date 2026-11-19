---
id: FR-1018
title: "Qualify exports against golden files and optional TLC and Lean oracles"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-818
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-815
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: references
---
# FR-1018: Qualify exports against golden files and optional TLC and Lean oracles

## Description

FSL SHALL hold FSpec FR-818's export conformance corpus under `tests/fixtures/export/` with reviewed golden `.tla`, `.lean` and records files, compare every export with them byte for byte, and run TLC and Lean as optional child-process oracles that print `<tool> not found` and skip when absent.

## Behavior

- The corpus's counter SHALL be FR-756-AC-1's counter model: one
  `Counter` with operation `inc` and the state clause `value <= 5`, starting
  at `value = 0`, with `inc` enabled when `value < 5` and adding 1, so its
  reachable states are the six values 0 to 5. Its golden
  files are `tests/fixtures/export/counter.tla` and `counter.lean`.
- The export outcome reports no native state count or verdict (FSpec
  FR-817): an export explores no states. The TLC oracle SHALL compare the
  observer's `reachable_states` and `verdict` with FSL's own explicit-state
  exploration of the same model, the `exploration.states` count and the
  verdict of an `analyze` run over it (FR-621), so the comparison holds two
  independent explorations side by side.
- No FSL crate SHALL depend on a TLA+ or Lean library.
- A TLC counterexample of a seeded violating model SHALL read back through the name map and replay natively.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1018-AC-1 | The export of FR-756-AC-1's counter (`inc`, `value <= 5`) equals `tests/fixtures/export/counter.tla` and `counter.lean` byte for byte. | Test (TC-2868) |
| FR-1018-AC-2 | With TLC present, TLC's distinct-state count and verdict for the counter equal the `exploration.states` count and verdict of FSL's `analyze` run over the same model (six states, the clause holding); with it absent, the oracle prints `tlc not found` and the golden comparison still runs. | Test (TC-2868) |

## Dependencies

- FSpec FR-818 (the corpus and its oracles), FSpec FR-815 (target results),
  FSpec FR-817 (the export outcome), FR-621 (the analyze exploration
  counts).
