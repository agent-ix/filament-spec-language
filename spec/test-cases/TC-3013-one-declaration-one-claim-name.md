---
id: TC-3013
title: "Each claim declaration is one name in the claim namespace"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
---
# TC-3013: Each claim declaration is one name in the claim namespace

## Description

Verify that S3 refuses two claim-bearing declarations of one name, two
probabilistic claims included, and that the package writer's rows for one
declaration (a derived conjunction, a step relation, a hyper clause) are
admitted and resolve to the declaration's own node. Scope: FR-940-AC-9.

## Test Procedure

1. Check a unit declaring two probabilistic claims named `Stable` with
   thresholds `0.5` and `0.25`. Then name the second `Other`, compile it and
   read `claim_names`.
2. Compile FR-105-AC-14's unit, with postconditions `Q1` and `Q2` on one
   field, read its package with the strict reader, and call `claim_name("Q1")`
   over the source route and over the package.
3. Compile a unit with a step relation, read its package, and call
   `claim_name` with the relation's name.
4. Compile `NonInterference`, read its package, and call
   `claim_name("NonInterference")`.

Test: `fsl-package/src/emit/tests/probabilistic.rs`, `fsl-analyze/tests/it/analyze_package_selection.rs`.

## Expected Results

- Step 1: `ambiguous_declaration`/`ambiguous-name` at the second `Stable`,
  naming both. Renamed, the unit checks and its package has one row per claim.
- Step 2: the package is admitted, and both routes return the occurrence on
  `Q1`'s own node, never the derived node.
- Step 3: rows for the `declaration` and `claim` occurrences carry one name
  and region, the package is admitted, and the lookup returns the `claim`
  occurrence.
- Step 4: the package is admitted, and the lookup returns the
  `hyperproperty` node's `claim` occurrence.
