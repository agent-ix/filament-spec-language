---
id: SR-967175026302197768900474181941935084844
title: "Code review — QSL-3239/3240/3241 timed evaluation and clock places"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@5b9e1d463adf7e1aa2dfe71c6d2e216f4c2c0382; fsl-eval/src/simulation/model.rs, fsl-eval/src/simulation/model/timed.rs, fsl-eval/src/value/collections.rs, fsl-eval/src/value/expression/evaluate.rs, fsl-semantics/src/check/lowering.rs, affected integration tests; FR-397-AC-10, FR-1040-AC-11, FR-230-AC-4/11, FSpec-FR-415-AC-6"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-397
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1040
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: references
  - target: ix://agent-ix/filament-specification/FR-415
    type: references
---

## Summary

Reviewed PR #3857 at `5b9e1d463adf7e1aa2dfe71c6d2e216f4c2c0382`, including its Rust implementation, touched tests, owning acceptance criteria, and gate evidence. The changed behaviors have direct assertions; one new cross-repository trace tag does not match the criterion it names. The full computed matrix was unavailable, and the full `fsl-eval` integration target was interrupted after reporting three unresolved failures.

## Verdict

**CONDITIONAL** — one low-severity trace-to-test mismatch remains; the three full-suite failures have not been established as baseline or PR regressions, and the matrix could not be computed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new FSpec-FR-415-AC-6 tag is on a direct-clock creation test, while AC-6 specifies a `Timer` record held by `t`, clock parameter/result/alias rules, and per-object `t.x` counts. | fsl-eval/tests/it/timed_subject.rs:1958 |

## Review Evidence

- The float constructor test checks the emitted expected rounding type and exact payloads for signaling and negative NaNs, negative zero, and positive infinity (`FR-397-AC-10`).
- The ClockReference Set test checks byte-sorted place ids and exercises `count`, `exists`, `all`, and `only` over the generated Set (`FR-1040-AC-11`).
- Timed-place tests assert 6,400 places over 100 objects and 64 clock fields, record-place ordering/counts, and the reset/liveness of a newly created object's clock (`FR-230-AC-4`, `FR-230-AC-11`).
- No CI workflow changed. `git diff --check` passed. `cargo fmt --check` reports inherited workspace formatting differences; the PR's recorded changed-file comparison found no new formatting delta. Scoped Clippy evidence on this exact head passes `fsl-semantics` and `fsl-eval` after the three confirmed baseline findings are allowed; unfiltered workspace Clippy exits 101 on known lints in `fsl-package/src/bare.rs`, `fsl-eval/src/automaton/qualified_bounded.rs`, and `fsl-eval/src/simulation/model/refine.rs`.
- `cargo deny check` was not run after the lead's instruction to stop additional builds/check processes while the shared target was being removed.
- The local command `quoin matrix --repo . --json` exited 1 with `CORE_REFUSED: request exceeds the accepted size`; repository-wide matrix evidence is unavailable. The exact full test command `cargo test --locked -p fsl-eval --test it -j 4` compiled and began 1,771 tests, but after reporting three failures and no further output for over eight minutes it was interrupted (exit 130). The changed tests observed before interruption passed. Failures reported were `clock_reference_candidates_follow_the_selector_maxima_and_keep_seeds`, `clock_reference_routes_range_over_the_declared_identities`, and `same_type_populations_refuse_colliding_owner_keys_before_clock_evaluation`; their base status is unresolved.
