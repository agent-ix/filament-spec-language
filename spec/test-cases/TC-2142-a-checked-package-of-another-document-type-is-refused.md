---
id: TC-2142
title: "A checked package of another document type is refused"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-760
    type: verifies
---
# TC-2142: A checked package of another document type is refused

## Description

A checked package of another document type is refused.

Scope: FR-760-AC-4.

## Test Procedure

Read the pinned standard's `positive-control-operations.json`, replace `contract_version` with `filament.checked-package/v3` and read the bytes through `filament_contract_model::read_checked_package` with bounded limits and empty evidence.

Tag the tests `#[trace("TC-2142", "FR-760-AC-4")]`.

## Expected Results

- The result is `Refused` with code `UnknownContractVersion` and `contract_version` `filament.checked-package/v3`; no package is admitted.
