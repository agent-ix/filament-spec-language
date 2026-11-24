# QSL-3060 / PR #3692 — Code Review

Reviewed head: `3fb53273127bc28399aaf2fef25237ca85bb93bc` (`titan/qsl-3060-calibrated-limits`)

## Verdict

**CONDITIONAL — changes requested.** The source qualification suite still omits required cases for FR-397-AC-27 and FR-397-AC-28, and the runner calibrates work budgets from a manually authored FunctionCall expectation instead of deriving that count structurally from the source expression.

## Findings

| ID | Severity | Criterion | Finding | Evidence |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-397-AC-27 | F32 is not source-qualified. | FR-397-AC-27 explicitly includes F32 negation over each F32 pattern and rounding mode, including `-x` and `-(-x)`. The source adapters register F33 at `fsl-eval/tests/it/qualification_vectors_from_source.rs:3080` and F24/F25/F27/F28/F31 at `:3083`, but there is no F32 adapter or explicit non-qualification status. The `members` list at `:3101-3106` also omits F32, so the status test cannot catch its absence. The source qualification suite can therefore pass without checking an obligation named by this AC. |
| FND-002 | medium | FR-397-AC-28 | The F17 source qualification covers only one of the required comparison and budget cases. | The sole F17 case at `fsl-eval/tests/it/qualification_vectors_from_source.rs:1034-1040` evaluates `numericEqual(qn, one)` at the default tuple and expects `false`. FR-397-AC-28 and TC-1091 step 23 also require `qn = one`, `qn != one`, `one = one`, the `work_units: 1` incomplete case, the `value_occurrences: 0` incomplete case, and repetitions over Float64 constants. None of those cases appear in the F17 adapter. |
| FND-003 | medium | FR-140/FR-141 ScalarLimits source accounting | FunctionCall work-budget calibration uses the authored expected count rather than a structural count of calls in the checked source expression. | `SourceCase.expected_function_call_charges` is a manually assigned literal (for example `Some(1)` or `Some(3)`). In `observed`, the runner adds that expected count to `limits.work_units` before calling the package (around lines 2530-2540), then counts actual `FunctionCall` charges and compares them later. `compare_accounting` verifies that tuple cases carry a literal expected count, but it does not derive or compare that count with the source/checked expression structure. This retains a hand-maintained calibration point instead of structurally accounting for source calls as required. |

## Requested-Fix Checks

- **T16 qualification under FSpec-FR-141-AC-4/6:** Addressed by direct per-profile `admit_text` checks for refusal, exact-bound admission, retained normalized text, charge sequences, and incomplete records (the `T16ProfileRow` cases and `t16_direct_text_type_admission_covers_each_profile_boundary`).
- **T15 nonvacuous qualification:** Addressed by two source cases that exercise distinct type-check refusals (`operator-ineligible` and `type-mismatch`), with explicit expected outcomes and empty charge/call expectations.
- **Ownership and narrow trace tags:** The new individual accounting test groups carry criterion-oriented trace tags; the separate status test is intentionally broad across the qualification test cases. No defect identified in the reviewed mapping.
- **Structural FunctionCall counting and node budgets:** Not fully addressed; see FND-003. Actual call charges are compared to literals, but literals also drive budget adjustment and are not structurally derived.
- **Nonvacuous compare()/tuple-member assertions:** Addressed. `compare_accounting` rejects empty cases and requires tuple-budget cases except the explicit result-only rows; the member test requires budget cases and explicit call expectations for tuple members.
- **FR-093-AC-56:** The calibrated runner case asserts the nearest-even 2/3 conversion result, node charge sequence, and one FunctionCall. The updated D24 source-form test also exercises the exact-mode refusal and the short-budget boundaries. No gap identified for the reviewed criterion.

## Verification

`git diff --check origin/main...HEAD` completed cleanly. No tests or test binaries were run, per the review brief; the lead reported the focused test had already passed on this head.

## Conditional Disposition Recommendation

**Changes requested:** add the missing F32 and remaining FR-397-AC-28 source cases, and derive/validate expected FunctionCall counts structurally before using them to calibrate node work budgets. Re-review these findings after the fix round; no finding is marked fixed or rejected in this review pass.
