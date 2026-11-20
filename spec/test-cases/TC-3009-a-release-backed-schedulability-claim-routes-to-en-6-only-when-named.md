---
id: TC-3009
title: "A release-backed schedulability claim routes to EN-6 only when named"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-246
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-075
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-962
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-249
    type: verifies
---
# TC-3009: A release-backed schedulability claim routes to EN-6 only when named

## Description

Verify that a `schedulable` claim's requirement record carries the
release-backed marker, that the marker becomes the `task-product` item
requirement, that EN-6 matches exactly the items carrying it, and that an
unnamed release-backed item narrows to EN-7. Scope: FR-246-AC-5,
FR-249-AC-13 (its release-backed clause),
FR-075-AC-23, FR-962-AC-6.

## Test Procedure

1. Check `Ctl` with `schedulable CtlFp on Ctl under fixed_priority;` and
   `response R on Ctl.c <= 12 ms under fixed_priority;`, once with no
   `release` and once with `release Ctl.a on M::o;` over a `time dense`
   model; read both units' records, `CtlFp`'s claim node identity, and the
   records read back from each compiled package. Check it once more with
   only `mode Ctl hi on M::o;` and no `release` (FR-249-AC-13).
2. Build items from each unit's records over `Engines::fsl()`'s registry,
   unnamed and naming `en-6`; repeat with the registration order reversed.
3. Add a process provider advertising `schedulability` to the registry and
   build the release-backed unnamed item again.

Tag tests `#[trace("TC-3009", "FR-246-AC-5")]`,
`#[trace("TC-3009", "FR-075-AC-23")]` and `#[trace("TC-3009", "FR-962-AC-6")]`.

## Expected Results

- Step 1: without a release both records are release-backed `false`; with
  it `CtlFp`'s is `true` and `R`'s `false`; `CtlFp`'s node identity is
  unchanged; the package route gives the same markers. With only the
  `mode` declaration `CtlFp`'s record is release-backed `false`.
- Step 2: without a release the item carries no `task-product`, unnamed
  candidates `[en-7]`, and naming `en-6` the empty set (`unsupported`).
  With the release it carries `task-product`; the matching set is
  `[en-6, en-7]`; unnamed it narrows to `[en-7]` with `backend` absent and
  never settles `ambiguous-backend`; naming `en-6` gives `[en-6]`. Reversed
  registration order gives the same sets.
- Step 3: the three-member set stays whole and the item settles
  `invalid-request` `ambiguous-backend` naming every candidate.
