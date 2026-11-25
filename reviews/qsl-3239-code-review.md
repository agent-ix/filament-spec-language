---
id: SR-967175026302197768900474181941935084844
title: "Code review — QSL-3239/3240/3241 timed evaluation and clock places"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@4d37df4dbf3f480a263174e9d998de739b5f14bb; fsl-eval/src/simulation/model.rs, fsl-eval/src/simulation/model/timed.rs, fsl-eval/src/value/collections.rs, fsl-eval/src/value/expression/evaluate.rs, fsl-semantics/src/check/lowering.rs, affected integration tests; FR-397-AC-10, FR-1040-AC-11, FR-230-AC-4/11, FSpec-FR-415-AC-6, FSpec-FR-181"
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
  - target: ix://agent-ix/filament-specification/FR-181
    type: references
---

## Summary

Round-two review of PR #3857 at `4d37df4dbf3f480a263174e9d998de739b5f14bb`, based on `bc203488e90b2cad9bcf417fa260b9f8341f6bcb`. The low-severity trace finding from round one is fixed: the Timer record-place test now traces to FSpec-FR-415-AC-6, and the creation test traces to FSpec-FR-181. Five requested tests passed; the three broad-suite failures were reproduced on main.

## Verdict

**PASS** — FND-001 is fixed, the five requested tests pass, and the three broader integration failures were reproduced on main; no PR-specific code-review findings remain. The lead owns the aggregate gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new FSpec-FR-415-AC-6 tag is on a direct-clock creation test, while AC-6 specifies a `Timer` record held by `t`, clock parameter/result/alias rules, and per-object `t.x` counts. | fsl-eval/tests/it/timed_subject.rs:1958 |

## Review Evidence

- The float constructor test checks the emitted expected rounding type and exact payloads for signaling and negative NaNs, negative zero, and positive infinity (`FR-397-AC-10`).
- The ClockReference Set test checks byte-sorted place ids and exercises `count`, `exists`, `all`, and `only` over the generated Set (`FR-1040-AC-11`).
- Timed-place tests assert 6,400 places over 100 objects and 64 clock fields, record-place ordering/counts, and the reset/liveness of a newly created object's clock (`FR-230-AC-4`, `FR-230-AC-11`). The Timer record-place test now also checks that each place group matches its declared clock id.
- No CI workflow changed. `git diff --check` passed. `cargo fmt --check` reports inherited workspace formatting differences; the PR's recorded changed-file comparison found no new formatting delta. Scoped Clippy evidence on this exact head passes `fsl-semantics` and `fsl-eval` after the three confirmed baseline findings are allowed; unfiltered workspace Clippy exits 101 on known lints in `fsl-package/src/bare.rs`, `fsl-eval/src/automaton/qualified_bounded.rs`, and `fsl-eval/src/simulation/model/refine.rs`.
- `cargo deny check` was not run after the lead's instruction to stop additional builds/check processes while the shared target was being removed.
- The local command `quoin matrix --repo . --json` exited 1 with `CORE_REFUSED: request exceeds the accepted size`; repository-wide matrix evidence is unavailable. The exact full test command `cargo test --locked -p fsl-eval --test it -j 4` compiled and began 1,771 tests, but after reporting three failures and no further output for over eight minutes it was interrupted (exit 130). The changed tests observed before interruption passed. Failures reported were `clock_reference_candidates_follow_the_selector_maxima_and_keep_seeds`, `clock_reference_routes_range_over_the_declared_identities`, and `same_type_populations_refuse_colliding_owner_keys_before_clock_evaluation`; their base status is unresolved.


## Dispositions

Round 1 disposition verified at `4d37df4dbf3f480a263174e9d998de739b5f14bb` against `bc203488e90b2cad9bcf417fa260b9f8341f6bcb`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The created-object test now traces to FSpec-FR-181; the Timer record-place test now traces to FSpec-FR-415-AC-6 and asserts the place clock id and owner order. |

### FND-001 after excerpt

```text
#[trace("TC-685", "FR-230-AC-11", "FSpec-FR-415-AC-6")]
#[test]
fn every_object_holds_its_own_clock_of_every_record_place() {
    assert!(places.iter().all(|place| place.clock() == clock));
}
```

## Gate Disposition

The five requested tests passed on the PR head in `review1-pr.log` (exit 0). The three failures from the round-one broad run were reproduced with matching symptoms on main commit `4ed7366f52e1b943de5212b8c5e2dc6591ed53f1` in `review1-main.log` (exit 101); they are pre-existing, not PR regressions. The engine matrix remains unavailable with `CORE_REFUSED: request exceeds the accepted size`; the lead owns the aggregate gate.
