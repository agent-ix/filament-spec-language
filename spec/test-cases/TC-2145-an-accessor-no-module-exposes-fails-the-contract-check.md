---
id: TC-2145
title: "An accessor no module exposes fails the contract check"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-761
    type: verifies
---
# TC-2145: An accessor no module exposes fails the contract check

## Description

An accessor no module exposes fails the contract check.

Scope: FR-761-AC-2.

## Test Procedure

Copy `docs/consumer-facades.json` to a scratch file with `CF-PR-03`'s accessors replaced by `["no_such_accessor"]`, and run `cargo xtask consumer-facades --bindings <scratch>`.

Tag the tests `#[trace("TC-2145", "FR-761-AC-2", "FSpec-TC-1646", "FSpec-FR-661-AC-3")]`.

## Expected Results

- Exit 1.
- The report names `CF-PR-03` and `fsl_package::protocol_view::no_such_accessor`.
