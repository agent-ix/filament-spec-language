---
id: TC-2575
title: "Every inventory fact has a passing contract call test"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-911
    type: verifies
---
# TC-2575: Every inventory fact has a passing contract call test

## Description

Every inventory fact has a passing contract call test.

Scope: FR-911-AC-2.

## Test Procedure

Run `cargo test -p facade-contract-consumer -- --list` and then `cargo test -p facade-contract-consumer`. Compare the listed `cf_<code>_<nn>_*` names with the pinned inventory's fact ids.

Tag the tests `#[trace("TC-2575", "FR-911-AC-2", "FSpec-TC-1646", "FSpec-FR-661-AC-3")]`.

## Expected Results

- One test per inventory fact, none ignored, all passing.
- The protocol tests read, over one compiled protocol clause, the declared roles and channels, the causal relation, each channel's sending role, receiving role and message type, an `await` anchor, a choice's owner and visible expressions and a compensation's attempts maximum, each equal to the declared value.
