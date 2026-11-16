---
id: TC-2576
title: "A wrong input to a bound accessor gives its typed refusal"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-911
    type: verifies
---
# TC-2576: A wrong input to a bound accessor gives its typed refusal

## Description

A wrong input to a bound accessor gives its typed refusal.

Scope: FR-911-AC-3.

## Test Procedure

In the protocol contract tests, call `fsl_package::protocol_view::channel_endpoints` with a role node, and each other node-taking bound accessor whose requirement gives a typed refusal with a node of another kind or an unknown key.

Tag the tests `#[trace("TC-2576", "FR-911-AC-3")]`.

## Expected Results

- Each call returns the accessor's typed refusal.
- No call panics or returns a default value.
