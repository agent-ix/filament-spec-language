---
id: SR-20261121003314005
title: "failure-domain review of package caller-source provisions"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/filament-spec-language; QSL-2726; spec/functional/FR-026-run-standalone-native-workflow.md, spec/functional/FR-100-run-a-named-function-through-the-spine.md, spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md, spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md, spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md"
review_set: subset
---

## Summary

Frozen head 463e467fce670fb5052f6489e88d7b23ddb37b55 against base d25728d74d; exactly five specification artifacts, +195/-7. Ticket: QSL-2726; PR #3314. This is specification readiness only. No implementation, runtime test, trace coverage, aggregate gate or unchanged-program census is claimed.

No applicable AssuranceProfile: no frontmatter `type: AssuranceProfile` exists in this specification tree. Direct review is authorized by the bounded dispatch; no enforced review operation was identified. Tool evidence: quoin 0.36.8; quire 0.52.0 (engine 0.62.2); spec-artifacts-process v0.36.0. Both explicit ISO and automatic scoped validation of all five changed artifacts exit 0; git diff --check exits 0. Base comparison shows exactly the same pre-existing advisories (ISO five, automatic four), with no newly introduced grammar warning.

Selected methods: code-review, base, integrity, ears-conformance, failure-domain, scope-boundary. Dependency analysis is not selected: added relationships are TC `verifies` edges, not new prerequisite edges; base checks their target identity. Evidence analysis is not selected: no verification-method change; the new `Test` obligations have concrete prospective procedures. Risk-complexity, architecture, object census, semantic strength/calibration and whole-program gap analysis are outside this bounded interface review. No production or Cargo diff means no Rust review/gates. Existing broader ticket findings are not dispositions of this PR.

## Verdict

PASS — no findings in the bounded frozen specification change.

## Method

Checked malformed carrier refusal before any selected-file I/O; existing formal constructor invalid_identifier/request/20; later unavailable file io_error/file/20; native Source::read_typed source/S0 failures; and subsequent SourceNotSupplied call/20 with source identity, clause when applicable and no rendered outcome. No source admission error is weakened into missing-source. Identity is exactly native authority/identity, unaffected by file, document, revision or bytes. Positive caller assertions do not authenticate content. Byte boundary failures, duplicate pairs, shared resource bounds and handoff fallback negatives are explicit. Rendering input is consumed once in request order after package/model admission; semantic evaluation never compiles it. No callback, graph algorithm or retry topology is added.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

The following machine-readable units preserve all examined additions and distinguish read-only context. Synthetic line/part suffixes locate statement fragments, not new requirement IDs.

```yaml
{
  "clean": true,
  "scope": [
    {
      "id": "FR-026-line-102",
      "path": "spec/functional/FR-026-run-standalone-native-workflow.md",
      "role": "examined",
      "excerpt": "size, whatever its role (model, each program unit, library, snapshot, invocation,"
    },
    {
      "id": "FR-026-line-103",
      "path": "spec/functional/FR-026-run-standalone-native-workflow.md",
      "role": "examined",
      "excerpt": "package or explicit runtime rendering source selected by"
    },
    {
      "id": "FR-1039-line-104",
      "path": "spec/functional/FR-026-run-standalone-native-workflow.md",
      "role": "examined",
      "excerpt": "[FR-1039](FR-1039-run-every-native-run-member-over-a-selected-checked-package.md)'s"
    },
    {
      "id": "FR-026-line-105",
      "path": "spec/functional/FR-026-run-standalone-native-workflow.md",
      "role": "examined",
      "excerpt": "`request.sources`), independently of the shared selected-file count ceiling."
    },
    {
      "id": "FR-026-line-106",
      "path": "spec/functional/FR-026-run-standalone-native-workflow.md",
      "role": "examined",
      "excerpt": "The command SHALL count each `sources` entry toward that same 64-file ceiling."
    },
    {
      "id": "FR-026-line-107",
      "path": "spec/functional/FR-026-run-standalone-native-workflow.md",
      "role": "examined",
      "excerpt": "The command SHALL read its selected file once, in array order, after package"
    },
    {
      "id": "FR-026-line-108",
      "path": "spec/functional/FR-026-run-standalone-native-workflow.md",
      "role": "examined",
      "excerpt": "and model admission and before member execution. The existing request-byte limit also bounds the"
    },
    {
      "id": "FR-026-line-109",
      "path": "spec/functional/FR-026-run-standalone-native-workflow.md",
      "role": "examined",
      "excerpt": "array's JSON; there is no separate source-count or runtime source-byte limit."
    },
    {
      "id": "FR-026-AC-8-line-121-part-1",
      "path": "spec/functional/FR-026-run-standalone-native-workflow.md",
      "role": "examined",
      "excerpt": "| FR-026-AC-8 | A package request whose dependent files, including every `sources` file, total exactly `dependent_bytes` runs; one additional byte settles incomplete with `resource_exhausted`, exit 22, naming `dependent_bytes` and its configured value. A valid package request selecting 64 files across all roles runs, while adding a 65th through `sources` settles incomplete with `resource_exhausted`, exit 22, naming the selected-file maximum 64. Padding the same request's JSON to exactly `request_bytes` runs; one extra byte settles incomplete with `resource_exhausted`, exit 22, naming"
    },
    {
      "id": "FR-026-AC-8-line-121-part-2",
      "path": "spec/functional/FR-026-run-standalone-native-workflow.md",
      "role": "examined",
      "excerpt": " `request_bytes`. The existing null cause/locus and limit-raising option diagnostics are preserved. | Test |"
    },
    {
      "id": "FR-100-line-299",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "- `sources`: optional, package-only caller source provisions for rendering,"
    },
    {
      "id": "FR-1039-line-300",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "  as [FR-1039](FR-1039-run-every-native-run-member-over-a-selected-checked-package.md)"
    },
    {
      "id": "FR-100-line-301",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "  \"Caller source provisions\" specifies: an ordered array of the existing"
    },
    {
      "id": "FR-100-line-302",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "  source selections, each with exactly `file`, `authority`, `identity`,"
    },
    {
      "id": "FR-100-line-303",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "  `document` and `formal_revision`. This is a new run-request member,"
    },
    {
      "id": "FR-100-line-304",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "  distinct from compiler program selections and `libraries`. A request"
    },
    {
      "id": "FR-100-line-305",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "  carrying `program` and `sources`, even `sources: []`, SHALL refuse"
    },
    {
      "id": "FR-100-line-306",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "  `invalid-request` at stage `request`, exit 20, naming `/request/sources`,"
    },
    {
      "id": "FR-100-line-307",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "  before any selected file is opened. Source runs retain the sources"
    },
    {
      "id": "FR-100-line-308",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "  admitted by their compilation."
    },
    {
      "id": "FR-100-AC-23-line-724",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "| FR-100-AC-23 | A package request admits the optional top-level `sources` array under FR-1039's caller source contract. The same member beside `program`, both as `[]` and as a non-empty valid array naming an unavailable file, refuses `invalid-request` at stage `request`, exit 20, naming `/request/sources`, before any file is opened. A source request without the member retains its existing compilation sources and outcomes. | Test |"
    },
    {
      "id": "FR-1039-line-42",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "the source the package was compiled from, with the same sources supplied for"
    },
    {
      "id": "FR-1039-line-43",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "any text or locus rendering that requires them. Optional caller source"
    },
    {
      "id": "FR-1039-line-44",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "provisions below do not participate in compilation or package execution."
    },
    {
      "id": "FR-1039-line-82",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "- `sources`: optional top-level caller source provisions, admitted only"
    },
    {
      "id": "FR-1039-line-83",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  beside `package`, as \"Caller source provisions\" specifies below."
    },
    {
      "id": "FR-1039-line-155",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "When a package request carries `sources`, FSL SHALL decode it as an ordered"
    },
    {
      "id": "FR-1039-line-156",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "array of the existing source-file selection records. Each entry is a closed"
    },
    {
      "id": "FR-1039-line-157",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "object with all five members required:"
    },
    {
      "id": "FR-1039-line-160",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "{\"file\":\"unit.native\",\"authority\":\"example\",\"identity\":\"unit-1\",\"document\":\"doc-1\",\"formal_revision\":1}"
    },
    {
      "id": "FR-1039-line-163",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "`file`, `authority`, `identity` and `document` are strings;"
    },
    {
      "id": "FR-1039-line-164",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "`formal_revision` is a JSON unsigned 64-bit integer admitted by the existing"
    },
    {
      "id": "FR-1039-line-165",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "positive `SourceRevision` constructor. The existing `SourceDocumentId`"
    },
    {
      "id": "FR-1039-line-166",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "constructor validates `document`. This reuses the command's `SourceFile`"
    },
    {
      "id": "FR-1039-line-167",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "record, including its formal metadata; it introduces no inline-text record."
    },
    {
      "id": "FR-1039-line-168",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "Formal metadata is a caller assignment, not authentication. Native source"
    },
    {
      "id": "FR-001-line-169",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "admission remains [FR-001](FR-001-read-exact-source.md)'s `Source::read_typed`."
    },
    {
      "id": "FR-1039-line-171",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "When `sources` is absent or `[]`, FSL SHALL supply no rendering source from"
    },
    {
      "id": "FR-1039-line-172",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "this member. When the array repeats an exact native `(authority, identity)`"
    },
    {
      "id": "FR-1039-line-173",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "pair, FSL SHALL refuse the second entry, even if its file or formal metadata"
    },
    {
      "id": "FR-1039-line-174",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "differs or its bytes would be equal. No label, path, text or metadata is"
    },
    {
      "id": "FR-1039-line-175",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "normalized. This member is admitted for every package run member; it supplies"
    },
    {
      "id": "FR-1039-line-176",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "neither compiler input, libraries, import resolution nor package bindings."
    },
    {
      "id": "FR-1039-line-178",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "Before opening any selected file, FSL SHALL reject a malformed carrier and"
    },
    {
      "id": "FR-1039-line-179",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "the source-route conflict with the following request-stage diagnostics, exit"
    },
    {
      "id": "FR-1039-line-180",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "20, with no member outcome:"
    },
    {
      "id": "FR-1039-line-182",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| Defect | Code | Member named |"
    },
    {
      "id": "FR-1039-line-184",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| `sources` is `null`, an object or another non-array value | `invalid-request` | `/request/sources` |"
    },
    {
      "id": "FR-1039-line-185",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| repeated top-level `sources` member | `invalid-request` | `/request/sources` |"
    },
    {
      "id": "FR-1039-line-186",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| an entry is not an object, including a positional record array | `invalid-request` | `/request/sources/<i>` |"
    },
    {
      "id": "FR-1039-line-187",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| an entry lacks, repeats or adds a field, or a field has the wrong JSON type | `invalid-request` | `/request/sources/<i>/<field>` |"
    },
    {
      "id": "FR-1039-line-188",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| an entry repeats an earlier native pair | `invalid-request` | `/request/sources/<i>` for the second entry |"
    },
    {
      "id": "FR-1039-line-189",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| `document` or `formal_revision` fails its existing constructor, including revision zero | `invalid_identifier` | `/request/sources/<i>/document` or `/request/sources/<i>/formal_revision`, retaining the constructor diagnostic |"
    },
    {
      "id": "FR-1039-line-190",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| `program` and `sources` both occur, including `sources: []` | `invalid-request` | `/request/sources` |"
    },
    {
      "id": "FR-1039-line-192",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "After package and model admission, FSL SHALL read and admit the explicitly"
    },
    {
      "id": "FR-1039-line-193",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "selected files once in array order, before executing the run member, using"
    },
    {
      "id": "FR-026-line-194",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "[FR-026](FR-026-run-standalone-native-workflow.md)'s path resolution,"
    },
    {
      "id": "FR-1039-line-195",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "`dependent_bytes` budget and shared 64-selected-file ceiling. The array's"
    },
    {
      "id": "FR-1039-line-196",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "JSON remains subject to `request_bytes`. These are the existing intake"
    },
    {
      "id": "FR-1039-line-197",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "limits; reaching one preserves `resource_exhausted`, category incomplete,"
    },
    {
      "id": "FR-1039-line-198",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "exit 22. An unavailable source file preserves the existing `io_error` at"
    },
    {
      "id": "FR-1039-line-199",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "stage `file`, exit 20. An admitted-file source failure, including a blank"
    },
    {
      "id": "FR-001-line-200",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "native label, invalid UTF-8, BOM or NUL, preserves FR-001's native source/S0"
    },
    {
      "id": "FR-1039-line-201",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "diagnostic and original offered-source details, exit 20. Neither failure is"
    },
    {
      "id": "FR-1039-line-202",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "reclassified as `SourceNotSupplied`."
    },
    {
      "id": "FR-1039-line-204",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "When execution renders a source region, FSL SHALL match it only by the exact"
    },
    {
      "id": "FR-1039-line-205",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "native `authority` and `identity` of the caller's admitted `Source`. The file"
    },
    {
      "id": "FR-1039-line-206",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "path, formal document/revision and source bytes confer no authentication of"
    },
    {
      "id": "FR-1039-line-207",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "that match. A locus renders its `file` from that source's authored path and"
    },
    {
      "id": "FR-1039-line-208",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "its span from the original source bytes. A finite-window decision-support"
    },
    {
      "id": "FR-1039-line-209",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "atom renders the exact whole atom slice, including authored spaces, under"
    },
    {
      "id": "FR-283-line-210",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "[FR-283](FR-283-monitor-a-supplied-trace-offline.md) and"
    },
    {
      "id": "FR-501-line-211",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "[FR-501](FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md),"
    },
    {
      "id": "FR-1039-line-212",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "for timestamped and progressed inputs, with or without a progress ledger."
    },
    {
      "id": "FR-1039-line-213",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "If no admitted source matches, or the matching source does not hold the"
    },
    {
      "id": "FR-283-line-214",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "region on UTF-8 boundaries, FSL SHALL preserve FR-283's `SourceNotSupplied`"
    },
    {
      "id": "FR-1039-line-215",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "contract: `invalid_runtime_input`/`missing-member` naming `sources`, stage"
    },
    {
      "id": "FR-1039-line-216",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "`call`, exit 20, with the region's source identity, the clause when applicable,"
    },
    {
      "id": "FR-1039-line-217",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "and no clause record. This also applies to source-native condition and call"
    },
    {
      "id": "FR-1039-line-218",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "refusal loci; it is not an internal fault or a package-reader refusal."
    },
    {
      "id": "FR-1039-line-220",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "FSL SHALL obtain rendering text only from these caller provisions. It SHALL"
    },
    {
      "id": "FR-1039-line-221",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "NOT discover source files in a handoff, open an unselected source path,"
    },
    {
      "id": "FR-1039-line-222",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "compile these sources, derive source authentication from a hash, or change"
    },
    {
      "id": "FR-1039-line-223",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "the package's semantic authority. Direct byte-only adapters receive admitted"
    },
    {
      "id": "FR-1039-line-224",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "`Source` values from their caller and perform no file intake. The canonical"
    },
    {
      "id": "FR-931-line-225",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "TL atom text of [FR-931](FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md)"
    },
    {
      "id": "FR-1039-line-226",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "is unchanged: its lowering uses package reader projections, not `sources`."
    },
    {
      "id": "FR-1039-AC-8-line-360",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| FR-1039-AC-8 | A package request admits absent `sources`, `[]`, and an ordered array of exactly the five-field records above. Every malformed carrier and constructor-invalid value in the caller source table refuses with its stated request-stage code, member path and exit 20 before any selected file opens, including when `package` names an unavailable file. A repeated native pair refuses at its second index despite different paths or formal metadata; distinct native pairs are admitted in array order. | Test |"
    },
    {
      "id": "FR-1039-AC-9-line-361",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| FR-1039-AC-9 | With valid package and model inputs, each explicit source file is read and admitted once in array order before member execution. An unavailable source file gives `io_error` at stage `file`, exit 20; invalid UTF-8, BOM, NUL and each blank native label preserve FR-001's source/S0 code, cause and offered-source details. These failures produce no member outcome and are distinct from missing-region-source refusals. FR-026's shared intake ceilings apply without an additional source limit. | Test |"
    },
    {
      "id": "FR-1039-AC-10-line-362-part-1",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| FR-1039-AC-10 | Source and package runs of TC-1295 rows 41 and 42 use the same admitted source bytes and native references: timestamped and progressed inputs, with and without a ledger, render non-empty decision support with the exact whole atom `holds( p )`. Removing or changing its reference, shortening the admitted source below the region, or placing a region endpoint inside a UTF-8 scalar gives `SourceNotSupplied`, `invalid_runtime_input`/`missing-member` naming `sources`, stage `call`, exit 20, source identity and clause, no clause record. Handoff source bytes cannot satisfy the"
    },
    {
      "id": "FR-1039-AC-10-line-362-part-2",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": " request. | Test |"
    },
    {
      "id": "FR-1039-AC-11-line-363",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| FR-1039-AC-11 | A package call refusal and a refused source-native condition with a region locus render the same code, cause, fields and native source span as their source runs, with `file` equal to the caller-supplied authored path even when it differs from the compiler's path. An omitted or wrong native reference, a short source or a non-UTF-8-boundary endpoint gives the caller-source refusal in AC-10 at stage `call`, exit 20, with no rendered member outcome. Different valid formal metadata does not change native region matching or atom text. | Test |"
    },
    {
      "id": "FR-1039-AC-12-line-364",
      "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| FR-1039-AC-12 | A package TL lowering with absent `sources`, `[]` and valid caller sources produces the same canonical atom text from the package projections as FR-931 specifies. Caller sources neither change package evaluation nor supply libraries, imports or bindings; no adapter discovers handoff source bytes, compiles caller sources or authenticates caller labels against their bytes. | Test |"
    },
    {
      "id": "FR-1039-line-10",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "  - target: ix://agent-ix/filament-spec-language/FR-1039"
    },
    {
      "id": "TC-1295-line-11",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "    type: verifies"
    },
    {
      "id": "TC-1295-line-43",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "These are direct byte-only adapter requests after caller provisioning of"
    },
    {
      "id": "TC-1295-line-44",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "admitted `Source` values. They perform no source-file intake during execution."
    },
    {
      "id": "TC-1295-line-52",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "For rows 43 and 44, exercise the command's `native-run/1` wire request with"
    },
    {
      "id": "TC-1295-line-53",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "`package` and the explicit top-level `sources` array from"
    },
    {
      "id": "FR-1039-line-54",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "[FR-1039](../functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md)."
    },
    {
      "id": "TC-1295-line-55",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "Use exactly the existing five-field source selections, for example"
    },
    {
      "id": "TC-1295-line-56",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "`{\"file\":\"unit.native\",\"authority\":\"example\",\"identity\":\"unit-1\",\"document\":\"doc-1\",\"formal_revision\":1}`."
    },
    {
      "id": "TC-1295-line-57",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "The command explicitly reads those selected files during intake, then passes"
    },
    {
      "id": "TC-1295-line-58",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "the admitted sources to execution. This file intake is distinct from the"
    },
    {
      "id": "TC-1295-line-59",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "direct byte-only adapter's absence of file reads. Source-run command controls"
    },
    {
      "id": "TC-1295-line-60",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "omit `sources` and use their compilation's admitted sources."
    },
    {
      "id": "FR-283-line-105-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "| 42 | PackageMissingSourceRefusal: row 41's direct byte-only package request after caller provisioning | remove the caller-supplied source whose reference matches the atom region; then replace it with a source of another reference; then supply the matching reference with bytes shorter than the span or with a span endpoint inside a UTF-8 scalar | each case preserves FR-283's `SourceNotSupplied` contract: category refusal, `invalid_runtime_input`/`missing-member` naming `sources`, exit 20, with the region's source identity and no clause record; the run refusal stays at stage `call`, as on the"
    },
    {
      "id": "FR-283-line-105-part-2",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": " source route, without using handoff source bytes, doing file intake during adapter execution or compiling source again |"
    },
    {
      "id": "TC-1295-line-106-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "| 43 | PackageCommandSupportAtomParity: row 41 through the command's package route | explicitly select the unit's exact original bytes in `request.sources`; use an authored file path different from the compile path and separately change only valid `document` and `formal_revision`; run timestamped and progressed inputs, with and without the ledger | non-empty support pairs equal the source command control and direct adapter; atom text is exactly `holds( p )`; native reference matching and text do not depend on the valid formal metadata or file path; explicitly selected files are admitted once"
    },
    {
      "id": "TC-1295-line-106-part-2",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": " before execution, no source compilation |"
    },
    {
      "id": "FR-001-line-107",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "| 44 | PackageCommandMissingSourceRefusal: row 43 | omit `sources`, use `[]`, replace the native reference, shorten the valid admitted UTF-8 bytes, then move the region endpoint inside a UTF-8 scalar; retain different source bytes in the handoff | each gives row 42's stage `call` refusal and no clause record; the command's explicit source intake is permitted, while the handoff supplies no fallback text; an invalid-UTF-8 source file instead retains FR-001's source/S0 intake refusal, not `SourceNotSupplied` |"
    },
    {
      "id": "FR-100-line-8",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  - target: ix://agent-ix/filament-spec-language/FR-100"
    },
    {
      "id": "TC-3019-line-9",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    type: verifies"
    },
    {
      "id": "FR-026-line-10",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  - target: ix://agent-ix/filament-spec-language/FR-026"
    },
    {
      "id": "TC-3019-line-11",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    type: verifies"
    },
    {
      "id": "TC-3019-line-18",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "own records, never discovers or compiles source, and refuses a package that"
    },
    {
      "id": "TC-3019-line-19",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "cannot run with the code and locus stated. Explicit caller-selected rendering"
    },
    {
      "id": "TC-3019-line-20",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "sources use the existing source-file intake and do not supply execution"
    },
    {
      "id": "FR-1039-AC-1-line-21",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "semantics. Scope: FR-1039-AC-1 to AC-12, FR-100-AC-23 and FR-026-AC-8."
    },
    {
      "id": "TC-3019-line-57",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "8. Send a package request with the optional top-level `sources` absent,"
    },
    {
      "id": "TC-3019-line-58",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "   then `[]`, then the ordered array"
    },
    {
      "id": "TC-3019-line-59",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "   `[{\"file\":\"unit.native\",\"authority\":\"example\",\"identity\":\"unit-1\",\"document\":\"doc-1\",\"formal_revision\":1}]`."
    },
    {
      "id": "TC-3019-line-60",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "   Exercise each carrier defect in the table below, with unavailable selected"
    },
    {
      "id": "TC-3019-line-61",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "   files to establish that structural refusal precedes I/O. Send duplicate"
    },
    {
      "id": "TC-3019-line-62",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "   native pairs with equal bytes, with different files, and with different"
    },
    {
      "id": "TC-3019-line-63",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "   valid formal metadata; then send distinct pairs. Send a source `program`"
    },
    {
      "id": "TC-3019-line-64",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "   request with `sources: []` and with a non-empty valid array, then omit it."
    },
    {
      "id": "TC-3019-line-65",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "9. After valid package and model admission, select an unavailable source"
    },
    {
      "id": "TC-3019-line-66",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "   file; separately select files containing invalid UTF-8, BOM and NUL, and"
    },
    {
      "id": "TC-3019-line-67",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "   each blank native label. Use two distinct valid entries to observe one"
    },
    {
      "id": "TC-3019-line-68",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "   admission per selected source, in array order, before member execution."
    },
    {
      "id": "TC-3019-line-69",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "   Vary `request_bytes`, `dependent_bytes` and the selected-file count as"
    },
    {
      "id": "TC-3019-line-70",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "   the resource rows below state, counting package, model and rendering"
    },
    {
      "id": "TC-3019-line-71",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "   files together. Start each invocation with fresh budgets."
    },
    {
      "id": "TC-3019-line-72",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "10. Compile a call that produces a genuine located refusal and a temporal"
    },
    {
      "id": "TC-3019-line-73",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    `holds` condition that produces a source-native refusal (the"
    },
    {
      "id": "FR-283-line-74",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    [FR-283](../functional/FR-283-monitor-a-supplied-trace-offline.md)"
    },
    {
      "id": "TC-3019-line-75",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    condition-refusal fixture), recording the source route's code, cause,"
    },
    {
      "id": "TC-3019-line-76",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    fields and span. Run the packages with explicit `sources` selecting"
    },
    {
      "id": "TC-3019-line-77",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    those original bytes from a different authored file path. Independently"
    },
    {
      "id": "TC-3019-line-78",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    omit the matching source, change its native pair, truncate its admitted"
    },
    {
      "id": "TC-3019-line-79",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    UTF-8 text below the span, and place a span endpoint inside a UTF-8"
    },
    {
      "id": "TC-3019-line-80",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    scalar. Change only valid formal metadata as a matching control."
    },
    {
      "id": "TC-1295-line-81",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "11. Run [TC-1295](TC-1295-evaluate-the-finite-window-profile-on-the-spine.md)"
    },
    {
      "id": "TC-3019-line-82",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    rows 43 and 44 for finite-window whole-atom text and missing-source"
    },
    {
      "id": "TC-3019-line-83",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    parity. Repeat step 6's TL lowering with absent `sources`, `[]` and valid"
    },
    {
      "id": "TC-3019-line-84",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    explicit caller sources; retain different source bytes in the handoff"
    },
    {
      "id": "TC-3019-line-85",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "    for the rendering tests."
    },
    {
      "id": "TC-3019-line-87",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "The carrier and resource cases for steps 8 and 9 are:"
    },
    {
      "id": "TC-3019-line-89",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| Input variation | Expected diagnostic and point of failure |"
    },
    {
      "id": "TC-3019-line-91",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| `sources` is `null`, `{}`, a string or a number | `invalid-request`, stage `request`, exit 20, `/request/sources`, before selected-file I/O |"
    },
    {
      "id": "TC-3019-line-92",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| repeated top-level `sources` | same code/stage/exit and `/request/sources`, before I/O |"
    },
    {
      "id": "TC-3019-line-93",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| entry is `null`, scalar or a positional array | same code/stage/exit, `/request/sources/0`, before I/O |"
    },
    {
      "id": "TC-3019-line-94",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| independently omit each of the five required fields, add `text`, or repeat each field | same code/stage/exit, naming `/request/sources/0/<field>`, before I/O |"
    },
    {
      "id": "TC-3019-line-95",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| non-string `file`, `authority`, `identity` or `document`; non-integer, negative, null or above-u64 `formal_revision` | same code/stage/exit, naming that field, before I/O |"
    },
    {
      "id": "TC-3019-line-96",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| invalid formal document, or `formal_revision` zero | existing `invalid_identifier`, stage `request`, exit 20, naming the constructor-invalid field, before I/O |"
    },
    {
      "id": "TC-3019-line-97",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| second entry repeats the first native pair, despite different path or formal metadata | `invalid-request`, stage `request`, exit 20, `/request/sources/1`, before I/O |"
    },
    {
      "id": "TC-3019-line-98",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| `program` beside either empty or non-empty `sources` | `invalid-request`, stage `request`, exit 20, `/request/sources`, before I/O |"
    },
    {
      "id": "TC-3019-line-99",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| valid package/model inputs, unavailable explicitly selected source | `io_error`, stage `file`, exit 20, retaining its file path |"
    },
    {
      "id": "FR-001-line-100",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| invalid UTF-8, BOM, NUL or a blank native label in selected source | unchanged FR-001 source/S0 diagnostic, exit 20, including original offered-source details |"
    },
    {
      "id": "TC-3019-line-101",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| request JSON at `request_bytes`, then one byte over | at the bound, run; over it, `resource_exhausted`, incomplete, exit 22, naming `request_bytes` and its configured value |"
    },
    {
      "id": "TC-3019-line-102",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| dependent file bytes including sources at `dependent_bytes`, then one byte over | at the bound, run; over it, `resource_exhausted`, incomplete, exit 22, naming `dependent_bytes` and its configured value |"
    },
    {
      "id": "TC-3019-line-103",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "| valid request with 64 selected files across all roles, then add a 65th via `sources` | 64 runs; 65 gives `resource_exhausted`, incomplete, exit 22, naming the selected-file maximum 64 |"
    },
    {
      "id": "TC-3019-line-141",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "- Steps 8 and 9: the table's diagnostic fields, stage, exit and intake"
    },
    {
      "id": "TC-3019-line-142",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  precedence hold, with no member outcome for failures. Distinct native pairs"
    },
    {
      "id": "TC-3019-line-143",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  remain in array order. Resource stops retain null diagnostic cause/locus"
    },
    {
      "id": "TC-3019-line-144",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  and the existing applicable limit-raising option; fresh invocations have"
    },
    {
      "id": "TC-3019-line-145",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  fresh budgets. Valid source requests without `sources` are unchanged."
    },
    {
      "id": "TC-3019-line-146",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "- Step 10: package refusals preserve the source result's code, cause and"
    },
    {
      "id": "TC-3019-line-147",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  fields; their locus `file` is the caller's new authored path and their span"
    },
    {
      "id": "TC-3019-line-148",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  equals the source span. Invalid or missing matches give"
    },
    {
      "id": "TC-3019-line-149",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  `invalid_runtime_input`/`missing-member` naming `sources`, stage `call`,"
    },
    {
      "id": "TC-3019-line-150",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  exit 20, with the region's native identity and clause when applicable,"
    },
    {
      "id": "TC-3019-line-151",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  no clause record or rendered member outcome. Valid changed formal metadata"
    },
    {
      "id": "TC-3019-line-152",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  leaves native matching unchanged. None becomes an internal fault."
    },
    {
      "id": "TC-3019-line-153",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "- Step 11: finite-window text is the exact whole `holds( p )` from caller"
    },
    {
      "id": "TC-3019-line-154",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  bytes; direct adapters do no source-file intake after caller provisioning."
    },
    {
      "id": "TC-3019-line-155",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  Command intake reads only explicitly selected files and never compiles"
    },
    {
      "id": "TC-3019-line-156",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  them or discovers handoff source bytes. TL atom text stays byte-equal to"
    },
    {
      "id": "TC-3019-line-157",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  step 6's canonical lowering. Caller labels and formal metadata remain"
    },
    {
      "id": "TC-3019-line-158",
      "path": "spec/test-cases/TC-3019-a-native-run-request-runs-over-a-selected-checked-package.md",
      "role": "examined",
      "excerpt": "  unauthenticated assertions; neither bytes nor a content hash validates them."
    },
    {
      "id": "SourceFile-context-1",
      "path": "fsl-command/src/command/wire.rs",
      "role": "context_only",
      "excerpt": "pub(super) struct SourceFile {\n    pub file: String,\n    #[serde(flatten)]\n    pub identity: Identity,\n}\n"
    },
    {
      "id": "Identity-context-1",
      "path": "fsl-command/src/command/wire.rs",
      "role": "context_only",
      "excerpt": "#[serde(deny_unknown_fields)]\npub(super) struct Identity {\n    pub authority: String,\n    pub identity: String,\n    pub document: String,\n    pub formal_revision: u64,\n}\n"
    },
    {
      "id": "Identity-bind-context-1",
      "path": "fsl-command/src/command/wire.rs",
      "role": "context_only",
      "excerpt": "    pub fn bind(\n        &self,\n        request_member: &str,\n    ) -> Result<crate::formal_source::SourceIdentities, ir::Diagnostic> {\n        let document = ir::SourceDocumentId::new(&self.document)?;\n        let revision =\n            ir::SourceRevision::new(self.formal_revision).map_err(|mut diagnostic| {\n                diagnostic.path = format!(\"{request_member}.formal_revision\");\n                diagnostic.message = format!(\"{request_member}.formal_revision must be positive\");\n                diagnostic\n            })?;\n        Ok(crate::formal_source::SourceIdentities {\n            nat"
    },
    {
      "id": "Identity-bind-context-2",
      "path": "fsl-command/src/command/wire.rs",
      "role": "context_only",
      "excerpt": "ive: fsl_foundation::SourceIdentity::new(\n                self.authority.clone(),"
    },
    {
      "id": "Intake-source-context-1",
      "path": "fsl-command/src/command.rs",
      "role": "context_only",
      "excerpt": "    fn source(\n        &mut self,\n        selected: &wire::SourceFile,\n        request_member: &str,\n    ) -> Result<FormalSource> {\n        let identity = selected.identity.bind(request_member)?;\n        let bytes = self.file(&selected.file, self.remaining)?;\n        self.admit_source(selected, identity, bytes, self.maximum)\n    }\n\n    /// Reads a source under `ceiling` (see `check_source_read_ceiling`) without"
    },
    {
      "id": "Intake-admit-source-context-1",
      "path": "fsl-command/src/command.rs",
      "role": "context_only",
      "excerpt": "        selected: &wire::SourceFile,\n        identity: crate::formal_source::SourceIdentities,\n        bytes: Vec<u8>,\n        byte_limit: usize,\n    ) -> Result<FormalSource> {\n        let source = Source::read_typed(identity.native, &selected.file, &bytes, byte_limit)\n            .map_err(|refusal| {\n                RunCause::Source(Box::new(SourceFailure {\n                    native: fsl_foundation::diagnostic::source_refusal(\n                        &refusal, &bytes, byte_limit,\n                    ),\n                    diagnostic: fsl_cst::diagnostic::source_refusal(*refusal, byte_limit)"
    },
    {
      "id": "Intake-admit-source-context-2",
      "path": "fsl-command/src/command.rs",
      "role": "context_only",
      "excerpt": ",\n                }))\n            })?;\n        self.sources.push(source.clone());\n        Ok(FormalSource::new(source, identity.formal))\n    }\n}\n\n/// FR-664: refuse a current-snapshot `parameters` member before reading the\n/// program or model sources. The selected snapshot is cached for the normal"
    },
    {
      "id": "FR-283-source-rendering-context-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "context_only",
      "excerpt": "- When a record names an atom (`InsufficientData`'s `atom`, an\n  `Indeterminate` reason's `atom`), the `monitor` operation shall name it by\n  the source text of the whole atom, read from the element of `sources`\n  whose reference equals the atom's region, and never by a\n  formula node index, and shall expose that name as the record's\n  `MonitorRecord::atom()`.\n- If no element of `sources` has the source of a record's locus or named\n  atom as its reference, then the `monitor` operation shall refuse the\n  whole request with `MonitorRefusal::SourceNotSupplied` naming the clause\n  and that source'"
    },
    {
      "id": "FR-283-source-rendering-context-2",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "context_only",
      "excerpt": "s identity, with no clause record.\n- If the matching element of `sources` does not hold that region's span on\n  UTF-8 boundaries, then the `monitor` operation shall refuse the whole\n  request with `MonitorRefusal::SourceNotSupplied` naming the clause and"
    }
  ],
  "findings": []
}
```
