---
id: SR-967175026302197768900474181941935084787
title: "Code review of QSL-2925/QSL-2927 fixed-N preflight and verdict bound"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3545 head a365faa13aa4fc5c3d5a41bc14b2e41f76f7c4f0 vs merge-base 9a2555a0; fsl-analyze/src/statistical.rs, fsl-analyze/src/statistical/tests.rs, fsl-analyze/src/zone_check/settle.rs, fsl-analyze/tests/it/{probability_from_source.rs,statistical_model.rs,statistical_witness_replay.rs}, fsl-replay/src/{outcome.rs,proof_result.rs,statistical.rs,statistical/settle.rs,model_check/settle.rs}, Cargo.toml, Cargo.lock"
review_set: subset
---

## Summary

Tickets: QSL-2925 (fixed-N preflight) and QSL-2927 (verdict `bound`). Code review with the rust-review lane folded in. Governing texts were read from merged FSL main (FR-191, FR-192, FR-741 as amended by #3547, ad08f091) and FSpec 57c662b1 (#816: FR-410, FR-413, FR-408, FR-331, backend-provider-v1 schema and vectors).

The preflight is correct on the main path. Okamoto and Clopper-Pearson compute `N = ceil(ln(1/min(α,β))/(2ι²))` before the draw loop. They compare `N` with `max_samples` and then `max_draws` using `>`, so a limit equal to `N` admits the run. On a stop they return `PreflightLimit{limit, value, required}` with zero draws. SPRT and regenerative runs keep stop-at-limit. Because the preflight runs before any draw, a sample that would evaluate undefined is never drawn, as FR-191 says ("no sample or draw occurs"). `N` is the same for every test, so test 0's check stops the run with zero counts for all tests. The wire `limit-reached` carries `required` only for the preflight variant. Every completed verdict carries `bound`.

N values checked by hand. With α=β=1/100: ι=1/2000 (NoFault) gives ln(100)·2·10^6 = 9,210,340.4, so N = 9,210,341; ι=0.00005 gives ln(100)·2·10^8 = 921,034,037.2, so N = 921,034,038. The new FR-741 test's 921,034,038 is right for its claim. 9,210,341 belongs to NoFault's ι, a different claim, so both numbers are right. 23,026 (ι=1/100) and the from-source 922 are consistent with the same formula.

The pin is clean. `rev` 57c662b1 is on FSpec origin/main. Cargo.toml has two entries and Cargo.lock one source line. The old rev appears nowhere else, and there are no copies elsewhere. Between 01f8dd31 and 57c662b1 the only Rust change in FSpec is a test file.

Rust-review lane: no `unsafe` and no new `unwrap` or panic in non-test code. `to_u64().unwrap_or(u64::MAX)` saturates, see FND-001. `StatisticalStop` drops `Copy`, which is harmless. The preflight is O(1) and deterministic.

Cargo was not run by the reviewer: nine other cargo jobs were running on the host (titan rule: one cargo job at a time). The CLI repros were not run, because this repo has no `fsl` binary.

## Verdict

**CONDITIONAL**: the preflight logic and N values are correct. Before merge, fix or record a ruling for FND-001 (out-of-range `required`), FND-002 (sibling variant against the spec's named `LimitReached{required}`), FND-003 (the in-repo FR-331 reader refuses `required`) and FND-005 (quantile bound). FND-007 needs a deferred ticket. FND-008 is a high finding against the exact-path lane: it is not in this diff, but this PR's pin bump makes it a violation of the pinned schema. It needs its own ticket and does not gate #3545.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `required` saturates at u64::MAX, but the pinned FR-331 schema caps `PositiveCount` at 2^53-1. Any ι below about 1.6e-8, which FSL admits (it requires only ι > 0 inside (0,1)), writes a `limit-reached.required` that the schema refuses and RFC 8785 cannot carry exactly. The spec states no saturation or overflow rule (spec gap) | fsl-analyze/src/statistical.rs:620-626; fsl-replay/src/outcome.rs:1438-1447; fsl-semantics/src/check/probabilistic.rs:1030-1048 |
| FND-002 | medium | Departure from merged FR-192: the table names `Incomplete(LimitReached{limit, value, setting, required: Some(N)})`, but the PR adds a sibling `IncompleteCause::PreflightLimit`. The wire is identical, but typed consumers keyed on `LimitReached` (ReachedLimit facts) treat the preflight stop as no reached limit, and one wire cause now has two typed spellings, a pre-release compat-layer smell. Fold it into `LimitReached{required: Option<u64>}` or get the spec amended | fsl-replay/src/proof_result.rs:490-500; fsl-replay/src/model_check/settle.rs:832-842; fsl-analyze/src/zone_check/settle.rs:102-112; fsl-replay/src/model_check/hyper_settle.rs:361-366 |
| FND-003 | medium | The in-repo FR-331 entry reader does not admit the new member. `planner::read_result_entry`'s INCOMPLETE table allows `limit-reached` with exactly {limit, setting, value}, so an entry carrying `required` (which the PR now writes, and pinned FR-331-AC-67 admits) is refused `unknown-member` at `/cause/required` | fsl-replay/src/planner/entry.rs:667-673; fsl-replay/src/planner/entry.rs:836-851 |
| FND-004 | low | `sampled_test` computes the same `N` twice, once for `fixed` and again in the preflight, each with an outward `ln` evaluation. Compute it once and reuse it | fsl-analyze/src/statistical.rs:590-627 |
| FND-005 | medium | A quantile verdict's `bound` is the reduced bound, not the claim's. For `quantile q of M >= c` it writes `{type: quantile, comparison: less-or-equal, threshold: q}`: the claim's threshold `c` and direction are dropped, and the strict `Pr(M<c\|act) < q` is written non-strict. A reader cannot check this against the checked claim. FR-410/FR-413 give no quantile or strict spelling (spec gap) | fsl-analyze/src/statistical.rs:1118-1125; fsl-analyze/src/statistical/claim.rs:166-196; fsl-replay/src/statistical.rs:284-292 |
| FND-006 | low | The bound `type` values `quantile`, `mean-of-fraction` and `long-run-fraction` are unspecified. The FR-413 schema `type` is a free string (minLength 1), and FR-410 and the vectors name only `probability`. FSpec needs to enumerate them (spec gap) | fsl-replay/src/statistical.rs:91-101 |
| FND-007 | medium | The FR-192 clause "the result reader SHALL verify it against the checked claim" and FR-192-AC-9's "missing, malformed or mismatched bound is refused by the result reader" are not implemented anywhere in this repo. The only FR-331 reader (`planner::read_result_entry`) refuses every `verdict` as unsupported-feature. This is not a defect in the produced output, but the AC is FSL's own and now carries trace tags, so it needs a deferred ticket or a narrowed AC | fsl-replay/src/planner/entry.rs:472-474; fsl-replay/src/planner/entry.rs:1422-1423 |
| FND-008 | high | The exact path's `ProbabilisticCounterexample.bound` is written `{comparison: ">=" or "<=", threshold}` with no `type`. This violates FR-413-AC-17 and the pinned schema's `ProbabilisticBound` (enum greater-or-equal/less-or-equal, `type` required). No in-repo test validates the counterexample against the schema. PR #3552 (QSL-2921) does not change this writer, but it adds a test that asserts the non-conformant spelling | fsl-replay/src/outcome/counterexample.rs:2112-2120; fsl-analyze/tests/it/exact_probabilistic_item/subsystem_refutation.rs:297 (PR 3552) |

## Coverage

- Merged-clause checklist (FR-191 Budgets, FR-192 map/Behavior, FR-741-AC-4, FSpec FR-408/FR-410/FR-413/FR-331). Implemented: the preflight for both fixed-N methods; the max_samples-then-max_draws order; equality admits; zero draws; `partial` counts 0; the undefined sample is never drawn; SPRT and regenerative unchanged; the wire `required` only on the preflight; `bound` on all three decisions. Gaps: FND-001 (range of `required`), FND-005/006 (quantile/type spelling), FND-007 (reader).
- Changed assertions are justified by the amended ACs and are not weakened. FR-191-AC-1 now asserts the exact stop, zero samples, draws and sampler calls, and adds the max_draws case. FR-191-AC-2 moves to SPRT with `(1000, 0)`, as the amended AC-2 states. FR-191-AC-5 is now the preflight stop with zero calls and adds the activation `required: 23_026` case. The FR-189-AC-12 tests move max_draws to 30,000, above Okamoto's 23,026, so the stop-at-limit path is still exercised. The witness-replay expectation changes to `PreflightLimit`, `required: 23_026`.
- Driver lane still to cover (CLI repro not run here): staged-sampling-claim-missed (ι=0.00005) should settle `incomplete` at once with `required: 921034038`; method selection spellings (`{"type":"sprt"}`, `{"type":"clopper-pearson"}`); the docs PRs filament-docs#130 and filament-examples#103.

## New findings (disposition pass 1)

Re-review of head 31645dfc36813a83879e598d85fe71cf3b28fe9d (merge-base f98d2037). The FSpec pin is now 3200a4ed (#820, on FSpec origin/main).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | `read_positive_decimal` admits the empty string. `"".bytes().all(is_ascii_digit)` is vacuously true and `!"".starts_with('0')` holds, so `required: ""` is read and written back, although the pinned schema `^[1-9][0-9]*$` refuses it. The new planner test does not try `""` | fsl-replay/src/planner/entry.rs:1282-1290; fsl-replay/src/planner/tests.rs:1972-1990 |
| FND-010 | medium | The 3200a4ed pin also adopts #820's FR-379 change: `refinement_failure.initial` and `position` are now `U64String`. The FSL counterexample writer still emits JSON numbers (`{"kind": "StutterChanged", "position": 1}`), and its own test asserts the number. With this PR's pin, the refinement wire contradicts the pinned schema, and no ticket tracks it | fsl-replay/src/outcome/counterexample.rs:1200-1210; fsl-replay/src/outcome/counterexample.rs:3377-3388 |
| FND-011 | low | Stale doc on `BoundComparison::LessOrEqual`: "The strict bound a quantile claim reduces to is written the same way". That contradicts FR-413 (a writer SHALL NOT serialize the reduced comparison) and the new behaviour | fsl-replay/src/statistical.rs:289-291 |
| FND-012 | low | A public `StatisticalClaim::new(ClaimForm::Quantile, ..)` built without `with_quantile` is accepted, and fails only at verdict time with InternalFault `claim-bound-as-written`. The written quantile could be a required argument of a quantile constructor rather than an optional builder step | fsl-analyze/src/statistical/claim.rs:118-162; fsl-analyze/src/statistical.rs:1101-1107 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `required` is an exact `Integer`, written as decimal text, and the saturation is gone (grep finds no statistical `unwrap_or(u64::MAX)`). The pinned FR-331 schema (3200a4ed) is `^[1-9][0-9]*$` |
| FND-002 | fixed | `IncompleteCause::LimitReached{limit, value, required: Option<Integer>}`; the sibling variant is removed. The engine-level `StatisticalStop::PreflightLimit` is FR-191's own name |
| FND-003 | fixed | The planner INCOMPLETE table admits optional `required` and reads it as canonical positive decimal text. Zero, signed, zero-padded, decimal and numeric values are refused. The empty string is not refused: new FND-009 |
| FND-004 | fixed | `count` is computed once. `fixed` and the preflight share it |
| FND-005 | fixed | `verdict_bound()` writes the claim's own comparison and threshold and the quantile `level`, matching FR-413 at 3200a4ed. The strict reduction stays internal |
| FND-006 | fixed | #820 closes `type` to probability, quantile, mean-of-fraction and long-run-fraction, which `bound_type` matches |
| FND-007 | deferred | QSL-2944. The reader clause is untracked in code and left untagged. One test still carries the whole FR-192-AC-9 tag (see gap FND-007) |
| FND-008 | deferred | QSL-2942. #3552 merged as 5b9c3ec3 with the non-conformant test assertion `{"comparison": ">=", ...}` at subsystem_refutation.rs:295-297. The writer counterexample.rs:2120 is unchanged on main |
