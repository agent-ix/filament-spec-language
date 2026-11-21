---
id: SR-202611210022442313144108502
title: "Code review — QSL-2158 PR #3274 FR-202 checked-subject undefined entry (AC-24 to AC-26)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3274 diff origin/main...96d6bd53e6 (base 23e65543a3): fsl-analyze/src/analyze/exact.rs, fsl-analyze/src/analyze/exact/undefined.rs, fsl-analyze/src/exact_probabilistic/build.rs, fsl-analyze/tests/it/exact_probabilistic_item.rs, fsl-eval/src/automaton/qualified_bounded.rs, fsl-eval/src/automaton/translate.rs, fsl-replay/src/lib.rs, fsl-replay/src/outcome.rs, fsl-replay/src/outcome/counterexample.rs, fsl-replay/src/probability_certificate/witness.rs, fsl-replay/src/probability_certificate/witness/bounded.rs, fsl-replay/src/probability_certificate/witness/undefined.rs"
review_set: subset
evaluated_revision: "96d6bd53e684c02790411e699ea759f99d20d017"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-202
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-196
    type: references
---

## Summary

Ticket: QSL-2158. This is a static review of PR #3274 at the frozen head `96d6bd53e6`, diff `origin/main...HEAD`: 12 files and 4 implementation commits. Scope is FR-202's checked-subject `Undefined` entry (AC-24, AC-25, AC-26) and the existing analyzer undefined path that now feeds it. FR-1041, FR-1042 and FR-1043 are out of scope, as the brief says.

The reviewer ran no cargo command. The only local evidence is the coder's single exact test (`an_undefined_evaluation_settles_refuted_whatever_the_threshold`, 1 passed), which ran before the last commit. The remote focused TEST-RUN (20 tests) was still pending when this review was written. `rustfmt --check --edition 2021` on the changed files reports no diff in any changed hunk. The two `build.rs` diffs it reports, at lines 12 and 275, are outside this PR's hunks.

What holds up:

- **Binding.** The replay entry binds the package by pointer identity and the item by `bind_claim`. It refuses timed subjects. It checks the `over` binding against the evidence and refuses a workload claim that carries scheduler entries.
- **Path re-execution.** Each step re-executes with a positive exact probability.
- **Under every scheduler**, each step takes the scheduler entry at its complete key. A repeated key settles `ReplayParity`, and an unused scheduler entry is rejected. Under a workload, a repeated key is accepted.
- **At the read.** The first undefined reading, in clause-node order (From before Until, matching ADR-018 UE-1/UE-2 and the builder's `plan.rs` reads), must match `where` exactly: position, state key, locus, cause and pre-read product key.
- **Cancellation** is polled before the token is built.
- **Fairness.** The entry correctly applies no fairness check, per UE-5 and UE-3.

## Verdict

**FAIL — fix round needed.** 3 high, 3 medium and 3 low findings.

- **FND-001** widens a shared monitor beyond the undefined slice and leaves the newly reachable verdicts untested.
- **FND-002 and FND-003** are in-repo duplications. The code-review rubric records duplication at `high`, and FND-003 has already drifted.

The core undefined-replay logic is sound for the forms it admits.

## Rust review

| Check | Result |
| --- | --- |
| Behaviour vs spec | FR-202's checked-subject undefined bullets are implemented for untimed origin bounded event-position forms and infinite-trace reach forms. The widened `QualifiedBoundedMonitor` also admits defined `always[0,h]` verdicts (FND-001). |
| Oracle integrity | No test was weakened or removed. One `#[ignore]` was lifted, and its test now asserts real behaviour. |
| Test fidelity | The decided-control half of AC-24 cannot fail (FND-004). The replay's own work and expansion limits are untested (FND-006). |
| Errors/API | Producer materialization faults collapse into replay `Refused` (FND-005). A copied error mapping diverges from its source (FND-003). |
| Safety/concurrency/wire | No unsafe code and no concurrency. Indexing at `witness/undefined.rs:239,307` is guarded by `position >= 1 && position <= steps.len()`. `serde_json` `IndexMut` in `probabilistic_undefined_wire` writes into an object produced by `undefined()`. |
| Gates | No cargo, clippy or fmt gate was run, by brief. `rustfmt --check` passes on the changed hunks. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Making `QualifiedBoundedMonitor::supports` and `new` accept `BoundedKind::Always` removes the eventually-only gate for all callers, not just the undefined entry. A defined `probability >= T [ always[0,h] holds(P) ]` claim used to return `not_yet` at `exact.rs:948`. It now reaches `decide_bounded_event`, certificate production, the fsl-replay bounded certificate checker, `bounded_fair`, and `witness/bounded.rs` refutation replay, with no test of any defined always path. For example, an `AtMost` always refutation whose accepting path ends at `Open{0}` on a non-terminal state would be returned as Parity by `bounded.rs:264-277`. That may be correct, but nothing shows it. Either keep the old gate on the defined path, or add focused proved and refuted tests for always. | fsl-eval/src/automaton/qualified_bounded.rs:40-46; fsl-eval/src/automaton/qualified_bounded.rs:84-90; fsl-analyze/src/analyze/exact.rs:948 |
| FND-002 | high | The new replay copies the step re-execution core of `witness/bounded.rs` instead of sharing it: the ordered scheduler parse, `model_distributions` validation, workload and action candidate collection, the unique-successor match, the positive-mass check, and the action and transition storage reservation. A fix to one copy, such as `bounded.rs`'s `check_bits(step_mass)` or its `checked_add` edge count, will not reach the other. I searched `witness.rs` and `witness/bounded.rs` for an existing shared helper and found only `visit`, `retain` and `arithmetic`. Extract one step function and call it from both replays. | fsl-replay/src/probability_certificate/witness/undefined.rs:98-118; fsl-replay/src/probability_certificate/witness/undefined.rs:325-424; fsl-replay/src/probability_certificate/witness/bounded.rs:67-79; fsl-replay/src/probability_certificate/witness/bounded.rs:142-262 |
| FND-003 | high | `fsl-analyze`'s new `document_error` copies `fsl-replay`'s `bounded::document_error` and has already diverged. The analyzer copy maps `Accounting` to an `EvaluationMeter` limit. The replay copy, which the new replay uses for every pre-read and post-read key, maps `Accounting` to `Refused`. If key construction exhausts its accounting meter during replay, the item settles `ReplayRefused`/`invalid_runtime_input` instead of `incomplete`, contrary to FR-202's "exhaustion ... SHALL remain incomplete". Use one mapping owned by fsl-replay, for example `From<ProbabilityDocumentError> for ProbabilityWitnessError`. | fsl-analyze/src/analyze/exact/undefined.rs:108-127; fsl-replay/src/probability_certificate/witness/bounded.rs:415-428; fsl-replay/src/probability_certificate/witness/undefined.rs:152-159 |
| FND-004 | medium | AC-24's "monitor rejects at position 0, forged path settles `ReplayParity`" check cannot fail. The forged witness only swaps `path.item` to `Decided` and keeps `Open`'s product keys. Those keys carry the claim occurrence in their monitor context, so line 309 returns Parity on `&key != offered_key` whether or not the decided checks exist. Deleting the `decided` disjunct at :309, and the open-control guard at :136-141, would leave the test green. Re-key the forged path under `Decided`'s own monitor, and add a case that decides after position 0 with matching keys. | fsl-analyze/tests/it/exact_probabilistic_item.rs:727-733; fsl-replay/src/probability_certificate/witness/undefined.rs:136-141; fsl-replay/src/probability_certificate/witness/undefined.rs:279-283; fsl-replay/src/probability_certificate/witness/undefined.rs:309 |
| FND-005 | medium | In `materialize`, producer-side faults (no matching or ambiguous builder successor, multiple initials, a `from_monitor` mismatch, `state_key` failure, `Schedule::of`) all become `ProbabilityWitnessError::Refused`. `failure` then settles them as `ExactReport::ReplayRefused`, which is `inconclusive` with `invalid_runtime_input`, although no replay has run. The existing producer code reports engine invariants as `broken()`. Return a distinct producer fault from `materialize` and settle it as `broken()`. | fsl-analyze/src/analyze/exact/undefined.rs:96-98; fsl-analyze/src/analyze/exact/undefined.rs:138; fsl-analyze/src/analyze/exact/undefined.rs:204-232; fsl-analyze/src/analyze/exact/undefined.rs:270-296 |
| FND-006 | medium | AC-26 and TC-3153 step 4 ask for exhaustion of each expansion, work and storage limit with no token. The analyzer-level `work` (`work_units = 0`) and `expansion` (`max_candidates = 0`) cases stop in the producer build before the replay runs. Direct replay calls cover only states, transitions, storage, paths and cancellation. Nothing shows that the replay's own meter (`bounded::visit`) or `actions_with_cancel`/`weighted_steps_with_cancel` exhaustion settles incomplete. Add direct `replay_probabilistic_undefined` cases with a work meter and an expansion limit that the producer survives. | fsl-analyze/tests/it/exact_probabilistic_item.rs:876-899; fsl-replay/src/probability_certificate/witness/undefined.rs:97; fsl-replay/src/probability_certificate/witness/undefined.rs:351-367 |
| FND-007 | low | Every budgeted run builds `undefined_request` up front, cloning `simulation` and `proof_bounds`, even when no undefined value occurs. When the budgeted build hits an undefined reading, the whole product is explored a second time by the legacy builder without the candidate key admission. Construct the request only inside the `Undefined` arm, and note why the second unbudgeted exploration is bounded. | fsl-analyze/src/analyze/exact.rs:973-992 |
| FND-008 | low | `bounded_deadline` now also projects a pending `Extent` (always) counter, but its doc comment still says "pending deadline of a single-interval eventuality". The invariant names `qualified-eventually-*` in it and in `QualifiedBoundedMonitor` are now wrong for always. | fsl-eval/src/automaton/translate.rs:168-186; fsl-eval/src/automaton/qualified_bounded.rs:58-73 |
| FND-009 | low | `wrong_item` at :847 is the matching `Work` item, and the `is_some()` assertion with it is the positive case. The names are inverted, so the reader sees a "wrong item accepted" assertion. Rename it, for example to `own_item`. | fsl-analyze/tests/it/exact_probabilistic_item.rs:847-862 |

## Requested focused probes (parent-owned remote runs)

1. `exact_probabilistic_item::undefined_replay` and `exact_probabilistic_item::an_undefined_evaluation_settles_refuted_whatever_the_threshold` at the frozen head (already queued).
2. After FND-004's fix, the re-keyed `Decided` case, with the `decided` disjunct temporarily removed locally, to show the test fails.
3. For FND-001, an analyzer-level defined `probability >= 0.5 [ always[0,1] holds(..) ]` item over a model where it is proved, and one where it is refuted, through `exact_probabilistic_item`, confirming the certificate and refutation replay settle as expected.
