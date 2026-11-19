---
id: TC-2751
title: "A malformed evidence ledger file refuses naming evidence_ledger"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-975
    type: verifies
---
# TC-2751: A malformed evidence ledger file refuses naming evidence_ledger

## Description

Verify that a run refuses an `evidence_ledger` file that is not a closed,
ordered and consistent `filament.finite-window-evidence-ledger/1` document,
and a path that does not exist, before evaluation.

Scope: FR-975-AC-4.

## Test Procedure

1. Write `r1`'s ledger with an unknown entry member, with `format`
   `filament.finite-window-evidence-ledger/2`, with an entry missing `key`,
   with a repeated key, with entries out of key order, and with `lineage` 1
   and an empty `superseded` list; run a `Superseding(r1)` request over each.
2. Run the same request with the ledger file removed.

Test: `tests/it/evidence_ledger_file.rs` `a_malformed_ledger_file_refuses_naming_evidence_ledger`.

## Expected Results

- Step 1: each refuses `invalid-request` at stage `request` naming
  `evidence_ledger`, with no result document and no ledger.
- Step 2: refuses at stage `request` naming the path.
