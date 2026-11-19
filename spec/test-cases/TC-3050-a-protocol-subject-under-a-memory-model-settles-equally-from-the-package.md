---
id: TC-3050
title: "A protocol subject under a memory model settles equally from the package"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-1033
    type: verifies
---
# TC-3050: A protocol subject under a memory model settles equally from the package

## Description

Verify that the package route builds the protocol subject from the package's
protocol nodes, model selections and `parallel` memory defaults, so an item
over the `SB` store-buffering protocol, with and without a `tso` selection,
and a race-freedom item over its racing variant settle with the source
route's records, and that the package-built `SB` clause and subject equal the
source route's. Scope: FR-850-AC-11, FR-940-AC-14.

## Test Procedure

1. Compile FR-1033-AC-1's `SB` unit and FR-1033-AC-2's racing variant, each
   through a compile request carrying the `handoff` member.
2. Analyze `NoBothZero` over `SB` with `subject.protocol` and `memory`
   `[{"parallel": "SB::Both", "model": "tso"}]`, once over source and once
   with `package` selecting the `SB` handoff directory.
3. Repeat step 2 with no `memory`.
4. Analyze the race-freedom item `SB::Both` under `tso` over the variant's
   source and over its handoff directory.
5. Build the `SB` protocol clause and subject on both routes, with `roles`
   `w` and `c` bound to `test/litmus`, with and without the `tso` entry, and
   with `w` bound to a model the package does not select; repeat with a
   test-only package whose `SB::Both` `parallel` node carries `memory` `tso`.

## Expected Results

- Step 2: both routes settle `NoBothZero` `refuted` with equal records,
  counterexamples carrying both pending stores, replay targets and
  `replay.reproduced` `true`, and equal echoed `temporal.subject` and
  effective model; the package request is not refused for carrying `memory`.
- Step 3: both routes settle `NoBothZero` `proved` with equal records.
- Step 4: both routes settle the item `refuted` with equal `Race`
  counterexamples naming both writes.
- Step 5: the clauses are equal in every member; `w` binds `M::Cell` and `c`
  `M::Core` with equal selections, `over` universes and resolved models (`Sc`,
  then `Tso`); the unselected model settles `declined` `wrong-role-mapping`
  naming `w` on both routes; the test-only package's clause differs.
- No package-route record is `unsupported`.
