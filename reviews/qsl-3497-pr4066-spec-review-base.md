---
id: SR-967196
title: "base review of PR 4066 spec changes"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-201; spec/test-cases/TC-1401..TC-1406, TC-3190"
review_set: subset
---

## Summary

Ticket: QSL-3497. PR 4066 at 8ae7ff9a03772b5bd38d40fa07eebc753885c103. 2 findings.

## Verdict

**CONDITIONAL**. The 'Instancing outcomes' order and refusals agree with FR-125, FR-203 and FR-621; several-binding text is unchanged (profiles still leave multiple initials or bindings unsupported, FR-203-AC-21 unchanged); no other file under spec/ outside reviews still states UnsupportedBinding for zero instances; `quire validate` on the changed files reports only an unrelated module warning.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | 'Zero over instances give NoInstance' has no non-empty-initial-list condition. A key is an instance only if an initial state admits it (FR-125), so a subject with an empty initial list always has zero instances; this text then requires NoInstance, while FR-125's order, FR-125-AC-13, FR-203-AC-20 and FR-201-AC-85 require NoInitialState. The same form is in the seven profile sentences (lines 574, 1168, 1270, 1335, 1447, 1564, 1727), AC-34/38/42/46/74 and TC-1401..1406 and TC-3190. Fix: say 'a subject with an initial state and no admitted over instance', or remove the copies and keep one statement under 'Instancing outcomes'. | spec/functional/FR-201-check-a-probability-certificate.md:1776 | wrong-requirement |
| FND-002 | low | The procedure only binds the claim, but the expected result asserts three routes: direct claim binding, the checker and the direct EN-5 item. A test written from the procedure leaves the checker and EN-5 results unexercised. TC-1401 step text (line 47-48) likewise names binding and EN-5 but not the checker. | spec/test-cases/TC-3190-check-every-scheduler-long-run-proofs.md:24-25 | correct-requirement-no-evidence |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Condition added (universe not missing, empty initial list vs initial state with no instance); agrees with FR-125 order and FR-201-AC-85/86. |
| FND-002 | fixed | TC-3190 and TC-1401 procedures now run the case on direct claim binding, the checker and the direct EN-5 item. |
