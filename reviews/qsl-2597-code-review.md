---
id: SR-2026111921182600000001
title: "Code review of QSL-2597 PR #3100 TL field label lowering"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@ed8197074a8c0b3b99201b8ea3344148cd1505f9; git diff origin/main...HEAD (PR 3100); fsl-command/src/command/tl/canonical.rs, fsl-replay/src/spine/clause.rs; QSL-2597"
review_set: subset
date: 2026-11-19
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: reviews
---

# Code review of QSL-2597 PR #3100

## Summary

Ticket: QSL-2597. Code review with the rust-review lane, over the two files
the PR changes. The PR removes `field_text`, `CanonicalRows.fields` and
`pub TemporalValuation.canonical_fields`, and prints `Attribute` and `Field`
reads from the checked member name.

Evidence, in a throwaway worktree with its own target dir under /home:

- `cargo-slot cargo test -p filament-spec-language --test it -- request_surface_s1_16 --test-threads=1` with the house skips: 17 passed, 0 failed, 22 ignored (on main the 10 tests fault with "field has a selected exact identity row", per the ticket).
- Probe (local only, never pushed): I strengthened two active tests to exact `assert_eq!` and both passed: the attribute atom `holds(c.versionNumber <= 2)` in `a_supported_clause_lowers_to_a_tl_formula`, and the source-record field atom `holds(c.level = 0)` in `a_fairness_constraint_over_a_lasso_settles_missing_fairness_premise`. Both spellings match FR-931 "Names" and AC-11. I reverted the probe afterwards.
- `cargo-slot cargo clippy -p fsl-command -p fsl-replay` with `RUSTFLAGS='--cap-lints warn'`: no warning on a changed line. The four warnings in fsl-replay/src/spine/clause.rs, at lines 581, 617, 686 and 1957, were already there and are outside the diff hunks, and no unused-import warning remains.

## Verdict

The code change is correct and matches the spec. FR-931 "Names" says a
field, operation or relationship-end member prints as its member name. No
FR requires a selected exact field row for a TL label. `grep -ri "selected
exact" spec/` finds only FR-251 (monitor plans). The #3030 PR body asks only
for "alias-free declaration/member identities". The removed `field_text`
looked up the row, discarded it and returned `name`. So for every input that
lowered before, the output is unchanged; the only behaviour change is that
the false fault goes away.

The coder's root cause checks out. `fsl-semantics/src/model/intake/unit.rs:616-639`
mints `SelectedExactFieldRow` only for selected-model record value types,
keyed `MemberOwner::Node(record_key)`. `Attribute` looked up
`MemberOwner::Effective(..)`, so it could never match. A source record's
field has no selected-model row. The TL rows are therefore not the place to
fix this, and minting rows at intake would be wrong: they are package
identity rows that `fsl-package` emit uses. The removal is correct.

Public API: `TemporalValuation.canonical_fields` is removed. A read-only
grep finds no use in this workspace, in ~/dev/filament-driver, in
~/dev/filament-contract-codegen, or in any other ~/dev Rust repo outside
target/ and worktrees/. The touched files are unchanged on main since the
branch point, which is 6 commits behind. The PR adds no `#[allow]`, panic or
unsafe code.

The PR body says "the existing tests pin the behavior". That holds for the
fault, but not for the spelling: see FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No active test asserts the exact atom text of a field or attribute read. The active `tl_lowering` tests only check `atoms.p0.is_string()`. The exact `c.versionNumber` spellings are in AC-13 tests that are `#[ignore]` (QSL-2562). No test checks a source-record `Field` read (`c.level`) at all. A regression in member spelling, such as printing an owner-qualified identity, would pass every active test | tests/it/request_surface_s1_16.rs:293; tests/it/request_surface_s1_16.rs:947; tests/it/request_surface_s1_16.rs:1002-1003; fsl-command/src/command/tl/canonical.rs:123-130 |
