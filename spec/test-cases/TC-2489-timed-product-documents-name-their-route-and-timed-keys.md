---
id: TC-2489
title: "Timed-product documents name their route and timed keys"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-920
    type: verifies
---
# TC-2489: Timed-product documents name their route and timed keys

## Description

Timed-product documents name their route and timed keys.

Scope: FR-920-AC-1, FR-920-AC-2, FR-920-AC-3, FR-920-AC-4.

## Test Procedure

Prove `Deadline` over `Retx` with the guards `self.x >= 1 ms` and `self.x > 1 ms` through `analyze_items`, read the retained document, check it with `check_probability_certificate_document`, round-trip its region keys, add a `components` member, and emit the unit's package; prove `>=` and `<=` workload bounds over a model with a free delay choice and over one with none, state the other spellings, and replay a witness of the other extremum. Tests: `fsl-analyze/tests/it/timed_product_certificate.rs` `digital_document_names_route_scale_and_clocks`, `region_document_keeps_fraction_order`, `timed_document_has_no_component_member` (negative), `a_workload_with_a_delay_choice_is_decided_on_its_extremum`, `a_witness_of_the_other_extremum_settles_parity` (negative), `a_workload_with_no_choice_is_spelled_workload`.

Tag each test with `#[trace("TC-2489", ...)]` and the FR-920 criterion it asserts.

## Expected Results

- The digital document has `objective.timed` `{digital, ms, 1, deadline}`, `x` in 0..2, deadline 0..5, initial `99/100`, and is accepted.
- The strict-guard document is `region`; keys differing only in fraction stay distinct.
- An added `components` member is rejected `Malformed`; the package has no certificate node.
- A workload `>=` bound over a delay choice is proved as `workload-minimum` and a `<=` bound as `workload-maximum`, each with its extremum's value; a bound between the extrema is refuted with the recomputed mass; the other spellings, and `workload` over a product with a delay choice, are rejected; a witness of the other extremum settles Parity; a product with one choice at every state is `workload`.
