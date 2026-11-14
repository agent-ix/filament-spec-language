---
id: TC-156
title: "Report FB-05 and FB-11 violations over the four-repository backend dependency graph"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-059
    type: verifies
---
# TC-156: Report FB-05 and FB-11 violations over the four-repository backend dependency graph

## Description

Verify that `arch-lint direction` classifies resolved packages by repository,
reports every FB-05 edge into FSL other than CG's normal dependency on
`fsl-replay`, reports
every FB-11 cycle over the combined normal+dev edge graph exactly once, and
reports no IR → FSL edge and no FSL ⇄ IR cycle when run against
filament-contract-ir's real current-head `cargo metadata` output, that resolving
FSL's own manifest classifies its real edge on `filament-contract-model` as
FSL → IR, and that the shared leaves `filament-exact` and
`filament-semantic-value` classify as no ecosystem repository. Scope: FR-059-AC-1
through FR-059-AC-6 and FR-059-AC-8 through FR-059-AC-11.

## Test Procedure

1. Build a synthetic four-repository graph with only CG's normal dependency on
   `fsl-replay` and no cycle. Run the check.
2. Build a graph with a normal edge from IR into FSL, then a separate graph
   with a dev edge from RT into FSL. Run the check on each.
3. Build a graph with a dev edge from CG into FSL (no normal edge). Run the
   check.
4. Build a graph with a 2-repository cycle (FSL depends on CG, normal edge,
   and CG's own manifest carries a dev edge back to FSL beyond the stated
   exception) and, separately, a longer cycle spanning all four repositories
   over a mix of normal and dev edges. Run the check starting the search from
   each of the four repositories in turn.
5. Build an acyclic graph with edges only running toward FSL and CG (no edge
   originates from FSL or CG). Run the check.
6. Run `arch-lint direction` against real `cargo metadata` output for FSL's
   own manifest and a real local checkout of filament-contract-ir's current head.
   Separately, resolve FSL's own workspace manifest through
   `edges_for_manifest`, the path `direction` uses for `--fsl` (automated).
7. Parse a synthetic `cargo metadata` document in which RT depends on
   `filament-exact`, sourced from the `agent-ix/filament-exact` git url, and on
   `fsl-eval`, sourced from the FSL repository's git url, and run the check on the resulting edges.
8. Parse a synthetic `cargo metadata` document in which RT depends on
   `filament-semantic-value`, sourced from the `agent-ix/filament-semantic-value` git
   url, and on `fsl-semantics` and `fsl-eval`, sourced from the FSL
   repository's git url, and run the check on the resulting edges.
9. Build a graph in which CG has normal dependencies on `fsl-replay` and
   `fsl-eval`, sourced from the FSL repository's git url, on
   `filament-semantic-value`, sourced from the `agent-ix/filament-semantic-value`
   git url, and on `filament-eval` and `filament-eval-corpus`, sourced from the
   `agent-ix/filament-eval` git url. Run the check, then run it on the same
   graph without `fsl-eval`.
10. Run `arch-lint direction` against real `cargo metadata` output for a
    local checkout of filament-contract-codegen's current head, and list CG's
    normal dependencies sourced from the FSL repository,
    `agent-ix/filament-semantic-value` or `agent-ix/filament-eval`.

## Expected Results

- Step 1: both the FB-05 and FB-11 reports are empty; the check passes.
- Step 2: both edges are reported as FB-05 violations; the FB-05 report names
  each violating edge's source and target.
- Step 3: the dev edge from CG is reported as an FB-05 violation; the stated
  exception excludes only CG's *normal* edge.
- Step 4: each cycle is reported exactly once in the FB-11 report regardless
  of which repository's edges the search started from; the report does not
  duplicate a cycle by rotation.
- Step 5: the FB-11 report is empty.
- Step 6: the FB-05 report names no edge from filament-contract-ir into FSL,
  and the FB-11 report names no cycle between FSL and filament-contract-ir,
  because neither IR manifest declares an FSL dependency (ADR-011 OBS-029);
  step 2's seeded IR → FSL edge is still reported. This output is captured
  for the PR body as real, not synthetic, evidence. Separately, an automated
  test resolves FSL's own workspace manifest through the edge-resolution
  path `direction` uses for `--fsl` and asserts that the edge set contains
  FSL's normal edge on the git-sourced `filament-contract-model`, classified as
  FSL → IR (the permitted direction); a classifier that no longer recognises
  the real IR crate drops that edge and fails the test.
- Step 7: the only edge is RT → FSL via `fsl-eval`, and the FB-05 report
  names exactly that edge; the `filament-exact` dependency contributes no edge.
- Step 8: the edges are RT → FSL via `fsl-eval` and via `fsl-semantics`, and
  the FB-05 report names exactly those two; the `filament-semantic-value`
  dependency contributes no edge.
- Step 9: the FB-05 report names exactly the CG → FSL edge via `fsl-eval`;
  the `fsl-replay`, `filament-semantic-value`, `filament-eval` and
  `filament-eval-corpus` dependencies are not reported.
  Without `fsl-eval`, the FB-05 report is empty (FR-059-AC-10).
- Step 10: the listed dependencies are only `fsl-replay`,
  `filament-semantic-value` and crates of `agent-ix/filament-eval`; every CG → FSL
  normal edge is via `fsl-replay`, and the FB-05 report names no CG edge
  (FR-059-AC-11). This output is captured for the
  PR body as real evidence, as step 6's is.

## Metadata

- Priority: P1
- Target Integration: `tools/arch-lint/graph.rs`, `tools/arch-lint/metadata.rs`
- Automation: Automated Rust unit tests, including step 6's resolution of FSL's own manifest, plus one manual real-data `direction` run (step 6)

## Dependencies

**Upstream:** [FR-059](../functional/FR-059-check-backend-dependency-direction.md).
**Downstream:** none.
