---
id: SR-20261123070012002
title: "gap analysis of PR 3560 object-free hyper clause"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3560 head e89d6e0dba29046bd13ab91c9c161ae82fe4e4f0; FR-488-AC-28, FSpec FR-402-AC-11, FR-631-AC-4, FR-630-AC-4, FR-630-AC-5, FR-630-AC-6, FR-135-AC-6"
review_set: subset
---

## Summary

Ticket: QSL-2887. This is a manual check of acceptance criteria against tests, scoped to the criteria the PR's tests are tagged with. It is not a full gap audit. `quoin matrix --json` (quoin 0.36.8) failed on this repository with `CORE_REFUSED: request exceeds the accepted size`, and `quire trace --id` printed only symbol-id-collision noise. The matrix, reverse-gap and stub steps were therefore not run. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Both tests are tagged with FR-488-AC-28 and FSpec FR-402-AC-11, but nothing tests TC-3010 step 4: a `Whole` wrapper whose `over` is the root of a package that no `subjects` binding names. Nothing tests FR-402-AC-11's third case either: a single-trace `temporal_clause` that is not a body wrapper, with `over` set to `model_root` | fsl-analyze/tests/it/hyper_object_free.rs |
| FND-002 | low | The FR-630-AC-5 test does not assert the refusal locus `self.value + 3` that the criterion names | fsl-analyze/tests/it/analyze_family_cli.rs:1362-1366 |

## Verdict

FR-631-AC-4 and the admitted half of FR-488-AC-28 are covered and pass. FR-630-AC-5 is covered by the remaining sibling test, and deleting the contradicting test is correct. FR-630-AC-4 and FR-135-AC-6 stay uncovered behind QSL-2945. FR-630-AC-6 stays uncovered behind the ignore discussed in the code-review SR, FND-003. The refusal half of FR-488-AC-28 is only partly covered (FND-001 here, and FND-001 in the code-review SR).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Added a_wrapper_over_the_root_of_a_package_no_subject_names_is_refused (Whole over test/vault-other's model_root) and a_single_trace_clause_that_is_no_body_wrapper_cannot_range_over_a_model_root. Both assert the exact code, cause and pointer, and both pass |
| FND-002 | fixed | The test asserts the bytes of program.fsl at the diagnostic locus equal `self.value + 3` |
