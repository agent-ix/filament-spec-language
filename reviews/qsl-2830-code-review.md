---
id: SR-2026112402190
title: "code-review of PR3636 fair every-scheduler eventual refutation and FS-9 replay"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/analyze/exact.rs, fsl-analyze/src/exact_probabilistic.rs, fsl-analyze/src/exact_probabilistic/witness.rs, fsl-analyze/tests/it/package_probabilistic_refutation.rs, fsl-replay/src/outcome/counterexample.rs, fsl-replay/src/probability_certificate/witness.rs; PR3636 f6b1a23f1e6c9d5b3ae279aa2233b14f27cb61ac"
review_set: subset
---

## Summary

Ticket: QSL-2830 (part 2, partial QSL-2828). Code review with the rust-review lane folded in. The analyzer admits unbounded eventual claims under every scheduler with a fairness set, solves them with the fair reachability solver, and on an acyclic subject refutes with a memoryless randomized witness. Replay now checks fairness itself by walking the chain the witness induces and testing each bottom component. The FR-413 randomized wire entry matches the spec shape. The replay fairness walk over-approximates draws with several post-states, which can hide an unfair bottom component. Fair-solver limit errors are reported as tool failures. No test exercises a failing fairness check.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | witness_is_fair unions every post-state of a draw instead of following the witness's PostState choice or refusing; the over-approximated chain can make an unfair bottom component non-closed, so replay accepts an unfair witness | fsl-replay/src/probability_certificate/witness.rs:1145-1163 |
| FND-002 | medium | decide_fair_reachability errors all map to broken(); policy-iteration and rational-bit limits settle as a tool failure instead of the limit outcome unbounded_failure gives | fsl-analyze/src/analyze/exact.rs:1316-1321 |
| FND-003 | medium | No test drives witness_is_fair to false or through a non-trivial bottom component, and no test spells a randomized entry with identity alternatives; on acyclic subjects the check is vacuously true | fsl-replay/src/probability_certificate/witness.rs:1061-1240 |
| FND-004 | low | ProducedRandomizedWitness doc says steps carry probability without the choice weight; produce_randomized_witness multiplies the weight into each step | fsl-analyze/src/exact_probabilistic/witness.rs:855-857 |
| FND-005 | low | The fairness walk is not charged to the replay meter or storage; bottom_components gets a no-op tick, unlike its sibling callers | fsl-replay/src/probability_certificate/witness.rs:1178 |
| FND-006 | low | replay_fair_refutation copies replay_refutation's O(states x edges) terminal-leaf scan and producer-budget closure instead of sharing them | fsl-analyze/src/analyze/exact.rs:2675-2720 |

## Verdict

Not mergeable: FND-001 (high) and FND-002/FND-003 (medium) need fixing.

Checked clean:

- The test bound change is a correction, not a weakening. In link.native, send_a loses with 1/10 and send_b with 1/5, with at most two attempts. The fair maximum is 1-(1/10)^2 = 99/100 (send_a twice), so `<= 99/100` holds at equality and could never be refuted. With `<= 98/100`, only all-send_a refutes, because the mixed schedule gives exactly 49/50. The bound is tight.
- every_fairness_preserves_source_package_refutation_parity now asserts refuted/exit 10. The fair minimum is 24/25 < 99/100, and fairness is vacuous at the terminal states (FSL FR-200-AC-4). This strengthens the test.
- Wire: `{"state", "choice": {"type": "randomized", "choices": [{"choice", "probability"}]}}` matches FR-413 (filament-specification). Identity alternatives are spelled as scheduled identities, matching how deterministic entries are spelled. The ordinal mapping holds because exact.rs pushes spelled rows in distribution order.
- The weak, strong, whole and each conditions in the bottom-component test are right for a memoryless randomized chain. Weak means enabled at every member and taken at none. Strong means enabled at some member and taken at none. Cutting at decisive leaves is sound because a fair continuation always exists in a finite MDP.
- Objective selection is Min for `>=` and Max for `<=`. Holding fair claims return not_yet, as stated in the PR.
- The BFS terminates: each key is enqueued once, and every key must exist in the checker graph, which is already bounded by max_states.
- rustfmt --check passes on every changed production file. The 22 rustfmt diffs in the test file are already present on origin/main.
- The code reads as compile-plausible: Induced::new() through the type alias, impl Into for ProducedRandomizedWitness, Option::is_none_or on toolchain 1.98, and BTreeMap<&K,_>::get(&K).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | randomized_scheduler_wire_entry's identity-alternative spelling (ordinal into spelled rows) still has no test; the residual part of FND-003, judged non-blocking because a misspelled wire entry fails closed on re-replay and the replay verdict reads the typed witness, not the wire | fsl-replay/src/outcome/counterexample.rs:2196-2212 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | b68a7fbc: zero-probability draws skipped; draw.successors.len() != 1 returns UnsupportedDraw (fail closed), matching path replay at line 913 |
| FND-002 | fixed | b68a7fbc: FairProductError::Product(error) routed through unbounded_failure |
| FND-003 | fixed | b68a7fbc: bottom_components_are_fair split out and covered by four fair_component_tests (non-trivial unfair BSCC under weak and strong, whole and each; vacuous and taken cases; escaping component), plus an integration test reaching replay-parity; identity-spelling residual recorded as FND-007 |
| FND-004 | fixed | b68a7fbc: doc now states that steps include the choice weight |
| FND-005 | fixed | b68a7fbc: per-node bounded::visit and bounded::retain; bottom_components and per-component ticks charge the meter |
| FND-006 | accepted-no-change | The leaf scan is extracted into reach_accepting and shared by both refutation paths. The remaining duplication is the roughly 15-line producer-budget control closure: cosmetic, with no behaviour or budget risk |
