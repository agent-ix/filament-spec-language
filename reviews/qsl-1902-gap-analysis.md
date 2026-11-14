---
id: SR-2026111413443800000002
title: "Gap analysis of QSL-1902 named populations in role predicates"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR 2703 head 1d7d757e30c89e785624ad96eec60bf7b2552867; FR-208-AC-6, FR-216-AC-6 against qsl-semantics/src/check/protocol_slots.rs and qsl-analyze/tests/it/protocol_roles.rs, protocol_footprints.rs"
review_set: subset
---

## Summary

Ticket: QSL-1902. A planless gap analysis of the PR diff, done by hand from
the code. No build was run and `quire matrix` was not run, because the dispatch
said to read code only.

FR-208-AC-6 is now tagged in full by
`a_retire_predicate_reads_a_named_population`
(`#[trace("TC-653", "FR-208-AC-6")]`). The test checks the predicate. It
asserts that `retire(r, c)` is disabled while `c.v = 1` and enabled exactly
for `c` once `c.v = 2`. It asserts that the negated membership enables no
retire. It asserts that the `M::absent` variant refuses
`missing_declaration`/`missing-name` located at `absent`, keeping the name
`M::absent` and the role owner `protocol R role r until`.

The test fails without the checker change. With no `NamedScope`,
`named_population` returns `None`, and `M::cells` in value position refuses
`MissingSegment` (typing.rs:1312-1325), so `World::new` panics on the
unchecked unit. `M::absent` would come back wrapped as a `SlotClause` cause,
not as a `MissingSegment` located at `absent`.

FR-216-AC-6 is still partial. The membership clause is now asserted in the
TC-661 test, which accepts either `Membership` or `AnyMembership` of `cells`
on the `retire` step. The record-binder guard clause is still blocked on
QSL-968. The test cites only `TC-661`, not the criterion, and its "Partial:
Remaining work: QSL-968" note names the one remaining clause. The tagging is
honest. The old untagged capture test's doc no longer claims a QSL-968 gap
for populations.

## Verdict

**PASS**, with one low test-intent finding. Both claims of this PR are backed
by tests that can fail. Nothing is over-claimed, and the diff has no stubs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The doc comment says "with the membership negated nothing ever retires", but the test checks only two states: both instances live, and after the attempt. The capture `j = k` is always a member of `cells`, so no world shows the `j`-not-a-member side of "exactly where ... j is a member". That side is covered only through the negated predicate | qsl-analyze/tests/it/protocol_roles.rs:2003, qsl-analyze/tests/it/protocol_roles.rs:2045-2050 |

## Coverage

- FR-208-AC-6: tagged (`a_retire_predicate_reads_a_named_population`, TC-653). It fails without the change.
- FR-216-AC-6: partial, untagged by design (TC-661 only). The membership clause is asserted, and the record-binder guard clause is deferred to QSL-968.
- Semantic review: intent was checked by hand for the two criteria above only.
- Plan completion: not assessed
