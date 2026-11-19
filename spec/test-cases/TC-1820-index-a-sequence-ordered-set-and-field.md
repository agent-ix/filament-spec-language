---
id: TC-1820
title: "Index a sequence, an ordered set and a projected field"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-680
    type: verifies
---
# TC-1820: Index a sequence, an ordered set and a projected field

## Description

Verify FR-680-AC-1: S2 builds every index form, S3 types it by the operand and records the index obligation, and S4 lowers it to `filament.op.collection.index`.

## Test Procedure

Run S1 to S3 under `CheckMode::Linked` on functions whose bodies are `s[0]`, `r.xs[0]` and `o[1]` (guarded), then on `s[0]` with no fact, `t[0]` over a set, `n[0]` over an integer and `s[true]`. Stubs live in `fsl-semantics/tests/it/type_decisions.rs` and `fsl-conformance/tests/it/type_decision_cases.rs` (cases `index-accepted.native`, `index-ill-typed.native`), tagged `#[trace("TC-1820", "FR-680-AC-1", "FSpec-FR-144-AC-31")]` or `FSpec-FR-144-AC-32`.

## Expected Results

The admitted reads check with result type `Integer`; the refused ones give exactly the codes, causes and loci of FR-680-AC-1.
