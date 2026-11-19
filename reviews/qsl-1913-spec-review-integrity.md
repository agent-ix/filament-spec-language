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

## New findings (disposition pass 1)

Checked at 6d3ffe68e.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | Stale references to the entry points. The FR-255 worker table names the request member of `explore.workers` as "`explore_request` `workers`", but only `explore_request_parallel` takes `workers` now. FR-101:233 still says integration tests "call the two entries", though there are now three. | FR-255:320, FR-101:233 |
| FND-006 | low | FR-255-AC-21 is verified by TC-720, an existing case on main. The diff does not amend TC-720: its scope and trace-tag list cover FR-255-AC-1 to AC-3 only, so nothing tells a coder to add the AC-21 test there. TC-720 is not a new id, and no open PR touches FR-255 or TC-720. | FR-255-AC-21, TC-720:32 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The FR-120-AC-28 row ends at `\| Test (TC-1270) \|` and AC-29 is on its own line. `quire matrix --scope .` at 6d3ffe68e lists FR-120-AC-1 to AC-33. |
| FND-002 | fixed | The blank line in FR-191's table is gone. The matrix lists FR-191-AC-1 to AC-7. |
| FND-003 | still-open | The new "Worker count setting names" section registers the three names and says the settings operation SHALL accept them. But Behavior 6, the rule that governs the settings operation, is unchanged. It still accepts only names "from any of the three tables" and refuses others, returns "the caller limits" (which have no worker member), and refuses value 0 as a usage refusal, not with `invalid_runtime_input`/`invalid-value`. FR-255-AC-21 says it "returns the request with `workers` 4", a return type Behavior 6 does not have. Amend Behavior 6 to cover the fourth table and say what it returns for a worker name and which refusal 0 gets. |
| FND-004 | fixed | FR-126 names `ModelCheckRefusal::Workers`. FR-191 names `StatisticalRefusal::Workers`, and `StatisticalRequest.workers` was added to FR-189. |
| FND-003 | fixed | Round 2 (a905dc14f): FR-255 Behavior 6 now accepts names from "the three tables or the worker table". It returns the caller limits with `workers` (`WorkerCounts { explore, model_check, statistical }`, each default 1, read by `explore_request_parallel`, `check_model` and `check_statistical`, which exists at qsl-analyze/src/statistical/model.rs:117). It refuses a worker name with value 0 as a usage refusal with cause `invalid_runtime_input`/`invalid-value`. This now agrees with the "Worker count setting names" section and with FR-255-AC-21, which reads `workers.explore` and the other two counts from the returned caller limits. |
| FND-005 | fixed | Round 2: the worker table names `explore_request_parallel` `workers`, and FR-101:233 says "the three entries". |
| FND-006 | fixed | Round 2: TC-720's scope adds FR-255-AC-21, step 4 drives every AC-21 operand, the expected results cover each one, and the trace-tag list includes AC-21. No open PR touches FR-255 or TC-720. |
