---
id: SR-2026111922220600000001
title: "spec-review of QSL-2629 FR-317 integer payload widening for union-member construction"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3117 at 1c4b05fc; spec/functional/FR-317-resolve-member-references-and-check-union-construction.md (Integer payload widening paragraph, FR-317-AC-7, FR-317-AC-8, Dependencies), spec/functional/FR-065-migrate-function-application-to-checked-family.md (widening bullet cross-reference), spec/test-cases/TC-3134-union-payload-integer-arguments-widen-like-function-call-arguments.md; context: fsl-semantics check/typing.rs Callee and Frame::Application, check.rs coerce and lowers_as_integer, family.rs call_parameter, lowering.rs NodeKind::Tuple, refusal.rs; Contract IR FR-347 and operations.rs union_defect/check_tuple_position at 2ec296a1; FSpec FR-149 and FR-143 at 04109664"
review_set: subset
---

## Summary

Spec review of FSL PR #3117. Ticket: QSL-2629 (ruling comment 10838a64); code follow-up QSL-2632. The PR adds FR-317 "Integer payload widening": a union-member payload argument of type `Int[..]` whose member payload type is `Integer` or a containing `Int[..]` gets the explicit exact conversion FR-065 gives a call argument, a narrowing payload gets the call's range obligation and `undefined_expression`/`unproved-range` at the argument, and `Some(e)` keeps FSpec FR-143-AC-21. It adds FR-317-AC-7, FR-317-AC-8, TC-3134 and a cross-reference in FR-065.

## Examined scope

- FR-317 Integer payload widening paragraph (examined). It matches the ruling. In the checker, union payloads already take the call path: `ApplicationFrame::application` checks each payload with `Goal::Expect(Expected::Checked(member.parameter(i)))`, and `coerce` (check.rs:2812) admits a containing integer type unchanged and gives a narrowing one a range-obligated `Coerce` node at the argument's location. The only difference from a call is `Callee::call_parameter` returning `None` for `Union` (typing.rs:563-568), so no `convert` is added (typing.rs:3053-3070). The code change for QSL-2632 is that one arm. FND-001 and FND-004 apply.
- Code and cause strings (examined). `Unproved(Obligation::Range{..})` gives `Code::UndefinedExpression` (refusal.rs:3762) with tag `unproved-range` (refusal.rs:3862). `coerce`'s mismatch is `ill_typed`/`type-mismatch` at the node's location. The strings in AC-7, AC-8 and TC-3134 match.
- Contract IR FR-347 "Case arm bodies and union payloads" (examined). The FR states that a non-record `union_value` payload term must have exactly the member's payload type, refused `ill_typed`/`type-mismatch` at the `union_value` node. `union_defect` (operations.rs:3602-3726) implements it through `conforms_to_type`. The PR cites it correctly.
- FSpec FR-143-AC-21 (examined). `Some(e)` is excluded explicitly, so the FSpec exact option rule ("no implicit widening applies, so `Some(b)` for `b: Int[0, 3]` ... is `type-mismatch`") is kept.
- FSpec FR-149 (examined; FND-001).
- FR-317-AC-7 (examined; FND-002). It can fail today: with `call_parameter` `None` for unions, `Raw(x)` has no conversion and the strict reader refuses the package.
- FR-317-AC-8 (examined). It pins behaviour main already has through the shared `Expect`/`coerce` path. It can fail if a union-specific path is added. `Tiny(true)` stays `type-mismatch` at `true`.
- Tuple-constructor positions, the author's open note (examined; FND-003).
- Golden packages (examined; FND-005). The union fixtures in fsl-package `emit/tests/union_nodes.rs`, `source_union_rows.rs`, `admission_corpus.rs` and fsl-replay union tests build `Integer` payloads from literals or `Integer` values, which take no conversion.
- Ids (examined). FR-317-AC-7, FR-317-AC-8 and TC-3134 are free on main and in the 25 other open FSL PRs. #3114 holds TC-3133, not TC-3134. No other open PR edits FR-317 or FR-065.
- `quire validate --scope .` passes on all three files.

## Verdict

Not mergeable as is, only because of FND-002, which needs one sentence in AC-7 and in TC-3134 step 3. After that it is mergeable. The rule is the call rule, the checker already routes union payloads through the call path, and the cited Contract IR rule is real. FND-001 and FND-003 need tickets, not changes to this PR. FND-004 and FND-005 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FSpec FR-149 says the reference upcast is "the only implicit subsumption admitted for arguments and bindings". FR-143 says of `Some` payloads that "no implicit widening applies". FSL FR-065's call-argument widening (QSL-1824) already goes beyond this. It does not cite FR-149, and no ADR records the deviation. This PR extends that widening to union payloads. The ruling's "no FSpec change is needed" covers the Contract IR reader, not FR-149. An FSpec FR-149 amendment ticket is needed, admitting exact integer widening at declared function-parameter and union-payload positions as an explicit `numeric.convert`, with option payloads excluded. Otherwise FSL must record the deviation | spec/functional/FR-317-resolve-member-references-and-check-union-construction.md:108-130 |
| FND-002 | medium | The AC-7 negative control, "conversion removed by hand", does not say that the edited package must otherwise be well formed. The V2 reader requires each node's `dependencies` to be exactly its reference targets, and it checks re-derived identities. A hand edit that only repoints the payload term at `x` can therefore be refused earlier with a different code. Then the `type-mismatch` at `union_value` is never observed, or the test asserts against the wrong stage. AC-7 and TC-3134 step 3 should require re-derived dependencies and identities and the orphan convert node dropped, with the edited package shown to pass the earlier stages, as Contract IR FR-038-AC-231 does | spec/functional/FR-317-resolve-member-references-and-check-union-construction.md:149 |
| FND-003 | low | Out of the diff, as the brief asked. Tuple-constructor positions get no widening conversion: `Application::call_parameter` returns `None` for `Tuple` (family.rs:1349-1354), and FR-065's tuple bullet says nothing about widening. `Pair(x)` with `x: Int[0, 1000]` at an `Integer` position therefore emits a `tuple_value` term typed `Int[0, 1000]`. This is not the union defect: the Contract IR reader has no `tuple_value` position-type join (`union_defect` covers unions only, and `check_tuple_position` is for `tuple.project`), so nothing is refused today. It is still an inconsistency with calls and unions, and it breaks once the reader checks tuple positions. Ticket scope: rule tuple-position widening in FR-065's tuple bullet, return the position type from `call_parameter` for `Tuple` with a TC, and in Contract IR decide whether `tuple_value` positions are matched exactly (FR-038/FR-069) | spec/functional/FR-065-migrate-function-application-to-checked-family.md:120-123 |
| FND-004 | low | The new SHALL is unconditional, as FR-065's bullet is, but the call path skips the conversion when the parameter is `Integer` and the argument already lowers as `Integer` (`lowers_as_integer`, typing.rs:3063-3066; integer division, integer-lowered locals). A union payload that mirrors the call path will skip it too, and the text does not state the exception. The package type node is already `Integer` there, so the reader's exact match holds. Add the exception to FR-065 and FR-317, or drop it in the code | spec/functional/FR-317-resolve-member-references-and-check-union-construction.md:108-114 |
| FND-005 | low | QSL-2629 says the widening "changes package bytes for sources with bounded-integer union payloads", but neither the PR nor FR-317 says whether any golden or published package changes. The sampled union fixtures use literal or `Integer` payload arguments, so probably none change. State this in the PR or in QSL-2632, so a golden diff in the code PR is not a surprise | spec/functional/FR-317-resolve-member-references-and-check-union-construction.md:108 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 at ef9ffdd2: FR-065 and FR-317 now cite FSpec FR-149 "Integer widening at declared parameter and payload positions", added by FSpec PR #762. FSL #3117 has to merge after #762 |
| FND-002 | fixed | Round 1 at ef9ffdd2: AC-7 and TC-3134 step 3 now require that the conversion node be removed and that dependencies and identities be re-derived. They also require that the edited package first pass strict wire, graph and identity admission without the construction |
| FND-003 | deferred | Tuple-constructor position widening is ticketed as QSL-2636, out of scope for this PR |
| FND-004 | fixed | Round 1 at ef9ffdd2: FR-065 and FR-317 now state the exception for an argument the package already carries as `Integer` (an integer division, or a local lowered as `Integer`) |
| FND-005 | fixed | Round 1 at ef9ffdd2: FR-317 now has a "Package impact" paragraph. Literal or same-typed payloads emit the same bytes, so union goldens built from them do not change; only sources with a bounded payload argument at a wider payload type change |
