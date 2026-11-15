---
id: TC-2146
title: "An unbound inventory fact fails the contract check"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-761
    type: verifies
---
# TC-2146: An unbound inventory fact fails the contract check

## Description

An unbound inventory fact fails the contract check.

Scope: FR-761-AC-3.

## Test Procedure

Copy `docs/consumer-facades.json` to a scratch file without the `CF-PR-04` binding and run `cargo xtask consumer-facades --bindings <scratch>`.

Tag the tests `#[trace("TC-2146", "FR-761-AC-3", "FSpec-TC-1646", "FSpec-FR-661-AC-3")]`.

## Expected Results

- Exit 1.
- The report names `CF-PR-04` as unbound.
