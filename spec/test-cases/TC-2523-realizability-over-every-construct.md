---
id: TC-2523
title: "Realizability settles over every construct, symbolic capacity and timeouts"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-891
    type: verifies
---
# TC-2523: Realizability settles over every construct, symbolic capacity and timeouts

## Description

Verify that realizability settles over every construct, symbolic capacity and timeouts. Scope: FR-891-AC-1, FR-891-AC-2, FR-891-AC-6.

Catches an arena that skips a construct or reads a symbolic capacity from a default.

## Test Procedure

1. Decide realizability under `bounded` for each FR-890-AC-1 protocol and for the unmerged `repeat`; decide the `symbolic(Bound)` channel without and with `Bound` = 1; decide the await whose timeout branch violates the terminal obligation, then repaired.

## Expected Results

- Step 1: No result is `unsupported`; the `repeat` is refuted with `IndistinguishableHistories` for `Seller`, which replays; the absent capacity is `inconclusive` naming `Bound`, and `Bound` = 1 equals literal 1; the timeout defeat is `EnvironmentDefeat` through `timeout`, and the repair is `proved`.
