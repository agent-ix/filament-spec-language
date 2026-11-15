---
id: TC-1903
title: "reaches over every destination multiplicity"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-365
    type: verifies
---
# TC-1903: reaches over every destination multiplicity

## Description

Verify FR-365-AC-3: `reaches` admits unbounded, ordered and multiplicity-less destination ends of the receiver's own type.

## Test Procedure

Over fixture R of TC-1046, check `not reaches(self, self, crowd)`, `not reaches(self, self, ranked)` and `not reaches(self, self, bare)` as invariants on `M::Order`. Test: `fsl-semantics/tests/it/checker_rulings.rs`.

## Expected Results

Each checks as `Boolean`; none refuses `unsupported_construct`.
