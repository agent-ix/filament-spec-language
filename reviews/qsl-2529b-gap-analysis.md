---
id: SR-2026111919340478491681420
title: "gap-analysis of QSL-2529 activation-state exact replay and QSL-2604 flow order"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2529, QSL-2604; PR3155 head 5fa6e0026eb527cfe3290746113dc951136bb73e; FR-450 :149-157, :204-273, FR-450-AC-12; FR-750-AC-12 (a); FR-1019 :44, FR-1019-AC-14, FR-1019-AC-15; TC-1221, TC-3037, TC-2869; W72; fsl-replay/src/hybrid_replay/**; fsl-analyze/tests/it/{hybrid_provider.rs,hybrid_requests.rs}"
review_set: subset
---

## Summary

Ticket: QSL-2529 (activation-state half) and QSL-2604, PR filament-spec-language#3155. This is a diff-scoped check that each acceptance criterion has a test and each requirement has code, for FR-450 "Exact replay" as amended by #3143, FR-450-AC-12, FR-750-AC-12 (a) and the FR-1019 flow-order rule. Plan completion: not assessed. I did not compute a whole-repository matrix.

## Verdict

CONDITIONAL. These criteria are covered by tests:
- FR-450-AC-12 (a) replay half: kind, cause, `where.position` 0 and `where.time` 1/2.
- FR-450-AC-12 (b) and (c).
- FR-450-AC-12 (d) replay half.
- FR-750-AC-12 (a) at both layers.

Every expectation is hand-derivable, and I re-derived them. Mutation shows the past-reach period count is load-bearing. Untested: FR-450-AC-12 (a)'s provider `where.time` settlement entries and (d)'s settlement.

FR-1019-AC-15 has no test, and as written it cannot be realised under FR-447's variable order. FR-450 :270-273 and W72 allow no claim form to settle `NotExact`. `ActivationInfimum` is a tracked exception (QSL-2690). The nested-window wrong answer (code-review SR-2026111919340478491681419 FND-001) is an untracked gap that the PR body does not mention.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-450-AC-12 (a)'s provider half has no test. A `refuted` entry with `where.time` `[1/2, 1/2]` should settle `refuted`, and one with `[3/4, 3/4]` should give `NotReproduced` at check 5 (TC-1221 step 14). (d)'s "settles `refuted`" is also untested. | fsl-replay/src/hybrid_replay/tests/activation_tests.rs:83-101, fsl-replay/src/hybrid_replay/tests/activation_tests.rs:170-182 |
| FND-002 | medium | FR-1019-AC-15 (added by #3143) has no test, and as written it cannot be realised. FR-447 orders a model's variables by first appearance as an equation's left side. In AC-15's one-mode model, which writes `a'` before `v'`, the variables are therefore declared `a` then `v`. "Declared `v` then `a`" cannot be written, because the hybrid grammar has no other variable declaration. The PR marks QSL-2604 done without flagging this. | fsl-analyze/tests/it/hybrid_provider.rs:539-573 |
| FND-003 | medium | The PR body frames `ActivationInfimum` as the only remaining activation-half gap. A nested window over tau-dependent atoms returns a wrong `NotReproduced` (code-review FND-001). That is neither `NotExact` nor ticketed, which breaks W72's rule that every gap is tracked. | fsl-replay/src/hybrid_replay/exact/activation.rs:129-175 |
| FND-004 | low | The FR-750-AC-12 (a) integration test declares the capture as `Rational[0, 2; 1, 2]` where the AC says `Rational`. It asserts neither "one counterexample" nor the replay row `reproduced-with-evaluated-witness`. | fsl-analyze/tests/it/hybrid_requests.rs:1141-1146, fsl-analyze/tests/it/hybrid_requests.rs:1232-1233 |

## Dispositions

Round 1, head 83965af6ab28295a586474584ef2beeefd16736c.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Two provider-half tests were added. `a_provider_entry_is_settled_by_the_reproduced_infimum`: an entry with `where.time` [1/2, 1/2] settles refuted, and one with [3/4, 3/4] settles inconclusive with replay-parity. `a_past_reach_of_two_periods_settles_refuted`: (d) settles refuted. The (a) fixture uses the capture `tau + 3/2` in `[0, 2]` instead of the AC's `tau + 1/2` in `[0, 1]`; the out-of-range set is the same, (1/2, 1), with infimum 1/2, so this is accepted. |
| FND-002 | deferred | FR-1019-AC-15 cannot be built under FR-447 (first appearance as an equation's left side) because the grammar has no other variable declaration. This is blocked on an arch amendment, which the coordinator is raising. No test was invented, which is correct. |
| FND-003 | still-open | The untracked wrong answer is gone, because nested windows are fixed (code-review FND-001). The PR body was not updated: it still cites "Remaining work: QSL-2529" for ActivationInfimum (it should be QSL-2690), still describes the gap's second trigger as "an atom with no value at an instant that moves with tau" (the code raises it whether or not the instant moves), does not say that FR-1019-AC-15 is blocked on arch, and still reports the old test counts (70 / 9 instead of 72 / 11). This is a PR-body edit only, no code. |
| FND-004 | fixed | The test now asserts `counterexample.kind == formula` and the replay row `settlement == reproduced-with-evaluated-witness`. Its doc records that the checker requires a range on a capture's Rational type, which explains the `Rational[0, 2; 1, 2]` fixture. |
