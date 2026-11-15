---
id: TC-2147
title: "A binding with no inventory fact fails the contract check"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-761
    type: verifies
---
# TC-2147: A binding with no inventory fact fails the contract check

## Description

A binding with no inventory fact fails the contract check.

Scope: FR-761-AC-4.

## Test Procedure

Copy `docs/consumer-facades.json` to a scratch file with an added protocol binding `{"fact": "CF-PR-99", "accessors": ["protocol_view"]}` and run `cargo xtask consumer-facades --bindings <scratch>`.

Tag the tests `#[trace("TC-2147", "FR-761-AC-4", "FSpec-TC-1646", "FSpec-FR-661-AC-3")]`.

## Expected Results

- Exit 1.
- The report names `CF-PR-99` as a binding with no inventory fact.
