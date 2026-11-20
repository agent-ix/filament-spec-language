---
id: SR-2026112017124277723988806
title: "Code review — QSL-2611 PR #3153 public witness derivation"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3153 diff: fsl-replay/src/execute.rs, fsl-replay/src/execute/state_clause.rs, fsl-replay/src/witness/derivation.rs, fsl-replay/src/spine/clause/tests/frame_replay.rs, fsl-replay/src/spine/clause/tests/witness.rs"
review_set: subset
evaluated_revision: "97f19b2837f17fd3c3436960c217eb6f3c993bdf"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-269
    type: references
---

## Summary

Ticket: QSL-2611. Reviewed PR #3153 at head `97f19b283` (base `cd8e81e72`, 64 commits behind FSL `origin/main` `ad5547bde`) by reading the diff, and checked it against FR-269-AC-6..9, TC-3121 and the two QSL-2611 arch rulings.

Most of the PR is right:

- The production changes are correct. `settle_separation` now keeps an absent `given`, as FR-269-AC-9 requires. Range violations still settle before every clause outcome, including an in-crate fault. `separation_refusal` is now shared, which resolves #3128's deferred FND-001.
- The fixtures follow the rulings: Linked check mode with `{u, z: Kernel}`, `pre`/`post` snapshot roles, one TC-3121 unit for the native run and the derivation, and `z` under Kernel for `Refused`.

Three gaps remain:

- The second range-violation row does not exercise the case its TC row specifies.
- The `Undefined` refusal names the clause root, not the domain expression.
- The one-unit-short exhaustion budget is computed from a charge count, not a work-unit count.

## Verdict

**FAIL — fix round needed.** No high findings; three mediums and two lows. The diff merges cleanly against current main (`git merge-tree` against `ad5547bde`). Main's changes to `ReplayRequestWire` (`certificate`), `ReplayRefusal` variants and the `EmptyPackage` locus do not touch anything this diff constructs or matches. Compiling against current main was not verified because the brief allows no build.

## Scope and skill provenance

The code-review and rust-review lanes ran by reading `gh pr diff 3153`. Context came from `git show` in the read-only checkout `/Users/peter/dev/qsl-2631-code`. With no worktree, build or test run, `quoin gap-analysis` could not run at the PR head; the manual AC-to-test check below replaces it. The PR has no Python and no shell.

## Rust review

| Check | Result |
| --- | --- |
| Behaviour vs spec | `settle_separation(outcome, given, record)` and `classify_with_range_violations` match FR-269-AC-8/9. The Undefined locus remap uses `ExpressionLocation::root` (FND-001). |
| Tests | AC-8 row 2 uses a fixture whose clause is false (FND-003). AC-7's one-unit-short budget uses the wrong counter (FND-004). |
| Formatting | The diff reflows a rustfmt single-line `if/else` and adds lines over 100 columns (FND-002, which continues #3128's deferred FND-002). |
| Safety/concurrency/wire | No unsafe code, no concurrency, no wire change. |

## Coverage (manual AC-to-test check)

| Criterion / TC row | Test | Result |
| --- | --- | --- |
| FR-269-AC-6 MatchesParent, AllBelow, `[600, 1]`, NoParent | `tc_3121_public_derivation_matches_clause_run_and_replay_records`, `tc_3121_no_parent_closed_scope_returns_none` | covered |
| FR-269-AC-6 VersionAdvances | same | covered for consistency only; the FR-266-AC-2 operands and the AC-9 settlement are not asserted (FND-005) |
| FR-269-AC-7 NotRefuted, ClauseIdentity, Undefined, Refused, second calls | `tc_3121_public_derivation_refusals_keep_catalog_metadata`, `..._refused_and_exhausted_cases_keep_metadata` | covered except FND-001 (locus) |
| FR-269-AC-7 Exhausted one unit below need | `..._refused_and_exhausted_cases_keep_metadata` | FND-004 |
| FR-269-AC-8 row 1 (0 to -1, true), in-crate fault | `tc_3121_public_derivation_range_violations_win_first`, `range_violation_precedes_an_in_crate_fault` | covered |
| FR-269-AC-8 row 2 (1000 to 1001 on root, clause `true`, never NotRefuted) | `tc_3121_public_derivation_range_violations_win_first` | FND-003 |
| FR-269-AC-9 separation one unit short, `given` absent, mismatch cases | `tc_3121_absent_comparison_record_is_no_value_one_unit_below_need`, `..._still_mismatch` | covered |

## Prior review dispositions (#3128, SR-2026112010200000000001)

FND-001 (duplicated `CallRefusal` to `SeparationRefusal` conversion) is fixed: the conversion is now a single `execute::separation_refusal`, shared by `stopped_reason` and the derivation. FND-002 (rustfmt) is still open and is carried here as FND-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The Undefined refusal is remapped to the clause root, not the domain expression that TC-3121 and the arch ruling name | fsl-replay/src/witness/derivation.rs:120-131, 176-182 |
| FND-002 | low | Non-rustfmt formatting: a reflowed single-line `if/else` and new lines over 100 columns | fsl-replay/src/witness/derivation.rs:506-510 |
| FND-003 | medium | The AC-8 row 2 fixture (self `root`, no `child`) makes the clause `false`, so "range violation beats NotRefuted" is never exercised | fsl-replay/src/spine/clause/tests/witness.rs:2646 |
| FND-004 | medium | The Exhausted budget is `usage.evaluation_admissions - 1`, a charge count, so the test does not establish "one unit below the clause evaluation's need" | fsl-replay/src/spine/clause/tests/witness.rs:2613-2616 |
| FND-005 | low | The VersionAdvances derivation and replay are checked only for consistency, not for FR-266-AC-2's operands or the AC-9 settlement | fsl-replay/src/spine/clause/tests/witness.rs:2176-2225 |

## Findings detail

- **FND-001.** TC-3121 step 5 expects "`Undefined` naming the domain expression". Arch ruling 2 says to map the location "to the clause's domain expression", that is, "the clause expression whose evaluation ended undefined, not the `mod` inside `u`". The code builds `ExpressionLocation::root(StateClause { clause, index })`, which is the `forall` root, and the test asserts only the owner. Fix: map to the clause-side location of the call that ended undefined (the `u(self.versionNumber)` call site, or the quantifier domain that holds it), and assert its path in the test.
- **FND-002.** The repo has no `rustfmt.toml`, so rustfmt's defaults keep `if stopped_on_left { left } else { right }` on one line, as the base has it at line 353. The new lines `frame_replay.rs:58` and `witness.rs:2537` exceed `max_width`. Fix: run `cargo fmt -p fsl-replay` (or rustfmt on the five touched files).
- **FND-003.** FR-269-AC-8 and TC-3121 step 6 say "taking `root.versionNumber` from 1000 to 1001 with result `false` (the clause is `true` and would otherwise refuse `NotRefuted`)". FR-122-AC-7's invocation leaves `child` unchanged, and the arch ruling says self is `child` in both rows. With self `root` and result false, `not result implies self.versionNumber = pre(self.versionNumber)` evaluates to `1001 = 1000`, which is false, so the clause is false. Fix: use `(1000, Some(2), 1001, Some(2), "child", false, "root", 1001)`.
- **FND-004.** `ClauseRunUsage::evaluation_admissions` is "how many charges the evaluation meter admitted". `default_accounting(n)` sets `work_units`. Any budget below the true need passes the assertion, and the old fixture's 0 passed too. Fix: take the `WorkUnits` entry of `usage.evaluation_consumed`, or binary-search the boundary as the AC-9 test does. Assert that the need itself completes.
- **FND-005.** Fix: assert the record's operands are `Integer` 4 and `Integer` 3 with the `and`/first `implies`/`=` decision path, and assert `witness_arm(&version_replay).settlement() == ReproducedWithEvaluatedWitness`.

## Findings data

```yaml
clean: false
scope:
  - {id: FR-269-AC-6, path: "spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md", role: examined, excerpt: "Over FR-265's `witness` unit, `fsl_replay::derive_state_clause_witness` called with the request and `Witness`-arm envelope for `MatchesParent` over `high` whose payload carries `witness: None` returns `Ok(Some(_))` holding FR-265-AC-11's comparison record, and for `AllBelow` over `high` `Ok(Some(_))` holding FR-265-AC-1's quantifier record; over FR-108's corpus, the changed-version `VersionAdvances` envelope with no record returns `Ok(Some(_))` holding FR-266-AC-2's comparison record of the first `=`."}
  - {id: FR-269-AC-7, path: "spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md", role: examined, excerpt: "FR-268-AC-3's `AllBelow` variant whose domain evaluates `undefined` on `high` `Undefined` naming that expression, `undefined_expression`; its refused-domain variant `Refused` with the refusal record's code and cause; and `MatchesParent` with the accounting limits one unit below the clause evaluation's need `Exhausted`, `resource_exhausted`/`insufficient-next-charge`. Each refusal is the same on a second call."}
  - {id: FR-269-AC-8, path: "spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md", role: examined, excerpt: "Range violations settle first: over FR-122-AC-7's invocations, `VersionAdvances` taking `child.versionNumber` from 0 to -1 with result `true` ... and taking `root.versionNumber` from 1000 to 1001 with result `false` (the clause is `true` and would otherwise refuse `NotRefuted`) each refuse `RangeViolation` holding FR-122-AC-7's one range violation"}
  - {id: FR-269-AC-9, path: "spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md", role: examined, excerpt: "An absent comparison record settles the agreement, never `inconclusive` ... FR-108's changed-version `VersionAdvances` envelope with no record settles `reproduced-with-evaluated-witness` holding FR-266-AC-2's record; the same `MatchesParent` envelope with the evaluation meter one unit below the separation check's need settles `inconclusive`, `NoValue`."}
  - {id: FR-122-AC-7, path: "spec/functional/FR-122-replay-a-state-clause-counterexample.md", role: context_only, excerpt: "Over an invocation taking `root.versionNumber` from 1000 to 1001 with `child` unchanged and result `false` it settles `reproduced-with-evaluated-witness` holding the evaluated `true` and one range violation naming `root`, `[0, 1000]` and 1001."}
  - {id: FR-266-AC-2, path: "spec/functional/FR-266-report-a-clause-run-s-witness-and-settlement-basis.md", role: context_only, excerpt: "changed-version `decisive-counterexample` with the comparison record of `VersionAdvances`'s first `=` (operands the `Integer` 4 of `child`'s converted post-call `versionNumber` then the `Integer` 3 of its pre-call `versionNumber` plus one, `decision_path` the `and`, the first `implies` and that `=`)"}
  - {id: TC-3121, path: "spec/test-cases/TC-3121-derive-a-state-clause-witness-through-the-public-entry.md", role: examined, excerpt: "| 5 | undefined domain | `Undefined` naming the domain expression, `undefined_expression` | ... | 6 | 1000 to 1001, clause `true` | `RangeViolation` naming `root`, `[0, 1000]`, 1001, never `NotRefuted` |"}
findings:
  - fnd: FND-001
    severity: medium
    confidence: medium
    method: code-review
    check: code-bug
    artifact_id: FR-269-AC-7
    related_ids: [TC-3121]
    path: fsl-replay/src/witness/derivation.rs
    lines: "120-131, 176-182"
    excerpt: |-
      FR-268-AC-3's `AllBelow` variant whose domain evaluates `undefined` on `high` `Undefined` naming that expression, `undefined_expression`
    finding: "A function-body Undefined location is remapped to ExpressionLocation::root(StateClause), which is the clause's forall root. TC-3121 and the arch ruling require the domain expression, the clause expression whose evaluation ended undefined. The test asserts only the owner, not the path."
  - fnd: FND-002
    severity: low
    confidence: medium
    method: code-review
    check: other
    artifact_id: fsl-replay/src/witness/derivation.rs
    related_ids: []
    path: fsl-replay/src/witness/derivation.rs
    lines: "506-510"
    excerpt: |-
      if stopped_on_left {
          left
      } else {
          right
      }
    finding: "Without a rustfmt.toml, rustfmt keeps this if/else on one line. The new lines frame_replay.rs:58 and witness.rs:2537 exceed max_width. This continues #3128's deferred rustfmt finding. Run cargo fmt on the touched files."
  - fnd: FND-003
    severity: medium
    confidence: high
    method: code-review
    check: test-intent
    artifact_id: FR-269-AC-8
    related_ids: [FR-122-AC-7, TC-3121]
    path: fsl-replay/src/spine/clause/tests/witness.rs
    lines: "2646"
    excerpt: |-
      and taking `root.versionNumber` from 1000 to 1001 with result `false` (the clause is `true` and would otherwise refuse `NotRefuted`) each refuse `RangeViolation` holding FR-122-AC-7's one range violation, `invalid_runtime_input`/`invalid-value`, `/family_payload/observation`.
    finding: "Row 2 runs with self root and no child, so `not result implies self.versionNumber = pre(self.versionNumber)` gives 1001 = 1000, which is false. The clause is false and the RangeViolation-over-NotRefuted precedence that AC-8 specifies is untested. Use child 2 to 2 with self child."
  - fnd: FND-004
    severity: medium
    confidence: medium
    method: code-review
    check: test-intent
    artifact_id: FR-269-AC-7
    related_ids: [TC-3121]
    path: fsl-replay/src/spine/clause/tests/witness.rs
    lines: "2613-2616"
    excerpt: |-
      and `MatchesParent` with the accounting limits one unit below the clause evaluation's need `Exhausted`, `resource_exhausted`/`insufficient-next-charge`.
    finding: "need is usage.evaluation_admissions, a count of admitted charges, but default_accounting sets the work_units ceiling, so need - 1 is not shown to be one unit below need. Any lower budget, including the old 0, passes. Use the WorkUnits entry of evaluation_consumed, or binary-search the boundary, and assert that need itself completes."
  - fnd: FND-005
    severity: low
    confidence: high
    method: code-review
    check: test-intent
    artifact_id: FR-269-AC-6
    related_ids: [FR-266-AC-2, FR-269-AC-9]
    path: fsl-replay/src/spine/clause/tests/witness.rs
    lines: "2176-2225"
    excerpt: |-
      over FR-108's corpus, the changed-version `VersionAdvances` envelope with no record returns `Ok(Some(_))` holding FR-266-AC-2's comparison record of the first `=`.
    finding: "The VersionAdvances record is compared only across entry, run and replay. Its FR-266-AC-2 operands (4, 3) and decision path, and the replay's reproduced-with-evaluated-witness settlement (AC-9), are not asserted."
bindings:
  - {test_id: tc_3121_public_derivation_range_violations_win_first, ac_id: FR-269-AC-8, trace: correct}
  - {test_id: range_violation_precedes_an_in_crate_fault, ac_id: FR-269-AC-8, trace: correct}
  - {test_id: tc_3121_absent_comparison_record_is_no_value_one_unit_below_need, ac_id: FR-269-AC-9, trace: correct}
  - {test_id: tc_3121_public_derivation_refusals_keep_catalog_metadata, ac_id: FR-269-AC-7, trace: correct}
  - {test_id: tc_3121_no_parent_closed_scope_returns_none, ac_id: FR-269-AC-6, trace: correct}
```

## New findings (disposition pass 1)

This pass checked the rebased head `5eee456eb0ec9a7b8b1ccc97f28cff03a5e05d66`. The PR's first three commits are unchanged from `97f19b283` (`git range-diff` shows them as `=`), and the fix is the single commit `5eee456eb`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | The fix removed the clause-level remap, and the Undefined test now pins the native `mod` location inside function `u` (`ExpressionOwner::Body { function: "u" }`). TC-3121 row 5 and arch ruling 2 require the clause's domain expression ("not the `mod` inside `u`"), so the oracle is bent to the behaviour the ruling rejects | fsl-replay/src/spine/clause/tests/witness.rs:2477-2482 |

- **FND-006.** Fix: restore a remap in `classify_clause_outcome` (`derivation.rs:158`) from a function-body location to the clause expression that ended undefined. That is the `u(self.versionNumber)` call site in `UndefinedDomain`'s quantifier domain, with a non-root path. Keep `division-by-zero` as the cause, and assert `ExpressionOwner::StateClause { clause: "UndefinedDomain" }` together with that path. If the evaluator cannot supply the call-site path without losing the cause, take it to arch as the ruling asks; do not pin the inner location.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The fix deleted the clause-root remap and did not replace it with the domain-expression locus. The Undefined refusal now names the `mod` inside `u`, which is further from TC-3121 row 5 (see FND-006). |
| FND-002 | still-open | The reflowed `if/else` and witness.rs:2537 are fixed. `frame_replay.rs:58` (102 columns, added by this PR) still exceeds max_width. |
| FND-003 | fixed | Row 2 is now `(1000, Some(2), 1001, Some(2), "child", false, "root", 1001)` (witness.rs:2613). The clause is true, so RangeViolation over NotRefuted is exercised. |
| FND-004 | fixed | `need` is now the `WorkUnits` entry of `usage.evaluation_consumed` (witness.rs:2572-2579). `need - 1` exhausts and `need` completes (witness.rs:2598-2602). |
| FND-005 | still-open | No change: the VersionAdvances block (witness.rs:2166-2212) still does not assert FR-266-AC-2's operands (4, 3) or decision path, or the replay's `reproduced-with-evaluated-witness` settlement. |
