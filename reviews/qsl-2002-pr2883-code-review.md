---
id: SR-2026111612172400000033
title: "Code review — QSL-2002 PR 2883 frame depth"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2002; fsl-eval/src/value/expression/{family.rs,mod.rs}; fsl-eval/tests/it/division_law_run.rs; fsl-replay/src/execute.rs; fsl-replay/src/execute/tests/application_claim.rs; fsl-replay/src/spine/clause/tests/clause_paths.rs"
review_set: subset
---

## Summary

Current independent code-review with the Rust lane of PR #2883, frozen head `b7d1b964c67a211f0d5b0501914fe12096b536a8`, against main `5a85e05e49d569223cdbd0759cc528ea98c1c3a7`. Ticket: QSL-2002. The current seven-file delta comprises five source/test files and these two review artifacts. The five source/test files are byte-for-byte unchanged from the earlier reviewed head. The clause fixture adjustment is already present in main and is no longer part of the PR delta.

Historical first pass: `a0d60ddc1fc3789ea6d8db5f7f5c2a23e83bbc0e` against stacked base `87d71c6203b8d5461712d48e8103b096b4246fe7`. Its publication and verification below are retained as history. The current-head evidence is in Re-review below.

## Verdict

**PASS** — no findings. The value-family hook initializes and resets depth to zero, retains the machine's reported depth, and the outer evaluation carries it unchanged. Replay now requires both depth zero and matching occurrence location. Removing the recursive-function blanket exclusion permits the selected recursive frame to reproduce while deeper frames remain NoValue. The synthetic clause fixture supplies the new field without changing its outcome mapping.

Mergeable from this review after the lead preserves the updated artifacts; the stacked dependency condition is satisfied by current main. No source fix is required. No applicable AssuranceProfile was found. No new unsafe operation, narrowing conversion, panic, ownership duplication, wire member, or weakened CI gate occurs in the delta. The protocol family retains its existing depth-zero behavior; its behavior is outside the value-function follow-up.

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

## Verification

The following is the historical first-pass evidence; see Re-review for the current candidate.

- `cargo test --locked -p fsl-eval --test it division_law_run::a_possibly_zero_mod_divisor_is_refused_under_linked_and_undefined_only_under_kernel -- --exact`: passed, 1 test. Includes Linked refusal and exact depths for d(0), d(1), d(2), d(3), e(1).
- `cargo test --locked -p fsl-replay --lib execute::tests::application_claim::tc_444_a_recursive_site_reproduces_only_at_frame_depth_zero -- --exact`: passed, 1 test. Includes reproduced false, nested NoValue, and unvisited-site Verdicts true. Compiling the library test binary also checks the adjusted clause fixture.
- `git diff --check origin/codex/qsl-2227-pins...HEAD`: passed.
- Inspected the caller, machine depth producer, hook early-return/reset behavior, precise replay settlement, and test helpers. Assertions exercise real checked-package calls and replay; resetting all reported depths to zero would fail d(2), d(3), e(1) and nested replay assertions, while retaining the old recursion exclusion would fail d(1) replay.
- No aggregate gate, full clippy/deny/test suite, or workspace fmt rerun, per the owner's targeted-only instruction. The dispatch reports existing workspace fmt differences in untouched files; this review does not claim workspace fmt passed.
- No spec/plan changes: spec-review and its sub-analyses are inapplicable. React lane is inapplicable. No source edits or pushes made.

## Dispositions

No real findings require disposition. Prior reviews of other QSL-2002 PRs remain historical and unchanged.

## Publication

Linear comment `b5184417-ea44-47d3-978d-175f8a57c066` on QSL-2002. `has-review` was already present. Read-back passed: body equality, real YAML parsing through `quire parse`, clean/findings agreement with the single placeholder, matching scope and trace bindings. The worktree remains clean at the frozen head. Lead custody: preserve this file under `reviews/qsl-2002-pr2883-code-review.md` before merge.

## Re-review

Current-head publication: Linear comment `bf121a94-26d0-472b-a652-39cdfc7d4e2c` on QSL-2002. Read-back body and structured data match; both comments are clean with empty findings, consistent with the preserved placeholder. `has-review` was already present.

**PASS**, no findings at `b7d1b964c67a211f0d5b0501914fe12096b536a8`, base main `5a85e05e49d569223cdbd0759cc528ea98c1c3a7`. GitHub and local git agree on the head, base, and seven-file scope. Inspected the complete current source delta; all five source/test files are byte-for-byte equal to the original reviewed versions. The governing FR-100, FR-098 and TC-444 documents and repository instructions are unchanged. The hook's reset/carry behavior, exact depth-zero plus location comparison, and active traced regression remain correct.

Independently reran both exact Cargo test commands in Verification at this head: S6a 1 passed (1359 filtered out), replay 1 passed (1345 filtered out). These are fresh results, not reliance on the first pass. `git diff --check 5a85e05e49d569223cdbd0759cc528ea98c1c3a7...HEAD` passed. No aggregate gate, source edits or pushes. No findings require a disposition. The dependency API and fixture field are already in main; no stacked dependency remains. Lead custody: replace the committed review artifact with this updated version.
