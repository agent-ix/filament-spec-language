---
id: SR-967184
title: "spec-integrity review of PR 4013 FR-1042-AC-28 and TC-3151"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md, spec/test-cases/TC-3151-certify-value-bounds.md; PR 4013 head 3bd1330ab159a085d94863acc33b32ad240485e0"
review_set: subset
---

## Summary

Ticket: QSL-3414. Integrity check of the new FR-1042-AC-28 and of TC-3151
steps 16 and 17. They were checked against FR-1042 "Pairing, side and
results", FR-1043 "Forms and documents" and FSpec FR-406-AC-3 (a claim holds
only when its bound holds for every initial state).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-1042 "Pairing" makes "0 with no tight activation" part of the `>= c` quantile threshold, and a missed threshold makes EN-5 raise the precision. AC-28 instead rejects a bound `InequalityFails`, so a valid `upper` rounded up to exactly 0 over a true claim (maximum below 0) settles CertificateRejected rather than raising precision | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:292 |
| FND-002 | low | TC-3151 step 17's expected result reads "InequalityFails at the initial key; nothing is proved". It does not state the `unsupported` settlement over two initial states that the test asserts | spec/test-cases/TC-3151-certify-value-bounds.md:110 |

## Verdict

The several-initial-states `unsupported` refusal agrees with FSpec
FR-406-AC-3 and with the exact route's existing refusal. It is a sound
refusal, not a truth value. FR-1043's bound rejection where activation is
unreachable agrees with "Forms and documents", which already names "bound
`shifted` document". AC-28's accepting half and its rejection locus agree
with FR-1043-AC-3. FND-001 is latent: the producer never offers a
claim-side `upper` today (code-review SR-967182 FND-001). It becomes live
once it does. Fix it in this PR by rejecting a bound's tight-zero failure
`ThresholdMissed`, or by stating in "Pairing" that EN-5 does not raise the
precision there.

## Scope and evidence

- FR-1042-AC-28: examined.
- FR-1042 "Pairing, side and results": context.
- FR-1043 "Forms and documents" and the every-scheduler quantile row: context.
- TC-3151 steps 16-17 and expected results: examined.
- FSpec FR-406-AC-3: context.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | The new FR-1042-AC-28 (`ThresholdMissed` at the initial key) contradicts FR-201 for the same document. FR-201's rejection table row, "General checking conditions" and FR-201-AC-5/AC-6 say a quantile `Upper` certificate with y(s0) = 0 and tight activation is rejected `InequalityFails` at the first activated state reached breadth-first. The FSL elapsed route and FSpec FR-753-AC-5 do the same | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:292 |
| FND-004 | low | FR-1042-AC-29 says any numeric `lower` or `upper` "as the claim side" is rejected `InequalityFails` where activation is unreachable, but a claim-side document of the wrong kind is rejected `Malformed` at `kind` first (AC-27; misplaced check precedes enumeration). The AC does not state that order | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:293 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC-28 and check_value_bounds now treat a bound's tight-zero failure as a missed threshold, matching FR-1042 "Pairing"; see FND-003 for the conflict this creates with FR-201 |
| FND-002 | fixed | Step 17's expected result now states `unsupported` over two initial states on both routes and no `proved` |
| FND-003 | fixed | Bound route reverted to `InequalityFails` at `tight_activation`'s breadth-first key; AC-28 and "Pairing" now state that rule and that EN-5 does not raise the precision, agreeing with FR-201 row 103, its checking conditions and AC-5/AC-6 |
| FND-004 | fixed | AC-29 now says a wrong-kind claim-side document is rejected `Malformed` at `kind` first (AC-27) |
