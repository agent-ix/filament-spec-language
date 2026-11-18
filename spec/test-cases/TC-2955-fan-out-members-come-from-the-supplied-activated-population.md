---
id: TC-2955
title: "Fan-out members come from the supplied activated population"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-990
    type: verifies
---
# TC-2955: Fan-out members come from the supplied activated population

## Description

Verify that a recording-mode `global-conformance` item enumerates each
fan-out member from its `population` member, not from the observations, so
a member absent from the recording is still required. Scope: FR-990-AC-7,
FR-990-AC-8, FR-990-AC-9 and FR-990-AC-10.

Catches an assessor that derives the member set from the observed
`role_instance` values, one that reports an omitted member as incomplete
closure while the population is known, and one that reports `proved` when
the population is unknown.

## Test Procedure

1. Over the FR-990-AC-7 fan-out fixture, analyze a closed recording in which
   `x` and `y` take both events, with `population` `[{role: r, members: [x, y]}]`.
2. Analyze a closed recording in which only `x` is observed, first with
   premises omitting `y`, then with `y`'s branch premises listed, then with
   closure `open`.
3. Analyze step 1's recording with no `population` member.
4. Analyze step 1's recording with each FR-990-AC-10 defective population.

## Expected Results

- Step 1: `proved`, `complete` `true`; four member `control` obligations,
  each `satisfied` with its `role_instance`.
- Step 2: refused `invalid_runtime_input` naming `y`'s `left` branch
  premise; then `refuted` with `y`'s two `control` obligations `violated`,
  cause `missing`; then those obligations `pending`, item `inconclusive`,
  cause `open-history`.
- Step 3: `inconclusive`, cause `incomplete-closure-scope`, `complete`
  `false`, `closure.state` `closed`.
- Step 4: the item refusals and request refusals FR-990-AC-10 names, with no
  verdict.
