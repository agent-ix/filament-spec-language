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

Verify that global conformance over every behaviour and over a recording. Scope: FR-896-AC-1, FR-896-AC-2, FR-896-AC-3, FR-896-AC-4, FR-896-AC-7, FR-896-AC-8, FR-896-AC-9.

Catches a stub engine, a mode chosen without the `recording` member, a check of the first admitted key alone and an ignored `over` binding.

## Test Procedure

1. Request `global-conformance` on the order fixture with no recording, on its no-receive variant, on the unmerged `repeat`, with FR-895-AC-5's recording, and on a protocol whose role names an unselected model.
2. Request it with no `subject.protocol` on FR-896-AC-7's keyed order fixture, with `q.r1 = 1` and with `q.r1 = 0`.
3. Request it on the first of those states with `subject.protocol` binding `over` to `p`, then to `q`.
4. Request two items claiming `Flow` under a request `subject.protocol` binding `Other`, the second with a `temporal.subject` binding `Flow` to `p`.

## Expected Results

- Step 1: `proved` with the verdicts listed and both projections reported beside them; `refuted` with the deadlock; `proved` for the unmerged `repeat`, with its refuted projection reported beside the verdict; `refuted` with `unexpected`; `Courier` refused `missing-name`; no item settles `unsupported` with a pending obligation.
- Step 2: `refuted` with the empty-prefix deadlock whose `over` names `q`, replayed, `closure.state` `open`; then `proved`, `closure.state` `closed`.
- Step 3: `proved`; then `refuted` with step 2's counterexample.
- Step 4: `declined` `invalid-input` naming `temporal.subject.protocol`; `proved`.
