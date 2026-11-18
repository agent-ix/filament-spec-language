---
id: SR-2026111612172400000037
title: "Spec review of filament-spec-language PR #2913"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #2913 diff origin/main...c8c9216dc: spec/functional/FR-569, spec/functional/FR-531, spec/test-cases/TC-1408"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-569", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-531", type: references}
  - {target: "ix://agent-ix/filament-spec-language/TC-1408", type: references}
---
# Spec review of PR #2913

## Summary

Ticket: QSL-1880. Reviewed the FR-569 "Source-object identity rows" statement edit, new FR-569-AC-16, the FR-531 `source_objects` row encoding and the TC-1408 scope/step-6 edits. Sub-analyses applied inline: integrity (FR-569 vs FR-531 vs TC-1408 consistency) and EARS/testability of AC-16. Other `quoin:spec-*` sub-analyses do not match the scope of this change.

## Verdict

PASS with low findings. FR-569, FR-531 and TC-1408 agree on the member name, the entry shape `{identity, query}`, own-only/source-order semantics and redefinition. AC-16's values match TC-1408 fixture S (four `Order` operations, two `Sub` redefinitions, six `operation_effects` rows). The query rule is cross-referenced to the existing "dispatch-eligible query" rule in FR-569 rather than restated differently, and the implementation uses the same predicate as `checked_dispatch.rs`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-569 statement defines `identity` as "the `source_identity` of its source-operation row (FR-390)" unconditionally, but source-operation rows exist only for operations with an anchor (in fixture S only `Order::bump`); AC-16 says "where one exists". The statement should carry the same qualifier. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:565-569 |
| FND-002 | low | FR-531 and FR-569 define a malformed `operations` member by shape only; they do not say whether a reader checks entries against the package (identity of the form `<source_identity>::<op>`, unique, one `operation_effects` row each), so two readers could differ on a file whose entries are well-shaped but name no operation. | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:244 |
| FND-003 | low | TC-1408 adds FR-569-AC-16 to step 6, which the TC says "waits on QC-30", although the emission half of AC-16 has a running test; the TC does not say which part of AC-16 runs now. | spec/test-cases/TC-1408-declare-object-types-populations-and-operations-in-source.md:24-28,104-109 |

## Review scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-569-AC-16 | examined | Over fixture S, the `operations` of the source-object rows are [...] a row whose `operations` member is absent, repeated, or holds an entry with another member set is `Malformed` at `source_objects`. D's emission has none. |
| FR-569 | examined | `operations` (one `{identity, query}` entry per operation the type itself declares, in source order [...] an operation the type only inherits has no entry, and a redefining operation has its own). |
| FR-531 | examined | `source_objects` [...] and `operations` its own operations in source order, each `{ "identity", "query" }` with `identity` the string `<type>::<operation>` and `query` a boolean |
| TC-1408 | examined | Step 6: [...] Read each row's `operations` and check them against the `operation_effects` rows. |
| FR-569-AC-11 | context_only | S4 [...] does not refuse the unit for declaring an `object`. |

- Compatibility: the identities/4 format string is unchanged while the row gains a required member, so pre-change /4 files read as `Malformed`. This is intentional (no-shims policy) and FR-531's "all members required, no empty fallback" wording covers it; not a finding.
