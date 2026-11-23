---
id: SR-599010
title: "Code review — QSL-2906 PR #3564 request-aware limit advice"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; git diff origin/main...6a5b910c8 (fsl-foundation/src/diagnostic.rs, fsl-foundation/src/diagnostic/stage.rs, fsl-replay/src/execute.rs, fsl-replay/src/outcome.rs)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: references
---

## Summary

Ticket: QSL-2906. Code review with the Rust lane (rust-review, rust-style; repository AGENTS.md/CLAUDE.md applied) of PR #3564 at head 6a5b910c8. The PR adds `RequestKind { NativeRun, Replay }` and `LimitExceeded::render(RequestKind) -> RenderedLimit`, changes `Display for LimitExceeded` to the request-neutral CLI-only form, renders replay recompile refusals through `recompile_text` (Replay advice), and replaces the `split_once(" or the request's `stage_limits` entry ")` string hack in `OutcomeDiagnostic::for_operation` with a typed match. Governing spec: ADR-030 D-3, FR-255-AC-2, TC-720 (merged in FSL PR #3527).

## Verdict

**CONDITIONAL — mergeable once FND-001 is fixed (a few lines) or explicitly deferred.** The message strings match FR-255-AC-2 and TC-720 exactly: native-run/1 renders "S3 node limit 4 reached (5)[ at <locus>]; raise it with `--limit s3.nodes=<n>`" and replay appends " or the replay request's `stage_limits` entry `s3.nodes`"; the locus comes before the advice as the AC orders it. The string-splitting hack is gone with no residue (no `split_once` on the advice text anywhere). Keeping `Display for LimitExceeded` is acceptable and is not a compatibility layer: it is the native-run/1, request-neutral rendering, which the spec makes the default for every request without `stage_limits` (check, package, execute, analyze, monitor, artifact reads), and it fixes paths the old hack never reached (for example `RunCause::Spine` diagnostics and `checked_v2` limit messages, which previously leaked `stage_limits` advice to non-replay callers). Its only weakness is a duplicated advice string (FND-002). The changed tests assert exact strings, carry `#[trace("TC-720", "FR-255-AC-2")]`, and are stronger than the `contains` checks they replace. Focused tests pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Replay loses the `stage_limits` advice when the limit was reached compiling a library: `recompile_text` matches only a top-level `CompileRefusal::Limit`, so `Recompile { Dependency { Limit } }` renders through the CLI-only `Display` | fsl-replay/src/execute.rs:153-160; fsl-replay/src/spine.rs:444-451; fsl-replay/src/spine/lifecycle.rs:4724-4738; fsl-replay/src/outcome.rs:2588-2593 |
| FND-002 | low | `Display for LimitExceeded` writes the `--limit` advice itself instead of delegating to `render(RequestKind::NativeRun)`; the advice text has two sources kept equal only by one test assertion | fsl-foundation/src/diagnostic/stage.rs:238,279-287 |
| FND-003 | low | The test whose `#[trace]` the PR extended keeps a deprecated `/// Trace: FR-255-AC-3` doc line that now disagrees with its tag list | fsl-replay/src/outcome.rs:3755-3756 |

FND-001: before the PR every replay limit message named `stage_limits`, because `LimitExceeded`'s `Display` always did and replay never stripped it. Now only a top-level limit gets Replay advice. A library compiled during replay is wrapped by `lifecycle::wrap` as `CompileRefusal::Dependency { path, refusal: Limit }` (limits are not closure-level), and `#[error("the library {} refused: {refusal}")]` renders the inner limit with the CLI-only `Display`. `execute.rs` already treats this nesting as a real replay case (`replay_input_limit` and `external_package_bound` recurse through `Dependency`). The same limit therefore gives different advice depending on whether the root unit or a library reached it. FR-255-AC-2 says replay output "may" name the entry, so this is a regression and an inconsistency rather than an AC violation. Fix: make `recompile_text` (or a small helper shared with `for_operation`) render a `Dependency` chain whose leaf is `Limit` with `RequestKind::Replay`, keeping the `the library … refused:` prefix, plus one test.

FND-002: `RenderedLimit::fmt` and `LimitExceeded::fmt` each format "; raise it with `--limit {name}=<n>`". Have `Display for LimitExceeded` call `self.render(RequestKind::NativeRun).fmt(formatter)` so the advice has one source (rust-style: one source of truth).

FND-003: the line reads "Trace: FR-255-AC-3, the operation hint follows the entry point's accepted inputs." while the tag now also names TC-720/FR-255-AC-2. `/// Trace:` doc lines are deprecated in favour of `#[trace]`; drop it or reword it as plain doc text.

## Gates

- `cargo test -p fsl-foundation --lib limit`: 3 passed, exit 0 (both new TC-720 tests).
- `cargo test -p fsl-replay --lib run_limit_message_omits_stage_limits_while_replay_keeps_it`: 1 passed, exit 0.
- `rustfmt --check` on the changed files: the PR's hunks are clean; the only diffs reported (`execute.rs:2526` `replay_input_limit` signature, `outcome.rs:29`) are outside the diff and pre-existing.
- `cargo clippy -p fsl-foundation -p fsl-replay --lib --tests -- -D warnings`: did not reach the changed crates; it fails first in the dependency `fsl-forms` (`large_enum_variant` at `fsl-forms/src/syntax.rs:4629`), which this PR does not touch. Pre-existing; not attributed to this PR.
- No CI workflow files changed.

## Coverage

- `fsl-foundation/src/diagnostic/stage.rs`: `RequestKind`, `RenderedLimit`, `LimitExceeded::render`, `write_facts`, `Display for LimitExceeded`, both new tests — examined; FND-002.
- `fsl-foundation/src/diagnostic.rs`: re-exports — examined, clean.
- `fsl-replay/src/execute.rs`: `recompile_text`, the `Recompile` and `RecompileWithProtocolResults` error formats — examined; FND-001.
- `fsl-replay/src/outcome.rs`: `for_operation`, its callers (`from_failure`, `from_stopped`, `run_refusal_document`, `from_replay_refusal_with_context`), the extended test — examined; FND-001 (related), FND-003.
- Call sites outside the diff read for context: `CompileRefusal` variants and `wrap` in `fsl-replay/src/spine*.rs`, model-check replay (`fsl-replay/src/model_check/replay.rs:909`, top-level `Limit`, gets Replay advice), `fsl-command` refusal documents, `fsl-package/src/checked_v2.rs` limit messages.
- FR-255-AC-2, TC-720 step 2, ADR-030 D-3 wording — examined against the rendered strings; clean.
