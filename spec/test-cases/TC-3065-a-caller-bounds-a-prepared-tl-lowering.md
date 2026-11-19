---
id: TC-3065
title: "A caller bounds a prepared TL lowering"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: verifies
---
# TC-3065: A caller bounds a prepared TL lowering

## Description

Verify that a prepared `tl_lowering` request exposes its clause request so a
caller can apply a bound to it, that the lowering runs under the bound as set,
and that setting one bound changes nothing else. Scope: FR-931-AC-15.

## Test Procedure

1. Write FR-931-AC-1's `Low` unit, the ConfigVersion model and the request
   `{"clause": "Low"}`; run it through `fsl_command::command::run` and keep the
   record.
2. Prepare the request through `prepare_run` with default limits and assert
   `PreparedRun::TlLowering`; run it through `execute_tl_lowering` unchanged.
3. Prepare it again, copy the request's selection, stage limits, observation
   limits and accounting limits, set `s1.nodes` to 1 on
   `clause_request_mut().limits` through `CallerLimits::set_bound`, and compare
   every copied member except `s1.nodes`; then run it through
   `execute_tl_lowering`.
4. Prepare it again, set `s1.nodes` to 1 and then to 50000, and run it.

Test: `tests/it/request_surface_s1_16.rs`
`a_caller_bound_reaches_a_prepared_tl_lowering`.

## Expected Results

- Step 2: the record is byte-equal to step 1's, exit 0.
- Step 3: the other members are unchanged; the run settles the typed limit
  outcome naming `s1.nodes` with configured value 1, category `incomplete`,
  exit 22, and no lowering record. Fails today: `PreparedLowering` has no
  `clause_request_mut`.
- Step 4: the record is byte-equal to step 1's, exit 0.
