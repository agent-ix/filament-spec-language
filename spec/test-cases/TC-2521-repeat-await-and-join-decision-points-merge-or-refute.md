---
id: TC-2521
title: "Repeat, await and join decision points merge or refute by PA-10"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-890
    type: verifies
---
# TC-2521: Repeat, await and join decision points merge or refute by PA-10

## Description

Verify that repeat, await and join decision points merge or refute by PA-10. Scope: FR-890-AC-2, FR-890-AC-3, FR-890-AC-4.

Catches a merge rule that ignores a decision point's owner or drops the exhausted or timeout branch.

## Test Procedure

1. Project FR-890-AC-2's `repeat` onto `Seller`, then its variant whose exit tells `Seller` nothing; project AC-3's timed `await` onto its owner and onto `Warehouse`; project AC-4's `join any` under `outstanding cancel` and `continue` onto `Warehouse`.

## Expected Results

- Step 1: The loop projects with its maximum and is refuted, `decision` `repeat`, without evidence; the await carries `[0, 120]` and clock `c` and is refuted onto an untold `Warehouse`, `decision` `await`; the cancelling join is refuted, owner `environment`, and the continuing join projects.
