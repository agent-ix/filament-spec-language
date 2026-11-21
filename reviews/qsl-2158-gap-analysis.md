---
id: SR-202611210022442313144108509
title: "QSL-2158 PR 3285 FR-1043 finite-horizon measure profile gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@986e56a8e157cc8cf694ff4daed7cd2dcdc144e0 (merge-base f3aa681d7ed753a2cdd998bc24842ef38dda3c86); criteria FR-1043-AC-1..AC-11, FR-202-AC-27..AC-30, FR-203-AC-10..AC-11, FR-1042-AC-12..AC-14 (added in this PR); computed quire matrix; tests fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs, tools/fsl-probability-check/tests/it/certificate_kinds.rs, fsl-eval/src/probabilistic.rs tests; production diff of PR 3285"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-1043", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-1042", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-202", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-203", type: references}
---

## Summary

Ticket: QSL-2158, PR 3285. This is a planless gap analysis scoped to the
criteria this PR implements or adds. It does not cover the global backlog.
The matrix was computed with `quire matrix --scope . --format json`
(quire 0.52.0, engine 0.62.2) at frozen head `986e56a8e1`.

The test audit was required by the dispatch brief, so a semantic review was
performed. It judged assertion strength, adverse cases, independent oracles
and real physical replay. The code-review artifact
SR-202611210022442313144108508 records the test-strength findings that belong
with code. This artifact records traceability and AC-oracle gaps.

Matrix result: FR-1043-AC-1 to AC-5 and AC-7 to AC-11, FR-202-AC-27 to AC-30,
and FR-203-AC-10 and AC-11 are tagged by real, non-ignored tests.
FR-1043-AC-6 is untagged. FR-1042-AC-12 to AC-14, which this PR adds, are
untagged. AC-12 is inside this PR's narrow upper-minimum and lower-maximum
scope.

## Verdict

**FAIL**. The verdict rule makes any untagged in-scope criterion a FAIL
(FR-1043-AC-6 and FR-1042-AC-12). There are no high findings. Every finding
is fixable within this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-1043-AC-6 is untagged in the computed matrix. `a_finite_horizon_quantile_certificate_is_accepted` asserts it (shifted `11/135`, separate-process acceptance, level 0.5 refuted) but carries only `TC-2065`/`FR-201-AC-51`. | tools/fsl-probability-check/tests/it/certificate_kinds.rs:120 |
| FND-002 | medium | FR-1042-AC-12, added by this PR and implemented by the narrow fraction-base `bounds.rs` path, has no tagged test. Its scenario is not exercised: an initial terminal state, a false predicate, window 2, an all-zero `upper` minimum and `lower` maximum with rederived ranks, accepted with no policy rows. | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:238 |
| FND-003 | medium | FR-1043-AC-8's stated every-scheduler extrema (minimum `11/48`, maximum `5/12`) and the checker-selected policies (`slow` for the upper on the minimum, `fast` for the lower on the maximum) are never asserted. The tagged tests check only `Refuted`, accept hand-edited bound values `1/4`/`2/5`, and check `fast` only in the FR-202-AC-30 slack fixture. | fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs:1076 |
| FND-004 | low | FR-1042-AC-13 and AC-14, added by this PR, have no tagged test, while TC-3151's amended scope claims "FR-1042-AC-7 to FR-1042-AC-14". Under the brief they belong to the general FR-1042 PR, but no `Remaining work` record marks them as not delivered here. | spec/test-cases/TC-3151-certify-value-bounds.md:21 |

## Coverage

- Plan completion: not assessed
- Matrix (scoped): 20 of 24 examined criteria are tagged. The untagged ones
  are FR-1043-AC-6, FR-1042-AC-12, FR-1042-AC-13 and FR-1042-AC-14.
  FR-1042-AC-1 to AC-11 are also untagged, but they are general FR-1042
  backlog outside this PR and are not counted. No criterion is tagged only by
  an ignored test.
- Reverse gap: the new production code has owning requirements: FR-1043,
  FR-196 (key and control ownership), FR-201, FR-202, FR-203 and FR-1042. The
  speculative `ProbabilityCertificatePart::Post`/`Reset` variants are recorded
  in the code review (FND-018).
- Stubs and inflation: no source stubs. The `#[ignore]` stubs that remain in
  `certificate_kinds.rs` (FR-1041, general FR-1042) predate this PR and are
  keyed to QSL-2158. Two `#[ignore]` markers were removed for delivered
  behavior (P95, TC-2065 quantile), and no test was deleted.
- Semantic review: AC-11 replays the actual `drop(0)` step and its zero
  reward. It verifies `c` is really absent, and rejects forged locus,
  position, control, sums and cause. AC-10 validates the FSpec `Result`
  schema. AC-4 uses the emitted package and alters one numerator value. The
  defined `Always` unsupported dispositions are preserved by exact regression
  tests 5 to 7 in the parent's focused list.
- Runtime: the 33 exact focused tests are pending on draco. This analysis
  does not claim them.
