---
id: TC-3030
title: "A state-graph zone certificate needs no time-divergent witness"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-244
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-245
    type: verifies
---
# TC-3030: A state-graph zone certificate needs no time-divergent witness

## Description

Verify that the zone certificate of a state-graph item over a timed subject
certifies the reachable timed graph without an `admitted` witness, that the
core checker accepts it over a Zeno-only graph, and that every certificate
whose claim depends on time divergence still requires the witness. Scope:
FR-244-AC-7 and FR-245-AC-11.

## Test Procedure

1. Build the Zeno-only variant of FR-852's `Rpc` subject: time invariant
   `x <= 0 ms` at every state and `reply` guarded by `x >= 0 ms`.
2. Analyze `possible Replied`, `always possible BackToIdle` and
   `temporal NoLateReply` over it.
3. Check the two state-graph certificates, then each with an `admitted`
   member added.
4. Check FR-244-AC-1's `NoLateReply` certificate over the unmodified `Rpc`
   with `admitted` removed.

## Expected Results

- Step 2: `Replied` and `BackToIdle` `proved`, each certificate without
  `admitted`, its reachability part covering phases 0, 1 and 2, and
  `BackToIdle`'s `state_graph.ef_nodes` non-empty; `NoLateReply`
  `inconclusive` `NoAdmittedBehaviour` with no certificate.
- Step 3: both `Accepted` and the items `certified`; with `admitted` added,
  `Rejected` `Malformed` at `CertificateMember("admitted")`.
- Step 4: `Rejected` `Malformed` at `CertificateMember("admitted")`.

Test: `fsl-analyze/tests/it/timed_state_graph.rs`.
