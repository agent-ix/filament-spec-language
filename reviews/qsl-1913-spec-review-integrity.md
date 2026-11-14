---
id: SR-2026111414035503
title: "integrity review of QSL-1913 and QSL-1914 model-checker scaling spec"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md; spec/test-cases/TC-1417-worker-count-hash-indexed-seen-set-and-packed-state-leave-results-unchanged.md; spec/test-cases/TC-1418-a-directly-solved-assignment-keeps-the-successors-and-counts-its-candidates.md; PR quire-spec-language#2705 at a6d625440"
review_set: subset
---

## Summary

Tickets: QSL-1913 and QSL-1914. Structural and consistency check. `quire
validate` passes on every changed file. However, `quire matrix --scope .`
on the PR head lists FR-120-AC-28 and AC-30 to AC-33 but no FR-120-AC-29. It
lists FR-191-AC-1 to AC-5 but no AC-6 or AC-7. Three of the criteria the
downstream tickets would tag do not exist for the tooling. TC-1417 and
TC-1418 have well-formed frontmatter, and their `verifies` targets resolve.
ADR-016 G-4 now names AC-29 to AC-33 and TC-1418.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The FR-120-AC-29 row is on the same line as the AC-28 row (`... \| Test (TC-1270) \|\| FR-120-AC-29 \| ...`), so it is not a table row. `quire matrix` has no FR-120-AC-29. TC-1418 step 1 and the code ticket's `#[trace]` tag would point at a criterion that does not exist. | FR-120:855 | wrong-requirement |
| FND-002 | high | FR-191's table has a blank line after the AC-5 row. That ends the table, so the AC-6 and AC-7 rows are not part of it. `quire matrix` lists FR-191-AC-1 to AC-5 only. The parallel sampling ticket cannot tag either worker criterion. | FR-191:133-135 | wrong-requirement |
| FND-003 | medium | The diff names three request settings, `explore.workers`, `model_check.workers` and `statistical.workers`, but FR-255 does not register them. FR-255 Behavior 6/8 refuses any name in neither table as an unknown setting. Its list of request members that are not limits (FR-255:302-306) does not include `workers`. FR-255 also reserves `explore.*` for FR-101's exploration limits. As written, the settings operation would refuse all three names. | FR-101:76-80, FR-126:63, FR-191:88-92, FR-255:87, FR-255:302-306 | missing-requirement |
| FND-004 | low | FR-101 names the refusal type for `workers` 0 (`NotSimulated::Workers`), but FR-126 and FR-191 give only the catalog code. FR-126 names no `ModelCheckRefusal` variant and FR-191 no refusal type, and FR-191's input structs have no `workers` field. Two implementers could model the refusal differently. | FR-126:328-340, FR-191:88-110 | wrong-requirement |

## Verdict

Not mergeable until FND-001 and FND-002 are fixed. Both are one-line fixes:
split the FR-120 line, and delete the blank line in FR-191. FND-003 needs an
FR-255 entry, or a statement that `workers` is a library parameter and not a
setting. The rest of the structure is sound: IDs, relationships, TC
frontmatter and the ADR-016 G-4 row.
