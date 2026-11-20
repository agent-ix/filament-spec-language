---
id: SR-1477
title: "Code and Rust review — timed interval boundary cases"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2081; codex/qsl-2081-zone-interval-cases@de858f003fbaabc2369744734667968443f33f99; fsl-analyze/tests/it/zone_mitl.rs interval_boundaries additions; existing timed_world/zone helper interfaces as context; FR-234, FR-237, FR-242, FR-1034"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Reviewed the frozen branch's added interval tests and their exact current requirements. Real checked pulse models isolate event instants; successful timed results are checked by the core, and returned counterexamples are replayed. One new trace binding claims concrete Rpc scenarios that these new fixtures do not exercise.

## Verdict

**PASS** — the eight new bindings were corrected in the accepted author receipt; no remaining review finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new pulse/nested/projection tests claim FR-237-AC-1, which requires the specific Rpc/Settles counterexamples rather than generic timed replay. | fsl-analyze/tests/it/zone_mitl.rs:367; spec/functional/FR-237-replay-a-timed-counterexample.md:219 |

## Finding details

FND-001: All eight verdict-test attributes in the new `interval_boundaries` module cite `FR-237-AC-1`. That criterion requires FR-236-AC-1's counterexample and the `Settles`/`Rpc` example with deadline 4, `send` at 0 and `reply` after delay 3. The new functions instead exercise `Boundary`, `NestedIntervals`, and a projection fixture. Their actual replay checks are valuable but do not implement those concrete examples. In the new module only, bind generic replay to `FR-237` or an exact applicable generic obligation. Keep the existing Rpc tests and their bindings unchanged. This is an attribute correction; no case rerun, new Rpc scenario or implementation change is requested.

## Rust and behavior review

- The helper's true/false branches require actual Holds or Violated outcomes; fault/incomplete outcomes cannot silently pass.
- Phase 1 is entered at the declared clock bound and cannot persist after that instant because its clock invariant permits no positive delay. This makes the endpoint pairs meaningful.
- Punctual origin-bound windows are admitted by FR-234/FR-242; the tests do not demand unsupported punctual unbounded forms.
- Certificates are checked against the same current world; returned timed counterexamples are replayed and required to reproduce.
- The pending unbounded comparison calls the actual untimed checker and restricts the equivalence to the fixture's discrete phase path; it does not assert a general equivalence for positive lower bounds.
- Reversed and empty-open intervals are checked through real source admission and the semantic checker, including the typed reason, exact span, code and cause tag.
- No production code, external fixture copy, fake proof, new CLI/TL surface, global configuration or additional dependency is introduced.

## Validation and limitations

The author's seven focused passes and two actual EN-6 faults are accepted without rerun. The two required scenarios remain strictly ignored under QSL-2689 and are not coverage. No pair index, unexecuted later-call result or EN-1 disagreement is inferred.

No tests, builds, lint, product probes, matrix or existing-corpus audit ran in this review. Scoped lint is unavailable because the accepted prior commands encountered 595 unchanged semantics dependency findings and then four unchanged analyze-library findings under `--no-deps`; no suppression or unrelated fixes were attempted. The original coder runs one applicable final component gate after disposition. This review targets an unopened frozen branch, not another ticket's PR; opening the unchanged reviewed feature does not require a second review.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Record-only disposition at cbcc2089ba5a61e01bf10ec2773a13040f0d2a7d accepts the in-lane author receipt: exactly eight new interval_boundaries attributes changed from FR-237-AC-1 to FR-237. Existing Rpc tags, fixtures, assertions and QSL-2689 pending annotations remain unchanged. |

This disposition performed no code reread, extra review, runtime, lint or gate. It records the explicitly accepted metadata-only implementation receipt. The original finding above remains unchanged. The original coder's one final component gate follows.
