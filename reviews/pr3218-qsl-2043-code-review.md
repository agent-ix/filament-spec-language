---
id: SR-967175026302197768900474181941935084825
title: "Code and Rust review of QSL-2043 protocol corpus and saga routes"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@c12fbe6b2ee156ba787f9573de573fd7f62dcc5b; git diff cd3412b60f24705ade58c5b212a149e30394e5ac...HEAD (QSL-2043 / PR 3218; 213 files); tests/corpus/protocol/**; tests/it/main.rs; tests/it/protocol_saga_route.rs; tests/it/protocol_source_routes.rs; tests/it/source_corpus.rs; tests/it/source_corpus_families.rs; ticket acceptance criteria"
review_set: subset
date: 2026-11-24
---

## Summary

Reviewed the QSL-2043 corpus and saga-route test changes at frozen head `c12fbe6b2ee156ba787f9573de573fd7f62dcc5b`, including the ticket's acceptance criteria. Two medium acceptance gaps remain, along with one low formatting issue in the touched integration-test module list.

## Verdict

**CONDITIONAL** — required protocol corpus and typed refusal coverage are incomplete; resolve the medium findings before treating the ticket acceptance criteria as met.

## Scope

- Examined QSL-2043 acceptance criteria for `corpus_protocol`, per-construct accept/reject coverage, pinned flips, bounded-static saga analyses, typed refusals, and complete saga classification.
- Inspected changed integration tests in `tests/it/` and protocol corpus fixtures under `tests/corpus/protocol/`.
- The changes are Rust test and fixture changes only; no production source or CI workflow was changed, so gap-analysis and spec-review methods did not apply.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The ticket requires `p-two-protocols-one-name` as a new pinned corpus program and requires every new pinned program to appear in the flip report. That fixture is absent from `tests/corpus/protocol/`, and `PROTOCOL_FLIPS` contains only `staged-saga-deadlock-free`, so the acceptance criterion cannot pass. | tests/it/source_corpus_families.rs:216-219 |
| FND-002 | medium | Typed-refusal acceptance is not checked for all required subsets. `declaration-on-each` expects `failed` for realizability instead of a typed refusal, and no symbolic-capacity saga is analyzed in `SAGAS`; `symbolic-capacity` is only compiled by the corpus. Consequently a wrong/missing refusal for those analysis cases can pass. | tests/it/protocol_saga_route.rs:84-89, tests/it/protocol_saga_route.rs:342-357 |
| FND-003 | low | The new `protocol_saga_route` module is listed before `protocol_source_routes`, but rustfmt orders these sibling modules alphabetically. `rustfmt --check --edition 2021 --config skip_children=true tests/it/main.rs` reports this touched-file formatting difference. | tests/it/main.rs:38-40 |

## Method Status

- `code-review` with Rust lane: completed; findings above.
- `quoin:gap-analysis`: not applicable because the diff changes no production code.
- `quoin:spec-review`: not applicable because no spec or plan files changed.
- `cargo fmt --check`: failed on widespread unrelated formatting differences in the workspace. Checking only the changed integration files found the module ordering issue in FND-003; remaining changed Rust files passed the isolated rustfmt check.
- Targeted `cargo test --features checked-handoff --test it protocol_saga_route::`: not completed. Other concurrent Cargo builds held the package-cache/build locks; the review invocation was stopped while waiting.
- Required workspace Clippy gate: not completed because other Cargo builds were active in the shared target directory.
- `cargo deny check`: failed with existing duplicate `quire-canonical` entries and a `Url::origin()` stack overflow advisory; this diff changes no dependency declarations.
- Linear issue access: `linear issue view QSL-2043` did not return within 30 seconds, so findings could not be posted or read back as comments.
