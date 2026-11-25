---
id: SR-967175026302197768900474181941935084845
title: "Gap analysis — QSL-3239/3240/3241 scoped acceptance"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@5b9e1d463adf7e1aa2dfe71c6d2e216f4c2c0382; FR-397-AC-10, FR-1040-AC-11, FR-230-AC-4/11, FSpec-FR-415-AC-6; changed evaluator, lowering, timed model, and integration tests"
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

Manual acceptance-criteria-to-test review of the requested QSL-3239/3240/3241 scope found direct behavioral assertions for float profile and payload, canonical ClockReference Set order and aggregation, universe clock-place counts, record places, and creation-time clock resets. The full repository gap audit could not complete because the computed matrix command was refused for request size; this result is limited to the requested criteria and changed behavior. Plan completion: not assessed.

## Verdict

**CONDITIONAL** — the targeted manual review found one inaccurate external trace tag, and a repository-wide computed matrix was unavailable.

## Coverage

- `FR-397-AC-10`: examined. `float_callables::bit_constructors_keep_expected_types_and_exact_payloads` asserts checked expected types and exact emitted/evaluated payloads.
- `FR-1040-AC-11`: examined. `clock_reference_v2::clock_reference_set_aggregators_use_the_canonical_member_order` asserts byte ordering and results for all four named aggregates; the enclosing test is tagged to the criterion.
- `FR-230-AC-4`: examined. The source-form, semantic lowering, and subject tests each carry the criterion tag; the changed subject test asserts 6,400 places, initial live clocks, and exact large bounds.
- `FR-230-AC-11`: examined. The record-place test is tagged and asserts nine places in owner order for the three record clock templates.
- `FSpec-FR-415-AC-6`: examined against `/home/peter/dev/filament-specification/spec/functional/temporal/FR-415-declare-clocks-and-timed-models.md`. Its new tag is attached to a test of a direct `Call.x: Clock` created by an operation; the AC specifies `Timer { x: Clock, n: Int }` in `t`, Clock parameter/result/alias rules, and `k` objects yielding `k` `t.x` clocks. This is the low-severity trace mismatch in the code-review artifact.
- Reverse-gap inspection of changed implementation paths found no new unowned behavior beyond those existing requirements. No stubs or placeholder returns were introduced.
- `quoin matrix --repo . --json` exited 1 after `CORE_REFUSED: request exceeds the accepted size`. The engine-backed matrix and repository-wide reverse-gap/stub inventory therefore remain unavailable; no repository-wide PASS is claimed. The manual review is not a substitute for the full matrix audit.
- Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new FSpec-FR-415-AC-6 trace tag does not match the direct-clock object-creation test; AC-6 names the `Timer.t.x` record scenario and related parameter/result/alias behavior. | fsl-eval/tests/it/timed_subject.rs:1958 |
