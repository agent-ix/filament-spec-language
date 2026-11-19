---
id: TC-3013
title: "Systems part, port and allocation records are admitted"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-103
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-086
    type: verifies
---
# TC-3013: Systems part, port and allocation records are admitted

## Description

Verify that model intake and the assembler admit a domain package's systems
part, port and allocation records as model structure, that intake reads each
with its own fields, and that a unit protocol can bind a role to the admitted
Port. Scope: FR-103-AC-11.

## Test Procedure

1. Build a Semantic IR document for package `test/plant` with interface
   `Flow` (field `rate`), object types `Sys` and `Pump`, part `SysPump`
   (owner `Sys`, declared type `Pump`, multiplicity `[1, 1]`), port `PumpOut`
   (owner `SysPump`, interface type `Flow`, direction `out`, multiplicity
   `[1, 1]`) and allocation `PumpAlloc` (source `PumpOut`, target `SysPump`).
   Admit it and read its records.
2. Assemble and check, in linked mode, the unit `model N = "test/plant";`
   whose protocol `P` declares `role r on N::PumpOut`.

Tag both steps `#[trace("TC-3013", "FR-103-AC-11")]`.

## Expected Results

- Step 1: the records hold one Part `SysPump` with owner `Sys`, type `Pump`
  and multiplicity `[1, 1]`, one Port `PumpOut` with owning part `SysPump`,
  interface type `Flow`, direction `out` and multiplicity `[1, 1]`, and one
  Allocation `PumpAlloc` with source `PumpOut` and target `SysPump`.
- Step 2: assembly succeeds with no `UnsupportedModelMember` refusal and
  checking succeeds; protocol `P` has one role whose model member is the
  declaration key `ix://test/plant/PumpOut` and whose model selection is
  `test/plant`.
