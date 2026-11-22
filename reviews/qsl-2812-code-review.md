---
id: SR-967175026302197768900474181941935084691
title: "Code review of QSL-2812 stage and exit criteria"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2812; spec/functional/FR-027-export-compiled-native-package.md FR-027-AC-8; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md FR-096-AC-12"
review_set: subset
---

## Summary

Reviewed the two acceptance-row amendments in PR 3463. The exit-code row agrees with FR-286-AC-23 and the exhaustive code-category mapping; the compile row agrees with FR-763 while preserving actual forms refusal propagation and source, assembly, check and emit coverage.

## Verdict

**PASS** — no defects found in the scoped specification amendments. No implementation or test change is included in this PR. QSL-2812's subsequent test corrections and titan execution remain required before closing the ticket.

## Scope

```yaml
{
  "scope": [
    {
      "id": "FR-027-AC-8",
      "path": "spec/functional/FR-027-export-compiled-native-package.md",
      "role": "examined",
      "excerpt": "A `1-draft` source refused at each of `source`, `assembly`, `check` and `emit` returns the refusing stage's cause code and reports that stage, with no package bytes. Any actual `forms` refusal retains the same stage-and-cause propagation. [FR-763](FR-763-name-the-construct-in-every-forms-stage-refusal.md)'s admitted constructs, including `synthesis` and function-body `null`, pass S2 and are not forms-refusal fixtures; its AC-5 exercises `UnrepresentedConstruct` directly because no source reaches it, so this criterion requires no source fixture for that cause."
    },
    {
      "id": "FR-096-AC-12",
      "path": "spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md",
      "role": "examined",
      "excerpt": "Exactly `Code::RuntimeInvariant`, the code of `InternalFault` (T-4), and `Code::OutputFailure` ([FR-286-AC-23](FR-286-serialize-every-outcome-as-one-json-outcome-document.md)) have the O-16 internal-failure category and resolve to FR-301 exit status 30 (tool failure) through the category's FR-285 exit code; every other `Code` resolves to 20, 21 or 22. A native `run` whose evaluation refuses with `runtime_invariant` exits 30. A report holding a `runtime_invariant` or `output-failure` diagnostic beside invalid, unsupported or incomplete ones exits 30."
    },
    {
      "id": "FR-286-AC-23",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "context_only",
      "excerpt": "The export of `SlowEnough` over `Braking` under `minimum_fidelity` `any` with `output` `{\"file\": \"missing-directory/m.tla\"}`, whose parent directory does not exist, exits `30` and serializes with `operation` `export`, `category` internal-failure, `last_stage` S4, `result` `null` and one diagnostic with `code` `output-failure`, `cause` `output-failure` and `details` `null`; no file is written."
    },
    {
      "id": "FR-763-AC-5",
      "path": "spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md",
      "role": "context_only",
      "excerpt": "The `FormsCause::UnrepresentedConstruct` refusal for the `declaration` row, built directly because no source reaches the row, has catalog code `unsupported_construct` and construct spelling `declaration`; a 200-byte `text` is cut to at most 123 bytes ending with `...` and a short `text` is kept whole; its message is `` `<text>` declarations are not supported yet `` followed by ` (declaration <declaration>)` when an owner is set."
    },
    {
      "id": "FR-763-AC-6",
      "path": "spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md",
      "role": "context_only",
      "excerpt": "A function body `null` passes S2 and its check outcome document has one diagnostic with `code` `ill_typed`, `cause` `type-mismatch`, the exact span of `null`, and a message naming `null`, `none` and `absent` (FR-986); the document's category is `refusal`, exit 20."
    },
    {
      "id": "FR-763/Construct kinds",
      "path": "spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md",
      "role": "context_only",
      "excerpt": "No construct reaches `FormsCause::UnrepresentedConstruct` today. It stays\nthe specified S2 cause for a construct that has no form yet, so a new row\nneeds no new cause; FR-763-AC-5 exercises it at unit level."
    }
  ]
}
```

## Review evidence

- Read the complete two-file diff and the superseding FR-286-AC-23, FR-763 construct-kind text and AC-5/6.
- The current `fsl-foundation/src/diagnostic.rs:343` maps exactly RuntimeInvariant and OutputFailure to InternalFailure.
- `fsl-forms/src/value.rs:121` documents that no source reaches UnrepresentedConstruct; the constructor immediately below is test-only.
- The old source-stage test still supplies synthesis as a forms refusal, and the old exit-code test excludes OutputFailure. Those are precisely the later test corrections assigned by QSL-2812, not a condition this spec-only PR claims to have fixed.
- Computed `quire matrix --scope /home/peter/dev/worktrees/qsl-2812-stage-spec --format json` exited 0 and marks both changed criteria `tagged`. Its repository-wide source-symbol and unmarked-trace diagnostics remain disclosed; tagged status is not runtime evidence. Matrix JSON is retained in the reviewer scratchpad only.
- Changed-document `quire validate` with the installed spec-artifacts-iso module and `git diff --check origin/main...HEAD` exited 0.
- No applicable AssuranceProfile is declared under this repository's spec tree.
- No builds, tests or titan jobs were run or queued, under the explicit owner restriction.

## Method selection

Applied the language-independent code-review checks to the two Markdown changes. Rust review and implementation gap-analysis are not applicable because the diff contains neither Rust/manifests nor production code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
