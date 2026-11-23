---
id: SR-967175026302197768900474181941935084799
title: "code-review (with rust-review lane) of PR3591 check --markdown argv and missing_import message"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3591 8cd9bd289a2aa573c1fffc5c7fafcfdcb5012ae2 diff over origin/QSL-2805-work; fsl-command/src/command.rs, fsl-replay/src/outcome.rs, fsl-semantics/src/model/intake.rs, tests/it/check_imports.rs, tests/it/request_surface_s1.rs"
review_set: subset
---

## Summary

Ticket: QSL-2977 (with QSL-2976). The PR's own change is one commit, 8cd9bd289a. The other commit the branch carries over origin/QSL-2805-work, 60fa21b085 (#3583 xtask merge-gate recipe), has the same patch-id as origin/main c1d47eddc3 and is not reviewed as this PR's work. The argv `check` parser now takes `--markdown`, `--format json` and `--extensions <file>` in any order and routes `--markdown` to `check_native_fences`. The admit `missing_import` detail now names `--model <identity>=<file>` and the `models` member, and the check-only helper that appended the same guidance was removed, so no path doubles it. rustfmt: the PR's own lines are formatted; the five rustfmt diffs reported in fsl-command/src/command.rs, clause.rs, map.rs, items.rs and tests/it/request_surface_s1.rs:96 are present on origin/main too, and the PR adds no foreign reformatting.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `check_markdown_document` and its call in `check_argv` use `fsl_replay::spine::check_native_fences`, `NativeFenceRequest`, `CheckLimits`, `OutcomeDocument::from_native_fences` and `from_native_fence_failure`, all `#[cfg(feature = "quire-extraction")]` in fsl-replay, but are not gated on fsl-command's own optional `quire-extraction` feature. The rest of command.rs gates its extraction paths, with a `cfg(not)` fallback at line 2696. Measured with `cargo check -p fsl-command --lib --no-default-features`: the base 60fa21b085 already fails with 1 error (output.rs:629 `extraction_original_regions`), and the head fails with 6, adding 5 new ungated uses. The workspace build hides this through feature unification. | fsl-command/src/command.rs:2356-2369; fsl-command/src/command.rs:2404-2483; fsl-command/Cargo.toml:11-13 |
| FND-002 | medium | The rewritten argv grammar adds no test for any refusal: `--markdown` without the two labels, an unknown `--flag`, a repeated `--markdown`/`--format`/`--extensions`, `--format`/`--extensions` with no value, `--format text`, a wrong positional count. The legacy-form change also has no test: `check --extensions <f> <source>` with one positional was refused and is now accepted. The refusal message `check_argv_refusal` changed text and no test pins it. | fsl-command/src/command.rs:2291-2329; fsl-command/src/command.rs:2394-2400 |
| FND-003 | medium | The new public `OutcomeDocument::from_native_fence_failure` has five arms (Refused, RefusedWithPayload, Limit/LimitWithPayload, Cancelled, Fault) and no test reaches any of them, at either the fsl-replay or the command layer. The whole-document refusal's category, `last_stage` (None for Refused, the limit's stage for Limit) and diagnostic shape are unpinned. | fsl-replay/src/outcome.rs:2963-2996 |
| FND-004 | low | `--format` and `--extensions` consume the next token unconditionally. `check --extensions --markdown a d spec.md` reads a file named `--markdown` and fails as an I/O error rather than an argv refusal. A trailing `--format`, or one given twice, falls to the generic `argv` refusal, which does not name the offending flag (version_argv names the member). | fsl-command/src/command.rs:2302-2316 |
| FND-005 | low | The new admit detail uses the literal `--model <identity>=<file>` placeholder. The removed check helper substituted the missing identity (`--model example/config-version=<file>`), so on `fsl check` the message is less specific than before. The same producer also serves run and replay requests, whose refusal now says `in a compile request's models member`. | fsl-semantics/src/model/intake.rs:1230-1233 |

## Verdict

CONDITIONAL. FR-866-AC-3 is met: the compile-path message now names the identity, `--model` and `` `models` ``. Removing `check_missing_model_help` loses nothing on the check path. The `selection` `model`, its hint and `last_stage` I1 come from `from_check` → `from_compile_refusal_diagnostics_for_operation`, and fsl-command check_imports and selection_text were run to confirm this. The guidance is no longer appended twice. The argv parser is correct on its main paths, and the three un-ignored request_surface_s1 tests pass with unchanged assertions. FND-002 and FND-003 need refusal and failure-arm tests in this PR. FND-001 is a cheap fix in the same PR: gate `check_markdown_document` and the `--markdown` branch on `quire-extraction`, with a refusal fallback. The no-default-features build was already broken before this PR.

## Scope and evidence

- fsl-command/src/command.rs check_argv (examined): argv parsing loop, positional match, format refusal, markdown branch.
- fsl-command/src/command.rs check_markdown_document (examined).
- fsl-command/src/command.rs check_outcome_inner select failure (examined): check_missing_model_help removal.
- fsl-replay/src/outcome.rs from_native_fence_failure (examined).
- fsl-semantics/src/model/intake.rs admit MissingSelection detail and its unit test (examined).
- tests/it/request_surface_s1.rs three un-ignored tests (examined): only the `#[ignore]` lines are removed; assertions are byte-identical.
- tests/it/check_imports.rs a_missing_model_names_the_identity_and_the_inputs_that_supply_it (examined): only `#[ignore]` removed. It discriminates through the backticked `` `models` `` the hint text lacks.
- Repo-wide grep for the removed help text (`supply --model`, `compile request's models member`): no test or spec example pinned it.
