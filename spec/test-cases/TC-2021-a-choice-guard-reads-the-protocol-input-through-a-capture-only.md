---
id: TC-2021
title: "A choice guard reads the protocol input through a capture only"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: verifies
---
# TC-2021: A choice guard reads the protocol input through a capture only

## Description

Verify that a decision over the protocol input reads a protocol capture of it with no listing, and that a direct read of `over` refuses at the read with the read as the reported region. Scope: FR-218-AC-42.

## Test Procedure

| # | Change | Expected |
| --- | --- | --- |
| 1 | `capture ready: Boolean = input.ready;`, `choice Pick by R visible ()` with guards `ready` and `not ready` | checks |
| 2 | the same choice with guards `input.ready` and `not input.ready` | two `ill_typed`/`operator-ineligible` refusals naming the guard slot; each refusal region's text is exactly `input.ready` |
| 3 | row 2's refusal regions | none covers the guard block, the `choice` or the protocol declaration |

## Expected Results

- The region asserted is the checker's refusal region (`PackageDeclarations::regions`), the one every reporting surface shows.
