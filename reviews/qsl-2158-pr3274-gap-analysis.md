---
id: SR-202611210022442313144108503
title: "Gap analysis — QSL-2158 PR #3274 FR-202 checked-subject undefined entry"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-202-AC-5, FR-202-AC-24..26, TC-3153; PR #3274 diff origin/main...96d6bd53e6 source and tests"
review_set: subset
evaluated_revision: "96d6bd53e684c02790411e699ea759f99d20d017"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-202
    type: references
---

## Summary

Ticket: QSL-2158. This is a planless gap audit of PR #3274 at `96d6bd53e6`, covering FR-202's checked-subject `Undefined` entry (AC-24 to AC-26, TC-3153) and the existing undefined criterion AC-5. The matrix was computed with `quire matrix --scope . --format json` (quire 0.52.0, engine 0.62.2; quoin 0.36.8).

- AC-24, AC-25 and AC-26 are `tagged`.
- FR-202-AC-5 is `untagged`.

The reverse-gap and stub checks were done by reading the source. No tests were run by the reviewer. The remote focused run was pending.

## Verdict

**FAIL.** One in-scope criterion (AC-5) is untagged, and one high reverse gap exists: the defined `always[0,h]` path was newly admitted with no owning test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Reverse gap. Widening `QualifiedBoundedMonitor` to `always[0,h]` admits defined bounded-always probability verdicts (certificate proof, bounded-fair, bounded witness refutation). Before this PR they returned `not_yet`. No criterion in this slice owns them, and no test exercises them. Same root as code-review SR-202611210022442313144108502 FND-001. | fsl-eval/src/automaton/qualified_bounded.rs:40-46; fsl-analyze/src/analyze/exact.rs:948 |
| FND-002 | medium | Hollow evidence for AC-24's second half. The forged `Decided` path keeps `Open`'s keys, so Parity comes from the key mismatch and never from the decided-control check. The test cannot fail if that control is removed. | fsl-analyze/tests/it/exact_probabilistic_item.rs:727-733 |
| FND-003 | medium | AC-26's limit branch is only partly evidenced. The analyzer-level `work` and `expansion` cases stop in the producer, not in the replay. The replay's own work-meter and expansion exhaustion have no test. | fsl-analyze/tests/it/exact_probabilistic_item.rs:876-899 |
| FND-004 | medium | FR-202-AC-5 (the `where` 1 path is refuted, `where` 0 gives `ReplayParity`) is untagged in the computed matrix. `undefined_replay_checks_first_read_location_control_and_package_ownership` and `an_undefined_evaluation_settles_refuted_whatever_the_threshold` assert exactly that behaviour, but neither carries the AC-5 tag. | fsl-analyze/tests/it/exact_probabilistic_item.rs:630-655; fsl-analyze/tests/it/exact_probabilistic_item.rs:673-700 |
| FND-005 | low | Wrong trace binding. `undefined_replay_checks_first_read_location_control_and_package_ownership` is tagged FR-202-AC-26, but it checks only bounded `always` claims, item and package ownership, and `where` mutations. It asserts nothing of AC-26: no infinite-trace claim, no Even repeat, no cancellation. AC-26 stays covered by the other two tests. | fsl-analyze/tests/it/exact_probabilistic_item.rs:673 |
| FND-006 | low | The FR-202 writer bullet says the `undefined-path` wire SHALL write `where.product_state.key` as the complete pre-read key. No test reads `wire["evidence"]["undefined"]["where"]["product_state"]["key"]` against `product_state`. The tests check only `evidence.type`. | fsl-replay/src/outcome/counterexample.rs:2038-2063; fsl-analyze/tests/it/exact_probabilistic_item.rs:657-671 |
| FND-007 | low | AC-25's refusal code (`invalid_runtime_input`/`invalid-value`) is asserted only as the typed `ProbabilityWitnessError::Refused` at the replay entry. No settled record is checked. The mapping exists at `exact.rs:141-147`, but no test exercises it for this path. | fsl-analyze/tests/it/exact_probabilistic_item.rs:839-846 |

## Coverage

| Criterion | Matrix status | Binders | Judgement |
| --- | --- | --- | --- |
| FR-202-AC-5 | untagged | none | Behaviour is asserted but not bound (FND-004). |
| FR-202-AC-24 | tagged | `an_undefined_evaluation_settles_refuted_whatever_the_threshold`, `undefined_replay_checks_first_read_location_control_and_package_ownership` | The first half is sound. The decided half is hollow (FND-002). |
| FR-202-AC-25 | tagged | `undefined_replay_distinguishes_workload_repeated_keys_and_scheduler_choices` | Refuted under every scheduler with `keep` (via `undefined_evidence("Bounded")`), and the wrong `keep` entry is refused. Binding is correct. |
| FR-202-AC-26 | tagged | all three new tests | The null monitor and accumulator, the Even `1/4` repeat, the every-scheduler repeat Parity, and pre-cancellation and final-poll cancellation are evidenced. Work and expansion are partial (FND-003). One binding is wrong (FND-005). |

Plan completion: not assessed

Semantic review: not run as a separate fan-out. Intent, test and code judgements come from the same reviewer's code-review pass (SR-202611210022442313144108502).
