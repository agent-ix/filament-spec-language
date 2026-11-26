---
id: NFR-142
org: agent-ix
title: "Reuse only a compatible prebuilt xtask for merge-gate recipes"
type: NFR
quality_attribute: performance_efficiency
relationships: []
---
# NFR-142: Reuse only a compatible prebuilt xtask for merge-gate recipes

## Statement

When a merge-gate recipe is generated from a prebuilt `xtask` executable, the
command SHALL verify that the executable's recipe implementation and build
inputs match the selected workspace before producing a recipe. It SHALL fetch
`origin/main` and verify that the selected workspace head contains the fetched
revision at execution time. A rejected executable or stale base SHALL produce
no green recipe.

## Scope

This requirement covers `xtask merge-gate-recipe` invocations that reuse a
previously compiled executable, including an executable built for another
checkout when the caller explicitly selects the intended workspace.

## Rationale

Running the existing command through Cargo recompiles `xtask` before its
runtime base check. Reusing a binary avoids repeating that build while source
fingerprinting and runtime ancestry verification preserve the recipe's
correctness.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Recipes emitted by incompatible executables or stale workspaces | 0 | 0 | Tagged refusal tests |
| Runtime origin/main revision checked per accepted recipe | 1 | 1 | Tagged command tests |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-142-AC-1 | A compatible prebuilt executable can generate a merge-gate recipe for an explicitly selected workspace, and every path in the recipe is interpreted relative to that workspace. | Tagged Rust tests and command-level check |
| NFR-142-AC-2 | A prebuilt executable refuses to generate a recipe when any tracked xtask recipe source or declared build input in the selected workspace differs from the inputs used to build it. | Tagged Rust tests that mutate each fingerprint input class |
| NFR-142-AC-3 | Every accepted prebuilt invocation fetches `origin/main`, verifies the selected workspace head contains the fetched commit, and reports the verified head and main revisions. If fetch or ancestry verification fails, no recipe is emitted. | Tagged Rust tests for fetch/ancestry outcomes and command-level check |

## Verification

Focused tests exercise workspace selection, input compatibility, runtime fetch,
and ancestry refusal. The documented reuse workflow invokes the prebuilt
executable after updating the selected PR workspace.
