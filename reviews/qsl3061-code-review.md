---
id: SR-9671750263021977689004741820
title: "Code review (Rust lane) of QSL-3061 handoff branch"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@3942ccc7de69cbc03763449667be8acb8895620d; fsl-eval/tests/it/qualification_vectors_from_packages.rs; requested QSL-3061 owned vector families"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-142
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-670
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-353
    type: reviews
---
# SR-9671750263021977689004741820: Code review of QSL-3061 handoff branch

## Summary

Reviewed only `origin/main...3942ccc7de69cbc03763449667be8acb8895620d`, per dispatch. This is the QSL-3061 branch output, distinct from live PR #3698, whose ref is `2bd99f1d3ca843e9cb19471f64622b6cb73c4a7a`. The branch changes a package qualification-vector test module, registers it, and adds trace tags to quantity negation tests. The worktree's unrelated unstaged files were ignored.

The named AC families are implemented with tagged tests. Existing tagged tests independently cover important lower-level semantics, including FR-091-AC-55 package-node identity/location/payload cases, FSpec FR-142-AC-24 equivalence classes, FSpec FR-142-AC-26 exact conversion values, FR-670-AC-2/3 division-law selection, FR-495-AC-7 request behavior, and FR-353-AC-2 qualification tampering. Those existing tests are not counted as defects in this branch. The new U38/U40 package tests assert successful assembly and refusal cases, while the vector assertions only look for a substring in the published vector text; they do not evaluate the imported-package conversion result. The owning ACs do not require a package-specific adapter, and equivalent conversion behavior is covered by the existing tagged AC-26 evaluation tests, so this is not raised as an owning-AC gap.

One concrete duplication is present: `Q_WITH_TIME_ROOT` and `Q_WITH_TIME_AND_MS` define byte-identical package source constants (lines 46-53). The latter's name suggests a package containing `ms`, but the unit is added later through `add_package_unit_edge`. Reuse the existing fixture constant or make the fixture genuinely distinct. No package/wire adapter requirement is inferred. DIV-14 remains unqualified pending QSL-3125; no synthetic cvc5 result is expected. U30-U33 / FSpec FR-142-AC-11..14 are outside this ticket's ownership and are not findings. No `QuantityCycle` or `TargetCycle` identifiers were introduced.

## Verdict

**CONDITIONAL**: one low-severity in-repository fixture duplication finding. `git diff --check origin/main...3942ccc7de69cbc03763449667be8acb8895620d` passed (exit 0). No cargo command or test-binary link was run, per dispatch constraint.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Identical Q package fixtures are declared twice | `fsl-eval/tests/it/qualification_vectors_from_packages.rs:46-53` |

### FND-001 detail

`Q_WITH_TIME_ROOT` and `Q_WITH_TIME_AND_MS` contain the same package document. The new constant duplicates an existing fixture, creating two names/owners for one source and a misleading fixture name. Search of the changed module confirms the same source is authored twice. Reuse one constant, or change the second fixture to actually include the intended unit. This review finding is limited to the test fixture duplication; the `ms` edge is separately created by `add_package_unit_edge`.

## Methods

- `code-review` and Rust lane: performed on the scoped branch diff; identified FND-001.
- `quoin:gap-analysis`: not applicable because the diff changes test code only; no full repository matrix/reverse-gap claim is made.
- `quoin:spec-review`: not applicable; no spec or plan files changed.
- Focused manual AC-to-test review: completed for the assigned owned families. Existing test evidence listed in Summary was verified in tagged Rust test sources. No cargo tests were run.
- Tracker resolution/posting unavailable by instruction/context: ticket text supplied as QSL-3061; Linear is unreadable and GitHub unauthenticated. No external findings were posted.
- External handoff blocker: branch head 3942 differs from live PR #3698 head 2bd99f1. This review concerns the branch only; PR association must be corrected by the lead.
