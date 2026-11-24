---
id: "SR-967175026302197768900474181941935084809"
title: "Base spec review — QSL-3012 acceptance tables"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; FR-099-AC-6/7, FR-174-AC-7/8, FR-315-AC-43/44 and the acceptance-table boundaries in the three changed files"
review_set: subset
---

## Summary

The three edits are whitespace-only and preserve criterion wording. Matrix validation shows all FR-099 and FR-315 rows, but FR-174-AC-8 remains separated by a blank line and is still hidden from Quire.

## Verdict

**CONDITIONAL** — FR-174-AC-8 is not extracted into the matrix.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A blank line remains between FR-174-AC-7 and FR-174-AC-8, so Quire omits AC-8 from the matrix. Remove the remaining blank line to satisfy the table visibility acceptance. | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:243 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Removed the remaining blank line before FR-174-AC-8; final matrix lists AC-1 through AC-8. |
