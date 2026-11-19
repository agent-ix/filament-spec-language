---
id: SR-2026111919340478491681406
title: "Gap analysis of QSL-2534 reference hybrid provider slice (filament-spec-language PR 3085)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@fde13341c3fe4acf6d22d1ad4ec0ed607cb4276d; git diff origin/main...HEAD (PR 3085) against FSL FR-1019, FR-750, FR-967 and FSpec FR-779-AC-2"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-2534. This is a diff-scoped gap analysis. Each acceptance criterion
that the PR's tests claim with `#[trace]`, or that the changed code implements,
was checked against what the tagged tests assert. The tests were run, in the
house focused form. Plan completion: not assessed. `quoin matrix --json` (quoin 0.29.1) was started but produced no output within a 300 s timeout on this thermally constrained host, so the Quoin matrix method is recorded as unavailable for this run. The AC-to-test check below was done by hand, separately.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-1019-AC-6 is now claimed by a passing, un-ignored test, `the_reference_provider_publishes_its_descriptor`, but the AC's `options {precision_bits: "64", step: "1/16", refine_depth: "32"}` clause is unmet: the descriptor holds names only, and the test compares a crate constant with its own literal. The matrix will show AC-6 as partly covered by a test that cannot fail on the descriptor's defaults (see SR-2026111919340478491681405 FND-004 and FND-005) | fsl-analyze/tests/it/hybrid_reference.rs:259-306; fsl-analyze/src/analyze.rs:461-463 |
| FND-002 | low | The PR implements FSpec FR-779-AC-2 (`invalid_capability`/`invalid-option-value` with `option` and `value`) and cites it in docs and commits, but no test carries an `FSpec-FR-779-AC-2` trace, although the repo tags FSpec criteria as `FSpec-FR-n-AC-m` | fsl-analyze/tests/it/hybrid_reference.rs:346-405; fsl-analyze/src/analyze.rs:2712-2724 |

## Scope and evidence

Examined:
- FR-1019-AC-3, on `a_non_polynomial_flow_is_a_located_unsupported` (un-ignored)
  and `the_class_admits_cyclic_flows_and_refuses_a_guard_division`: the flow
  `v' = -a / (1 + v)` settles `unsupported`, `unsupported_construct`/`expression-form`,
  mode `Moving`, variable `v`, located at the division inside the flow. The
  guard division is located in `Moving` with no variable, and `v' = v * a - a`
  is admitted. Clean for what the tests claim. The class-boundary defects are in
  the code-review SR (FND-002, FND-003).
- FR-1019-AC-7, refusal half, on `an_out_of_domain_or_unknown_option_settles_invalid_request`
  and the three fsl-hybrid-reference unit tests: `{step: "0"}` settles
  `invalid_capability`/`invalid-option-value`, settled by disposition, payload
  `request_index`, `backend`, `option` and `value`, with no `result`. Over an
  out-of-class model it settles before the class check, and `{colour: "blue"}`
  stays FR-967 `unknown-option`. The domain tests cover both bounds of each
  integer range, overflow, sign, leading zeros and the malformed `step` forms.
  Clean. The coarse-option proof half stays ignored (QSL-2534, QSL-2531),
  stated honestly.
- FR-750 `check_options`: the trait default is `Ok(())`, and `fsl-analyze` calls
  it once per item after FR-967 and before `solve`. FR-750 names FR-1019-AC-7 as
  its verifier. Clean.
- FR-1019-AC-4, AC-5 and AC-12: their existing tests still pass on this head
  and are untouched by the diff.
- FR-1019-AC-1, AC-2, AC-6 (step budget), AC-8, AC-9, AC-10, AC-11, AC-13
  (TC-2997) and FR-1021-AC-3: still ignored, each citing QSL-2534. The solver
  is not built, as the PR body states. The ignore-reason accuracy is in the
  code-review SR (FND-009, FND-010).

## Verdict

The partial framing is honest. The PR claims coverage only for AC-3, the AC-7
refusal half and the AC-6 descriptor. Of those, the AC-6 options clause is
claimed but not met (FND-001).

## Dispositions

Round 1, head 269ecdaa44ebd141577a0396d68efcd6fe79509f.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The descriptor now carries the defaults through `with_options` (name, default) pairs, and the AC-6 test reads them from `option_defaults()` and checks `dynamics` == ["hybrid"] |
| FND-002 | fixed | `#[trace("FSpec-TC-1891", "FSpec-FR-779-AC-2")]` added to `an_out_of_domain_or_unknown_option_settles_invalid_request`. FSpec TC-1891 scopes FR-779-AC-1 and AC-2 |

