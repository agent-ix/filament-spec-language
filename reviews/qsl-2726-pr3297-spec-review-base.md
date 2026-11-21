---
id: SR-20261121003297002
title: "spec-review/base review of finite-window support atom source text"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; QSL-2726; spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md; spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md"
review_set: subset
---

## Summary

Base review of IDs, references, clarity, positive and negative test oracles and the hash/digest/pin antipattern. Ticket: QSL-2726. PR: filament-spec-language#3297. Frozen candidate: 6750dcca9907485d755def281daf3a9ac5cb4908; actual parent: f8686b7d5bfb152604ee917340b238beb8966382.

## Verdict

**PASS** — no defects found within the two-file specification diff. Exact whole-atom text `holds( p )` distinguishes authored text from canonical printing, an operand-only spelling or a node index. Non-empty support makes parity non-vacuous; positions remain trace positions. Timestamped, FR-504 Progressed and valid-ledger calls are explicit. Missing source, wrong reference, short span and split UTF-8 preserve the original SourceNotSupplied refusal, call stage, source identity and no-record result. Handoff bytes and implicit file access cannot satisfy a missing caller source.

## Review Scope

Selected methods: code-review, base, ears-conformance and failure-domain, under the dispatch and repository instruction to choose applicable lenses. No applicable AssuranceProfile: the local spec tree contains no AssuranceProfile document. No enforced policy applies. Unchanged structure, objects and relationship edges do not warrant integrity, object or dependency analyses. No new verification technology, architecture boundary or algorithm warrants evidence, risk-complexity or scope-boundary analyses. No Rust/React/Python implementation or Cargo files changed; language lanes and runtime gap analysis do not apply to this bounded spec-only diff. No Cargo checks or runtime coverage claim is made.

FR-501 delegates FR-283's contract rather than duplicating its refusal rules. FR-330 and FR-504 already assign this report to Timestamped and Progressed calls. FR-931's TL atoms table retains canonical checked-expression printing. Caller text has the source route's trust policy; no digest, source-byte authentication or new request member is introduced.

Package region provenance remains occurrence-based: fsl-package/src/checked_v2.rs:476 reconstructs an OccurrenceKey from node, role and ordinal and retains the entry's regions. The statement refers to the atom's region, never an arbitrary first node match. The planned shared occurrence-to-region helper must preserve that selected occurrence and return a region only; implementation is outside this PR. Repeated semantic nodes do not permit another occurrence's text.

Read-only grounding: fsl-replay/src/spine/clause/temporal.rs:3515 maps support through atom_name; :3601 slices caller sources and raises SourceNotSupplied on absent reference or unusable UTF-8 span. spine/call.rs:483 assigns call stage; native_result.rs:3090 maps missing-member and :3185 names sources. The brief's native_result/temporal.rs path for these helpers is outdated; they live in spine/clause/temporal.rs. Runtime behavior was neither changed nor certified.

## Examined Units

```json
[
  {
    "id": "FR-501-support-atom-name",
    "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
    "role": "examined",
    "excerpt": "In each decision-support `(position, atom)` pair, the source and checked-package\nroutes SHALL name `atom` by the source text of the whole atom from the\ncaller-supplied `sources` element whose reference equals the atom's region's\nsource, preserving FR-283's source-span and missing-source refusal contract.\n"
  },
  {
    "id": "TC-1295-source-authority",
    "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
    "role": "examined",
    "excerpt": "For rows 41 and 42, use the source and checked-package run routes with the\nsame caller-supplied `sources`, matching each atom region by source reference.\nExercise a `Timestamped` trace, an FR-504 `Progressed` trace and a call carrying\nthe corresponding progress ledger; each fixture must produce non-empty\ndecision support. The package route receives the emitted checked package and\ndoes not compile source again. Text comes only from the caller's `sources`,\nincluding when a package handoff holds other source bytes; no implicit file\nread supplies missing text."
  },
  {
    "id": "TC-1295-row-41-fixture",
    "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
    "role": "examined",
    "excerpt": "BothRoutesSupportAtomParity: row 28's clauses with the atom authored as `holds( p )`"
  },
  {
    "id": "TC-1295-row-41-input",
    "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
    "role": "examined",
    "excerpt": "row 28's timestamped inputs and equivalent FR-504 progressed inputs, with and without a valid ledger; the same caller-supplied sources on both routes"
  },
  {
    "id": "TC-1295-row-41-oracle",
    "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
    "role": "examined",
    "excerpt": "equal decision-support pairs on both routes, with the atom name exactly `holds( p )`, including its internal spaces and the whole `holds` atom; positions remain those of the corresponding trace"
  },
  {
    "id": "TC-1295-row-42-fixture",
    "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
    "role": "examined",
    "excerpt": "PackageMissingSourceRefusal: row 41's package request"
  },
  {
    "id": "TC-1295-row-42-input",
    "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
    "role": "examined",
    "excerpt": "remove the caller-supplied source whose reference matches the atom region; then replace it with a source of another reference; then supply the matching reference with bytes shorter than the span or with a span endpoint inside a UTF-8 scalar"
  },
  {
    "id": "TC-1295-row-42-oracle",
    "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
    "role": "examined",
    "excerpt": "each case preserves FR-283's `SourceNotSupplied` contract: category refusal, `invalid_runtime_input`/`missing-member` naming `sources`, exit 20, with the region's source identity and no clause record; the run refusal stays at stage `call`, as on the source route, without using handoff source bytes, reading a file or compiling source again"
  },
  {
    "id": "FR-283-atom-name",
    "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
    "role": "context_only",
    "excerpt": "- When a record names an atom (`InsufficientData`'s `atom`, an\n  `Indeterminate` reason's `atom`), the `monitor` operation shall name it by\n  the source text of the whole atom, read from the element of `sources`\n  whose reference equals the atom's region, and never by a\n  formula node index, and shall expose that name as the record's\n  `MonitorRecord::atom()`."
  },
  {
    "id": "FR-283-missing-source",
    "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
    "role": "context_only",
    "excerpt": "- If no element of `sources` has the source of a record's locus or named\n  atom as its reference, then the `monitor` operation shall refuse the\n  whole request with `MonitorRefusal::SourceNotSupplied` naming the clause\n  and that source's identity, with no clause record."
  },
  {
    "id": "FR-283-source-span",
    "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
    "role": "context_only",
    "excerpt": "- If the matching element of `sources` does not hold that region's span on\n  UTF-8 boundaries, then the `monitor` operation shall refuse the whole\n  request with `MonitorRefusal::SourceNotSupplied` naming the clause and\n  that source's identity, with no clause record."
  },
  {
    "id": "FR-330-finite-window-report",
    "path": "spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md",
    "role": "context_only",
    "excerpt": "FR-109's `ClauseRunReport`: the disposition, the compiled `package_id` and\nthe usage; for a `Progressed` or `Timestamped` trace it also returns the\nupdated `ledger` (FR-502) and FR-501's result report (basis, decision\nsupport, premises and result identity) as `ClauseRunReport::finite_window`,\nwhose `basis` is FR-504's or FR-501's and may differ from the report's\ntop-level FR-266 `basis`; a request refused by FR-502 returns the ledger it supplied,"
  },
  {
    "id": "FR-504-finite-window-report",
    "path": "spec/functional/FR-504-carry-progress-on-event-position-and-fixed-sample-traces.md",
    "role": "context_only",
    "excerpt": "FR-501's outcomes and result report (`Completed`, `Pending`, `Indeterminate`,\nbasis, decision support, premises, result identity), with `Pending.deadline`\ndefined below. The result report is the clause run's finite-window report,\n`ClauseRunReport::finite_window` (FR-330), and its `basis` is the basis this\nrequirement names: for an open-scope verdict settled through silent samples\nit is `decisive-counterexample` or `decisive-witness`, because the verdict"
  },
  {
    "id": "FR-931-canonical-atom-text",
    "path": "spec/functional/FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md",
    "role": "context_only",
    "excerpt": "The `atoms` entry of an atom SHALL be its **atom text**: the canonical\ntext of the atom's `holds(e)` leaf, printed from the leaf's checked\nexpression `e` by one canonical printer. The source route and the package\nroute (FR-1039) SHALL call the same printer on the same checked expression,\nso the two routes give byte-equal `atoms`. The printer reads the checked\nexpression and the names the checked package records for it, and never\nreads source text or a source span: a package carries no source bytes, and"
  }
]
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Validation

Contract fetched with quoin write . --types SpecReview. Tools: quoin 0.36.8; quire 0.52.0 (engine 0.62.2); spec-artifacts-process@v0.36.0. Both changed artifacts validate: exit 0, 2/2 grammar-clean, zero grammar findings. git diff HEAD^ HEAD --check exits 0. This review is validated at its explicit scratchpad path before posting. Existing runtime implementation gaps and unrelated older criteria are outside this specification-readiness verdict.
