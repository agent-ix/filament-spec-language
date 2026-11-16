---
id: TC-2393
title: "A point-bound clock clause names its point anchor"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: verifies
---
# TC-2393: A point-bound clock clause names its point anchor

## Description

A point-bound clock clause names its point anchor.

Scope: FR-230-AC-50.

## Test Procedure

Assemble `pre Late using v on Config::ConfigVersion::attemptUpdate { self.x >= 1 ms }` with the execution point `pre{operation: "attemptUpdate"}`, check, emit and read the guard. Test: `fsl-package/src/emit/tests/timed_clauses.rs` `a_point_bound_clock_clause_names_its_point_anchor`.

Tag the tests `#[trace("TC-2393", "FR-230-AC-50")]`.

## Expected Results

- One `clock_guard` whose `operation` is the no-point anchor and whose `point_anchor` is the point-qualified anchor.
