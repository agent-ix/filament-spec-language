---
id: SR-2026112520400003
title: "Gap analysis — filament-spec-language QSL-3403 replay-wire settings"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-replay/src/request.rs, fsl-command/src/command/limits.rs, fsl-foundation/src/setting.rs, fsl-replay/src/limits.rs, fsl-replay/src/proof_result/plugin_tests.rs, fsl-replay/tests/it/stage_limit_named_root_only.rs, tests/it/replay_request_file.rs, tests/it/producer_unit_identity.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-255", type: references }
---

## Summary

Ticket: QSL-3403. PR #3969 at head `512f1536a56fa260548122ced17857baa330f648`. Scoped to the criteria the PR changes or tags: FR-255-AC-14, AC-20, AC-24; FR-263-AC-3; FR-290-AC-14; FR-399-AC-4, AC-5. The trace index is a grep of `#[trace]`/`#[implements]` tags at this head (`quire trace` over the whole repository did not finish in 120 s and was stopped).

## Verdict

**CONDITIONAL** — one low finding. Every examined criterion has a tagged test; no stub or tautology.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-255-AC-14's new sentence and TC-1250 step 5 say a replay request's `smt_encoding.max_constants` entry "refuses at decode"; the AC-14-tagged test asserts it through `with_named` only. The decode refusal is asserted by `replay_wire_refuses_every_setting_for_an_operation_it_does_not_run`, tagged FR-255-AC-24, not AC-14. Same check, so low. | fsl-replay/src/limits.rs:907-913, fsl-replay/src/request.rs:1391 |

## Coverage

- Tagged tests: FR-255-AC-14 (limits.rs), AC-20 (output_limit.rs, limits.rs, plugin_tests.rs new), AC-24 (12 files incl. new fsl-command test and replay_request_file.rs), FR-263-AC-3 (bounds.rs), FR-290-AC-14 (plugin_tests.rs), FR-399-AC-4/AC-5 (stage_limit_named_root_only.rs).
- Reverse gap: no new production function; changed predicates are owned by FR-255 "Lifecycle caller-limit coverage" and now tagged.
- Semantic review: not run as a separate pass (opt-in); rulings judged in the spec-review SR.
- Plan completion: not assessed
