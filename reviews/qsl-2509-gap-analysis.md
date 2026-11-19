---
id: SR-2026111911313600000004
title: "Gap analysis — QSL-2509 verification plan wire contract"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language PR #3022@570680d8fc6ab45ce7ead59f20786bd654a07335; FR-753-AC-3, FR-753-AC-5; FSpec FR-192 Package form; TC-2103; changed fsl-package and fsl-semantics paths"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-753", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/TC-2103", type: reviews }
---

## Summary

Planless scoped audit of the QSL-2509 change at reviewed head `570680d8fc6ab45ce7ead59f20786bd654a07335`. The computed matrix tags FR-753-AC-3 and FR-753-AC-5, and the changed tests exercise those criteria. The implementation does not emit the full node identity and method binding data required by FSpec FR-192 Package form.

Plan completion: not assessed

## Verdict

**FAIL** — one high-severity wire-contract gap leaves the emitted plan without the specified links to checked clauses, claim occurrences, method semantics, and domain declarations.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The emitted node contains claim/method/domain names only. FSpec FR-192 Package form requires the bound clause node identity and claim occurrence, FR-290 method kind/extent/evidence kind, and resolved domain declaration key, which are absent from the new document. Consumers therefore cannot establish the specified checked bindings from the document. | fsl-package/src/verification_plan.rs:102-108; FSpec FR-192 Package form |

## Coverage

- Gap-analysis CLI: the installed `quoin 0.36.7` exposes no `gap-analysis` subcommand (`quoin gap-analysis --help` returned “command gap-analysis not found”); the gap assessment was therefore performed manually from the matrix, tests, source, and requirements.
- Focused test: `cargo test -p fsl-package --test verification_plan` through `locked-build.sh` passed (2/2; 0 failures).
- Matrix method: `quoin matrix --repo /home/peter/dev/worktrees/qsl-2509 --json`; FR-753-AC-3 and FR-753-AC-5 are tagged; neither has recorded run evidence. The change's targeted test is run separately and recorded below.
- Manual criterion-to-test trace: FR-753-AC-3 → `emission_keeps_package_identity_and_writes_the_plan_graph` (`#[trace("TC-2103", "FR-753-AC-3")]`); FR-753-AC-5 mismatch prerequisite → `strict_reader_refuses_a_plan_bound_to_another_package` (`#[trace("TC-3028", "FR-753-AC-5")]`). The first test checks three nodes and one `requires` edge. Neither test checks the complete FSpec FR-192 node fields.
- Reverse gap: the new S4 emitter, reader and checked-plan retention are owned by FR-753; no additional unowned behavior found in the changed paths.
- Stub and coverage-inflation review: no stubs or vacuous assertions found. The missing linked fields are a wire-contract implementation gap, recorded as FND-001.
- Optional semantic review: skipped; no opt-in was supplied.
- Plan completion: not assessed.
