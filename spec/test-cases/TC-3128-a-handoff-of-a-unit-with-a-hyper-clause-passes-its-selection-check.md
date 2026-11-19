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
back and passes `Selection::check`, that a hybrid claim's `temporal_clause`
node is counted, and that a clock-configured hyper clause emits one
admitted `temporal_bindings` row per quantifier. Scope: FR-517-AC-25 and
FR-517-AC-26.

## Test Procedure

1. `write` a handoff of a unit declaring `Bounded` and the hyper clause
   `Equal`; read the sidecar's node counts.
2. `compile` the directory and run `Selection::check` on its admitted
   package; run a `native-compile/1` request with the `handoff` member over
   the unit.
3. Set the sidecar's `temporal/temporal_clause` count to 1 and check again.
4. Repeat steps 1 and 2 with a hybrid claim added to the unit.
5. `write` a handoff of a unit with the event-position false-extension hyper
   clause `Ordered` over traces `a` and `b`; read its `temporal_bindings`;
   then present a package with one wrapper-keyed row with no `trace` to the
   strict reader.

## Expected Results

- Step 1: `temporal/temporal_clause` is 2 and `claim/hyperproperty` is 1.
- Step 2: the check passes and the request exits 0.
- Step 3: the check refuses on `package.nodes`.
- Step 4: `temporal/temporal_clause` is 3 and the check passes.
- Step 5: two rows on the `hyperproperty` node, `trace` `a` then `b`, none
  on the wrapper; the package is admitted and the check passes; the
  wrapper-keyed row is refused at its `declaration`.
