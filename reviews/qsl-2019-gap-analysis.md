---
id: SR-2026111612172400000032
title: "Gap analysis — QSL-2019 FSL command extraction slice"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #2891 @1ff2b2beaa36076def73d9d596154804a65cc6bf; FR-662-AC-3, TC-1744 steps 1-2; tests/it/request_surface_s1.rs::the_run_command_reaches_markdown_extraction_by_default"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-662", type: references }
---

## Summary

This planless gap analysis is scoped to PR #2891's FSL slice and FR-662-AC-3. The computed matrix marks FR-662-AC-3 tagged, but its one bound test covers only TC-1744 step 1, leaving step 2 without command-level evidence; plan completion was not assessed.

## Verdict

**CONDITIONAL** — TC-1744 step 2 is untested through the command route in this PR slice.

## Coverage

- Reconciliation: `quoin matrix` (quoin 0.36.6, quire 0.52.0); computed repository-wide matrix had 5,684 criteria, including FR-662-AC-3 `tagged` with one binder and no run evidence. This focused review manually checked only the FR-662-AC-3/TC-1744 slice.
- Plan completion: not assessed
- Focused criteria: FR-662-AC-3 tagged; its bound test asserts the two-binding refusal only.
- TC-1744 step 2 / reverse gap: no test in the command path names a missing clause and asserts FR-030's `extraction-selection` error with original identity and binding.
- Semantic review: skipped (not opted in).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The only test tagged to FR-662-AC-3, `the_run_command_reaches_markdown_extraction_by_default`, checks TC-1744 step 1's two-binding refusal. No command-level test exercises step 2: one binding naming a clause the Markdown document does not hold and the resulting FR-030 `extraction-selection` refusal retaining original identity and binding. The AC's trace is present, but its required test evidence is partial. | tests/it/request_surface_s1.rs:326-347; spec/test-cases/TC-1744-run-command-reaches-markdown-extraction-in-its-default-build.md:17-25; spec/functional/FR-662-check-every-native-fence-of-a-markdown-specification.md:117 |
