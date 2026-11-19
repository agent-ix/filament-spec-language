---
id: TC-3028
title: "The plan coordinator dispatches each step by its method"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-753
    type: verifies
---
# TC-3028: The plan coordinator dispatches each step by its method

## Description

Verify that `fsl_analyze::plan::execute_plan` negotiates each FR-290 step
through the caller's `Negotiate`, runs a language method on its own route,
settles a step with no registered engine `unsupported` and blocks its
dependents. Scope: FR-753-AC-5.

## Test Procedure

1. Execute `Release` with an engine set holding no `temporal-satisfaction`
   engine, a recording `Negotiate`, seed 7 and `max_trace_length` 8.
2. Execute `Release` with a plan document whose `package` is another
   identity, seed 7 and `max_trace_length` `Some(8)`; then the same with
   `max_trace_length` `None`.

## Expected Results

- Step 1: `safe` `unsupported` with
  `unsupported_projection`/`unsupported-requested-capability`; `tested`
  `tested`, run by `fsl-eval`'s model-based test entry with `max_cases` 500,
  seed 7 and `max_trace_length` 8; `timely` `blocked` by `safe`; the `Negotiate` saw only `safe`.
- Step 2: a whole-plan refusal naming both identities, and no step runs, in both runs: the identity mismatch is refused before the missing `max_trace_length`.
