---
id: SR-967175026302197768900474181941935084806
title: "Gap analysis of claim-side and policy checking in PR 3672"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3672, TC-3212 steps 1 and 2; fsl-replay, fsl-analyze, tools/fsl-probability-check"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-1042", type: references}
---

## Summary

QSL-2988, PR #3672 at `e81b140ee1663e92105b26efc2ffb15ea0c5eae3`. FR-1042 owns the claim-side request flag, per-action checks, and independently retained policy. The changed tests exercise these paths through the checker process. A malformed policy member escapes the new parser and has no negative assertion in the changed tests.

## Verdict

**CONDITIONAL**. Reject an explicitly null policy at the policy member and add a negative assertion to the existing checker test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The bound-document reader treats an explicit null policy as absent. An otherwise valid document with this malformed member can be accepted. The checker must reject it as Malformed at policy. | fsl-replay/src/probability_certificate/document/value_bounds.rs:53, fsl-replay/src/probability_certificate/document/value_bounds.rs:162 |

## Coverage

`quire matrix` 0.52.1 (engine 0.62.2) reports `tagged` for AC-1, AC-2 and AC-16 at `tools/fsl-probability-check/tests/it/value_bounds.rs:224` and for AC-3, AC-14 and AC-17 at line 332. AC-1 and AC-3 supply checker scenarios here. Their complete analyzer settlement behavior is outside this slice. The code-review record separately identifies the omitted AC-17 minimum-upper claim-side scenario.

The parser removes a present policy field before parsing it as `Option<Vec<Row>>`. JSON null becomes `None`, the same value passed to the checker for an omitted member. The tests reject a disallowed empty array and rejected actions but do not assert rejection of null.

The touched role and hint code has a requirement owner and the changed tests exercise production paths. No stub was identified in the slice. The coder reported both scoped tests passed. The optional semantic review was skipped.

Plan completion: not assessed

## Reviewed units

- `FR-1042` (examined, `spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md`) — Duplicate, extraneous or structurally malformed rows SHALL retain FR-201's `Malformed` rejection and member locus.
- `FR-1042-AC-1` (context_only, `spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md`) — Over the cyclic `link` chain (from the undelivered state `s`, `1/3` each to `s`, delivered `g` and the stopped terminal `f`; value `1/2`) under `Even`, `probability >= rational(2,5) [eventually holds(m.delivered)]` with `max_rational_bits` 1 settles `proved`, `ValueBounds{lower, upper, …}` with `lower <= 1/2 <= upper`, and its `lower` and `upper` documents, `base.profile` `cyclic-dtmc-eventual/v1`, are each accepted by `fsl-probability-check` in a separate process and rejected with one value altered.
- `FR-1042-AC-2` (examined, `spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md`) — Over AC-1's chain, offered documents: `lower` `s` `7/16`, rank 1, with counterpart `upper` `s` `9/16` is accepted for `>= rational(2,5)`; `lower` `5/8` and `upper` `7/16` are rejected `InequalityFails` at `s`; `lower` `7/16` for `>= rational(1,2)` is rejected `ThresholdMissed` at `s`; `lower` without `ranking` is rejected `Malformed` at `ranking`; an `upper` document offered as the claim side of that `AtLeast` proof is rejected `Malformed` at `kind`; `f` at `1/16` in the `upper` document is rejected `InequalityFails` at `f`.
- `FR-1042-AC-3` (context_only, `spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md`) — Over `W` (FR-1041-AC-1) with no fairness set, maximum `1/2`: for `probability <= rational(3,5)`, `upper` `s` `3/5` (both actions satisfy `Q_a(y) <= y`) with counterpart `lower` `s` `7/16`, policy `go`, rank 1, settles `proved`; `lower` `s` 1 with policy `wait` is rejected `RankBroken` at `s`; the `lower` `s` `7/16`, rank 1, without `policy` is accepted with independently selected `go`, because `wait` fails progress while `go` satisfies inequality and rank. For the minimum (0), `upper` `s` `1/4` with policy `wait` is accepted and with policy `go` rejected `InequalityFails` at `s`.
- `FR-1042-AC-14` (examined, `spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md`) — In AC-3, minimum `upper` `s` `1/4` with hint `go` rejects `InequalityFails` although `wait` satisfies, and the same document without that hint accepts selecting `wait`. A nonexistent hint rejects `InequalityFails` even when an enabled action satisfies. Maximum `lower` `s` 1 with hint `wait` rejects `RankBroken`; removing the hint does not accept it, since no action satisfies both inequality and progress. When several actions satisfy every condition, an omitted hint selects the first in canonical full-product-action order.
- `FR-1042-AC-16` (examined, `spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md`) — Over AC-1's chain, `lower` `s` `7/16` for the claim `probability >= rational(2,5)` is accepted when checked as the claim side; its `upper` counterpart `s` `9/16` is accepted when checked with the claim-side flag unset. The `lower` checked as claim side for `>= rational(1,2)` rejects `ThresholdMissed`, and the `upper` offered as claim side of the `AtLeast` claim rejects `Malformed` at `kind`.
- `FR-1042-AC-17` (examined, `spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md`) — Over AC-3's `W`, the minimum-probability `upper` bound `s` `1/4` with policy `wait` is accepted for `probability <= rational(1,4)`, while policy `go` rejects `InequalityFails`; the maximum-probability `lower` bound `7/16`, rank 1 and policy `wait` rejects `RankBroken`, while policy `go` is accepted. When no policy is supplied the checker retains its first satisfying action in canonical full-product-action order.
- `TC-3212` (examined, `spec/test-cases/TC-3212-build-value-bounds-capabilities-in-conformance-order.md`) — 1. Check AC-1's lower and upper documents with the claim-side flag both set and unset, including the threshold miss and wrong-kind cases of AC-16. 2. Check AC-3's hinted and unhinted policies, including the rejected nonsatisfying hint of AC-17.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | Amended FR-1042-AC-17 calls for offering the Minimum upper document at 1/4 against the same Every AtMost 3/5 claim that accepted the Maximum upper. The refusal assertion changes the source threshold to 1/4 first, leaving the specified same-claim comparison uncovered. | tools/fsl-probability-check/tests/it/value_bounds.rs:371 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | At b8b2b596 the parser requires a vector on presence. The executable checker test asserts a null policy rejects as malformed at the policy member. |
| FND-002 | still-open | Reuse the unmodified 3/5 source claim for the mismatch assertion. The Minimum document's offered upper value and wait hint stay at 1/4 and wait. |
| FND-002 | fixed | At e4edb05b the test retains the original Every AtMost 3/5 claim and submits the Minimum upper document at 1/4 with wait. It asserts objective mismatch. |

The original verdict and AC-17 quotation above describe the reviewed head e81b140e. At b8b2b596, the final test accepts the Maximum upper at 3/5 in claim role and the Minimum upper at 1/4 with wait in counterpart role. The literal canonical action and the timed deadline wrong-kind assertion are present. The coder reported the locked workspace check passed for all targets and features. The remaining gap concerns the source threshold in the negative assertion.

**Final disposition at e4edb05b: PASS.** All findings are resolved. The coder reported the focused AC-17 regression passed (1/1).
