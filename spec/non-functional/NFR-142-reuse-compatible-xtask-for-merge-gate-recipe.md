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
an explicit PR workspace and require that workspace to contain current
`origin/main`. When the executable comes from a separate stable tool checkout,
the command SHALL also require the tool checkout to be on `origin/main` history
and refuse if its Git tree differs from the PR Git tree in any xtask recipe
source or Cargo build input.

## Scope

This requirement covers merge-gate recipe invocations that reuse a prebuilt
executable built in a clean checkout. Cargo's existing freshness checks decide
whether that executable needs rebuilding. A tooling PR may build and run the
executable from its own clean PR checkout.

## Rationale

Running `cargo run` in each PR checkout can repeatedly compile the xtask
dependency graph before its runtime base check. Building from a stable checkout
lets Cargo reuse its existing build and lets the recipe compare Git paths
before interpreting changed paths in the selected PR checkout.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Accepted recipes from checkouts with differing xtask build inputs | 0 | 0 | unit-testing |
| Accepted invocations from a separate tool checkout that verify main ancestry in both checkouts | 1 | 1 | unit-testing |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-142-AC-1 | The command requires `--workspace <path>` and interprets every changed path relative to that PR workspace. | unit-testing |
| NFR-142-AC-2 | The command uses Git changed-path data to refuse reuse when `Cargo.toml`, `Cargo.lock`, `.cargo`, `rust-toolchain.toml`, `xtask/Cargo.toml`, `xtask/build.rs`, or `xtask/src/**` differ between the clean tool checkout and the selected PR head; it refuses when relevant build paths in either checkout have uncommitted changes. | unit-testing |
| NFR-142-AC-3 | With a separate tool checkout, the command fetches `origin/main` in both checkouts, verifies the tool checkout `HEAD` is an ancestor of its fetched `origin/main`, and verifies the PR checkout contains its fetched `origin/main`. If either fetch or ancestry check fails, the command emits no recipe. | unit-testing |
| NFR-142-AC-4 | When the selected PR workspace is the same checkout from which the executable was built, the command checks that workspace's relevant build paths are clean, fetches `origin/main`, verifies `HEAD` contains it, and skips the separate-tool history and tree comparison. | unit-testing |

## Verification

Focused tests cover explicit workspace selection, matching and differing Git
trees, uncommitted build inputs, fetch/ancestry refusals, and the same-checkout
tooling-PR fallback. The documented workflow runs Cargo in the stable tool
checkout before invoking its executable.
