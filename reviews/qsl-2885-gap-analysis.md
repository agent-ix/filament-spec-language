---
id: SR-3595002
title: "gap-analysis of PR 3595 replay-mode enabledness against FR-895"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-895, FR-891-AC-2, FSpec FR-059, FR-737, FR-052, ADR-027 CI-1/CI-3; fsl-analyze replay-mode tests at dac87bebc84fd670e32aba5fcd13e8996a9d11eb"
review_set: subset
---

## Summary

Ticket: QSL-2885. Planless gap audit of the PR's criteria: FR-895-AC-1..AC-7, FR-891-AC-2, FSpec FR-059-AC-4/AC-5,
FR-737-AC-1/AC-2. Plan completion: not assessed. Every touched test carries a trace to a criterion it asserts;
no new `#[ignore]` was added (the one remaining was re-reasoned). The `quoin matrix` step was not run;
binding checks were done by reading the trace attributes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | SPEC GAP: FR-895's Enabledness row ("enabled in the protocol state that the earlier observations reach, by ADR-027's step rules") does not define "earlier" over the recording's partial causal order, nor how a sequence predecessor owned by another role, or of a non-message kind, that the recording's causal order does not relate to the observation is judged. The engine's own-role send/receive reading is the coder's interpretation | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:70 |
| FND-002 | medium | SPEC GAP: FR-895-AC-2 says the misordered obligation names "observations 3 and 0" but not the witness order; the engine emits [receive, send] = [0, 3] for violations and [send, receive] for satisfied obligations, the passing protocol_recording test asserts [0, 3], the ignored replay test asserts [3, 0] | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:144 |
| FND-003 | medium | The new enabledness rule has one test, the violated side with no `Got` at all (AC-5). No test asserts the satisfied side for a role with an earlier own step (a full told_protocol recording settles `proved`) or the violated side where the earlier own step exists but follows in local order | fsl-analyze/tests/it/protocol_replay_mode.rs:205-230 |
| FND-004 | high | FR-895 "Result" and AC-6 require batch and incremental assessment to give equal verdicts, but FSL has no incremental assessment surface (no request member, no API in fsl-analyze protocol_analysis), so the clause has no implementation for a test to exercise | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:148 |

## Verdict

Coverage of the repaired tests matches their criteria and assertions stay exact. Open gaps: the
enabledness definition and witness order need spec rulings (FND-001, FND-002), the new rule lacks
accepted-side and precedence tests (FND-003), and incremental assessment exists only in the spec (FND-004).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-895 Enabledness row now defines predecessors and causal precedence per the arch ruling; repeat wording defects are SR-3595003 findings |
| FND-002 | fixed | FR-895 states witness [receiving, sending] for every causal-order verdict; AC-2 names [0, 3]; engine emits it for satisfied verdicts too |
| FND-003 | fixed | a_send_after_its_earlier_own_receive_is_enabled (satisfied side) and two precedence tests added |
| FND-004 | fixed | FR-896 `assessment` member and FR-895 "Incremental assessment" specified and implemented; monotonicity defect is SR-3595001 FND-008 |

Disposition pass 2 (head 69344ffc97b7a3ed1fb556e85fa2eab765d48ee7): no findings of this artifact remain open; all four were fixed in round 1.
