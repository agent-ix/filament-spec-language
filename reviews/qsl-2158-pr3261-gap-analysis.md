---
id: SR-20261121050326
title: "Gap analysis — QSL-2158 remainder / QSL-2159 PR #3261 against FR-741-AC-2 and FR-876"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3261 diff origin/peter/qsl-2156-digital-completion...2104fccb69; FR-741-AC-2, FR-876-AC-1..3, FR-196-AC-15, FR-201-AC-49/51, FR-282-AC-20, FR-203-AC-1/5/8"
review_set: subset
evaluated_revision: "2104fccb69"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-741
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-876
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: references
---

## Summary

Tickets: QSL-2158 (remainder) and QSL-2159. `quoin matrix --repo . --json` (quoin 0.36.8) was run from the worktree root at `2104fccb69`. The audit is scoped to the criteria this stacked PR claims or un-ignores. The test-to-spec claims were checked against the spec text of FR-741, FR-876, FR-196, FR-201, FR-202, FR-203 and FR-282.

FR-741-AC-2 is still partial after this PR. This PR adds the `long-run` kind accepted by the checker (FR-201-AC-51, via the library helper) and the later check over the emitted package (FR-282-AC-20 / FR-203-AC-8). The classes still not shown through `fsl analyze` are: `P95` `ExactValue{24233/25000}`, the fair every-scheduler unbounded item, §15.3's long-run item of kind `long-run` through `fsl analyze`, and §15.1's `lower`/`upper` certificates at `max_rational_bits` 1,024. QSL-2158 therefore stays open, and Refs is correct.

## Verdict

**FAIL** (the rule says any untagged criterion fails). FR-741-AC-2 and FR-876-AC-1..3 are untagged in the computed matrix (FND-001). FR-201-AC-47..55 are not minted at all (FND-002). The ignore reasons misstate the spec (FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The matrix reports FR-741-AC-2 and FR-876-AC-1, AC-2 and AC-3 as `untagged`, with `unbacked_claims` reason `unmarked-trace-id`. The tests in `probability_from_source.rs` use the path form `#[ix_trace_rs::trace(..)]`, which the scanner does not bind. The `#[trace(..)]` form after `use ix_trace_rs::trace` does bind (conditioned_product_key.rs, certificate_kinds.rs). The FR-876 tests this PR un-ignores therefore back nothing in the matrix. Fix: import `trace` and use `#[trace(..)]` in probability_from_source.rs. | fsl-analyze/tests/it/probability_from_source.rs:311-323, 697-725 |
| FND-002 | medium | FR-201's Acceptance Criteria table is split by a blank line before FR-201-AC-47. The matrix mints only FR-201-AC-1..46, so FR-201-AC-47..55 do not exist for traceability. That includes FR-201-AC-49, which `bounded_fair_keys_carry_all_eight_members` un-ignores, and FR-201-AC-51, which the certificate_kinds tests back. The spec file is not in this diff, but this PR's tags depend on it. Fix: delete the blank row separator at spec/functional/FR-201 line 1275. | spec/functional/FR-201-check-a-probability-certificate.md:1274-1276 |
| FND-003 | medium | Four ignore reasons say "no FR-201 certificate profile exists". FR-201 does specify these: Emission (lines 111-123) requires `Lower`/`Upper` from a dyadic interval and a general-form certificate for every exact class. Lines 573-580 and 586-592 give quantiles over every scheduler (XF-2 `Upper`) and workload quantiles (`Conditional`). FR-201-AC-3 accepts `FairTerminates` `Exact` over fair end components. FR-203-AC-5's undefined witness is specified too: FR-202 `Undefined` evidence (lines 58, 368-371, FR-202-AC-5). Each of these is an implementation gap, not a spec gap. Routing FR-203-AC-5 to arch as a spec gap is incorrect. Fix: reword each reason to name the missing implementation and its owning ticket. | tools/fsl-probability-check/tests/it/certificate_kinds.rs:90, 104, 122; fsl-analyze/tests/it/exact_probabilistic_item.rs:603, 632 |
| FND-004 | medium | FR-741-AC-2's long-run item is required "through `fsl analyze`" (TC-2066), of kind `long-run`. The only long-run certificate test in this PR is `a_long_run_certificate_is_accepted` (TC-2065, FR-201-AC-51). It builds the certificate with the library helper `request_with`, not the analyze verb, so the analyze document's `certificate.kind` is unshown for a long-run item. Fix: add an exact long-run item to probability_from_source.rs that asserts `certificate.kind == "long-run"` and `accepted`. | tools/fsl-probability-check/tests/it/certificate_kinds.rs:135-151; fsl-analyze/src/analyze/document.rs:4061-4075 |
| FND-005 | medium | FR-876-AC-2's "its value 0" is not asserted, and FR-282-AC-20's package binding has no negative case. These are code-review SR-20261121050325 FND-003 and FND-004, repeated here as AC coverage gaps. | fsl-analyze/tests/it/probability_from_source.rs:713-721; tools/fsl-probability-check/tests/it/certificate_kinds.rs:153-209 |

## Coverage

- FR-196-AC-15: tagged and active (`an_on_each_key_carries_its_instance_and_phase`, `a_six_member_key_is_refused`). The focused log `-9` shows 2 passed.
- FR-201-AC-49/51: tagged in code but not minted (FND-002). `bounded_fair_keys_carry_all_eight_members` and `a_long_run_certificate_is_accepted` are active and pass (log `-9`).
- FR-282-AC-20 / FR-203-AC-8: tagged and active (`an_analyze_certificate_is_accepted_later_over_the_emitted_package`). Positive and altered-value cases only.
- FR-876-AC-1/2/3: active but untagged in the matrix (FND-001). AC-2 is missing its value assertion.
- FR-203-AC-1 (P95) and FR-203-AC-5: tagged only by ignored tests. The ignore reasons are inaccurate (FND-003).
- FR-741-AC-2: untagged. Partial classes are listed in Summary.
- Reverse gap: the new public API (`check_probability_certificate_package`, `ProbabilityPackageRequest`, `ReplayPackage::name_and_sources`) is owned by FR-282-AC-20. The retry in `replay_refutation` is owned by FR-876-AC-2. No unowned behaviour was found. No stubs were found.
- Semantic review: skipped (not requested). The reviewer's code review covers intent for the changed code.
- Plan completion: not assessed
