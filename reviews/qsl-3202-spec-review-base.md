---
id: SR-967175026302197768900474181941935084834
title: "Base spec review — QSL-3202 FR-1049"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-1049-classify-alpha-tutorial-outcomes.md"
review_set: subset
---

## Summary

Reviewed FR-1049's structure, IDs, links, anti-pin policy, criterion coverage, and the requested QSL-3202 outcomes. The new rows are mostly source-grounded, but P4's derived-item outcomes are absent and two ticket requirements remain unresolved.

## Verdict

CONDITIONAL — findings identify missing outcome coverage and ticket-scope gaps that need an explicit owner decision or owning criteria.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | P4's omitted-items variation has no FR-1049 criterion for its output outcomes. FR-620 owns selecting the ten derived items, but not their eight unsupported_projection/unbounded-extent results and two proved results. Add an FR-1049 criterion for the outcome while referencing FR-620 for the selection rule. | FR-1049:44-89; FR-1049:95 |
| FND-002 | medium | QSL-3202 requires the journey gate's exact unsupported and budget messages, but no criterion specifies them. The PR body records that the candidate gate does not expose native message text. This scope cannot be completed without an explicit decision or follow-up owning requirement; keep the message contract reported as a gap rather than treating this batch as complete. | FR-1049:44-99 |
| FND-003 | medium | QSL-3202 also requires cancellation/fault outcome classification, but no concrete tutorial input or trigger is covered by the new criteria. The supplied matrix does not establish such an outcome, so this remains an explicit FR gap until an evidenced case or scope decision is provided. | FR-1049:44-89 |
| FND-004 | low | AC-39, AC-40, and AC-42 say “a” proved/refuted item but do not require exactly one item, although each supplied baseline request produces one. State the cardinality so the criteria reject extra or missing items. | FR-1049-AC-39; FR-1049-AC-40; FR-1049-AC-42 |

## Examined Scope

- FR-1049 title, relationships, description, inputs, outputs, behavior, references, and dependencies.
- FR-1049-AC-1..21 reviewed as context for existing probability coverage.
- FR-1049-AC-22..42 checked against the supplied non-probability variation matrix and candidate outcomes.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: “Exactly eight items have `category: unsupported` and `record: {result: unsupported, settled_by: disposition, diagnostic: {code: unsupported_projection, cause: unbounded-extent, detail: {candidate: contract-checker, kind: operation-contract, modes: [bounded]}}}`. The other two items have `category: success` and `record: {backend: en-1, basis: {type: exhaustive}, cause: null, certification: certified, result: proved, settled_by: result}.” |
| FND-002 | fixed | after_excerpt: “These criteria specify category, exit status, terminal stage, cause/code location and structured outcome shape; they do not specify native message text. FR-286 owns the outcome document and FR-865 owns its text rendering.” |
| FND-003 | fixed | after_excerpt: “The alpha tutorial CLI inputs do not expose a cancellation or fault trigger. No cancellation or fault row is specified here. Library API cancellation is owned by FR-286-AC-6; fault behavior has no tutorial input in this scope.” |
| FND-004 | fixed | after_excerpt: “exactly one proved `NeverDeadheads` item with certificate and record evidence” (AC-40 and AC-42 likewise now require exactly one item). |
