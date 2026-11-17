---
id: TC-2533
title: "A channel is carried over a systems connection between Port roles"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-897
    type: verifies
---
# TC-2533: A channel is carried over a systems connection between Port roles

## Description

Verify that a channel is carried over a systems connection between Port roles. Scope: FR-897-AC-1, FR-897-AC-2, FR-897-AC-3, FR-897-AC-4, FR-897-AC-5.

Catches a connection accepted as a role binding, or a channel checked against the wrong port direction.

## Test Procedure

1. Compile `channel Feed from Outlet to Inlet via Link`; then `via Missing`, `via Pipe`, the reversed channel, a `from` role on `Sys`, `carries Note`, a `related by Link` clause and `role R on M::link`.

## Expected Results

- Step 1: The channel's `connection` argument references `link` and the package is admitted; each variant refuses at the stated span with the stated code and retained members.
