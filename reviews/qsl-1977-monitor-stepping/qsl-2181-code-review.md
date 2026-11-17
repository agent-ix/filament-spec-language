---
id: SR-14862
title: "Code and Rust review of the fsl-* crate and filament.* id migration"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-driver; QSL-2181; PR 155 at e757f7158381394e0f54414a9aa80385163cf417; Cargo manifests, src/**, fsl-cli/**, filament-cache, filament-plugin-host, tests/**, spec/**, README.md"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-driver/FR-001", type: references}
  - {target: "ix://agent-ix/filament-driver/AD-001", type: references}
---

## Summary

Ticket: QSL-2181. Read-only code and Rust review of the diff origin/main...HEAD (74 files). The manifests move every qsl-* and quire-* language and layer dependency to fsl-* and filament-* on branch main; quire-canonical stays because no filament-canonical repository exists. Profile and operation ids move to filament.*, the outcome schema to filament-outcome/1 and native type refs to ix://filament/native. One stale wire id remains in documentation.

## Verdict

REQUEST CHANGES (one low finding). No rev, hash or tag pins were added outside Cargo.lock; no #[allow]; unwrap/expect counts are unchanged (+11/-11, all renamed lines); no Python. The FR-287-AC-6 split keeps every assertion: the UTF-8 refusal half still runs everywhere, and the raw-path half keeps its exit 0, empty stderr and success-category checks, gated off macOS only because APFS cannot create a non-UTF-8 name. The assert_eq reflow in tests/drive.rs follows rustfmt fn_call_width. The spec edits change ids only and agree with the code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Stale `quire.checked-package/v2` wire id; fsl-package on language main emits `filament.checked-package/v2` | src/lib.rs:12; spec/functional/FR-001-orchestrate-the-driver-steps.md:28; spec/assurance/AD-001-driver-architecture.md:145 |

## Coverage

rust-style baseline, repository rustfmt.toml applied. No build or test was run (the lead's make ci was using the worktree). Gap analysis and spec-review were not run: the change is an id rename with no behavior or acceptance-criterion change.
