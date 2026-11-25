---
id: SR-7932
title: "spec review of QSL-3398 FR-569-AC-5/6 twin effective precondition (FSL PR #3963)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@25173079b60e0031e1fe0c86e295ed0cf856a7de; spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md, spec/test-cases/TC-1408-declare-object-types-populations-and-operations-in-source.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-1408
    type: reviews
---
# SR-7932: spec review of QSL-3398 FR-569-AC-5/6 twin effective precondition (FSL PR #3963)

## Summary

Ticket: QSL-3398. PR agent-ix/filament-spec-language#3963 at 25173079b6, merge base f9db25eff9.
Method: spec-review (consistency and integrity of the edited FR-569 bullet, FR-569-AC-5,
FR-569-AC-6 and TC-1408 steps 4 and 5), checked against arch ruling c9e092a7, FSpec PR #848
(FR-322 "Clause functions" item 5 and FR-322-AC-544), FR-094 "Clause function nodes" and the code
in `fsl-semantics/src/check/checked_dispatch.rs:1136-1176` and `dispatch_link.rs:473-482, 541-550,
642-648`. Cargo was not run; the probe evidence is the author's logs in
the local probe logs.

The new FR-569 bullet, AC-5 and AC-6 match ruling c9e092a7 items 1 and 3 and FSpec #848's item 5
sentence. AC-5 now states the one S/D table difference (D reuses `Qs`'s authored function for
`Order.size`; S names its own function with the same body). AC-6's "Sub.size has none in S" is
right: `effective_terms` returns `None` for a member with no own `pre`
(`checked_dispatch.rs:548-552`), and the probe's S row shows `Sub` with `effective=None`.

Check charges: no FR-569 text (AC-10, "Every evaluation over a source-declared object type SHALL
give the outcome and the ordered charge log ...") and no FSpec text promises equal S3 check
charges between S and D. The promise covers evaluation only, and the probe supports it: ordered
charge logs equal (10/10 with `Qs` true, 5/5 with it false); the false-precondition outcomes differ
only in the owner spelled in `selected`, which is the allowed "node ids, owners" difference. The
counterfactual shows S and D check work already differed without the function (235 against 262),
so equal check work never held. Arch accepted the S3 difference (comment e2495c29) with an added
FR-569 sentence.

Occurrence contradiction: the spec is right and the code is wrong (FND-001). FR-569's clause
function bullet is the only normative statement of the span (FSpec item 5 says only "one
`declaration` occurrence"); the code places the effective precondition at the first `pre` clause
of the candidate (`precondition_span`), so in S `["Order","size","effective_precondition"]` and
`["Order","size","precondition","Qs"]` share the `pre Qs` span, and in S-R Sub's effective
function sits on `pre Qs2` although it also lowers `Qs`. The member span is distinct per candidate
and matches `<type>` being the candidate. This PR also makes the bullet incomplete (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-569-AC-6, edited by this PR, keeps "with its occurrence at `Sub`'s `size` member" for `["Sub","size","effective_precondition"]`, and the FR-569 bullet says "the span of the redefining `operation` member". The code places it at the first `pre` clause (`dispatch_link.rs:541-550` via `precondition_span`, 642-648); the probe-emit log shows the `pre Qs2` span in S-R and the `pre Qs` span for Order's. No test asserts the occurrence. The spec side is right. Fix the code here if arch extends scope, or file a defect ticket with a failing test before merge; do not merge AC-6 asserting it silently. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:544-547, :685, fsl-semantics/src/check/dispatch_link.rs:541-550, :642-648 |
| FND-002 | medium | The new bullet gives a non-redefining candidate (`Order.size`) an effective-precondition function, but the occurrence rule only covers "the redefining `operation` member", so no text gives `["Order","size","effective_precondition"]` its occurrence; AC-6 and TC-1408 step 5 state none. Reword to "the span of the candidate's `operation` member" and state Order's in AC-6. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:520-523, :544-547, :685 |
| FND-003 | low | "Only the node ids, owners and identity rows differ." now reads wider than true: S also has one more clause function per single-term candidate and larger S3 check charges. Scope the sentence to evaluation, or let the arch-accepted e2495c29 sentence name the difference next to it. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:652-661 |

## Verdict

Not mergeable yet. The AC-5/AC-6 rewrites are correct and match the ruling, FSpec #848 and the
code for table contents. FND-002 is a spec edit inside this PR's scope. FND-001 needs a disposition
before merge: a code fix (outside the ruling's "test and spec only") or a filed defect ticket with
a failing test. FND-003 can ride with the arch-accepted check-charge sentence.

## Dispositions

Round 1 reviewed head dd73650661eebaa7142ca31dedc717e95d524e1d (205ba46972 production, tests and
spec; 68da2d5f1a `implements` tags; 3d2f69bed6 review files; dd73650661 merge of main 550adf30da).
Cargo was not run; gate logs local gate logs.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `dispatch_link.rs` `clause_declaration_span` places an effective precondition at `source_operation_spans` (the candidate's `operation` member, filled by the assembler) and `region.rs` `clause_declaration_region` returns it for the `declaration` occurrence; `a_source_clause_function_is_located_at_the_declaration_it_is_declared_by` asserts the literal member text for Order's and Sub's. |
| FND-002 | fixed | The FR-569 bullet now reads "for an effective precondition, the span of the candidate's `operation` member", AC-6 states Order's occurrence at `Order`'s `size` member, and TC-1408 step 5 and its expected result name every occurrence. |
| FND-003 | fixed | The sentence is scoped: "In these evaluations only the node ids, owners and identity rows differ; S3 check charges can differ too", beside the arch-accepted e2495c29 sentence in the effective-precondition bullet. |
