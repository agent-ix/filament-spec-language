---
id: SR-2026112013172122021003265
title: "Code review of QSL-2156 PR #3204 timed digital certificates"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2156; PR #3204 head 85ff4ce2e72ebebcf220671e0ab96f43fc48d49a; 25 changed files across fsl-analyze and fsl-replay"
review_set: subset
---
# Code review: QSL-2156 PR #3204

## Summary

Ticket: QSL-2156. Read-only static review of the frozen PR diff against `origin/main` (25 files, 6,034 additions and 463 deletions). No Rust build or tests were run, per the lead's build-lane hold. `git diff --check` passed. No CI workflow or dependency pin changes are present. The reviewed Rust paths have no unsafe code in the relevant changes, and the checker reconstructs timed products independently as FR-921 requires.

FR-924's zero-time component handling now ignores zero-mass edges when selecting an exit route. The region checker still returns `Unsupported` for a pending quiescent state instead of admitting the specified time-lock terminal. The focused test/build gates remain outstanding.

## Verdict

CONDITIONAL. One medium implementation finding remains; focused checks are pending the explicit build release.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The region-route checker refuses a pending quiescent state, although FR-924 requires the checker to classify such states as time-lock terminals. | fsl-replay/src/probability_certificate/document/timed.rs:1683-1689 |

## Scope examined

- FR-924-AC-1 through FR-924-AC-6: timed-product producer/checker tests and component value/rank handling.
- FR-921-AC-1 through FR-921-AC-4: identity, route/scale, row coverage, and independent checker behavior.
- FR-922-AC-1 through FR-922-AC-3: strict region and boundary-region behavior.
- FR-923-AC-1 through FR-923-AC-3: discrete delays and scheduler ties.
- FR-877-AC-1 through FR-877-AC-4: timed quantile transforms.
- FR-198-AC-5: selected MEC exit policy and replay.

## Finding detail

**FND-001 — medium — quiescent states on the region route remain unsupported.**

`build_region_graph` returns `ProbabilityDocumentError::Unsupported` when `timed.is_quiescent(...)` is true at a pending state (`fsl-replay/src/probability_certificate/document/timed.rs:1683-1689`). FR-924 requires the checker to identify time-locks and treat a state with no admissible delay and no enabled discrete move as terminal. The current branch can therefore refuse a strict-clock/region certificate whose pending state is a time-lock instead of checking the terminal value/rank. Extend the checker-owned region product with the required time-lock transition/terminal treatment and a focused case when the build lane is released.

## Review limits

`cargo check` and targeted tests were not run because the lead has withheld the wave-1-steady build release. The matrix and traceability review is recorded separately in the gap-analysis artifact.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | At 2b16a60f, the pending region state is inserted with `terminal: choices.is_empty().then_some(false)`, so a quiescent state with no enabled delay or action becomes a nonaccepting terminal instead of returning Unsupported. |

## Disposition pass summary

At frozen head `2b16a60fef4b7382fecdf1f9148961c0baeaecfa`, the early quiescent-state refusal is removed. A pending state with no delay or enabled action is inserted as a terminal rejection (`terminal: choices.is_empty().then_some(false)`), matching FR-924's event time-lock value. No build or test was run, so test verification remains pending.

## New findings (disposition pass 2)

Pass 2 reviews the whole PR diff at head `b42bed4b90` (34 files, 6,443 additions, 541 deletions against `origin/main`), with the API adaptation after the rebase and the fix round (`e1b44611bb`, `b42bed4b90`). Static review only: no cargo build or test ran in this pass; the lead's focused log `/tmp/qsl2156-focused-3.log` (exit 0; fsl-replay `probability_certificate` 31/31; fsl-analyze `it` 156 passed, 0 failed, 13 ignored) is the test evidence.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | `check_exact_reachability` no longer rejects cyclic graphs. It collapses every closed end component to its best exit, also for untimed graphs, so the public untimed checker `check_claim_exact_reachability` accepts a forged every-scheduler minimum. Under FR-198 that minimum is 0 when the scheduler can stay in the component forever, and FR-201 keeps the unbounded subset acyclic. | fsl-replay/src/probability_certificate.rs:1473-1563 |
| FND-003 | medium | `run_timed` maps every `decide_bounded_event` error to `broken()`, which is `Refusal::Failed`. On the FR-924-AC-6 reset-loop model, the region product holds a positive-time, nonprogressive cycle, so bounded induction returns `FhError::Cycle` and the item reports an engine failure, not unsupported. The test's ignore reason describes this failure as missing scope. | fsl-analyze/src/analyze/exact.rs:318-322 |
| FND-004 | low | `collapse_zero_time_components` maps `UnboundedError::MissingAction` from `Graph::with_terminals` to `FhError::InvalidProbability`. The same function's policy-iteration mapping keeps `MissingAction`, so a real missing action at a nonterminal state is mislabelled. | fsl-analyze/src/exact_probabilistic/finite_horizon.rs:249-252 |
| FND-005 | low | Production paths use `unreachable!()` for invariants that only control flow enforces: the digital/region key split and the workload objective in the mean-time checker. Against rust-style, a future caller change panics instead of returning a typed error. | fsl-analyze/src/exact_probabilistic/digital.rs:1061; fsl-replay/src/probability_certificate/document/timed.rs:455 |

### Detail

**FND-002 (high, soundness).** On `origin/main`, `check_exact_reachability` began with `topological_order`, which returns `ExactProbabilityGraphError::Cycle` on any cycle. That error maps to `Unsupported`, and FR-201 requires it: "The unbounded subset SHALL retain its acyclic continuation graph restriction". The PR replaces the topological pass with `nonprogressive_end_components(graph, cancel, None)` and a local Bellman check against the *certified* values. `timed_key_progresses` returns `false` for an untimed key, so every closed cycle in an untimed graph counts as a "zero-time" end component, and its value is fixed at the minimum (or maximum) exit value. Must-exit is correct for FR-924 timed components, because staying is Zeno. It is wrong for untimed models: FR-198's own use case says that when "the scheduler can keep the coin unflipped forever, ... the minimum is 0".

Failure scenario: an untimed model has state A with two actions, `wait` (self-loop) and `flip` (to target T). The claim is `probability >= 0.9 [eventually T]` under every scheduler. The certificate {A: 1, T: 1} is a forgery, because the true minimum is 0. `check_claim_exact_reachability` returns `Accepted`, where `main` returned `Err(Cycle)`. The fix-round change to `exact_certificate_rejects_threshold_miss_and_cycles` (`probability_certificate.rs:2417-2447`) moved the only cycle assertion to `Workload` (`UnsupportedClaimForm`). The unit test `checker_reconstructs_action_level_zero_time_mec_best_exit` (`:2100`) uses untimed `key(n)` keys and asserts must-exit semantics, which coincides with the true answer only because that graph also has a 0-valued exit.

Fix: apply end-component collapse only to timed (deadline) graphs. One option is to pass a flag from the `timed.rs` caller. Another is to require a timed key on every component member. Keep `topological_order` (Cycle, so Unsupported) for untimed graphs. Add a unit case with a `wait` self-loop and a single exit to the target under `Minimum`, expecting `Err(Cycle)`.

**FND-003 (medium).** `reset_loop_zeno_values_are_rejected` is ignored "on observation". The spec'd FR-924-AC-6 model takes the region route, which this PR implements. The producer's zero-time collapse only removes zero-elapsed cycles, so the positive-elapsed reset loop reaches `induction`, which refuses with `FhError::Cycle`. `run_timed` turns that refusal into `broken()`. Users get a `Failed` refusal for a valid spec'd model, where an unsupported result is due until FR-924-AC-6's progressive-cycle rule lands. Fix: map `FhError::Cycle` to `not_yet()`, and keep `broken()` for real invariant faults. Confidence is medium: the analysis is traced statically, not run.

**FND-004 (low).** Fix-round item 3, `Graph::with_terminals`, is correct for its only caller. `terminal` holds exactly the values that `induction` treats as absorbing, and `with_terminals` still raises `MissingAction` for nonterminal states with no edges, so it does not mask a real missing action. The `_ => FhError::InvalidProbability` arm then mislabels that error. Map `MissingAction` to `FhError::MissingAction`, as the policy-iteration arm below it does.

**FND-005 (low).** `digital.rs:1061,1066,1114` and `timed.rs:455,462` panic when the invariant does not hold. Return a typed fault (`fault("...")` / `ProbabilityDocumentError::Unsupported`), or split the types so that the match cannot see the other variant.

### Fix-round items checked (no finding)

- Item 1, `WireObjective.timed` `skip_serializing_if` (`document.rs:340-341`): correct, and it matches the sibling optional members and the canonical writer (`writer.rs:311` writes `timed` only when present). A document with `"timed": null` re-serializes without the member and fails the canonical read-back, so it is rejected.
- Item 2, cycle test expectation (`probability_certificate.rs:2417-2447`): the new expectation matches the code, but the change removed the last untimed cycle guard. See FND-002.
- Item 4, `digital_clocks_of` (`fsl-analyze/tests/it/digital_clocks.rs:487-507`): it decodes `product[2][1] = ["digital", discrete, unit, scale, clocks, deadline, …]`, which matches `digital_probability_product_key` (clock rows `[identity, value, cap]`, deadline `["deadline", limit, elapsed]`). The tests that use it are single-clock (`clocks[0]`), so the identity sort order does not matter. A format drift panics through `expect`, never passes silently.
- Item 5, `Max <= 0.9999`: a legitimate oracle correction. Under GUARD (`x >= 1`, reset, loss 1/10, 4 ms window), the maximum is 4 attempts, 1 − 10⁻⁴ = 9999/10000. That is the value `zero_time_components_take_their_best_exit` asserts as *proved*, so the old `<= 0.999` bound would have refuted it. STRICT's maximum of 999/1000 still proves. The change also strengthens FR-922-AC-2: the forged 9999/10000 now meets the bound, so the rejection must come from `InequalityFails`, not from a threshold miss.
- Item 6, ignore reasons: the reasons for the workload-scheduler cases (`time_locked_state_is_certified_terminal`, the `Weighted` half of `every_scheduler_tie_is_the_schedulers_choice`) match `timed.rs`, which returns `Unsupported` for any non-`Every` scheduler. The digital mean-time case (`delay_cycles_are_not_collapsed`) matches `check_expected_elapsed`, which requires `Region`. The quantile cases match `run_timed`, which handles only `ProbForm::Probability`. For the refutation cases, no active test exercises a timed deadline refutation, which is consistent with the stated reason. `reset_loop_zeno_values_are_rejected` is FND-003.

## Disposition pass 2 summary

At head `b42bed4b90`, FND-001's fix still holds: the region checker (`enumerate_region_deadline`, `timed.rs:1803`; digital counterpart at `:1557`) inserts a pending state with no delay or action as a nonaccepting terminal. Its latest outcome is `fixed`, so no new row is due. FND-002 to FND-005 are new and have no outcome yet; they go back to the coder. FND-002 blocks merge: it is a soundness regression on a public checker API that `main` handled correctly.
