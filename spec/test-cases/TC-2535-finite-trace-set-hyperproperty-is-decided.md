---
id: TC-2535
title: "A finite trace-set hyperproperty is decided over supplied traces and replays"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-898
    type: verifies
---
# TC-2535: A finite trace-set hyperproperty is decided over supplied traces and replays

## Description

Verify that a finite trace-set hyperproperty is decided over supplied traces and replays. Scope: FR-898-AC-3, FR-898-AC-4, FR-898-AC-5, FR-898-AC-6.

Catches positions compared past a trace's end, or quantifiers evaluated out of order.

## Test Procedure

1. Decide `Det` over three agreeing traces, then with a disagreeing third trace; decide the two quantifier orders, four traces under bound 3, an open population, a read past a trace's end; replay the refutation and edited copies.

## Expected Results

- Step 1: `proved`, then `refuted` naming the traces and position 1; the orders differ; `requires-bound` naming 3; `inconclusive` twice; the refutation reproduces and the edits do not.
