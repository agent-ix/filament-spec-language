---
id: SR-967175026302197768900474181941935084865
title: "Gap analysis — filament-spec-language QSL-3333 State-object rejection loci (PR #3949)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3949 diff; FR-163-AC-6/10/12, FR-169-AC-14/15, FR-245-AC-7, FR-338-AC-12/13"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-338", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-169", type: references }
---

## Summary

Ticket: QSL-3333. This is a planless gap audit scoped to the criteria that PR #3949 (head `9edac2a8a33f638a059f3a5a0aaa30a898557726`) touches or that the ticket's Impact names. The matrix was computed with `quire matrix --scope . --format json` on the worktree at that head.

Tagged and exercised: FR-163-AC-6, FR-163-AC-12 and FR-163-AC-10 (relation_certificate.rs), FR-169-AC-14 and FR-169-AC-15 (state_graph_settle.rs, state_graph_certificate.rs), and FR-245-AC-7. The new assertions check the written `at` by equality against an independently encoded State (`state_locus`), and against the schema's `StateLocus`/`EntryLocus`, `HyperTransitionTupleLocus` and `CertificateRejected`. The relation test also proves the schema refuses a bare `pre_state`/`post_state`, so each new assertion can fail.

Reverse gap and stubs: none. The new `EdgeWire`/`wire` helper is used by both serializers, and there is no stub code.

## Verdict

**CONDITIONAL.** FR-338-AC-12, the criterion that literally specifies `{RankMissing, ModelState(d)}` written "with `at` the state form of `d`" and admitted by the FR-331 schema, is untagged. No test writes that spelling. The PR's new tests cover the ModelState State form under `malformed` and `initial-missing` only. FR-338-AC-13 is untagged too, but that is QSL-967's existing scope (unlisted-pair check unimplemented).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-338-AC-12 (Rejection spellings, including `{RankMissing, ModelState(d)}` at the state form of `d`, each admitted by the FR-331 schema) is untagged in the computed matrix, and no test writes the RankMissing/ModelState cause. This ticket changes exactly that written form and lists FR-338-AC-12 in its Impact. A writer test tagged FR-338-AC-12, validating each spelling against `CertificateRejected`, belongs in this PR. | spec/functional/FR-338-check-an-en-1-closure-certificate.md (AC-12); fsl-replay/src/outcome.rs:4719 (query-part wire test, untagged for AC-12) |

## Coverage

- Criteria examined: 8. Tagged: 6. Untagged: 2 (FR-338-AC-12 in scope; FR-338-AC-13 belongs to QSL-967).
- Semantic review: skipped (not requested). Intent was checked by hand for the six changed tests only.
- Plan completion: not assessed

## Dispositions

Round 1, head `c2b7d10ab8bb90231e42f578a2fcda7c62648849`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `outcome::tests::certificate_rejection_spellings_conform_to_fr331` (fsl-replay/src/outcome.rs:4718-4823, fn at :4722) is tagged `FR-338-AC-12`. It writes all six spellings through `OutcomeDocument`, including `{RankMissing, ModelState(d)}` as `{"key": …}`, and checks each against `CertificateRejected`. It passes. |
