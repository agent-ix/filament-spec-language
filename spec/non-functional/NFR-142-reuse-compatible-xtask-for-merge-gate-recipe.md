---
id: NFR-142
org: agent-ix
title: "Reuse a compatible xtask from a stable tool checkout"
type: NFR
quality_attribute: performance_efficiency
relationships: []
---
# NFR-142: Reuse a compatible xtask from a stable tool checkout

## Statement

When the merge-gate recipe uses a prebuilt `xtask`, the command SHALL require
an explicit PR workspace and refuse reuse if the tool checkout's Git tree
differs from the PR Git tree in any xtask recipe source or Cargo build input.
The command SHALL fetch `origin/main` in the PR workspace and accept a PR head
behind that fetched main only when the PR's recipe-relevant path set has no
overlap with main's changes since their merge-base. Otherwise, it SHALL require
the PR workspace to merge current `origin/main` before emitting a recipe.

## Scope

This requirement covers merge-gate recipe invocations that reuse a prebuilt
executable built in a clean checkout. Cargo's existing freshness checks decide
whether that executable needs rebuilding. A tooling PR may build and run the
executable from its own clean PR checkout. For the rolling check on `main`, the
PR head is `main`, so the check continues to evaluate the current main tree.

## Rationale

Running `cargo run` in each PR checkout can repeatedly compile the xtask
dependency graph before its runtime base check. Building from a stable checkout
lets Cargo reuse its existing build. Requiring non-overlapping recipe-relevant
paths allows unrelated PRs to proceed while `main` advances, without allowing
changes to the package, tests, or recipe inputs that the selected gate uses.

Path non-overlap is a textual compatibility check; it cannot prove semantic
compatibility between independent changes. The rolling merge-gate check on the
current `main` head covers this limitation by evaluating the integrated tree.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Accepted recipes from checkouts with differing xtask build inputs | 0 | 0 | unit-testing |
| Accepted invocations from a separate tool checkout with compatible recipe inputs | 1 | 1 | unit-testing |
| Accepted behind-main PRs with disjoint recipe-relevant paths | 1 | 1 | unit-testing |
| Accepted behind-main PRs with overlapping or unconditional paths | 0 | 0 | unit-testing |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-142-AC-1 | The command requires `--workspace <path>` and interprets every changed path relative to that PR workspace. | unit-testing |
| NFR-142-AC-2 | The command uses Git changed-path data to refuse reuse when `Cargo.toml`, `Cargo.lock`, `.cargo`, `rust-toolchain.toml`, `xtask/Cargo.toml`, `xtask/build.rs`, or `xtask/src/**` differ between the clean tool checkout and the selected PR head; it refuses when relevant build paths in either checkout have uncommitted changes. | unit-testing |
| NFR-142-AC-3 | With a separate tool checkout, the command fetches `origin/main` in both checkouts, verifies the tool checkout `HEAD` is an ancestor of its fetched `origin/main`, and verifies the PR checkout's merge-base with fetched `origin/main` exists. It accepts a behind-main PR only when its recipe-relevant paths do not overlap main's changed paths since that merge-base. If either fetch, tool ancestry, merge-base, or overlap check fails, the command emits no recipe. | unit-testing |
| NFR-142-AC-4 | When the selected PR workspace is the same checkout from which the executable was built, the command checks that workspace's relevant build paths are clean, fetches `origin/main`, and skips the separate-tool history and tree comparison. It requires merging current `origin/main` when recipe-relevant paths overlap. | unit-testing |
| NFR-142-AC-5 | The PR recipe-relevant path set contains paths reported by Git for PR changes, every package footprint selected by the recipe, and the exact source paths of selected integration-test modules. The command compares this set with main's changed paths using exact and directory-prefix overlap. Any main change within a selected package footprint is overlap, including a sibling file. Changes to `Cargo.toml`, `Cargo.lock`, `.cargo/**`, `rust-toolchain.toml`, or `xtask/**` on either side are unconditional overlap. | unit-testing |
| NFR-142-AC-6 | Before deciding, the command prints the PR-head, fetched-main, and merge-base commit SHAs. It reports deterministic overlapping paths and an explicit accept or merge-required verdict. | unit-testing |
| NFR-142-AC-7 | Documentation states that textual path non-overlap cannot establish semantic compatibility and that the rolling merge-gate check on current `main` covers this limitation. | review |
| NFR-142-AC-8 | When `--preflight` validates recipe selectors, the command invokes `cargo test --no-run --message-format=json` at most once for each selected package, resolves each selector to its test executable, and checks that selector by listing both ordinary and ignored tests from the executable. The recipe lines printed to stdout remain identical to a non-preflight invocation for the same paths. | unit-testing |

## Verification

Focused tests cover explicit workspace selection, matching and differing Git
trees, uncommitted build inputs, fetch and ancestry refusals, same-checkout
tooling-PR behavior, a behind-main disjoint PR, direct path overlap, package
footprint overlap, exact selected integration-test modules, unconditional
manifest/lock/xtask overlap, and deterministic SHA/path/verdict output. The
documented workflow runs Cargo in the stable tool checkout before invoking its
executable.
