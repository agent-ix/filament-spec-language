---
id: SR-967175026302197768900474181941935084819
title: "Gap analysis — QSL-2988 slice 2"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md, fsl-analyze, fsl-replay, tools/fsl-probability-check"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/spec
    type: references
---

## Summary

The computed criterion matrix and changed tests show missing trace bindings for the newly added FR-1042 value-bound criteria. AC-22 also has a production path gap: its fair-eventual interval helper is only exercised by a helper unit test and is not called by the analyzer.

## Verdict

**FAIL** — live criteria FR-1042-AC-19 through AC-24 have no test carrying their criterion ids, and the AC-22 interval path is disconnected.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-1042-AC-19 through AC-24 lack trace-tagged tests | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:259 | correct-requirement-no-evidence |
| FND-002 | medium | Fair eventual interval capability has no analyzer callsite | fsl-analyze/src/analyze/exact.rs:1407 | implementation-bug-despite-evidence |

## Coverage

Matrix: `quire matrix --scope /home/peter/dev/worktrees/QSL-2988-slice2 --format tsv` completed; `quoin matrix --repo ... --json` refused the request size, so I used the quire fallback. Reverse-gap and stub checks were performed over changed production modules; no other actionable issue identified. Semantic review skipped (not opted in). Plan completion: not assessed.
