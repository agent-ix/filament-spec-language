---
id: FR-059
title: "Check the backend dependency direction across FSL, Contract IR, Runtime and Codegen"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-356
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-401
    type: references
---
# FR-059: Check the backend dependency direction across FSL, Contract IR, Runtime and Codegen

## Description

ADR-011 §3 fixes two forbidden bypasses over the four-repository ecosystem
(FSL, filament-contract-ir, filament-contract-runtime, filament-contract-codegen):

- **FB-05**: no backend (IR, RT or CG) depends on FSL, except CG's normal
  dependency on the FSL layer-6 `replay` facade, the package `fsl-replay`.
- **FB-11**: no dependency edge, normal or dev, closes a cycle among the four
  repositories.

FSL SHALL provide a check, runnable over real `cargo metadata` output for the
four repositories, that reports every edge violating FB-05 and every cycle
violating FB-11, and passes only when neither set of findings is non-empty.

## Inputs

- Resolved dependency graphs (`cargo metadata --format-version=1`) for each of
  the four repositories' own manifests.
- Each resolved edge's source repository, target repository, dependency kind
  (normal or dev) and dependency package name.

## Outputs

- An FB-05 report: the list of edges into FSL that are not CG's normal
  dependency on `fsl-replay`.
- An FB-11 report: the list of simple cycles over the combined normal+dev
  edge graph of the four repositories, each reported once regardless of which
  repository the cycle is discovered from.
- A pass/fail result: pass exactly when both reports are empty.

## Behavior

The check SHALL classify a resolved package as one of the four repositories by
package name and dependency source URL, treating `filament-contract-model` (the
crate name filament-contract-ir's workspace member publishes under) as
`filament-contract-ir`. When it extracts edges, a package sourced from its own repository, such as
`filament-exact` (`agent-ix/filament-exact`) or `filament-semantic-value`
(`agent-ix/filament-semantic-value`), classifies as no ecosystem repository. These are ADR-011 FB-05's
shared `no_std` leaf crates: the kernel K (§6.1 "K is a leaf") and the
semantic-value leaf SV, whose dependencies are K, ADR-013's one RFC 8785
encoder `quire-canonical`, `serde` and `thiserror`. Neither depends on an FSL
layer or on an IR, RT or CG crate, so an edge into either closes no cycle. Every other package sourced from the FSL repository classifies as
FSL. This exemption is local to edge extraction; the shared `graph::classify`
classifies an FSL-sourced shared leaf as FSL.

The check SHALL treat a `build`-kind dependency as a normal edge for FB-05/
FB-11 purposes, and a `dev`-kind dependency as a dev edge.

The check SHALL treat CG's normal dependency on the FSL package
`fsl-replay` as the one FB-05 exception. This is the rule
[FR-356](FR-356-walk-nested-structures-through-one-iterative-walker-toolkit.md)-AC-1
states for the same direction check ("refuses one to any `fsl-*` crate
other than CG's normal edge to `fsl-replay`"), which
`tc_arch_lint_direction_004` (`tools/arch-lint/graph.rs`) tests; FR-059-AC-10
and FR-059-AC-1 are read with it as one rule.

The check SHALL report every other CG normal dependency on a package that
classifies as FSL, such as `fsl-eval` or `fsl-semantics`, as an FB-05
violation naming that package.

The check SHALL report a *dev* dependency from CG on FSL, since a dev
dependency is not the stated exception. CG's dependencies on the crates of
`agent-ix/filament-eval` (`filament-eval`, `filament-eval-corpus`, `filament-checked`),
on the semantic-value leaf `filament-semantic-value` (SV, ADR-011 X-11) and on
the other shared leaves classify as no ecosystem repository, so they are no
FB-05 edge. CG names the types FR-401 moves to those leaves, such as
`Category`, from the leaves directly
([FR-401](FR-401-host-the-evaluation-interface-in-the-filament-eval-leaf.md));
`fsl-replay` does not re-export the types FR-401 moves. Among CG's
FSL-owned dependencies, the allowed set is `fsl-replay`, the one FSL crate,
plus the shared leaves `filament-semantic-value` and the `agent-ix/filament-eval`
crates, and no other FSL-owned crate (FR-060, "T12-A and the evaluator
differential corpus"). CG's edges to the other shared leaves, such as
`filament-exact` and `filament-walk`, are not FSL-owned and remain as ADR-011
FB-05 states.

The check SHALL combine normal and dev edges into one directed graph for
FB-11, and SHALL report a cycle exactly once regardless of its rotation or
which repository it started the search from.

The checked-term leaf `filament-checked` (ADR-011 layer C, in the repository
`agent-ix/filament-eval`) is also a shared leaf, sourced from no ecosystem
repository, so an edge into it classifies as no ecosystem repository. When
a resolved IR, RT or CG dependency on `filament-checked` has its `construct`
feature enabled, the check SHALL report that edge as an FB-05 violation: a
backend reads checked terms and never builds one ([FR-400](FR-400-separate-checked-terms-from-the-checker.md),
ADR-011 FB-14).

This check is crate-level only: it establishes that an edge into FSL exists at
all, not that the edge's caller code stays inside the layer-6 `replay` facade.
[FR-060](FR-060-check-fsl-api-surface-boundary.md) verifies the latter.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-059-AC-1 | A graph with only CG's normal dependency on `fsl-replay`, and no cycle, reports both FB-05 and FB-11 clean. | Test (TC-156) |
| FR-059-AC-2 | Any normal or dev edge from IR or RT into FSL is reported as an FB-05 violation. | Test (TC-156) |
| FR-059-AC-3 | A *dev* edge from CG into FSL is reported as an FB-05 violation; the stated exception covers only CG's normal edge. | Test (TC-156) |
| FR-059-AC-4 | A 2-repository or longer cycle over normal and/or dev edges among the four repositories is reported exactly once as an FB-11 violation, however many repositories the search starts from. | Test (TC-156) |
| FR-059-AC-5 | An acyclic graph with edges only running toward FSL and CG reports no FB-11 violation. | Test (TC-156) |
| FR-059-AC-6 | Run against the real, current-head resolution of filament-contract-ir's manifest, the check reports no FB-05 edge from IR into FSL and no FB-11 cycle between FSL and IR, since neither IR manifest declares an FSL dependency (ADR-011 OBS-029). Resolving FSL's own workspace manifest through the same edge-resolution path the check uses for `--fsl` yields FSL's normal edge on the git-sourced `filament-contract-model`, classified as FSL → IR, the permitted direction. | Test (TC-156) |
| FR-059-AC-8 | A resolved RT dependency on the git-sourced `filament-exact` yields no edge and no finding; a resolved RT dependency on another crate sourced from the FSL repository, such as `fsl-eval`, yields an RT → FSL edge reported as an FB-05 violation. | Test (TC-156) |
| FR-059-AC-9 | A resolved RT dependency on the git-sourced `filament-semantic-value` yields no edge and no finding; resolved RT dependencies on `fsl-eval` and on `fsl-semantics`, from the same FSL source, each yield an RT → FSL edge reported as an FB-05 violation. | Test (TC-156) |
| FR-059-AC-10 | A graph in which CG has normal dependencies on `fsl-replay` and on `fsl-eval`, both sourced from the FSL repository, on `filament-semantic-value` sourced from `agent-ix/filament-semantic-value`, and on `filament-eval` and `filament-eval-corpus` sourced from `agent-ix/filament-eval`, reports exactly one FB-05 violation, the CG → FSL edge via `fsl-eval`; the same graph without the `fsl-eval` dependency reports FB-05 clean. | Test (TC-156) |
| FR-059-AC-11 | Run against the real, current-head resolution of filament-contract-codegen's manifest, every CG normal dependency sourced from the FSL repository, `agent-ix/filament-semantic-value` or `agent-ix/filament-eval` is `fsl-replay`, `filament-semantic-value` or a crate of `agent-ix/filament-eval`; its only CG → FSL normal edge is via `fsl-replay`, and the check reports no FB-05 edge from CG. | Test (TC-156) |

## Dependencies

- ADR-011 §3 FB-05, FB-11 and §7.1 T-12
  (`ix://agent-ix/filament-spec-language/ADR-011`).
- [FR-060](FR-060-check-fsl-api-surface-boundary.md) checks the FB-05
  exception's call-site boundary.
- [FR-356](FR-356-walk-nested-structures-through-one-iterative-walker-toolkit.md)-AC-1
  states the same `fsl-replay`-only FB-05 exception for the direction check.
