---
id: TC-3026
title: "Declarations of a shared clause node keep their own claim occurrence on both routes"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
---
# TC-3026: Declarations of a shared clause node keep their own claim occurrence on both routes

## Description

Verify that two structurally equal temporal clauses that share one node are numbered per node in source order, that both routes resolve each name to its own `claim` occurrence, and that negotiation, settlement and replay key on that whole occurrence. Scope: FR-940-AC-10.

## Test Procedure

1. Compile AC-4's Counter unit declaring `BelowThree` and then `AtMostTwo`, both `always holds(c.value <= 2)`, through a compile request carrying the `handoff` member.
2. Call `claim_name("BelowThree")` and `claim_name("AtMostTwo")` over the `AdmittedPackageRead` and over the `BarePackage` of the handoff directory, and read the source checked clauses' `claim` origins and the package's `temporal_clause_names` rows.
3. Repeat steps 1 and 2 with the two declarations swapped in source.
4. Analyze `[{"claim": "BelowThree"}, {"claim": "AtMostTwo"}]`, then `[{"claim": "AtMostTwo"}]`, as a `program` request and as a `package` request over the handoff directory.

Test: `fsl-analyze/tests/it/analyze_package_selection.rs`.

## Expected Results

- Step 1: the package holds one `temporal_clause` node for both clauses.
- Step 2: `BelowThree` is occurrence 0 and `AtMostTwo` occurrence 1 on both routes, equal to the checked origins and the name rows.
- Step 3: `AtMostTwo` is occurrence 0 and `BelowThree` occurrence 1.
- Step 4: every item is negotiated and settles `refuted` with `replay.reproduced` `true` at its own occurrence key; item records, counterexamples and replays are equal across the routes; no item is `unsupported` or `invalid-request`.
