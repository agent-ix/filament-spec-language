---
id: SR-2026112508191402
title: "integrity review of PR 3876 snapshot tuple and union components spelling"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; FR-106, TC-464 (edited); FR-650, FR-360 item 9, FR-321 (context); PR 3876 head 46864ca27adae0679c423bc8f07c89f86377e13e"
review_set: subset
---

## Summary

Ticket: QSL-3263. Cross-file consistency of the `components` spelling and the refusal causes. FR-106 paragraphs, closed value list, AC-84 and TC-464 step 11 agree with each other and with FR-650's tuple/union rows (FR-650:146-174, 323-326) and FR-360 item 9's causes. No other spec file still spells snapshot tuple or union payloads `values`. Every named cause (missing-member, unknown-member, wrong-value-kind, invalid-value under invalid_runtime_input) is one the snapshot reader already emits (fsl-semantics/src/model/observation/document.rs admit_variant / admit_union_case). One carrier is left without a stated cause.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-106 states no cause for an extra (or missing) record payload member, though TC-464 step 11 exercises "each carrier with an extra payload member", record included; today's reader sends a record payload with a third member down the fallback arm to `wrong-value-kind`, while the PR makes the same defect `unknown-member` on tuple and variant, so the three carriers disagree and the test cannot catch it | spec/functional/FR-106-admit-snapshots-and-invocations.md:655-658, spec/test-cases/TC-464-snapshots-and-invocations-admit.md:127, spec/test-cases/TC-464-snapshots-and-invocations-admit.md:204 |

## Verdict

Spelling is consistent across FR-106, AC-84, TC-464 step 11 and FR-650; the code's typed-value writer and restore readers already use `components`. FR-321's use of wrong-value-kind for a non-member VariantId and a wrong payload count is a different surface (in-process kernel argument admission under FSpec FR-143, traced by FSpec-FR-143-AC-15 in fsl-eval/src/value/expression/union_admission.rs); FR-106 and FR-650 do not cite it, so it does not affect AC-84 and is not a finding here. FND-001 is not needed for AC-84 testability (the expected result asserts only refusal before an observation for the record control) but is a one-sentence fix.

## Scope

- FR-106 tuple and union paragraphs, AC-84: examined.
- TC-464 step 11: examined.
- FR-650 tuple/union rows and cause table: context_only.
- FR-360 item 9: context_only.
- FR-321 Behavior: context_only.
