---
id: FR-757
title: "Lower a TL-supported temporal clause to a TL formula and cross-evaluate it with the TL evaluator"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-527
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-045
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-095
    type: depends_on
---
# FR-757: Lower a TL-supported temporal clause to a TL formula and cross-evaluate it with the TL evaluator

## Description

For a temporal clause that FR-527 classifies as supported under a versioned
TL correspondence, FSL SHALL lower the clause's checked formula to a
`tl-syntax` Formula-v2 value under that correspondence, and SHALL evaluate
the lowered formula with `tl-mltl` beside its native evaluation on the same
finite trace, treating any disagreement as a fault. The correspondence is
then a lowering, not only a classification.

## Inputs

A checked temporal clause, its FR-527 classification with its authenticated
selection, and, for cross-evaluation, a finite trace of the clause's subject.

## Outputs

The lowered `tl-syntax` formula with the correspondence identity (for
example `mltl.closed-trace/v1`) and its atom table mapping each TL atom to
the clause's `holds` leaf identity; or the classification's own
unsupported disposition, unchanged.

## Behavior

- The lowering SHALL accept only a classification that FR-527 marked
  supported and whose selection belongs to the clause's package; any other
  input returns the classification's disposition and no formula.
- Each `holds(e)` leaf SHALL become one TL atom whose truth at a position is
  the native value of `e` there; operators and intervals SHALL map one to one
  under the correspondence's table, with no rewrite, profile substitution or
  bound change.
- Cross-evaluation SHALL evaluate the lowered formula with `tl-mltl` over the
  atom valuation the native evaluator computes, and compare its verdict with
  the native verdict at every activation position; a disagreement SHALL be a
  `correspondence-loss` fault naming the clause, position and both verdicts.
- The Contract IR bridge SHALL consume this lowering instead of deriving its
  own.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-757-AC-1 | `always [0, 3] holds(c.value <= 2)` under the event-position false-extension profile lowers to the TL formula `G[0,3] p0` with `p0` naming the `holds` leaf, under `mltl.closed-trace/v1`. | Test (TC-2107) |
| FR-757-AC-2 | On a fixed corpus of finite traces, the native and `tl-mltl` verdicts agree at every activation position; a deliberately mis-mapped atom yields a `correspondence-loss` fault naming the clause and position. | Test (TC-2107) |
| FR-757-AC-3 | A clause that checks and that FR-527 classifies unsupported returns that disposition and no formula: `always [0 ms, 3 ms] holds(c.versionNumber <= 2)` over the ConfigVersion model timed by `time Config dense unit ms` (FR-233-AC-5), under `filament.temporal.timed/v1` with no `clock` member, so bound `model-time`, lowers to no formula and reports the `unsupported` disposition; the timed profile never lowers to a TL target (FSpec FR-250-AC-7). | Test (TC-2107) |

## Dependencies

- FR-527 and FR-045 (classification); FSpec FR-095 (TL correspondence).
- FR-931 (the `tl_lowering` request member, correspondence selection and
  placement in `fsl-command`); FR-932 (the `tl-mltl` differential).
