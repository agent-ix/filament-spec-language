---
id: SR-2026112402322100000002
title: "spec-review/integrity-analysis of FSL PR 3638 companion ACs (QSL-2779/2799/2801/2991/3001)"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR 3638 at c379cf79; spec/functional/FR-140,FR-141,FR-202,FR-337,FR-852,FR-930,FR-1040; TC-1326,TC-2600,TC-3030,TC-3084,TC-3172; TC-546 and fsl-analyze/tests/it/refinement_protocol.rs, fsl-replay/tests/it/refinement_unmapped.rs at main"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-141
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-202
    type: references
---

## Summary

Ticket: QSL-2779 (also QSL-2799, QSL-2801, QSL-2991, QSL-3001). This review covers PR agent-ix/filament-spec-language#3638 at c379cf79. It checks agreement with FSpec PR 828 at c46160c5, whether the FR-141-AC-14 and FR-141-AC-15 amendments orphan tagged tests, and AC ID uniqueness.

Clean:
- FR-337-AC-30 agrees with FSpec FR-540-AC-4.
- FR-930-AC-5 and FR-930-AC-6 agree with FSpec FR-760-AC-4 and FR-760-AC-5.
- FR-140-AC-18 and the FR-141 body agree with FSpec FR-375-AC-37 and FR-375-AC-38 at c46160c5.
- FR-1040-AC-11 and FR-1040-AC-12 agree with FSpec FR-829-AC-9 and FR-829-AC-10.
- Every new AC ID occurs once in its FR file at head.
- The FR-141-AC-14 amendment keeps the existing test `the_receiver_lifts_against_the_type_that_declares_the_operation` valid, because the target type there is the declaring type.

## Verdict

**NOT MERGEABLE.** There are two high findings: the amended FR-141-AC-15 contradicts its tagged test and TC-546, and FR-202-AC-46 contradicts FR-202-AC-41/42. There are also one medium and five low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-141-AC-15 is amended from `StepVerdict::Fault` to "passes with the argument lifted ... by the node's declared field type". TC-546 step 10's expected result still reads `StepVerdict::Fault`. The test `a_protocol_node_row_faults_when_its_reference_type_has_multiple_object_rows` (fsl-analyze/tests/it/refinement_protocol.rs:2233, traced TC-546/FR-141-AC-15) asserts `StepVerdict::Fault` on main. The test is now tagged to an AC it contradicts, and the TC was not updated. | FR-141-AC-15; TC-546 step 10; refinement_protocol.rs:2230-2314 |
| FND-002 | medium | FR-141-AC-16 cites TC-546, but TC-546's scope (FR-141-AC-1..5, FR-141-AC-11..14) has no step for AC-15 or AC-16. The criterion is untraced. | FR-141-AC-16; TC-546 |
| FND-003 | high | FR-202-AC-46 refuses "a part whose proof needs a ranking over the cyclic graph" with `invalid_runtime_input`. This contradicts FR-202-AC-41 and FR-202-AC-42, which accept `lower` certificates with ranks over cyclic subjects (for example, AC-41 ranks `s` 1 and `e` 0 on the s/e cycle). It mirrors FSpec FR-412-AC-28, which names no code, so the two repos also disagree on the refusal code. | FR-202-AC-46; FR-202-AC-41; FR-202-AC-42 |
| FND-004 | low | FR-202-AC-46's `eventually holds(E)` case gives no threshold or comparison direction, so "settles `refuted`" is underdetermined. | FR-202-AC-46 |
| FND-005 | low | FR-852-AC-7 refutes `possible Late` while the FSpec twin FR-712-AC-5 refutes `possible` of `phase = TimedOut` over the same Zeno-only vector. Name the same item in both. | FR-852-AC-7 |
| FND-006 | low | FR-1040-AC-10 says "the owner object's own reference is a different value from any of its places", which reads as a value comparison. FSpec FR-829-AC-8 refuses a clock-reference/object-reference comparison at check time (`ill_typed`/`operator-ineligible`). | FR-1040-AC-10 |
| FND-007 | low | FR-141-AC-16 says "`enq` and `enq2` are one inherited abstract operation", which gives two names to one operation. The vector is unclear about whether the rows map to one operation or to two. | FR-141-AC-16 |
| FND-008 | low | The FR-1040 "Runtime value of `Reference<Clock>`" bullet is appended after the closing summary ("Each form either lowers or refuses ..."), so it sits outside the list it belongs to. | FR-1040:194-210 |

## Scope

- FR-140 most-specific text, FR-140-AC-18 (examined, clean)
- FR-141 lifting text, FR-141-AC-14, FR-141-AC-15, FR-141-AC-16 (examined); TC-546 and tagged tests on main (context)
- FR-202-AC-21 amendment, FR-202-AC-46 (examined); FR-202-AC-41, FR-202-AC-42 (context)
- FR-337 read-back text, FR-337-AC-30 (examined, clean)
- FR-852-AC-7 (examined)
- FR-930 text, FR-930-AC-5, FR-930-AC-6 (examined, clean)
- FR-1040 runtime-value text, FR-1040-AC-10..12 (examined)
- TC-1326, TC-2600, TC-3030, TC-3084, TC-3172 additions (examined)

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | The FSpec fix round adds `taken` `{type: "failing"}` and `positions[].candidate_blocks` (FR-418, FR-418-AC-22, FR-740-AC-6), but FSL FR-244 and FR-245 are not amended. FR-244 writes `taken` `None` for "the failing edge into the entry", and its `positions` carry no clock blocks. FR-245 rejects `StepFails` for any failing step whose target is not a locked entry. FSpec now rejects `null` for an undelayed failing edge and accepts `failing`. The native emitter and checker spec therefore disagree with the FSpec wire on both items. | FR-244:78,159; FR-245:83-84,121; FSpec FR-418-AC-22; FSpec FR-740-AC-6 |
| FND-010 | low | TC-546 now says the tagged test "is replaced" by `a_protocol_node_row_lifts_by_the_declared_field_type` and `a_protocol_node_row_faults_when_no_object_row_conforms`. Neither exists on main (0 matches in refinement_protocol.rs), and the old `Fault` test is still tagged FR-141-AC-15. This states a code change as done in a spec PR. Phrase it as the expected tests, and track the code change on a ticket so the contradicting test does not stay tagged. | TC-546; fsl-analyze/tests/it/refinement_protocol.rs:2233 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | At d9cb3b64, TC-546 step 10's expected result is the pass by declared field type, and `Fault` is limited to a declared type no row conforms to, which agrees with the amended FR-141-AC-15. The old test on main is tracked as FND-010. |
| FND-002 | fixed | TC-546 scope is now FR-141-AC-11 to FR-141-AC-16, with step 11 for AC-16. |
| FND-003 | fixed | FR-202-AC-46 accepts ranked `lower` certificates over the cyclic chain per AC-41/42, and refuses only a fair-minimum (FR-1041) ranking part over a cyclic graph, as `inconclusive`/`certificate-rejected`/`malformed` at `{"member": "ranking"}`. This is identical to FSpec FR-412-AC-28 at 373286f6. |
| FND-004 | fixed | The `eventually` case is now `probability >= 1/2` with value 0. |
| FND-005 | fixed | FR-852-AC-7 says `Late` is the item for `c.phase = 3`, the `TimedOut` state of FSpec FR-712-AC-5. |
| FND-006 | fixed | FR-1040-AC-10 states that `eq` over a clock reference and an object reference refuses `ill_typed`/`operator-ineligible` at check time. |
| FND-007 | fixed | Both rows now map to the one operation `enq`, with target types `FastQueue` and `SlowQueue`. |
| FND-008 | fixed | The closing summary paragraph now follows the runtime-value bullet, and the wire form cites FSpec FR-181. |
