---
id: FR-270
title: "Gate execution, routing and replay entries to checked inputs"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-009
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-032
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-087
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-064
    type: traces_to
---
# FR-270: Gate execution, routing and replay entries to checked inputs

## Description

FSL SHALL provide `cargo xtask checked-input`, a gate that fails when an
execution, routing or replay stage entry names a pre-check representation
in its signature, or when shipped code in a stage-entry crate calls a
pre-check stage, as ADR-032 CK-1 to CK-5 decide. With FR-087's typestate
scans (TC-243, TC-244), which keep stage-output constructors private to
their stage, it closes the path: a stage entry receives a checked value,
and only a stage constructor produces one. The gate is a prerequisite of
`make ci`.

## Inputs

- The shipped source (test code excluded, as `typestate_scan` excludes it)
  of the workspace crates, read by `syn`.

## Outputs

- One finding per violation, each naming the file and line, the rule
  (`signature` or `reconstruction`), the stage entry or function, and the
  pre-check type or function it names.
- Exit status 0 when there is no finding, non-zero otherwise.

## Behavior

- The gate SHALL take as stage entries every `pub` function and `pub`
  method in the shipped code of `fsl-eval` and `fsl-route`, and every `pub`
  function of the `replay` facade crate `fsl-replay` outside its `spine`
  module. Membership comes from crate and module alone.
- The gate SHALL take as pre-check representations:
  - a type with fields, defined in `fsl-source`, `fsl-cst` or `fsl-forms`,
    that the return type of one of those crates' `pub` functions reaches;
  - `PackageDeclarations`, the receiver of the S3 stage constructor in
    TC-244's constructor table;
  - every type defined in `filament-contract-model` except its addressing
    values.
- The gate SHALL treat a fieldless enum from `fsl-source`, `fsl-cst` or
  `fsl-forms` (such as `StateClauseKind`) as closed vocabulary that S3
  carries forward; it is outside the set.
- The gate SHALL treat as an addressing value, outside the set, a
  `filament-contract-model` identity key or clock row that a stage entry
  selects with from a package the I2 reader already admitted, matched by
  exact type name: `CheckedNodeId` and `CheckedOccurrence` (the occurrence
  `{ role, ordinal }` of FR-516), `TemporalBinding` and `ClockConfiguration`
  (the `temporal_bindings` row of FR-522), and `SourceIdentity` (the
  selected formal identity of FR-514), and `NativeJoinBudget` (the
  caller-owned native allowance a stage entry that joins identities takes).
  None holds a checked node or term,
  so none can be executed without that reader. Every other
  `filament-contract-model` type stays in the set, also beside an exempt one.
- The gate SHALL treat as configuration, outside the set, a type that a
  `pub` free function of `fsl-source`, `fsl-cst` or `fsl-forms` takes as a
  parameter and that no such function's return type names, even when a
  stage output holds it (such as `fsl_cst::Limits`, which `ParsedSource`
  keeps as its effective limits).
- If a stage entry's receiver or parameter type mentions a pre-check
  representation at any depth, seen through `use … as` renames and `type`
  aliases as `typestate_scan` sees them, and through the `pub use`
  re-exports of the workspace crates the stage crates depend on, then the
  gate SHALL report a `signature` finding.
- If a shipped function in `fsl-eval`, `fsl-route`, or `fsl-replay` outside
  `spine` names a `pub` function of `fsl-source`, `fsl-cst` or `fsl-forms`,
  or the S3 stage constructor (also as `<PackageDeclarations>::check`),
  then the gate SHALL report a `reconstruction` finding, whatever the
  function's own signature.
- If such a function calls a method named like the S3 stage constructor
  (`.check(..)`), then the gate SHALL report a `reconstruction` finding,
  whatever the receiver's type, since a `pub` function can hand back the S3
  input without its type being named.
- The gate SHALL report every finding it meets before it exits.
- The gate SHALL derive both sets from the rules above applied to the
  scanned code.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-270-AC-1 | Planted signatures fail the gate: over a copy of the workspace's scanned trees with one planted item each, a `pub fn` in `fsl-eval` taking `&fsl_forms::Expression`; the same through `use fsl_forms::Expression as Expr`; the same through a `type` alias of a CST node; the same through a `pub use` of `fsl_forms::Expression` in `fsl-semantics`; a `pub` method in `fsl-route` taking `Option<Vec<&PackageDeclarations>>`; and a `pub fn` in `fsl-replay` outside `spine` taking a `filament-contract-model` package; and a `pub fn` in `fsl-eval` taking `&CheckedNodeId` beside a `&ContractPackage`, which gives exactly one finding naming `ContractPackage`, each make the gate exit non-zero with one `signature` finding naming the planted file and line, the entry and the type. | Test (TC-745) |
| FR-270-AC-2 | Planted reconstruction fails the gate: a private function in `fsl-eval` with a checked-only signature that calls a `fsl-cst` parse function, one in `fsl-route` that calls `PackageDeclarations::check`, one there that calls `<PackageDeclarations>::check`, and one in `fsl-eval` that calls `.check(..)` on the result of `checked_dispatch_operation(..)?`, each give one `reconstruction` finding naming the file, line, function and the called stage function. The same parse call planted in `fsl-replay`'s `spine` module gives no finding. | Test (TC-745) |
| FR-270-AC-3 | No false findings: a `pub fn` in `fsl-eval` taking `StateClauseKind`, a function in `fsl-replay` outside `spine` that constructs `fsl_cst::Limits`, a `pub fn` in `fsl-eval` taking `fsl_cst::Limits`, a `pub fn` in `fsl-eval` taking `filament_contract_model::CheckedNodeId`, a `pub fn` in `fsl-replay` outside `spine` taking `filament_contract_model::SourceIdentity`, and a `#[cfg(test)]` module in `fsl-eval` that takes a `fsl_forms::Expression`, give no finding. Two planted signatures in one run give two findings. | Test (TC-745) |
| FR-270-AC-4 | Over the FSL workspace as it is, `cargo xtask checked-input` reports nothing and exits 0, and with one FR-270-AC-1 signature planted in a copy of `fsl-eval`'s shipped code the command fails with a finding exit status. | Test (TC-745) |
| FR-270-AC-5 | The representative outcome-and-witness round-trip gate is these tests, which `make ci` runs through `ci-default-features`: the witness envelope's build, `to_packet` and `reconstruct` round trip, which fails when the reconstructed envelope differs from the built one (TC-182, FR-070-AC-3); the `/2` run-result frame change and grant write and read round trip, which fails when the value read back differs from the value written (TC-514, FR-267-AC-11); and the `filament-outcome/1` document a driver writes through the `fsl_replay` root, which has a writer and no reader, so its test fails when a written field differs from the expected one (TC-770). | Test (TC-745) |

## Dependencies

- [ADR-032](../decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md)
  CK-1 to CK-5.
- [FR-087](FR-087-typestate-and-cross-package-node-key.md) (the typestate
  scans and the S3 stage constructor table, TC-243, TC-244).
- ADR-011 §4 (checking precedes lowering) and §6.1 (the layers).

## References

- Owning ticket: Linear QSL-391. Implementation: Linear QSL-13.
