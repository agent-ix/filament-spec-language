---
id: SR-2026111608570002
title: "Gap analysis of filament-driver PR 160 QSL-2273 acceptance criteria"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-driver; spec/functional/FR-023-*.md, spec/test-cases/TC-037-*.md, fsl-cli/tests/clocks_flag.rs, fsl-cli/src/clocks.rs, fsl-cli/src/cli.rs at bbaf6586845972aa7825757e3917cd56a28222de"
review_set: subset
evaluated_revision: "bbaf6586845972aa7825757e3917cd56a28222de"
relationships:
  - target: ix://agent-ix/filament-driver/FR-023
    type: reviews
---

## Summary

Ticket: QSL-2273. A manual check of each AC against its tests, plus `quoin matrix --json`. The matrix shows FR-023-AC-1 to AC-6 each tagged by one test in `fsl-cli/tests/clocks_flag.rs` (static status `tagged`). All six pass. The ticket's two ACs are: (1) `fsl run --clocks` matches the request member; (2) one negative test per refusal, checking code, cause and stage.

The replay-document refusal (FR-640-AC-8's driver obligation) has no code path, because the driver has no replay verb. It is accepted as stated, pending QSL-2292, which exists in Backlog and names this obligation.

## Verdict

REQUEST CHANGES. Ticket AC 1 is met (`run_with_clocks_flag_matches_the_request_member` compares stdout and exit bytes, and shows the bare request fails). Ticket AC 2 is not fully met. The unreadable and not-an-object refusals check code, cause and stage. The both-given refusal checks neither cause nor stage. The too-large refusal has no test. The member-refusal test does not compare with the request member, which its AC says it should.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The both-given test checks code, exit code and message only. It asserts no `last_stage` and no `cause`, although the ticket AC requires code, cause and stage, and FR-023's own Behavior bullet says "at stage `request`". Today the assertion would fail because FSL emits null (code-review SR FND-001). Fix: once the language fix lands and the lock is bumped, assert `last_stage == "request"` on `run` and `compile`. Until then, record the AC as partial in tests.md, naming the language ticket. | fsl-cli/tests/clocks_flag.rs:185-206 |
| FND-002 | medium | `clocks-file-too-large` has no test, and no FR-023 AC covers it. The ticket AC asks for one negative test per refusal. Fix: add an AC and a test (for example, `fsl run --request-bytes 30 --clocks <64-byte file>`) checking code, cause, stage null and exit, plus the `compile` bound. | fsl-cli/src/clocks.rs:45-50, spec/functional/FR-023-supply-clock-configurations-with-clocks.md:53-54 |
| FND-003 | medium | FR-023-AC-6 says a member FSL refuses in the file "refuses as the same member in the request does". The test never runs the request-member case. It asserts only code `invalid-request` and a disjunctive substring (`plant-ms` or `clocks`), which any clocks-related refusal satisfies. Its stage is not checked either. Fix: for the unknown kind, run the same object as the request's `clocks` member, and compare code, cause, `last_stage` and the message up to its ` at line` position suffix; assert the repeated-name message names `plant-ms`. | fsl-cli/tests/clocks_flag.rs:212-243 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | At 8a514fa, `clocks_in_the_request_and_on_the_command_line_refuse` asserts `last_stage` null on `run` and `compile`, which FR-286 and FR-287-AC-11 require (the stage `request` this finding asked for was withdrawn; see the code-review SR FND-001). The cause is FSL's null and is not asserted separately. |
| FND-002 | fixed | New FR-023-AC-7 and `an_oversize_clocks_file_refuses_resource_exhausted` check exit 22, `resource_exhausted`, cause `clocks-file-too-large`, `last_stage` null, and a message naming `--clocks`, 30 and `--request-bytes` on `run`, plus the 1,048,576-byte bound on `compile`. |
| FND-003 | fixed | `the_file_members_are_read_as_request_members` now runs the unknown-kind object as the request member too, and compares code, cause, `last_stage` and the message up to ` at line`. The repeated-name case asserts the message names `plant-ms`. |

## Verdict (disposition pass 1)

APPROVE at 8a514fa. All three findings are fixed. FR-023-AC-1 to AC-7 are tagged and pass (`clocks_flag`: 7 passed). The replay-document refusal stays accepted, pending QSL-2292, which FR-023 and tests.md now name.
