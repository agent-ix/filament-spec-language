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
