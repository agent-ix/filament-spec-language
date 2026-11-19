---
id: TC-390
title: "FamilyOutcome, FamilyResult and EvalOutcome live once in filament-eval, no FSL layer below 5 names them, and the check core names no family cause"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-090
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-400
    type: verifies
---
# TC-390: FamilyOutcome, FamilyResult and EvalOutcome live once in filament-eval, no FSL layer below 5 names them, and the check core names no family cause

## Description

Verify FR-090-AC-9. `FamilyOutcome`, `FamilyResult` and `EvalOutcome` are
each defined once, in the layer-E crate `filament-eval` (ADR-011 §6.1,
FR-401), and no FSL crate defines any of them. Layer 1 (`fsl-cst`), layer 2
`forms`, layer 3 `fsl-semantics` and layer 4 `fsl-package` never reach any
of the three. K (`filament-exact`) and SV (`filament-semantic-value`) cannot reach
them, because of the crate DAG. The `check` core names no family cause type: it holds the
`ProtocolClause` snapshot cause, `ModelRefusal` and the `StateModel`
undefined cause only through `CatalogCoded` and `UndefinedCoded` (ADR-013
O-16, O-17). Scope: FR-090-AC-9.

The test uses the same resolved-import and definition-scan approach as
TC-256, TC-170 and TC-176 (`xtask/src/import_graph.rs`,
`xtask/src/definition_scan.rs`). A `use` scan alone is not enough, because a
fully-qualified inline path such as `crate::family::FamilyResult` with no
`use` line would pass it.

This catches five faults: a second definition, such as an FSL copy under
`value::expression` or `family`; a `fsl-semantics` module that imports the
family outcome to report a query result, which makes the checker depend on
the evaluation leaf; F growing an evaluation-aware category map; the kernel
gaining a family variant; and a `check`-core item that names a family cause
type, which turns `FamilyResult` back into a shared cause list.

## Test Procedure

1. Scan every `.rs` file of every member of the FSL workspace (each
   `members` entry of the root `Cargo.toml`, the root crate's `src/`
   included) for an item definition named `FamilyOutcome`,
   `FamilyResult` or `EvalOutcome` (`enum`, `struct` or
   `type`). Use the `syn`-based definition scan the repository already has
   (`xtask/src/definition_scan.rs`).
2. Resolve every `use` edge and every inline path under `fsl-cst/src/`,
   `fsl-forms/src/`, `fsl-semantics/src/` and `fsl-package/src/`, and check
   whether any of them names one of the three types.
3. Resolve every `use` edge and every inline path under the `check` core, and
   check whether any of them names the `ProtocolClause` snapshot cause type,
   `ModelRefusal` or the `StateModel` undefined cause type.
4. Read every dependency table of `fsl-foundation/Cargo.toml`,
   `fsl-cst/Cargo.toml`, `fsl-forms/Cargo.toml`, `fsl-semantics/Cargo.toml`
   and `fsl-package/Cargo.toml`, and the `[dependencies]` table of
   `fsl-eval/Cargo.toml`.

5. Read every dependency table of every workspace member and of a fixture
   manifest in which `fsl-eval` enables `filament-checked`'s `construct`
   feature (FR-400-AC-1).

Tag the layering test `#[trace("FR-090-AC-9", "TC-390")]` and the `construct`
table check `#[trace("FR-400-AC-1", "TC-390")]`.

## Expected Results

- Step 1 finds no definition of any of the three types in any workspace
  member. (That each resolves at its `filament_eval::` path is FR-401-AC-3.)
- Step 2 finds no edge.
- Step 3 finds no edge.
- Step 4 finds that none of the five layer F to 4 manifests names
  `filament-eval` in any table. `fsl-foundation` may name `filament-exact` and
  `filament-semantic-value`, and `fsl-cst` may name `fsl-foundation` and
  `filament-exact` (ADR-011 §6.1). `fsl-package`'s `[dependencies]` are
  exactly `ix-trace-rs`, `fsl-foundation`, `fsl-semantics`,
  `quire-canonical`, `filament-contract-model`, `filament-exact`,
  `filament-semantic-value`, `serde` and `thiserror` (ADR-011 X-7), plus
  `filament-checked` once FR-400's X-13 lands; its `[dev-dependencies]` may also
  name `fsl-forms`. `fsl-eval`'s `[dependencies]` are exactly `ix-trace-rs`,
  `fsl-foundation`, `fsl-package`, `fsl-semantics`, `quire-canonical`,
  `filament-exact`, `filament-semantic-value`, `serde` and `thiserror` (ADR-011
  X-8, FR-101), plus `filament-checked` once X-13 lands, `filament-eval` once
  X-14 lands and `filament-observation` once FR-861 lands (ADR-011 X-8); it has no `[build-dependencies]`, and its
  `[dev-dependencies]` may also name `fsl-forms` and `fsl-cst`, and no other
  workspace crate.
- Step 5 finds `fsl-semantics` as the only workspace member that enables
  `filament-checked`'s `construct` feature in any dependency table, and the
  fixture manifest fails the check, naming `fsl-eval` and the table.
