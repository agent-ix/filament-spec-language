---
id: TC-2532
title: "Global conformance over every behaviour and over a recording"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-896
    type: verifies
---
# TC-2532: Global conformance over every behaviour and over a recording

## Description

Verify that global conformance over every behaviour and over a recording. Scope: FR-896-AC-1, FR-896-AC-2, FR-896-AC-3, FR-896-AC-4.

Catches a stub engine, or a mode chosen without the `recording` member.

## Test Procedure

1. Request `global-conformance` on the order fixture with no recording, on its no-receive variant, on the unmerged `repeat`, with FR-895-AC-5's recording, and on a protocol whose role names an unselected model.

## Expected Results

- Step 1: `proved` with both projections and the verdicts listed; `refuted` with the deadlock and with the projection counterexample; `refuted` with `unexpected`; `Courier` refused `missing-name`; no item settles `unsupported` with a pending obligation.
