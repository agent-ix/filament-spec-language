---
id: SR-967175026302197768900474181941935084850
title: "Base specification review — QSL-3222 TC-1581 update"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/test-cases/TC-1581-analyze-outcome-document-carries-verdict-certificate-counterexample-and-accounting.md; FR-126-AC-26; FR-621-AC-19 through FR-621-AC-22"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-126", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/FR-621", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/TC-1581", type: reviews }
---

## Summary

The TC-1581 change adds a one-state bounded fairness case and identifies FR-126-AC-26 as its owner. The new expected result checks incomplete status, one explored state, no counterexample and exit 22, but omits the exact `MaxStates` cause/value required by FR-126-AC-26. The existing ordered-results procedure also requires request indices and claim names that the touched source test does not assert. No applicable AssuranceProfile exists under `spec/assurance/`.

## Verdict

**CONDITIONAL** — the added case is clear and tied to an existing requirement, but its expected result is weaker than the owning acceptance criterion.

## Checklist

| Check | Result |
| --- | --- |
| IDs, relationships and links | pass — referenced IDs resolve; TC-1581 is linked to FR-621 |
| Requirement clarity and scope | pass — step 9 names one fairness claim with max_states 1 and one admitted state |
| Error and boundary behavior | finding — the expected result omits the exact MaxStates cause/value |
| Criterion coverage | finding — the source test does not assert the item identity/order required by the existing step 8 |
| Verification method | pass — FR-126-AC-26 declares Test (TC-521); this addition supplements its runtime evidence |
| Prohibited hash/digest/pin use | pass — none added |
| Review policy | no applicable AssuranceProfile is present in `spec/assurance/` |
| Validation | pending reviewer artifact validation |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Step 9 does not state that the incomplete result's cause is `LimitReached{max_states}` with value 1, as FR-126-AC-26 requires. | TC-1581 step 9; FR-126-AC-26 |
| FND-002 | medium | Step 8 requires exact request-index and claim-name ordering, but the test bound to this TC only checks result kinds and counterexample/certificate presence. | TC-1581 step 8; FR-621-AC-19..22 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Step 9 now requires the MaxStates cause, setting and value 1. |
| FND-002 | fixed | The source test now checks request_index and claim_name for every fairness item. |

After excerpt (FND-001):

```markdown
- Step 9 (FR-126-AC-26): the limited item is `incomplete`, reports `exploration.states` `1`, has `record.cause.kind` `limit-reached`, `record.cause.limit` `max_states`, `record.cause.setting` `model_check.max_states` and `record.cause.value` `1`, has no `counterexample`, and the document exits 22.
```

After excerpt (FND-002):

```rust
assert_eq!(items[index]["request_index"], index, "{}", items[index]);
assert_eq!(items[index]["claim_name"], claim_name, "{}", items[index]);
```
