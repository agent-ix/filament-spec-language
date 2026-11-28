---
id: SR-20261107070951249606
title: "Code review — QSL-3342 fixture corrections"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/tests/it/model_normalization.rs, fsl-semantics/tests/it/record_valued_model.rs; FR-056-AC-9/21/34, FSpec TC-195/196, NFR-012"
review_set: subset
---

## Summary

Reviewed PR #3944 at `f34b18de40fdb7a84547dc442246efdc6aade928` against QSL-3342 and the listed requirements. Both fixture edits preserve the intended test behavior: the ancestry case now uses a legal inherited redefinition target while retaining its 128/200-wide contested graph and winner/loser assertions, and the K2 test no longer introduces the unrelated record reader while retaining its exact String/UUID leaf assertions. One existing AC trace on the changed ancestry test does not describe the behavior it exercises.

## Verdict

**CONDITIONAL** — the changed contested-redefinition test is still tagged to a cycle/variance refusal criterion it does not exercise; correct the binding or identify an owning criterion. No applicable AssuranceProfile was found. The PR checks report gate and internal checks passing; the PR description reports both focused tests and required clippy passing. I did not rerun Cargo locally because the exact PR head was not present in an accessible ticket worktree.

## Findings

| ID      | Severity | Summary                   | Refs |
| ------- | -------- | ------------------------- | ---- |
| FND-001 | medium   | The wide-ancestry test is tagged to the cycle/variance refusal AC, which it does not exercise | fsl-semantics/tests/it/model_normalization.rs:2796 |

## Scope

| Unit | Role | Excerpt / review focus |
| --- | --- | --- |
| `fsl-semantics/tests/it/model_normalization.rs:2737-2848` | examined | Fixture and assertions for contested wide ancestry at 128 and 200 direct generalizations. |
| `fsl-semantics/tests/it/record_valued_model.rs:174-215, 902-950` | examined | Optional record-reader helper and K2 type-leaf acceptance and swapped-declaration assertions. |
| FR-056-AC-9 | examined | Package object types, subtype conformance and model-member checks after admitted package selection. |
| FR-056-AC-21 | examined | Record value-type exports and field/operation type environment. |
| FR-056-AC-34 | examined | Distinct String/UUID leaves, mapped K2 members, and existing profile/payload controls. |
| FSpec TC-195 | context_only | Model normalization provenance vectors; the changed test's cross-repository trace. |
| FSpec TC-196 | context_only | Conformance/redefinition test case; the changed test's cross-repository trace. |
| FSpec FR-151-AC-2 | context_only | Existing test trace; criterion concerns inheritance cycles or incompatible variance. |
| NFR-012 | examined | Normalization work ceilings and requirement that fixtures complete at defaults with the same result as unlimited normalization. |

## Finding detail

`n06_wide_ancestry_with_two_contesting_redefiners_completes` constructs a valid competing-redefinition graph and asserts successful normalization plus winner/loser visibility. It does not construct an inheritance cycle or incompatible variance, so `FSpec-FR-151-AC-2` is a false test-to-AC binding and can make the matrix report coverage that the test does not provide. Keep the broad TC traces; replace this AC trace with a criterion that owns the tested dominance behavior, or remove the AC trace if no such criterion exists.

## Test-only impact

No Impact note is required: the PR changes only test fixtures/helpers and assertions, and makes no production behavior or public contract change.

## Validation and evidence

- Exact PR head verified with `gh pr view`: `f34b18de40fdb7a84547dc442246efdc6aade928`.
- Changed paths are only the two integration-test files listed above.
- GitHub PR checks: `cla / gate` passed; `cla / cla-check-internal` passed; `cla / cla-check` was skipped.
- PR description reports both focused tests and required clippy passing. No local Cargo invocation was made because no accessible worktree was at the exact reviewed head.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The fix commit removed only the inaccurate FR-151-AC-2 trace and retained the two test-case traces; the test body and assertions are unchanged. |

### Disposition round 1 evidence

Verified PR head `28c79395ed4d7890be5edae32d4d76b5b4f0a0f4`. The only change since the reviewed head in `model_normalization.rs` is the trace attribute, now `#[trace("FSpec-TC-195", "FSpec-TC-196")]`; `n06_wide_ancestry_with_two_contesting_redefiners_completes` and its assertions are unchanged. The same commit adds this SR artifact under `reviews/` for the lead to retain in the branch.

## Current disposition verdict

**PASS** — FND-001 is fixed at head `28c79395ed4d7890be5edae32d4d76b5b4f0a0f4`; no findings remain open.
