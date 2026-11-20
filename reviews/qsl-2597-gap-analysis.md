---
id: SR-2026111921182600000002
title: "Gap analysis of QSL-2597 PR #3100 against FR-931 label ACs"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@ed8197074a8c0b3b99201b8ea3344148cd1505f9; git diff origin/main...HEAD (PR 3100); FR-931 Names, FR-931-AC-1, AC-11, AC-12, AC-13; fsl-command/tests/it/package_run.rs, tests/it/request_surface_s1_16.rs; QSL-2597"
review_set: subset
date: 2026-11-19
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: reviews
---

# Gap analysis of QSL-2597 PR #3100

## Summary

Ticket: QSL-2597. This is a focused AC-to-test check of the FR-931
canonical-label criteria that the changed printer serves. It is a manual
check scoped to this diff, not a repo-wide `quoin matrix` run. Plan
completion: not assessed.

Examined:

- FR-931 "Names": a field, operation or relationship-end member prints as
  its member name.
- FR-931-AC-1: `atoms` has an entry for `p0`.
- FR-931-AC-11: the atom is exactly `holds(c.versionNumber <= 2)`. The
  Boolean, Integer and Text field variant prints exact text, and the package
  route gives byte-equal atoms.
- FR-931-AC-12: alias-free declaration keys.
- FR-931-AC-13: case printing, including `c.versionNumber`.

## Verdict

The production change matches FR-931 "Names". No criterion asks for a
selected exact row, so removing the row requirement leaves no AC without its
implementation.

The gap is in the tests. FR-931-AC-11 is the criterion that fixes field and
attribute label bytes. It is claimed by
`source_tl_labels_use_checked_precedence_and_alias_free_declaration_keys`
(`#[trace("FR-931-AC-11", "FR-931-AC-12")]`), but that test's leaves contain
no field read at all. That is why #3030 shipped a fault on every field read
without failing its own test. This gap predates the PR, but it is the root
cause of the red main that this PR fixes, and the PR adds nothing to close it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-931-AC-11 has no test that exercises its fixture. The tagged package_run test prints only calls, enum literals and record constructors, and no field or attribute read. Nothing tests the exact `holds(c.versionNumber <= 2)` atom, the `c.a`/`c.x`/`c.name` variant, or package-route byte equality for field reads | fsl-command/tests/it/package_run.rs:312-330; spec/functional/FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md:489 |
