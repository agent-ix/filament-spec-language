---
id: SR-202611141401004
title: "Integrity review of the QSL-1942 AOT owner change"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; PR #2704 at 8ef45a09e17ae92270ebd7a973209a86e40f05d3, diff origin/main...HEAD: ADR-029 (CB-3 item 3, §7 table, EB-6), FR-294, FR-295, FR-563 Overlap and description; consistency checked against every `quire-aot` mention under spec/"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/ADR-029, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-284, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-562, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-567, type: references }
---

## Summary

Ticket: QSL-1942. The ticket asks for one owner of the AOT backend. The diff makes QSL the
owner in FR-294 (description and Overlap), FR-295 (AOT paragraph and Overlap), FR-563
(description and Overlap), the ADR-029 §7 table and EB-6. A grep of `quire-aot` over `spec/`
at the PR head finds four places that still give `quire-aot` to the driver, one of them in
the bullet list the diff edits.

## Verdict

FAIL on FND-001 (high): ADR-029 CB-3 item 3 now calls `quire-aot` a QSL crate and, two lines
later, lists it under "Driver repository". FND-002 to FND-004 (medium) are the other
requirements that still name the driver's `quire-aot`.

Examined and clean: FR-294 line 101 and Overlap, FR-295 AOT paragraph and Overlap, FR-563
description and Overlap, ADR-029 §7 table row and EB-6 all name QSL as owner and the driver
and CG as owners of lowering and generation with the AOT target. ADR-029 EB-6's amendment
note records that it replaces the 2026-11-08 amendment. `quire validate` passes on all six
changed files.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | ADR-029 CB-3 item 3: the edited `qsl-conformance` bullet says "`quire-aot` is a QSL crate that `cargo xtask parity` links", but the next bullet still reads "Driver repository: `quire-plugin-host` (§5), `quire-aot` (§7) and the binary crate `quire-cli`". The record states two owners. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:251-252 |
| FND-002 | medium | FR-284 still lists "the driver's `quire-plugin-host`, `quire-aot` and `quire-cli` (ADR-029 CB-3 item 3)" in its description and "the driver's `quire-driver`, `quire-plugin-host`, `quire-aot` or `quire-cli`" in Behavior; with QSL owning `quire-aot`, it belongs with `qsl-analyze`, `qsl-inspect`, `qsl-jit` as a QSL crate above the core. | spec/functional/FR-284-keep-the-qualified-core-separable-by-crate.md:40, spec/functional/FR-284-keep-the-qualified-core-separable-by-crate.md:65 |
| FND-003 | medium | FR-562 Overlap still reads "The driver's `quire-aot` follows the same rule in the code it generates, under its own record". | spec/functional/FR-562-take-a-machine-width-fast-path-only-where-checked-bounds-prove-the-result-fits.md:134 |
| FND-004 | medium | FR-567 Overlap still reads "The driver repository qualifies its own half of the path: its CLI half of the matrix (FR-565) and the parity of `quire-aot` (FR-563)", while FR-563 now says `xtask parity` gates `quire-aot` and the driver uses `qsl-conformance` for the CLI half only. | spec/functional/FR-567-qualify-qsl-on-the-final-path-the-driver-calls.md:118-119 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
