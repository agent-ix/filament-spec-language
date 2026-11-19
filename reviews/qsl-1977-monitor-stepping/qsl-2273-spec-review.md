---
id: SR-2026111608570003
title: "Spec review of filament-driver PR 160 FR-023 and TC-037"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-driver; spec/functional/FR-023-supply-clock-configurations-with-clocks.md, spec/test-cases/TC-037-supply-clocks-from-the-command-line.md, spec/spec.md, spec/tests.md at bbaf6586845972aa7825757e3917cd56a28222de; language FR-640 at origin/main 45705f2 and FR-640-AC-13/14 in the pending W5 batch f38affa"
review_set: subset
evaluated_revision: "bbaf6586845972aa7825757e3917cd56a28222de"
relationships:
  - target: ix://agent-ix/filament-driver/FR-023
    type: reviews
  - target: ix://agent-ix/filament-driver/TC-037
    type: reviews
---

## Summary

Ticket: QSL-2273. Spec review, with the integrity and EARS sub-analyses, of the new driver FR-023 (6 ACs) and TC-037 against language FR-640 (the `--clocks` contract, lines 158-162, 170-180 and 197-202) and the W5 batch's FR-640-AC-13 and AC-14. `quire validate` on FR-023, TC-037 and tests.md reports two EARS warnings on FR-023 line 43 and the module's duplicate-archetype notices, with no errors. FR-023 and TC-037 are indexed in spec.md.

FR-023 matches FR-640 on the driver's half: `io-error` for an unreadable file, `invalid-request` naming `--clocks` for a non-object, the members read as the request member, the both-given refusal left to FSL, and the replay-document obligation recorded with no code path. FR-023 does not cite QSL-2292 for that obligation.

## Verdict

REQUEST CHANGES. FR-023-AC-5 drops the stage its own Behavior bullet and FR-640 require. The size limit is stated only as "the request byte limit", with no AC. tests.md misplaces the TC-037 row and cites the wrong test file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-023-AC-5 says the both-given request "refuses `invalid-request` (FSL's refusal), naming `clocks` and `--clocks`, exit 20". It leaves out "at stage `request`", which FR-023's Behavior bullet (lines 45-47), FR-640 (lines 158-162) and FR-640-AC-14 all state. The AC was written to what FSL emits today rather than to the contract. The closing bullet (lines 55-56), "A refusal records no stage because none was reached", reads as covering this refusal too, and contradicts lines 45-47. Fix: add "at stage `request`" to AC-5, and mark it partial until the language fix (code-review SR FND-001) lands. | spec/functional/FR-023-supply-clock-configurations-with-clocks.md:66 |
| FND-002 | medium | The `clocks-file-too-large` bullet says "over the request byte limit" but gives no AC. It does not say which limit applies per verb: `run` follows `--request-bytes`, while `compile` has no such flag and uses FSL's fixed 1,048,576 bytes. It also gives `invalid-request` where FSL gives an oversize request `resource_exhausted` (code-review SR FND-002). Fix: state the bound for each verb and the code, and add an AC. | spec/functional/FR-023-supply-clock-configurations-with-clocks.md:53-54 |
| FND-003 | low | tests.md puts the TC-037 row (line 132) in the Functional Requirement Coverage table, not the test-case table beside TC-034 to TC-036 (lines 167-169). The FR-023 row (line 133) cites `fsl-cli/tests/frontend_cli.rs`, but the tests are in `fsl-cli/tests/clocks_flag.rs`. It has no status marker and does not name QSL-2292 for the replay gap. Fix: move the TC-037 row, cite `clocks_flag.rs`, add a status (🚧 while AC-5 is partial), and name QSL-2292 here and in FR-023's replay bullet. | spec/tests.md:132-133 |
| FND-004 | low | quire reports `ears:unclassifiable` and `ears:missing-subject` on FR-023 line 43. The not-an-object bullet (lines 42-44) makes "A `--clocks` file that is not UTF-8 JSON ..." the subject of SHALL, not the system. The other refusal bullets have the same shape. Fix: name the system, for example "If the `--clocks` file is not one UTF-8 JSON object, `fsl` SHALL refuse it with code `invalid-request` ...". | spec/functional/FR-023-supply-clock-configurations-with-clocks.md:42-44 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | rejected | The premise is withdrawn with the code-review SR FND-001. FR-640's "stage `request`" is the refusal envelope's stage, and the outcome document's `last_stage` is null for request-intake refusals (FSL FR-286, FR-287-AC-11). At 8a514fa, FR-023-AC-5 states `last_stage` null citing FR-286 and FR-287-AC-11. The Behavior bullet explains the envelope versus outcome distinction, so the closing bullet no longer contradicts it. |
| FND-002 | fixed | At 8a514fa, the size bullet states `resource_exhausted` (incomplete, exit 22), naming `--clocks` and the limit, with `--request-bytes` on `run` and FSL's fixed 1,048,576 bytes on `compile`. New FR-023-AC-7 covers it. |
| FND-003 | fixed | tests.md now has the TC-037 row in the test-case table after TC-036. The FR-023 row cites `fsl-cli/tests/clocks_flag.rs`, carries 🚧, covers AC-1..AC-7, and names QSL-2292. FR-023's replay bullet names QSL-2292 too. |
| FND-004 | fixed | The refusal bullets now read "If the `--clocks` file is ..., `fsl` SHALL refuse it ...". `quire validate` on FR-023, TC-037 and tests.md reports no EARS warning. |

## Verdict (disposition pass 1)

APPROVE at 8a514fa. FND-001 is rejected (premise withdrawn), and FND-002 to FND-004 are fixed. No new findings.
