---
id: SR-967186
title: "QSL-3513 base spec review of the cross-binding order, conformance objects and policy instances (filament-spec-language PR 4040)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@adf503520a905d72041960c5832c201e87296279; spec/decisions/ADR-018, spec/functional/FR-126, FR-621, FR-896, FR-990, FR-1012, spec/test-cases/TC-1085, TC-2532, TC-2800"
review_set: subset
date: 2026-11-26
---

## Summary

Ticket: QSL-3513. PR filament-spec-language#4040, head adf503520a905d72041960c5832c201e87296279.

Checklist pass over the nine changed files. `quire validate` on them reports no
error; its two grammar warnings on FR-1012 are on a statement this PR does not
change. New criterion ids are unique and each names a test case. No hash, digest
or pin is added. No passage in the spec tree still states stop-at-first-incomplete
for TP-4 bindings. The split between stops that end the run and stops that move to
the next binding agrees with FR-276 and FR-338, and the safety re-search satisfies
the rule that a refutation the run found is reported.

Not run: gap-analysis (the diff has no production code).

## Verdict

One medium finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new obligation has no acceptance criterion and does not say how a policy over several admitted keys is kept (presumably every key) or what the search returns when the type has none. FR-125 "Instancing" gives `NoInstance` for a clause item, but a plan request is not one. Failure: for `policy` over an empty population one implementer prunes nothing and reports `optimal`, another reports `no-instance`; a monitor of the first key only also passes every FR-1012 criterion. | spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md:37 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | The new refusal of `InductionStep` with `SmtInstances::First` has no acceptance criterion. Failure: an encoder that prints a step query restricted to the first instances passes AC-60 and every other criterion. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:141 |
| FND-003 | low | The text says the protocol deadlock item gets no subject per object but not what it decides when it carries no `subject.protocol` and the claim's `over` is an object type. Failure: FR-1033 sends it to the model subject, which binds no `over` object; the ticket records, unrun, that the code returns `invalid-value`. State that outcome. | spec/functional/FR-896-decide-a-global-conformance-claim-over-the-protocol-system-or-a-recording.md:90-92 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Checked at head 8bc5cfda3663de78b37a32d5be17391ce9345c54. |
| FND-002 | fixed | Checked at head 1de8eecdc4f9de17bd8e690943721fc493c3f671. |
| FND-003 | fixed | Checked at head 1de8eecdc4f9de17bd8e690943721fc493c3f671. |
