---
id: SR-202611230430002
title: "gap-analysis of PR3592 ValueBounds settlement for a decided interval"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR3592 8c2b83a273b54f02af007665c817eb4e6c124c17; FR-203-AC-1, FR-1042, FR-741 counterpart, tests in fsl-analyze/tests/it and tools/fsl-probability-check/tests/it"
review_set: subset
---

## Summary

Ticket: QSL-2880. Diff-scoped gap audit using `quire matrix --scope . --format json` over
the PR head. FR-203-AC-1 is tagged by 4 tests. This PR newly tags FR-1042-AC-6 with 2
tests. FR-1042-AC-1..5, 7..12 and FR-203-AC-9 were untagged before this PR and are
outside its ticket, except where this diff builds the behaviour (below). QSL-2903's
ignored `value_bounds_certificates_are_accepted` uses an unbounded `eventually` (base
`cyclic-dtmc-eventual/v1`). This PR builds intervals only for bounded reachability and
the timed deadline, so that test is not fixed and correctly stays ignored. It was not
run: the base is statically unbuilt and titan was loaded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-1042-AC-6 is tagged on tests that cover only part of it: neither checks that the `lower` ranking equals remaining horizon, and acceptance is in-process with hand-raised limits, not by `fsl-probability-check` from the emitted package | fsl-analyze/tests/it/exact_probabilistic_item.rs:1496 |
| FND-002 | medium | Behaviour this diff builds has no test: the counterpart-rejected settlement (`finish_interval`, FR-203-AC-9's row: inconclusive with the counterpart's rule and locus) and the item `counterpart` emission (document.rs) | fsl-analyze/src/analyze/exact.rs:1894 |

## Verdict

CONDITIONAL. There are no stubs and no hollow assertions. The value assertions are exact
(containment, a width bound, `> 999/1000`). The reverse gap is clean: every new code
path is owned by FR-203 or FR-1042. The two coverage gaps above should be closed in
this PR.

## Coverage

- FR-203-AC-1: tagged (exact_probabilistic_item.rs:610, :1430; measure_profile.rs:578; timed_product_certificate.rs:1551).
- FR-1042-AC-6: tagged (exact_probabilistic_item.rs:1430, :1496); partial, see FND-001.
- FR-203-AC-9: untagged; this diff builds the behaviour it names (FND-002).
- Semantic review: skipped (not opted in); intent checks were done in the code-review SR.
- Plan completion: not assessed
