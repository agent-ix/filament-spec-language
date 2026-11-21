---
id: SR-20261121003322001
title: "code-review review of caller source outer command codes"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2726; spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md, spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md, spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md"
review_set: base
---

## Summary

Ticket: QSL-2726; PR #3322. Frozen head d4aa79d1f91eb7273659ea016b1596b2272d05fe against base e3ed0df2ce01b03dc663ed0910698ed5c75ce3cb; exactly FR-1039, TC-1295 and TC-3019, +25/-5. This review examines the changed outer-versus-inner diagnostic spellings only. No code, schema, catalog, budget, trust boundary, phase or FR-100 change is present.

No applicable AssuranceProfile exists in the local spec tree (frontmatter search); direct base review is authorized by the dispatch. Installed quoin 0.36.8, quire 0.52.0 (engine 0.62.2), spec-artifacts-process v0.36.0. Explicit ISO validation of the three changed artifacts exits 0 with four existing EARS advisories at unchanged FR-1039 lines 114/116; diff whitespace validation exits 0. The three artifact ids are unique in spec/.

## Verdict

PASS — no findings in the bounded frozen spelling correction. This is specification readiness only.

## Method

The source corroborates the correction: fsl-foundation/src/diagnostic.rs:191,193 returns `io-error` and `invalid-identifier`; fsl-command/src/command.rs:1176,1195 selects those codes. fsl-command/src/command/output.rs:423 serializes cause.code().as_str(). Its Io arm at 180-185 retains file stage and path/os_code; its Identifier arm at 304-307 retains request stage and the constructor diagnostic. output/types.rs:64 uses serde(untagged), with Identifier at 107, so the nested constructor code is `/details/code`, not `/details/diagnostic/code`. The supplied RED log /tmp/qsl-2726-caller-sources-3.log:34-47 corroborates those envelopes; it is supporting evidence, not a test run performed here.

The correction distinguishes a well-shaped but constructor-invalid formal document/revision from malformed carrier JSON. Native authority/identity are independent labels; no metadata authentication or normalization is introduced. FR-001 source/S0 and FR-283 SourceNotSupplied call behavior remain contextual, unchanged. No runtime or test implementation is modified or verified; the dependent caller-source tests still require their approved expectation update after this spec merges.

Methods run: bounded code-review (language-independent diff integrity and source faithfulness), spec-review/base. No Rust files or Cargo files changed; Rust review and build gates are not triggered by read-only serializer context. No production change means no gap-analysis. The requested base selection runs no optional analyses: no structural/domain-object/dependency/verification-method/boundary change; replacing code labels and explanatory prose does not introduce a new EARS requirement statement. No unchanged-program analysis or matrix census was run.

No implementation copy, new runtime oracle, weakened gate or unrelated spec change occurs in the diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

Synthetic line/part suffixes locate verbatim fragments; they are not new specification ids.

```yaml
{
  "clean": true,
  "scope": [
    {
      "id": "FR-1039-line-182-part-1",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "The table's codes name the outer `code` of the `native-run-result/2`"
    },
    {
      "id": "FR-1039-line-183-part-1",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "command envelope. Retained host spellings there are `invalid-identifier`"
    },
    {
      "id": "FR-1039-line-184-part-1",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "and `io-error`. Native diagnostic catalog spellings `invalid_identifier`"
    },
    {
      "id": "FR-1039-line-185-part-1",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "and `io_error` remain unchanged wherever a native diagnostic is carried;"
    },
    {
      "id": "FR-1039-line-186-part-1",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "they are not substitutes for the outer command code. In the existing formal"
    },
    {
      "id": "FR-1039-line-187-part-1",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "constructor refusal, `details.code` retains `invalid_identifier`; file-I/O"
    },
    {
      "id": "FR-1039-line-188-part-1",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "`details` retains its path and OS code without adding a diagnostic record."
    },
    {
      "id": "FR-1039-line-197-part-1",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| `document` or `formal_revision` fails its existing constructor, including revision zero | `invalid-identifier` | `/request/sources/<i>/document` or `/request/sources/<i>/formal_revision`, retaining the constructor diagnostic |"
    },
    {
      "id": "FR-1039-line-206-part-1",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "exit 22. An unavailable source file preserves the existing outer `io-error` at"
    },
    {
      "id": "FR-1039-line-369-part-1",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| FR-1039-AC-9 | With valid package and model inputs, each explicit source file is read and admitted once in array order before member execution. An unavailable source file gives outer command code `io-error` at stage `file`, exit 20; invalid UTF-8, BOM, NUL and each blank native label preserve FR-001's source/S0 code, cause and offered-source details. These failures produce no member outcome and are distinct from missing-region-source refusals. FR-026's shared intake ceilings apply without an additional source limit. | Test |"
    },
    {
      "id": "TC-1295-line-113-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "- Command-intake controls assert the outer `native-run-result/2.code`:"
    },
    {
      "id": "TC-1295-line-114-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "  `io-error` for a selected-file failure at stage `file` and"
    },
    {
      "id": "TC-1295-line-115-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "  `invalid-identifier` for a formal constructor failure at stage `request`,"
    },
    {
      "id": "TC-1295-line-116-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "  each exit 20. Native diagnostic catalog codes retain their underscore"
    },
    {
      "id": "TC-1295-line-117-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "  spellings; the constructor's existing `details.code` is `invalid_identifier`."
    },
    {
      "id": "TC-1295-line-118-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "  FR-001 source/S0 and `SourceNotSupplied` call refusals are unchanged."
    },
    {
      "id": "TC-3019-line-89-part-1",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "For retained host failures, assert the outer `native-run-result/2.code`"
    },
    {
      "id": "TC-3019-line-90-part-1",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "separately from any native diagnostic code. The formal constructor refusal"
    },
    {
      "id": "TC-3019-line-91-part-1",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "has outer `invalid-identifier` and `details.code` `invalid_identifier`;"
    },
    {
      "id": "TC-3019-line-92-part-1",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "the file failure has outer `io-error` and its existing path/OS-code details."
    },
    {
      "id": "TC-3019-line-93-part-1",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "The native catalog's underscore spellings are unchanged."
    },
    {
      "id": "TC-3019-line-102-part-1",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| invalid formal document, or `formal_revision` zero | outer `invalid-identifier`, stage `request`, exit 20, naming the constructor-invalid field, before I/O; constructor `details.code` remains `invalid_identifier` |"
    },
    {
      "id": "TC-3019-line-105-part-1",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| valid package/model inputs, unavailable explicitly selected source | outer `io-error`, stage `file`, exit 20, retaining its file path |"
    },
    {
      "id": "FR-1039-context-line-178",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "Before opening any selected file, FSL SHALL reject a malformed carrier and"
    },
    {
      "id": "FR-1039-context-line-179",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "the source-route conflict with the following request-stage diagnostics, exit"
    },
    {
      "id": "FR-1039-context-line-180",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "20, with no member outcome:"
    },
    {
      "id": "FR-1039-context-line-193",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "| repeated top-level `sources` member | `invalid-request` | `/request/sources` |"
    },
    {
      "id": "FR-1039-context-line-194",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "| an entry is not an object, including a positional record array | `invalid-request` | `/request/sources/<i>` |"
    },
    {
      "id": "FR-1039-context-line-195",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "| an entry lacks, repeats or adds a field, or a field has the wrong JSON type | `invalid-request` | `/request/sources/<i>/<field>` |"
    },
    {
      "id": "FR-1039-context-line-198",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "| `program` and `sources` both occur, including `sources: []` | `invalid-request` | `/request/sources` |"
    },
    {
      "id": "FR-1039-context-line-202",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "[FR-026](FR-026-run-standalone-native-workflow.md)'s path resolution,"
    },
    {
      "id": "FR-1039-context-line-203",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "`dependent_bytes` budget and shared 64-selected-file ceiling. The array's"
    },
    {
      "id": "FR-1039-context-line-204",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "JSON remains subject to `request_bytes`. These are the existing intake"
    },
    {
      "id": "FR-1039-context-line-208",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "native label, invalid UTF-8, BOM or NUL, preserves FR-001's native source/S0"
    },
    {
      "id": "FR-1039-context-line-209",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "diagnostic and original offered-source details, exit 20. Neither failure is"
    },
    {
      "id": "FR-1039-context-line-210",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "reclassified as `SourceNotSupplied`."
    },
    {
      "id": "FR-1039-context-line-212",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "When execution renders a source region, FSL SHALL match it only by the exact"
    },
    {
      "id": "FR-1039-context-line-224",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "`call`, exit 20, with the region's source identity, the clause when applicable,"
    },
    {
      "id": "FR-1039-context-line-225",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "and no clause record. This also applies to source-native condition and call"
    },
    {
      "id": "FR-1039-context-line-226",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "context_only",
      "excerpt": "refusal loci; it is not an internal fault or a package-reader refusal."
    }
  ],
  "findings": [],
  "bindings": []
}
```
