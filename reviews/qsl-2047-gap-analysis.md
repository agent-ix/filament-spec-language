---
id: SR-2026111911313600000012
title: "gap-analysis of QSL-2047 documented FSL stages"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3063; README.md; tests/it/doc_snippets.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-017
    type: references
---

## Summary

Ticket QSL-2047, PR 3063, frozen reviewed head 30688c74dd9dc89f3c43e4bfaf6dd8400ad5206d. Diff-scoped gap analysis found no defects in the delivered diff. Rust/style review is included in code-review; no spec artifacts changed.

## Verdict

**PASS for the reviewed diff.** Final gate remains the lead's responsibility. This is not a full CI, clean-clone, or whole-repository assurance completion claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reviewed the entire two-file diff: README.md's six FSL fence stage declarations and the added value profile; tests/it/doc_snippets.rs's public check/compile execution, OutcomeDocument refusal projection, bare-fence check integration, trace/test attribute ordering, removal of ignores, and four behavioral tests. Existing fence extraction and check policy are context, not requalified beyond the changed routes.

The declared parse stages match excerpts that intentionally lack their containing profiles/model context. The value example includes its profile and declares compile. No stage is downgraded from a previously checked declaration. The synthetic ill-typed Boolean function parses but refuses at check and compile with the named code/cause; a well-typed function succeeds. Reject and pending tests exercise real parser errors and mismatch/promotion failures, not only counts. Unsupported or malformed outcome diagnostics return an error rather than being accepted as a successful refusal.

Public library entry points execute the language; no unit under test is doubled. Compile fixtures use temporary files and public native-compile request shape. Temporary directories clean themselves on return. Test-only panic/assert patterns follow the existing harness. No production API, unsafe, concurrency, dependency, CI, gate policy, or cross-repository source copy is introduced.

Trace tags bind the four registered tests to NFR-017-AC-5/6/7 and TC-2242. Clean-clone criteria AC-1 through AC-4 are not claimed by these tests, notwithstanding older ticket prose naming their test cases. The README/profile changes and executed helper paths own the changed behavior; there is no new unowned production path or hollow stub. No whole-repository assurance or whole-program completion is claimed.

Coder evidence accepted without repetition: four actual tests registered after the trace/test attribute correction; three passed in the corrected scoped run and the remaining document test passed after its missing README stage annotation was fixed. Zero ignored tests. The initial zero-selected command is not passing evidence. No tests, builds, computed matrix, whole-repository inventory, or peer measurements were repeated. The lead owns the final gate.

No applicable AssuranceProfile was selected in the lead's established assurance context; no new profile inventory was performed. No prose style finding applies to the README's code-fence metadata/profile-only edit.

Plan completion: not assessed

## Examined Scope

```json
{
  "scope": [
    {
      "id": "NFR-017-AC-5",
      "path": "spec/non-functional/NFR-017-build-a-clean-clone-with-locked-dependencies-and-run-the-documented-examples.md",
      "role": "examined",
      "excerpt": "`doc_snippets` finds every FSL fence of `README.md` and `docs/**/*.md`, its count equal to a second line scan's count of opening fence lines whose info string starts with `ix:native` or `fsl`, and every fence with no `reject` or `pending` passes its declared stage; a fence with no `stage=` fails the test, naming its file and line."
    },
    {
      "id": "NFR-017-AC-6",
      "path": "spec/non-functional/NFR-017-build-a-clean-clone-with-locked-dependencies-and-run-the-documented-examples.md",
      "role": "examined",
      "excerpt": "A fence holding a deliberate syntax error tagged `stage=parse reject=invalid_syntax/unexpected-token` passes; the same fence without `reject` fails, and with `reject=unknown_profile/unsupported-selection` fails."
    },
    {
      "id": "NFR-017-AC-7",
      "path": "spec/non-functional/NFR-017-build-a-clean-clone-with-locked-dependencies-and-run-the-documented-examples.md",
      "role": "examined",
      "excerpt": "A `pending=<ticket>` fence whose `<!-- expect: ... -->` comment names its current refusal passes; the same fence when it passes its stage fails the test, and so does one whose comment names a different code, or that has no comment."
    },
    {
      "id": "run_at_stage",
      "path": "tests/it/doc_snippets.rs",
      "role": "examined",
      "excerpt": "fn run_at_stage(fence: &Fence) -> Result<Option<String>, String> {"
    },
    {
      "id": "first_refusal",
      "path": "tests/it/doc_snippets.rs",
      "role": "examined",
      "excerpt": "fn first_refusal(document: &OutcomeDocument) -> Result<Option<String>, String> {"
    },
    {
      "id": "every_fsl_fence_runs_at_its_declared_stage",
      "path": "tests/it/doc_snippets.rs",
      "role": "examined",
      "excerpt": "fn every_fsl_fence_runs_at_its_declared_stage() {"
    },
    {
      "id": "assert_no_bare_fsl_excerpts",
      "path": "tests/it/doc_snippets.rs",
      "role": "examined",
      "excerpt": "fn assert_no_bare_fsl_excerpts(file: &Path, text: &str) {"
    },
    {
      "id": "reject_fences_refuse_with_the_named_code",
      "path": "tests/it/doc_snippets.rs",
      "role": "examined",
      "excerpt": "fn reject_fences_refuse_with_the_named_code() {"
    },
    {
      "id": "pending_fences_match_their_current_refusal",
      "path": "tests/it/doc_snippets.rs",
      "role": "examined",
      "excerpt": "fn pending_fences_match_their_current_refusal() {"
    },
    {
      "id": "check_and_compile_fences_execute_the_language_stages",
      "path": "tests/it/doc_snippets.rs",
      "role": "examined",
      "excerpt": "fn check_and_compile_fences_execute_the_language_stages() {"
    },
    {
      "id": "README-fence-stages",
      "path": "README.md",
      "role": "examined",
      "excerpt": "```fsl stage=compile\nprofile v = \"filament.value.complete/v1\";\ntype Digit = Int[0, 9];\nrecord Point { x: Digit; y: Digit; }\nfunction px using v(p: Point): Digit pure { if p.x > p.y then p.x else p.y }"
    }
  ]
}
```

