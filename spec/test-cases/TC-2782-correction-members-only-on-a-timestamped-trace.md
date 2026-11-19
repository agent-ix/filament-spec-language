---
id: TC-2782
title: "Correction members are refused off a timestamped trace and a correction needs a ledger"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-985
    type: verifies
---
# TC-2782: Correction members are refused off a timestamped trace and a correction needs a ledger

## Description

Correction members are refused off a timestamped trace and a correction needs a ledger.

Scope: FR-985-AC-3, FR-985-AC-4.

## Test Procedure

Run the `finite` selections of FR-985-AC-3 and the ledger-less correction of FR-985-AC-4 through `command::run`. Test: `tests/it/temporal_selection_members.rs`.

## Expected Results

- `invalid-request` at stage `request`, exit 20, naming each member.
- Stage `evaluate`, `invalid_runtime_input`/`invalid-value`, member `relation`, cause `no-ledger`, no result document.
