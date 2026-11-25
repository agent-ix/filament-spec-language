---
id: SR-967175026302197768900474181941935084845
title: "Gap analysis — QSL-3239/3240/3241 scoped acceptance"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@4d37df4dbf3f480a263174e9d998de739b5f14bb; FR-397-AC-10, FR-1040-AC-11, FR-230-AC-4/11, FSpec-FR-415-AC-6, FSpec-FR-181; changed evaluator, lowering, timed model, and integration tests"
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

Round-two manual acceptance-criteria-to-test review of PR #3857 at `4d37df4dbf3f480a263174e9d998de739b5f14bb`, based on `bc203488e90b2cad9bcf417fa260b9f8341f6bcb`. FND-001 is fixed: FSpec-FR-415-AC-6 now tags the Timer record-place test, and the new-object clock test traces to FSpec-FR-181. The five requested tests pass and the three broad-suite failures reproduce on main. The computed repository matrix remains unavailable. Plan completion: not assessed.

## Verdict

**CONDITIONAL** — the targeted trace mismatch is fixed, but the engine-backed repository matrix remains unavailable, so this review does not claim a repository-wide PASS.

## Coverage

- `FR-397-AC-10`: examined. `float_callables::bit_constructors_keep_expected_types_and_exact_payloads` asserts checked expected types and exact emitted/evaluated payloads.
- `FR-1040-AC-11`: examined. `clock_reference_v2::clock_reference_set_aggregators_use_the_canonical_member_order` asserts byte ordering and results for all four named aggregates; the enclosing test is tagged to the criterion.
- `FR-230-AC-4`: examined. The source-form, semantic lowering, and subject tests each carry the criterion tag; the changed subject test asserts 6,400 places, initial live clocks, and exact large bounds.
- `FR-230-AC-11`: examined. The record-place test is tagged and asserts nine places in owner order for the three record clock templates.
- `FSpec-FR-415-AC-6`: examined against `/home/peter/dev/filament-specification/spec/functional/temporal/FR-415-declare-clocks-and-timed-models.md`. The Timer record-place test now has this tag and asserts the per-object place count, clock-template identity, and owner order. The direct-clock creation test instead traces to FSpec-FR-181, whose timed-subject section states that a clock newly made present takes its write constant.
- `FSpec-FR-181`: examined against `/home/peter/dev/filament-specification/spec/functional/runtime/FR-181-simulate-finite-executions.md`. Its timed-subject section states that a clock a step makes present takes the constant of its write; the newly created object's clock test now carries this tag.
- Reverse-gap inspection of changed implementation paths found no new unowned behavior beyond those existing requirements. No stubs or placeholder returns were introduced.
- `quoin matrix --repo . --json` exited 1 after `CORE_REFUSED: request exceeds the accepted size`. The engine-backed matrix and repository-wide reverse-gap/stub inventory therefore remain unavailable; no repository-wide PASS is claimed. The manual review is not a substitute for the full matrix audit.
- Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new FSpec-FR-415-AC-6 trace tag does not match the direct-clock object-creation test; AC-6 names the `Timer.t.x` record scenario and related parameter/result/alias behavior. | fsl-eval/tests/it/timed_subject.rs:1958 |


## Dispositions

Round 1 disposition verified at `4d37df4dbf3f480a263174e9d998de739b5f14bb` against `bc203488e90b2cad9bcf417fa260b9f8341f6bcb`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The created-object test now traces to FSpec-FR-181; FSpec-FR-415-AC-6 now tags the Timer record-place test, which asserts clock-template identity and owner order. |

### FND-001 after excerpt

```text
#[trace("TC-685", "FR-230-AC-11", "FSpec-FR-415-AC-6")]
#[test]
fn every_object_holds_its_own_clock_of_every_record_place() {
    assert!(places.iter().all(|place| place.clock() == clock));
}
```

## Gate Disposition

The five requested tests passed on the PR head in `review1-pr.log` (exit 0). The three failures from the round-one broad run were reproduced with matching symptoms on main commit `4ed7366f52e1b943de5212b8c5e2dc6591ed53f1` in `review1-main.log` (exit 101); they are pre-existing, not PR regressions. The engine matrix remains unavailable with `CORE_REFUSED: request exceeds the accepted size`; a repository-wide PASS is not claimed. Plan completion: not assessed.
