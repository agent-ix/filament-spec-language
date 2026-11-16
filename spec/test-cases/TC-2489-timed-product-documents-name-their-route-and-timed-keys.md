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

Scope: FR-920-AC-1, FR-920-AC-2, FR-920-AC-3.

## Test Procedure

Prove `Deadline` over `Retx` with the guards `self.x >= 1 ms` and `self.x > 1 ms` through `analyze_items`, read the retained document, check it with `check_probability_certificate_document`, round-trip its region keys, add a `components` member, and emit the unit's package. Tests: `fsl-analyze/tests/it/timed_product_certificate.rs` `digital_document_names_route_scale_and_clocks`, `region_document_keeps_fraction_order`, `timed_document_has_no_component_member` (negative).

Tag each test with `#[trace("TC-2489", ...)]` and the FR-920 criterion it asserts.

## Expected Results

- The digital document has `objective.timed` `{digital, ms, 1, deadline}`, `x` in 0..2, deadline 0..5, initial `99/100`, and is accepted.
- The strict-guard document is `region`; keys differing only in fraction stay distinct.
- An added `components` member is rejected `Malformed`; the package has no certificate node.
