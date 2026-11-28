---
id: SR-2026112520490003
title: "Spec review — FR-1042-AC-27 and TC-3151 step 15"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md, spec/test-cases/TC-3151-certify-value-bounds.md; FR-1042-AC-27"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-1042", type: references }
---

## Summary

Ticket: QSL-3352. PR #3970 at head `0b64d104a3b74c1377c7bb1dd777cb5b2397c3c9`. Checked the new FR-1042-AC-27 against FR-1042 "Pairing" and "Document", FR-1043's measure certificate table, FSpec FR-411 "Evidence on a partial product" and FSpec FR-412-AC-5/AC-16, and the TC-3151 scope and step 15 edits.

## Verdict

**PASS** — AC-27 is consistent with FR-1042 "Pairing" and FR-1043 and contradicts no FSpec FR-411/FR-412 clause; the id is unused on main and in other open PRs. One low wording defect in the test case.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-3151 step 15 says "Over FR-1043-AC-2's model", but the AC-27 test uses the cyclic `link` model (`quantile()`/`measured()` helpers), not FR-1043-AC-2's `fast`/`slow` model. | spec/test-cases/TC-3151-certify-value-bounds.md:58 |

## Review Notes

- FR-1042 "Document" says a `lower` may name "any subset" of the stopped graph; the checker keeps the `max_states` stop for a partial `lower` naming a key not reached through named keys. That is a stop, not a rejection, and is the conservative choice; not recorded as a defect.
- FR-1042-AC-27 does not mention `mean of F` claims, which the code also refuses at `kind`; the AC lists examples and is not wrong.
