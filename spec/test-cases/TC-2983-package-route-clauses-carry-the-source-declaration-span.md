---
id: TC-2983
title: "Package-route clauses carry the source declaration span"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: verifies
  - target: ix://agent-ix/filament-specification/FR-322
    type: verifies
---
# TC-2983: Package-route clauses carry the source declaration span

## Description

Verify that a clause built from a checked package's bytes equals the source
route's checked clause, `span` included, because the package records each
clause's whole declaration region. Scope: FR-850-AC-8, FSpec FR-322-AC-460
and FR-322-AC-461.

## Test Procedure

1. Compile a recipe unit's package and read each `temporal_clause_names`
   row's `declaration_region` against the unit's source text.
2. Build each clause of FR-940-AC-4's Counter handoff on the package route and
   compare it with the source route's checked clause.
3. Read the package with one row's `declaration_region` removed, and with it
   shrunk to one byte outside the name token.

## Expected Results

- Step 1: each region's source text is the whole declaration, from
  `temporal <Name>` to its closing `}`.
- Step 2: every clause is equal in every member, `span` included.
- Step 3: the reader refuses: `malformed_wire` for the removed member,
  `invalid_package`/`invalid-value` at the row's `declaration_region` for the
  shrunk one.
