---
id: TC-2490
title: "The timed checker trusts no stated timed fact"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-921
    type: verifies
---
# TC-2490: The timed checker trusts no stated timed fact

## Description

The timed checker trusts no stated timed fact.

Scope: FR-921-AC-1, FR-921-AC-2, FR-921-AC-3, FR-921-AC-5, FR-921-AC-6.

## Test Procedure

Alter TC-2489's digital document: one value, the identity, `route`, `scale`, one key's cap, one removed row, one added unreachable row; check each. Tests: `fsl-analyze/tests/it/timed_product_certificate.rs` `altered_timed_value_is_rejected`, `stated_route_and_scale_are_compared`, `timed_rows_cover_exactly_the_rebuilt_product`, `an_infinite_expected_time_is_rederived_not_claimed` (all negative).

For FR-921-AC-6, add a row to the infinite document, and empty the rows of a finite expected-elapsed document.

For FR-921-AC-5, give each row of the route admission table and each entry of the locked-run value mapping to EN-5 and to the checker, and compare each answer with the route or value written by hand in the test.

Tag each test with `#[trace("TC-2490", ...)]` and the FR-921 criterion it asserts.

## Expected Results

- Value: `InequalityFails`; identity: `IdentityMismatch`.
- Route and scale: `ObjectiveMismatch`; cap: `Malformed` at `values`.
- Missing or extra row: `Malformed` at `values`.
- Table and mapping: EN-5 and the checker each give every hand-written route and value.
- A row in the infinite document, or no row in the finite one: `Malformed`.
