---
id: SR-2026111612172400000034
title: "Gap analysis — QSL-2002 PR 2883 frame depth"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2002; FR-100-AC-20; FR-098-AC-27; TC-444 step 11; fsl-eval/src/value/expression/{family.rs,mod.rs}; fsl-eval/tests/it/division_law_run.rs; fsl-replay/src/execute.rs; fsl-replay/src/execute/tests/application_claim.rs; fsl-replay/src/spine/clause/tests/clause_paths.rs"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-100
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-098
---

## Summary

PR-delta gap analysis for QSL-2002 PR #2883 at `a0d60ddc1fc3789ea6d8db5f7f5c2a23e83bbc0e`, relative to stacked base `87d71c6203b8d5461712d48e8103b096b4246fe7`. The dispatch explicitly confines acceptance to the frame-depth follow-up. The matrix was computed over the repository, but unrelated repository gaps and PR #2841 are not attributed to this change.

## Verdict

**PASS** for the requested follow-up scope. Both acceptance criteria have active trace-bound tests, real assertions and passing targeted execution. The depth carrier and replay consumer have explicit owning requirements. No new unspecified behavior, stub, ignored acceptance test, or hollow evidence was found. This is not a repository-wide assurance PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
  - id: FR-100-AC-20
    path: spec/functional/FR-100-run-a-named-function-through-the-spine.md
    role: examined
    excerpt: "`d(1)` returns `Undefined` at the `Location` of `10 mod (n - 1)` with `frame_depth` 0; `d(2)` and `d(3)` return `Undefined` at that same `Location` with `frame_depth` 1 and 2; `d(0)` completes `0` with `frame_depth` 0; and `function e using v(n: Int[0, 9]): Integer pure { d(n) }` called with `1` returns `Undefined` at that `Location` with `frame_depth` 1."
  - id: FR-098-AC-27
    path: spec/functional/FR-098-execute-a-replay-request.md
    role: examined
    excerpt: "`n = 1` settles `reproduced-without-witness` with value `false`, the selected call's own evaluation of the site being undefined (`frame_depth` 0) after `d(0)` completes `0`; `n = 2` and `n = 3` settle `inconclusive` with cause `NoValue`, the call stopping undefined at the same `Location` inside a nested call (`frame_depth` 1 for `n = 2`, 2 for `n = 3`), which is no observation; `n = 0` evaluates the site on no path and settles `inconclusive` with cause `Verdicts` and value `true`."
  - id: TC-444
    path: spec/test-cases/TC-444-the-replay-executor-recompiles-selects-calls-and-refuses.md
    role: examined
    excerpt: "Under `CheckMode::Kernel`, `d(0)` completes `0` at depth 0;"
  - id: FR-109-AC-14
    path: spec/functional/FR-109-run-a-state-clause-through-the-spine.md
    role: context_only
    excerpt: "a constructed `Outcome::Undefined(Undefined::SumOutOfDomain)` passed through `run_clause`'s mapping reports stage `evaluate`, category `violation`, `truth` false and `UndefinedEvaluation{cause: sum-out-of-domain}`, exit 10, and its cause is present, never null."
bindings:
  - {test_id: a_possibly_zero_mod_divisor_is_refused_under_linked_and_undefined_only_under_kernel, ac_id: FR-100-AC-20, trace: correct}
  - {test_id: tc_444_a_recursive_site_reproduces_only_at_frame_depth_zero, ac_id: FR-098-AC-27, trace: correct}
```

## Coverage

Plan completion: not assessed

`quire matrix --scope . --format json` completed with exit 0 using quire 0.52.0 (engine 0.62.2); quoin is 0.36.6, spec-artifacts-process v0.29.2. Static matrix data is at `/private/tmp/qsl2002-pr2883-matrix.json`; no Quoin stored-run evidence was claimed. Both FR-100-AC-20 and FR-098-AC-27 are `tagged`, each bound to its corresponding targeted test. The latter is no longer ignored. The two exact targeted Cargo tests independently passed, one test each, at the frozen head.

Whole-repository counts: 4,134 tagged, 258 tagged-by-ignored-test, 1,170 untagged, 103 method-without-symbol and 13 retired. The command also emits diagnostics for other documents, unmarked prose references and symbol collisions. Those counts are neither introduced by this narrow diff nor evidence of full repository qualification; broader assurance remains outside the dispatch. No relevant target tag is missing or unrecognized.

All seven changed files were inspected. No CI or qualification threshold changes. The synthetic clause fixture's field addition compiles with the replay test binary and preserves its existing purpose. The separately opt-in repository-wide semantic audit was not run; ordinary code-review still checked the scoped acceptance intent. Full/aggregate gates were not run under the owner's instruction. No spec, source, plan, or trace tags were edited.

## Dispositions

No real findings require disposition. Prior QSL-2002 review history was not reopened.

## Publication

Linear comment `854a544d-268f-4383-8c15-f6bb13c6edc6` on QSL-2002. `has-review` was already present. Read-back passed: body equality, real YAML parsing through `quire parse`, clean/findings agreement with the single placeholder, matching scope and trace bindings. Lead custody: preserve this file under `reviews/qsl-2002-pr2883-gap-analysis.md` before merge.
