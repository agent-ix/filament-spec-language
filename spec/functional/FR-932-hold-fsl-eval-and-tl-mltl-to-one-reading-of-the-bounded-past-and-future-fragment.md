---
id: FR-932
title: "Hold fsl-eval and tl-mltl to one reading of the bounded past and future fragment"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-757
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-091
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-092
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-095
    type: depends_on
---
# FR-932: Hold fsl-eval and tl-mltl to one reading of the bounded past and future fragment

## Description

FSpec FR-092 "Positions before the origin" binds every evaluator of the
false-extension profiles to one reading of a past operator whose offset
reaches before position 0: the pre-origin position reads every atom false, and
the offset is never skipped. FSpec FR-091 fixes the false extension after
closure for future operators. fsl-eval follows both. FSL SHALL hold `tl-mltl`
to the same reading by a differential suite that runs one corpus of formulas
and traces through fsl-eval, through an independent reference written from
FR-091 and FR-092 alone, and through `tl-mltl` by FR-757's cross-evaluation,
and requires the three verdicts to be equal. A formula then has one truth value
whichever evaluator settles it.

## Inputs

The corpus: the bounded past and future formulas over two atoms below and every
trace of length 1 to 4 over three values, under the event-position
false-extension profile, `on each`.

## Outputs

A per-case verdict comparison.

## Behavior

- The corpus SHALL hold at least: `once[0,2] q`, `historically[1,2] p`,
  `historically[0,1] not p`, `p since[0,2] q`, `p triggered[1,2] q`,
  `once[1,1] p`, `once[1,1] not p`, `historically[0,2] p`,
  `historically[0,2] true`, `true since[1,1] not p` (the past cases),
  `eventually[0,2] q`, `always[1,2] p`, `p until[0,2] q`, `p release[0,1] q`
  (the future cases) and `eventually[1,1] once[1,1] not p` (a mixed case), with
  `p` the atom `holds(c.level = 0)` and `q` the atom `holds(c.level = 1)`.
- For every case and trace, the fsl-eval outcome SHALL equal the reference
  outcome (true at every activation, or false at the first activation that
  fails).
- For every past case and every future case, the clause SHALL lower through
  `tl_lowering` (FR-931) under its default correspondence
  (`mltl.origin-complete-history/v1` for a past case, `mltl.closed-trace/v1`
  for a future case), and its `cross_evaluation` over the trace SHALL report
  `agree` true and no fault, so `tl-mltl`'s verdict equals fsl-eval's at every
  activation position.
- A mixed case SHALL lower to `disposition` `unsupported` with `dimensions`
  `["direction"]` (FSpec FR-095, mixed-direction row) and is held by the
  reference comparison alone; that is the standard's support table, not an
  exclusion of the case.
- The cases `once[1,1] not p` at position 0 and `historically[0,2] p` at
  position 1 over a trace where `p` holds everywhere SHALL stay in the corpus
  as the regression of the reading that clips a closed past window at the
  origin, which gives `false` and `true` there instead of FR-092's `true` and
  `false`.
- The suite SHALL live in the root crate's integration tests and reach
  `tl-mltl` only through `fsl-command`'s `tl_lowering` (FR-931 "Placement");
  fsl-eval names no TL crate, not even as a dev-dependency (ADR-011).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-932-AC-1 | Over every trace of length 1 to 4 over the values 0, 1 and 2, each corpus case's fsl-eval outcome equals the reference outcome. | Test (TC-2604) |
| FR-932-AC-2 | Over the same traces, each past and future case lowers under its default correspondence and its cross-evaluation reports `agree` true with `faults` `[]`; the mixed case returns `unsupported` with `dimensions` `["direction"]`. | Test (TC-2604) |
| FR-932-AC-3 | Over the trace `0, 0, 0` (`p` true everywhere), `once[1,1] not p` is `Completed(true)` at position 0 and `historically[0,2] p` is false at position 0 in fsl-eval, in the reference and in `tl-mltl`'s cross-evaluation, and a reference that clips the closed past window at the origin disagrees with all three on both cases. | Test (TC-2604) |

## Dependencies

- FR-757 and FR-931 (lowering and cross-evaluation), FR-327 (native trace
  evaluation), ADR-011 (TL edges).
- FSpec FR-091, FR-092 ("Positions before the origin", FR-092-AC-9), FR-095.
