---
id: TC-2723
title: "Identity of uncertainty and lateness inputs, and a refused correction through a run"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-966
    type: verifies
---
# TC-2723: Identity of uncertainty and lateness inputs, and a refused correction through a run

## Description

Verify that the cover, interval stamps, arrival watermarks and the late-data cutoff are identity members, and that a run reports a relation refusal at stage `evaluate` with no result.

Scope: FR-966-AC-5, FR-966-AC-7.

## Test Procedure

1. Vary each input of FR-966-AC-5 alone over FR-966-AC-4's request.
2. Re-send `r1`'s own request as `Superseding(r1)`, then a `Superseding` naming an unknown result.

Test: `tests/it/temporal_uncertain_late.rs` `uncertainty_and_lateness_inputs_enter_both_identities`, `a_refused_correction_writes_no_result`.

## Expected Results

- Step 1: every variation gives a distinct `result_identity` and `input_identity`.
- Step 2: `invalid_runtime_input`/`invalid-value` at `evaluate`, member `relation`, `cause` `unchanged-input` then `predecessor-unknown`; no result document and no ledger.
