---
id: TC-745
title: "The checked-input gate rejects pre-check signatures and reconstruction and passes the workspace"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-270
    type: verifies
---
# TC-745: The checked-input gate rejects pre-check signatures and reconstruction and passes the workspace

## Description

Verify FR-270 by planting forbidden signatures and stage calls in copies of
the scanned trees, planting look-alikes that are allowed, and running the
gate over the real workspace.

Scope: FR-270-AC-1 to FR-270-AC-5.

## Test Procedure

Copy the scanned crates' `src/` trees into a temporary directory and run the
gate's scan over the copy, planting one item per run unless a step says
otherwise.

1. In `fsl-eval`: `pub fn plant(e: &fsl_forms::Expression)`; the same after
   `use fsl_forms::Expression as Expr;`; the same through `type Node =`
   a CST node type; the same through `fsl_semantics::plant::Expression`,
   where `fsl-semantics` holds `pub use fsl_forms::Expression;`. In
   `fsl-route`: a `pub` method taking
   `Option<Vec<&PackageDeclarations>>`. In `fsl-replay` outside `spine`: a
   `pub fn` taking a `filament_contract_model` package type.
2. In `fsl-eval`: a private function with only checked parameters whose
   body calls a `fsl-cst` parse function. In `fsl-route`: one that calls
   `PackageDeclarations::check`; one that calls
   `<PackageDeclarations>::check`. In `fsl-eval`: one that calls
   `checked_dispatch_operation(..)?.check(limits)`. In `fsl-replay`'s
   `spine`: the same parse call.
3. In `fsl-eval`: `pub fn plant(k: StateClauseKind)`;
   `pub fn plant(l: fsl_cst::Limits)`; a `#[cfg(test)]`
   module with `fn t(e: fsl_forms::Expression)`. In `fsl-replay` outside
   `spine`: a function that builds `fsl_cst::Limits`. Then step 1's first
   two plants together.
4. Run `cargo xtask checked-input` over the workspace. Run the command's
   entry over a copy with step 1's first item planted in `fsl-eval`.
5. Run the witness round-trip tests of TC-182, the `/2` frame round-trip
   test of TC-514 and the outcome-document test of TC-770, which carry
   `FR-270-AC-5`.

Tag the tests `#[trace("TC-745", "FR-270-AC-n")]`; the three existing
round-trip tests also carry `FR-270-AC-5`.

## Expected Results

- Step 1: six runs, each exiting non-zero with one `signature` finding
  naming the planted file and line, the entry and the type.
- Step 2: four `reconstruction` findings naming the function and the called
  stage function; no finding for the `spine` plant.
- Step 3: no finding for the four look-alikes; two findings for the two
  plants together.
- Step 4: no finding and exit 0; the planted copy fails with a finding exit
  status.
- Step 5: every listed test passes under `make ci`'s default-features lane.
