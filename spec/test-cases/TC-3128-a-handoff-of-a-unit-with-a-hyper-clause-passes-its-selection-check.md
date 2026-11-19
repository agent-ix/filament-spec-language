---
id: TC-3128
title: "A handoff of a unit with a hyper clause passes its selection check"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-517
    type: verifies
---
# TC-3128: A handoff of a unit with a hyper clause passes its selection check

## Description

Verify that the selection sidecar counts a hyper clause's body wrapper and
`hyperproperty` node, so a handoff of a unit with a hyper clause compiles
back and passes `Selection::check`. Scope: FR-517-AC-25.

## Test Procedure

1. `write` a handoff of a unit declaring `Bounded` and the hyper clause
   `Equal`; read the sidecar's node counts.
2. `compile` the directory and run `Selection::check` on its admitted
   package; run a `native-compile/1` request with the `handoff` member over
   the unit.
3. Set the sidecar's `temporal/temporal_clause` count to 1 and check again.

## Expected Results

- Step 1: `temporal/temporal_clause` is 2 and `claim/hyperproperty` is 1.
- Step 2: the check passes and the request exits 0.
- Step 3: the check refuses on `package.nodes`.
