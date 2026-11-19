---
id: SR-2026111518064800000001
title: "code-review review of PR2720 acquire/require unmangle"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-package/src/emit.rs, fsl-replay/src/application.rs, fsl-replay/src/application/division_tests.rs, fsl-replay/src/lib.rs, fsl-replay/tests/checked_nominal_declaration.rs, fsl-replay/tests/it/checked_body_failure.rs, fsl-replay/tests/it/refinement_query_facade.rs, tests/fixtures/checked-closure-consumer/src/lib.rs; PR2720 c216d31b3f0d4a7c65f6109f6c4e0f44a396c725"
review_set: subset
---

## Summary

Ticket: QSL-2250. Code-review (with rust-review lane) of a pure identifier rename: refilament_protocol_results_for_refusals -> require_protocol_results_for_refusals (private, fsl-package) and the public fsl_replay exports acfilament_checked_closure[_with_integer_division] -> acquire_checked_closure[_with_integer_division]. One finding: the public rename ships with no deprecated alias and the in-org consumer filament-contract-codegen still calls the old names on its main.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Public fsl_replay rename has no deprecated alias; filament-contract-codegen main still calls acfilament_checked_closure (owner_path.rs and tests) and tracks fsl-replay branch=main, so its next lock bump fails to compile until a CG catch-up PR lands (ticket AC 2) | fsl-replay/src/lib.rs:96 |

## Scope and evidence

- Diff is 36/36 lines, 11 files, one commit; every changed line is a mangled-word restore except the rustfmt reflow of the fsl_replay import list in fsl-replay/tests/it/checked_body_failure.rs:20-22 (caused by the shorter name; same item set).
- git grep -niE '[a-z]+filament[a-z]*' at HEAD: only libfilament_spec_language (rlib file prefix) and a \nfilament-spec-language string escape remain; both legitimate. No filament(d|s|ment) suffix stems, no uppercase *FILAMENT forms, no tracked file names with mangled stems. origin/main had 27 acfilament + 8 refilament; all gone.
- Renames are case-preserving (all occurrences were lowercase snake_case); doc comments already read Acquire.
- No new collision: the test helper fn acquire(..) in application.rs mod tests remains distinct from acquire_checked_closure.
- Cargo check not re-run by the reviewer (a Kani job held the machine; one cargo job at a time). Coder reports make ci green; static grep shows no remaining reference to the old names, so nothing dangles in-repo.
- Rust-review lane: no logic, unsafe, panic, integer-conversion or trait-seam change; signatures unchanged apart from the name.

## Verdict

Correct, complete and consistent rename within the repo. Mergeable; FND-001 is a cross-repo coordination item (CG caller update, ticket AC 2) to be landed right after, or an alias added if CG cannot follow in the same wave.
