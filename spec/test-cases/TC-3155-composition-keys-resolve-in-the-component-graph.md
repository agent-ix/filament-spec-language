---
id: TC-3155
title: "Composition keys resolve in the component graph"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: verifies
---
# TC-3155: Composition keys resolve in the component graph

## Description

Composition keys resolve in the component graph.

Scope: FR-1044-AC-15.

## Test Procedure

Restate the premise keys a composition item wrote, and corrupt one: an index segment beyond the component graph, a subject identity of another package with and without an index segment. Test: `fsl-analyze/tests/it/analyze_composition_cli.rs` `a_held_premise_key_resolves_in_its_components_graph`. Resolve keys over FSpec's `positive-dependency-call.json` and `positive-dependency-library.json`: NI-25, NI-26, NI-27, NI-28, the first out-of-range segment in canonical order and a peer assumption id. Test: `fsl-semantics/src/check/wire_id.rs` `a_component_key_resolves_in_the_component_graph`, `an_interface_id_resolves_in_the_component_graph`, `a_component_key_index_out_of_range_names_its_member`, `a_component_key_of_another_package_is_refused_before_the_range_check`, `the_first_out_of_range_segment_is_in_canonical_order` and `a_peer_assumption_id_resolves_in_the_peer_components_graph`. Read held slot identities in a planner request. Test: `fsl-replay/src/planner/request.rs` `held_slot_tests`.

Tag the tests `#[trace("FSpec-TC-233", "FSpec-FR-322-AC-520")]`.

## Expected Results

- A restated key resolves in the graph of its slot's component package, a peer assumption id in the graph of the peer guarantee's component package.
- An index segment the graph does not hold refuses `malformed_wire`, with no cause, at its `node` member, the first in canonical order; a key of another package refuses `invalid-value` at the key, before the range check, with or without an index segment.
- A planner request's held slot identity is read after the shape checks and refuses at `/slots/<i>`.
