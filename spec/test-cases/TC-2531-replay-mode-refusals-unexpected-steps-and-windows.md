---
id: TC-2531
title: "Replay mode refuses unreadable recordings and reports unexpected steps and windows"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-895
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-896
    type: verifies
---
# TC-2531: Replay mode refuses unreadable recordings and reports unexpected steps and windows

## Description

Verify that replay mode refuses unreadable recordings and reports unexpected steps and windows. Scope: FR-895-AC-4, FR-895-AC-5, FR-895-AC-6, FR-895-AC-9, FR-895-AC-10, FR-895-AC-11, FR-895-AC-12 to FR-895-AC-45, FR-896-AC-5, FR-896-AC-6.

Catches a recording defect reported as behaviour, or behaviour refused as a defect.

## Test Procedure

1. Assess a recording naming `Courier`; one with a send on `Tell` where none is enabled; one whose attempt reads 130 on clock `c` under `within [0, 120]`, then with the reading absent; assess each in batch and incrementally.
2. Assess the nested-repeat recordings `P`, `L`, `P`, `L`; `L`, `L` with no observed `P`; and `P`, `L`, `L` in one enclosing iteration. Compare each batch result with every incremental prefix and inspect the outer and inner repeat-control obligations.

## Expected Results

- Step 1: `Courier` is refused `missing-name` with its index; the send is `unexpected` and `refuted`; the window is `violated`, then `pending`; batch and incremental verdicts are equal.
- Step 2: the first recording has every repeat-control obligation `satisfied` in batch; in every prefix none is `violated`, and a prefix before descendant evidence may leave one `pending`. The second has no inner repeat-control obligation `violated` in batch or any prefix. The third has the inner repeat-control obligation `violated`, cause `duplicated`, from the second `L`; earlier prefixes may leave it `pending`. Each final prefix equals batch, and no verdict advances from `satisfied` to `violated`.
