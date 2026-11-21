---
id: SR-20261121003339002
title: "spec-review/base review of caller source constructor diagnostics"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; QSL-2726; FR-1039, TC-1295, TC-3019"
review_set: subset
---

## Summary

Ticket: QSL-2726; PR #3339. Frozen head 48a8953eb462e6561021e819ae8b104407aa5e84; merge base 2ed7bdfe23eaae6117788649b265ace5623d6589. Exactly three specification files; no production or Rust/Cargo diff. Read-only inspection of the pinned Contract IR constructors and native command serializer corroborates the requested distinction. No Cargo, runtime test, gate, pin change or author-branch edit was performed. quoin 0.36.8; quire 0.52.0 (engine 0.62.2); spec-artifacts-process v0.36.0. The changed three FR/TC files pass explicit ISO validation; the four EARS advisories at unchanged FR-1039 lines 114/116 are outside this diff. No added statement raises a new grammar diagnostic.

No AssuranceProfile document exists in this repository's spec tree (frontmatter search); no applicable enforced spec-review policy was found. Direct review uses the assigned subset: base, failure-domain, EARS-conformance and scope-boundary, plus the mandatory code-review. Rust review and gap-analysis are inapplicable because no Rust/Cargo or production code changed. Integrity, dependency, evidence, risk-complexity and object lenses are not separately selected: IDs, artifact structure, relationship edges, verification methods, component inventory and validation algorithms are unchanged. No criterion-strength/Jev assessment is claimed. Runtime trace coverage remains with the implementation PR; specification readiness is not runtime acceptance.

## Verdict

PASS — no findings for this method. This is bounded specification readiness only, not runtime or ticket completion.

## Method and evidence

All changed behavior and AC-8 are concrete and testable: blank document and zero revision are independently varied, the differing inner codes and their exact request field paths are asserted, while outer invalid-identifier/request/exit20 stays invariant. Wrong JSON types retain the separate invalid-request table row. Existing constructors own validation, including their ceilings; the amendment does not broaden accepted input. TC-1295 and TC-3019 provide direct acceptance procedures, not a claim that implementation tests have passed. No new tracking/digest/pin mechanism is introduced.

Pinned dependency evidence: Cargo.toml selects Contract IR 743b5e5f1983642a118dc1db2ee945d370e4a3ee. Its crates/filament-contract-model/src/identity.rs:53,55 defines invalid_identifier and invalid_source_revision. identifier_type! at 271–324 gives SourceDocumentId the InvalidIdentifier constructor; positive_revision! at 446–498 gives SourceRevision InvalidSourceRevision for zero and the existing exact-integer ceiling. No validation rule or diagnostic vocabulary is redefined locally.

Native boundary evidence: fsl-command/src/command.rs:1158–1169 retains the supplied Diagnostic in RunCause::Identifier and :1195 selects the outer InvalidIdentifier code. command/output.rs:304–307 keeps request stage and Details::Identifier; output/types.rs:64–65,107 marks that details enum untagged. Thus native Diagnostic fields are directly under /details, with no diagnostic wrapper. The separately active implementation runtime_sources.rs:240–253 uses the two existing constructors and replaces only their diagnostic.path with the corresponding /request/sources/{i}/document or /formal_revision selector. Its actual command log /tmp/qsl-2726-remainder-commands-2.log:14–16 corroborates revision zero, outer invalid-identifier and inner invalid_source_revision; that run was not executed by this reviewer and is not claimed green.

Computed coverage evidence: `quire matrix --strict --scope . --format json` exits 1; the changed FR-1039-AC-8 is `untagged` in this specification branch. The focused record is retained in matrix-excerpt.json. This is explicit remaining implementation/trace work, not a runtime acceptance claim or a requirement to mix code into this spec-first PR. Other whole-repository rows are outside this diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

Synthetic line/part suffixes locate verbatim changed fragments; they are not new specification IDs.

```yaml
{
  "clean": true,
  "scope": [
    {
      "id": "FR-1039-line-186-part-1",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "they are not substitutes for the outer command code. The formal constructor\nrefusal SHALL retain its original diagnostic at `/details`: a rejected\n`document` has `/details/code` `invalid_identifier`, while a rejected\n`formal_revision`, including zero, has `/details/code`\n`invalid_source_revision`. Both keep outer `invalid-identifier`, stage\n`request` and exit 20; neither constructor code is normalized to the other.\nFile-I/O `details` retains its path and OS code without adding a diagnostic\nrecord."
    },
    {
      "id": "FR-1039-line-373-part-1",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| FR-1039-AC-8 | A package request admits absent `sources`, `[]`, and an ordered array of exactly the five-field records above. Every malformed carrier and constructor-invalid value in the caller source table refuses with its stated request-stage code, member path and exit 20 before any selected file opens, including when `package` names an unavailable file. Independently replacing `document` with `\"\"` and `formal_revision` with `0` gives outer `invalid-identifier`, stage `request`, exit 20, with `/details/code` respectively `invalid_identifier` and `invalid_source_revision`, and `/details/pat"
    },
    {
      "id": "FR-1039-line-373-part-2",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "h` naming the corresponding field. A repeated native pair refuses at its second index despite different paths or formal metadata; distinct native pairs are admitted in array order. | Test |"
    },
    {
      "id": "TC-1295-line-181-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "  each exit 20. Independently set the selected source's `document` to `\"\"`\n  and its `formal_revision` to `0`: `/details/code` is respectively\n  `invalid_identifier` and `invalid_source_revision`, and `/details/path`\n  names the rejected field. Native diagnostic catalog codes retain their\n  underscore spellings; the original constructor diagnostic is not\n  normalized to another code or nested under `details.diagnostic`."
    },
    {
      "id": "TC-3019-line-90-part-1",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "separately from any native diagnostic code. Both formal constructor failures\nhave outer `invalid-identifier`, stage `request` and exit 20. The original\ndiagnostic at `/details/code` is `invalid_identifier` for a rejected\n`document` and `invalid_source_revision` for a rejected `formal_revision`;\n`/details/path` names the corresponding field. The file failure has outer\n`io-error` and its existing path/OS-code details. The native catalog's\nunderscore spellings are unchanged; no `details.diagnostic` wrapper is added."
    },
    {
      "id": "TC-3019-line-105-part-1",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| `document` is `\"\"`, all other fields valid | outer `invalid-identifier`, stage `request`, exit 20, `/details/code` exactly `invalid_identifier` and `/details/path` `/request/sources/0/document`, before I/O |\n| `formal_revision` is `0`, all other fields valid | outer `invalid-identifier`, stage `request`, exit 20, `/details/code` exactly `invalid_source_revision` and `/details/path` `/request/sources/0/formal_revision`, before I/O |"
    }
  ],
  "findings": [],
  "bindings": []
}
```
