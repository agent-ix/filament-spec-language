---
id: FR-1018
title: "Qualify exports against golden files and optional TLC and Lean oracles"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-818 filament-specification/FR-815
    type: depends_on
---
# FR-1018: Qualify exports against golden files and optional TLC and Lean oracles

## Description

FSL SHALL hold FSpec FR-818's export conformance corpus under `tests/fixtures/export/` with reviewed golden `.tla`, `.lean` and records files, compare every export with them byte for byte, and run TLC and Lean as optional child-process oracles that print `<tool> not found` and skip when absent.

## Behavior

- No FSL crate SHALL depend on a TLA+ or Lean library.
- A TLC counterexample of a seeded violating model SHALL read back through the name map and replay natively.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1018-AC-1 | The counter export equals `tests/fixtures/export/counter.tla` and `counter.lean` byte for byte. | Test (TC-2868) |
| FR-1018-AC-2 | With TLC present, TLC's distinct-state count and verdict for the counter equal FSL's exploration; with it absent, the oracle prints `tlc not found` and the golden comparison still runs. | Test (TC-2868) |

## Dependencies

- FR-818 filament-specification/FR-815 (filament-specification).
