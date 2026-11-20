---
id: SR-2026111911313600000008
title: "Gap analysis of native collection element admission"
type: SpecReview
analysis: gap-analysis
scope: "QSL-1068 delta only; FR-106-AC-53..61/66/68/69, TC-1171, TC-1231; changed fsl-semantics admission, model mapping, frame identity, encoding and focused tests"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: references
---

## Summary

Compared the ticket's changed behavior and tests with the computed repository
matrix and governing requirements. The owner's focused-check instruction
limits this review to the ticket delta; it is not a repository-wide assurance
claim or a declaration that every vector of TC-1171/TC-1231 has been executed.

## Verdict

CONDITIONAL: the numeric admission criteria have computed test bindings;
the matrix omits the later collection-document criteria. Direct inspection
and focused tests cover the requested document examples.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The computed FR-106 matrix ends at AC-61 and omits AC-66/68/69 despite their later acceptance table and active trace tags. An exit-zero matrix cannot establish coverage of those rows. | spec/functional/FR-106-admit-snapshots-and-invocations.md; /tmp/qsl-1068-matrix.json; fsl-semantics/tests/it/native_snapshot_values.rs::native_collection_document_members_follow_the_declared_kind |

## Coverage

Plan completion: not assessed

Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2), with explicit scope
/Users/peter/dev/qsl-1068-code; no run evidence store read. All nine criteria
AC-53 through AC-61 compute tagged. AC-66/68/69 were inspected directly because
the matrix contains no corresponding rows. The matrix command exits 0 and
also reports fixture symbol collisions and unmarked trace warnings; these
diagnostics are not treated as passing evidence for omitted requirements.

Reproduction on Draco: executable
/Users/peter/.nvm/versions/node/v24.15.0/bin/quire; command
`quire matrix --scope /Users/peter/dev/qsl-1068-code --format json`.
Expected: the FR-106 row includes AC-66/68/69 from its acceptance section.
Actual: its 60 extracted criteria end at AC-61; none of the three later ids
occurs in the JSON. Output and stderr are retained in
/tmp/qsl-1068-matrix.json and /tmp/qsl-1068-matrix.stderr. No older binary or
cached matrix was substituted.

The changed behaviors have owners in FR-106, FR-144 and FR-181: Decimal value
membership and recursive canonical representation; Rational normalization;
Float width/bits and membership; Quantity magnitude/unit; String/Bytes/JsonObject
payloads; Duration's declared second; collection envelope causes and paths.
No new unowned semantics, source stubs, test stubs or skipped focused tests
were found in the delta. Optional semantic-review fan-out was not run.

Executed acceptance assertions cover Decimal duplicates and retained/normalized
representations, nested Options/sequences, numeric envelope/member refusals,
Rational equivalent spellings, both Float widths including signed zero and
quiet/signalling NaNs, Float set/ordered-set uniqueness and bag multiplicity,
Quantity uniqueness/multiplicity, String scalar identity, Bytes/JsonObject
canonical payloads/refusals, Duration over s in scalar/set/bag/ordered-set slots,
collection field/parameter document
examples, document-order naming and nested typed paths, and validation order.

Remaining assurance includes the full TC-1171 numeric snapshot fixture and all
frame/witness/malformed-payload vectors, context-root integration cases, the
matrix omission, lint debt and the code lead's independent review/gate. The
unit tests' criterion tags identify exercised portions, not full-vector proof.

Focused results: 11 unit tests in /tmp/qsl-1068-final-unit.log and three
integration tests in /tmp/qsl-1068-final-it.log pass. Initial expected failures
are retained in /tmp/qsl-1068-resume-red-unit-final.log,
/tmp/qsl-1068-resume-red-it.log and /tmp/qsl-1068-resume-red-precedence.log.
Final builds use the ticket target and qsl-slot, CARGO_BUILD_JOBS=5,
CARGO_INCREMENTAL=0 and a command-line line-tables-only debug override for
fsl-semantics. One earlier rerun exhausted disk; only its abandoned temporary
working directory was removed, preserving completed artifacts.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | Report the installed CLI, command, exit status and omitted rows to the owner; no spec or tooling change was authorized for this ticket. |
| FND-001 | rejected | Not a defect of this PR: the matrix omits FR-106-AC-62..69 because a blank line at spec/functional/FR-106-admit-snapshots-and-invocations.md:1309 on main splits the acceptance table, and that spec file is not in this diff; re-checked 2026-11-20 with quire 0.52.0 on main (rows still end at AC-61). The spec owner removes the blank line outside this PR. |
