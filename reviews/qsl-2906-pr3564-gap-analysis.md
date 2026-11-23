---
id: SR-599011
title: "Gap analysis — QSL-2906 PR #3564 request-aware limit advice"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; git diff origin/main...6a5b910c8, FR-255-AC-2, TC-720 step 2"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: references
---

## Summary

Ticket: QSL-2906. Planless gap analysis of PR #3564 at head 6a5b910c8, scoped to the diff and the criterion it implements, FR-255-AC-2 (TC-720 step 2, ADR-030 D-3). Matrix source: `quire matrix --scope . --format json` (quire 0.52.1, engine 0.62.2), exit 0. FR-255-AC-2 is `tagged`, with binders `a_native_run_limit_names_the_cli_option_and_no_request_entry`, `a_replay_limit_also_names_the_stage_limits_entry` (fsl-foundation), `run_limit_message_omits_stage_limits_while_replay_keeps_it` (fsl-replay outcome), and the pre-existing `tests/it/lifecycle_boundaries.rs::the_fifth_expression_node_names_its_s3_setting_and_source_locus` plus two clause tests. The matrix's `unmarked-trace-id` claims for `RequestKind`, `LimitExceeded::render` and `recompile_text` are doc-comment citations of FR-255-AC-2, so the new code has an owning requirement (no reverse gap). No stubs or placeholder returns in the diff.

## Verdict

**CONDITIONAL.** No untagged criterion and no high finding. The criterion is tagged and its two message tails are asserted exactly, but the AC's full example is not: no test asserts the exact message with `at <locus>` for the five-node `f`, and none drives that limit through a native-run/1 request and a replay request as TC-720 step 2 requires (FND-001). One pre-existing trace binding on the extended test is wrong (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-255-AC-2's full example is not asserted: the exact-string tests use a hand-built locus-less `LimitExceeded`, and the end-to-end five-node test checks only `contains` on the request-neutral `Display`; no test renders the real `f` limit with its locus through a native-run/1 request and a replay request | fsl-foundation/src/diagnostic/stage.rs:421-449; tests/it/lifecycle_boundaries.rs:136-170; fsl-replay/src/outcome.rs:3755-3788 |
| FND-002 | low | `run_limit_message_omits_stage_limits_while_replay_keeps_it` stays tagged FR-255-AC-3 (limits-type field mapping), which it does not exercise; the PR's added FR-255-AC-2 tag is the correct one | fsl-replay/src/outcome.rs:3755-3758 |

FND-001: TC-720 step 2 says "Check `f`, whose body is `1 + 1 + 1`, with `s3.nodes` at 4, once through a native-run/1 request and once through a replay request, and render each diagnostic", expecting "S3 node limit 4 reached (5) at <locus>; raise it with `--limit s3.nodes=<n>`" and the replay form. The closest test, `the_fifth_expression_node_names_its_s3_setting_and_source_locus`, has the real `f` and locus but asserts only `contains("S3 node limit 4 reached (5)")` and `contains("--limit s3.nodes=")`, which would still pass if the replay advice leaked in. Tightening that test to `assert_eq!` on `denied.render(RequestKind::NativeRun)` and `denied.render(RequestKind::Replay)` with the rendered locus closes most of the gap; the replay half ideally goes through the replay outcome document.

FND-002: FR-255-AC-3 is about every limits type mapping its fields to one setting name; this test renders limit messages. The FR-255-AC-3 binding predates the PR and was left in place when the tag was extended.

## Coverage

- FR-255-AC-2 — examined; tagged; FND-001.
- FR-255-AC-3 — context_only (binding on the extended test); FND-002.
- TC-720 step 2 and expected result — examined against the tests; FND-001.
- ADR-030 D-3 — context_only; the request-schema rule is what `RequestKind` encodes.
- Reverse gap over the diff: `RequestKind`, `RenderedLimit`, `LimitExceeded::render`, `recompile_text`, `for_operation` — each owned by FR-255-AC-2; clean.
- Stubs and coverage inflation over the diff: none.
- Semantic review: not run (not requested).
- Plan completion: not assessed

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `tests/it/lifecycle_boundaries.rs::the_fifth_node_message_is_request_aware_with_its_locus` checks the real five-node `f` at `s3.nodes` 4 and asserts both full messages, locus included, with `assert_eq!`; the replay outcome-document path is covered for root and library limits in `fsl-replay/src/outcome.rs` |
| FND-002 | fixed | `run_limit_message_omits_stage_limits_while_replay_keeps_it` now carries only `#[trace("TC-720", "FR-255-AC-2")]`; FR-255-AC-3 keeps its other binders |
