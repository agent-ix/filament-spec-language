---
id: SR-2026112105500003
title: "Spec review — PR #3090 FR-766 and TC-2240 edits"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR #3090 at 901c5e27ba: spec/functional/FR-766-prove-the-result-range-of-count-at-check.md, spec/test-cases/TC-2240-a-count-range-in-a-state-clause-refuses-under-both-modes.md"
review_set: subset
evaluated_revision: "901c5e27ba"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-766
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-2240
    type: reviews
---

## Summary

Ticket: QSL-2003. This review covers the FR-766 and TC-2240 edits, with the integrity and EARS checks folded into the base review. The amended FR-766-AC-8 gives the invariant's source an annotation, and the new FR-766-AC-9 refuses an unannotated literal source with `ambiguous-literal`. Both match the arch ruling, and both match the tests in check_mode.rs exactly.

**TC-2240.** The edit is correct and consistent with FR-766, so keep it. FR-766-AC-9's verification column names TC-2240, so TC-2240 has to gain the AC-9 scope, step 3, the expected result and the second test name, and it did. Reverting it would leave AC-9 with no verifying test case. The edit misses one thing: the Description was not updated for AC-9 (FND-005).

## Verdict

**FAIL. One fix round is needed: two medium findings and three low.** FND-001 and FND-002 are on lines this PR changed. FND-003 is pre-existing text in the same file; fixing it now is recommended but does not block the merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The Notes paragraph that this PR rewrote still says a fixture "SHALL write the `count<Int[0, 0]>` atom in a called function body". The parser refuses an inline type argument to `count` (fsl-cst/src/parser.rs:1987-2012: "`count`, `sum`, `fold`, and `reduce` require a declared type name here"), so a reader cannot follow the instruction as written. | spec/functional/FR-766-prove-the-result-range-of-count-at-check.md:135 |
| FND-002 | medium | The Notes now say the kernel run-time refusal "applies only to a `count` outside a declared clause", where they used to say a declared state clause. They also drop the temporal-clause-atom route. The Behavior bullet still says Kernel records no obligation for a count "in a function body ... or in a temporal clause atom". A temporal clause is a declared clause, so the Notes and the Behavior now disagree on whether a temporal atom reaches S6a. | spec/functional/FR-766-prove-the-result-range-of-count-at-check.md:132; spec/functional/FR-766-prove-the-result-range-of-count-at-check.md:93-95 |
| FND-003 | low | Pre-existing text in the touched file uses the same parser-refused inline form: Behavior's example `count<Int[0, 3]>` and AC-1 to AC-4 (`count<Int[0, 3]>`, `count<Int[1, 3]>`, `count<Int[0, 5]>`). The tests already use declared names (`Three`, `Positive`, `Tiny`). | spec/functional/FR-766-prove-the-result-range-of-count-at-check.md:84; spec/functional/FR-766-prove-the-result-range-of-count-at-check.md:117-120 |
| FND-004 | low | FR-766-AC-9 cites "(FSpec FR-144-AC-10, FR-981-AC-4)". FR-981 is this repository's FR, but the citation reads as if both are FSpec, and this repository's FR-144 is an unrelated refinement FR. FR-766 has no relationship edge to either, and no Behavior bullet states the ambiguous-literal outcome AC-9 tests. | spec/functional/FR-766-prove-the-result-range-of-count-at-check.md:125 |
| FND-005 | low | TC-2240's Description still says it verifies only an unproved count range and the function-body contrast. It now also verifies AC-9's ambiguous-literal refusal. | spec/test-cases/TC-2240-a-count-range-in-a-state-clause-refuses-under-both-modes.md:15-18 |

## Findings detail

- **FND-001. Fix:** write "SHALL write the `count<Zero>` atom, with `type Zero = Int[0, 0];`, in a called function body checked under `CheckMode::Kernel`", which matches the example two sentences later.
- **FND-002. Fix:** say "outside a declared state clause" in the Notes again, and keep or restore the temporal-atom route. If the intent is that temporal atoms must also use a called function, edit the Behavior bullet at line 95 to match. In both cases the two sections must agree.
- **FND-003. Fix:** declare `type Three = Int[0, 3];` and the other names in the AC text, as AC-8 now does, and use `count<Three>` and so on. If this is not done here, record a follow-up.
- **FND-004. Fix:** write "(FSpec FR-144-AC-10; FR-981-AC-4)". Add a `references` edge to `ix://agent-ix/filament-specification/FR-144` and to `ix://agent-ix/filament-spec-language/FR-981`. Add one Behavior sentence: a collection literal as `count`'s source has no expected type and refuses `ill_typed`/`ambiguous-literal`.
- **FND-005. Fix:** add a sentence to the Description: "and that a bare collection literal as the count source refuses `ambiguous-literal` (AC-9)". The title may stay.

## Findings data

```yaml
clean: false
scope:
  - {id: FR-766-AC-8, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "invariant I using v on Config::ConfigVersion at current { let q: Sequence<Integer>[0, 1] = sequence[self.versionNumber] in count<Zero>(m in q: m >= 0) = 0 }, whose source q is annotated so its literal has an expected type, refuses undefined_expression/unproved-range at the count in both modes"}
  - {id: FR-766-AC-9, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "the AC-8 invariant written with the literal as count's source, count<Zero>(m in sequence[self.versionNumber]: m >= 0) = 0, refuses ill_typed/ambiguous-literal in both modes, because count gives its source no expected type (FSpec FR-144-AC-10, FR-981-AC-4)."}
  - {id: FR-766, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "Notes: The kernel run-time refusal applies only to a count outside a declared clause. ... SHALL write the count<Int[0, 0]> atom in a called function body checked under CheckMode::Kernel."}
  - {id: FR-766-AC-4, path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", role: examined, excerpt: "Under CheckMode::Kernel, count<Int[0, 3]>(x in q: x) for q of Sequence<Boolean>[0, 5] checks"}
  - {id: TC-2240, path: "spec/test-cases/TC-2240-a-count-range-in-a-state-clause-refuses-under-both-modes.md", role: examined, excerpt: "Scope: FR-766-AC-8, FR-766-AC-9. ... 3. Check the invariant of FR-766-AC-9, whose count source is a bare literal, under each mode."}
findings:
  - {fnd: FND-001, severity: medium, confidence: high, method: spec-review, check: soundness, artifact_id: FR-766, related_ids: [], path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", lines: "135"}
  - {fnd: FND-002, severity: medium, confidence: high, method: spec-review, check: ambiguous, artifact_id: FR-766, related_ids: [FR-495], path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", lines: "132"}
  - {fnd: FND-003, severity: low, confidence: high, method: spec-review, check: other, artifact_id: FR-766-AC-4, related_ids: [FR-766-AC-1, FR-766-AC-2, FR-766-AC-3], path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", lines: "117-120"}
  - {fnd: FND-004, severity: low, confidence: medium, method: spec-review, check: trace, artifact_id: FR-766-AC-9, related_ids: [FR-981-AC-4], path: "spec/functional/FR-766-prove-the-result-range-of-count-at-check.md", lines: "125"}
  - {fnd: FND-005, severity: low, confidence: high, method: spec-review, check: other, artifact_id: TC-2240, related_ids: [FR-766-AC-9], path: "spec/test-cases/TC-2240-a-count-range-in-a-state-clause-refuses-under-both-modes.md", lines: "15-18"}
```
