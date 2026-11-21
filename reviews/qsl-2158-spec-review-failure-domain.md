---
id: SR-564
title: spec-review/failure-domain review of QSL-2158 NotMarkov diagnostic
type: SpecReview
analysis: failure-domain
scope: agent-ix/filament-spec-language; QSL-2158 PR3336 diagnostic slice; spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-187
  type: references
---

## Summary

Frozen review head: c084d6bc10c7323acf1a0e18fdf2f640eb9c85ba; accepted base: 80270d8dcae0a4c389df03183fc9bee2f9edcc61. Ticket: QSL-2158; PR: filament-spec-language#3336. This review covers the diagnostic slice and authorized restore.rs lint cleanup, not closure of QSL-2158. Native Codex judgment only; no external model calls. Source, specs, tests and branch were not modified by this reviewer.

No applicable AssuranceProfile is declared under this repository's spec tree (searched for AssuranceProfile); ordinary review policy applies. Installed tools actually used: quoin 0.36.8, quire 0.52.0 (engine 0.62.2), spec-artifacts-process@v0.36.0. Direct analyses, no enforced workflow required.

## Verdict

PASS for the scoped method. No scoped defect found; repository-wide completion is not asserted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Examined zero, one, two and more-than-two post-state topology and identity boundaries. Actual count 0/multiple evidence is retained, count1 cannot be exported by terminal construction, first-two order/distinctness is checked, and interleaved ordinary/random arguments preserve their true declared positions. Model StateKeys remain discrete owner identities; no product/clock identity is invented. Current digital filtering preserves unique model candidate order with one reset per complete transition, as source inspection confirms; no hypothetical future clock-only multiplicity is claimed supported.

Cancellation and work/storage refusals follow current caller admission and KeyScope.finish; malformed evidence is an internal invariant failure with its diagnostic preserved in EN4 and EN5/direct/planner boundaries. Restore task metadata stays immutable for the entire borrowed lifetime. Finite iteration takes at most two refusal evidence states; model exploration's existing budget remains authoritative. There are no new user callbacks, side effects or graph exploration loops to specify. The already assigned global shared reference conversion is QSL-2726, independently verified, and excluded from local codec duplication.

## Verification Evidence

Local read-only checks: git diff --check over accepted-base...HEAD exit 0; direct rustfmt --check --edition 2021 --config skip_children=true on all 24 changed Rust files exit 0; FR-187 Quire validation exit 0, 1/1 grammar-clean, no grammar findings. No CI workflow changes or weakened lints/gates. No local Cargo, compilation, runtime or Clippy execution by reviewer.

Remote evidence is reported in the dispatch brief, not relabeled as reviewer execution: final c084d6 seven-package --tests check exit 0 and narrow fsl-eval Clippy -D warnings exit 0; three exact focused selectors each selected 1/passed 1/failed 0/ignored 0: not_markov_box_admission_preserves_payload_and_caller_refusals, not_markov_retains_the_full_identity_with_interleaved_random_arguments, native_not_markov_preserves_actual_zero_two_and_four_post_payloads. Receipt logs are on titan at ~/arch-test-logs/pr3336-c084d6-c0.log,c5.log,c1.log,c2.log,c3.log; not independently opened from this local review session.

Historical evidence retains source attribution: old 7c4597a compiled and passed selectors 1-10 (owner identity/distinctness, sampled 0/2, malformed EN4/EN5/direct/planner, settlement controls and native EN4+EN5 0/2/4); its first digital selector failed before the engine and statistical selector was held. ded188bcce8782e7a18860d365d3da06f8f72bf4 compiled and passed the corrected 4-raw/2-filtered digital test, statistical budget control and two restore tests; its only narrow Clippy failure was large_enum_variant. c084d6 addresses that with admitted Box ownership and final green evidence above. No whole-workspace/whole-cause provider-schema/certificate campaign pass is claimed.

## Examined Units

The following is the exact machine-readable scope; clean units are listed deliberately.

```yaml
scope:
- id: FR-187-Outputs-Draw
  path: spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md
  role: examined
  excerpt: "pub struct Draw {\n    pub random: Vec<Value>,                     //\
    \ one per random parameter, declared order\n    pub probability: Rational,   \
    \               // product of the PM-1 probabilities\n    pub successors: Vec<(ModelTransition,\
    \ ModelSimState)>, // FR-120 post-states, canonical order\n    pub rewards: Vec<(Identifier,\
    \ Value)>,\n}"
- id: FR-187-Outputs-ScheduledIdentity-1
  path: spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md
  role: examined
  excerpt: "pub struct ScheduledIdentity {\n    pub operation: String,           \
    \ // the operation's declaration identity text, as ModelTransition::operation\
    \ carries it\n    pub receiver: ObjectReference,\n    pub arguments: Vec<Value>,\
    \        // one per non-random parameter, declared order\n}"
- id: FR-187-Outputs-Action-1
  path: spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md
  role: examined
  excerpt: "pub struct Action {\n    pub identity: ScheduledIdentity,\n    pub draws:\
    \ Vec<Draw>,                       // canonical order of random vectors\n}"
- id: FR-187-Outputs-NotMarkov-1
  path: spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md
  role: examined
  excerpt: "pub struct NotMarkov {\n    pub state: StateKey, // the canonical state\
    \ key (the RFC 8785 byte string of the state)\n    pub transition: ScheduledIdentity,\n\
    \    pub drawn_transition: ModelTransition, // the existing complete identity,\
    \ including random arguments in declared order\n    pub random: Vec<Value>,\n\
    \    pub post_states: u64, // the actual number of distinct model post-states,\
    \ checked without saturation\n    pub post_state_examples: Vec<StateKey>, // the\
    \ first min(post_states, 2) actual distinct keys in deterministic owner order\n\
    }"
- id: FR-187-Outputs-Unsteppable
  path: spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md
  role: examined
  excerpt: pub enum Unsteppable { NotMarkov(Box<NotMarkov>), UndecidedSuccessor(UndecidedSuccessor)
    }
- id: FR-1043-AC-9-part-1
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: AC-5's exact workload mean `5/12` at `>= 5/12` and `<= 5/12`, a valid workload
    `upper` bound `1/2` at `>= 1/2`, and a valid `lower` bound `2/5` at `<= 2/5` each
    reject `ThresholdMissed` at the initial key and settle `inconclusive`, `CertificateRejected`.
    In AC-8, a valid `lower` minimum document with initial bound `1/5` at `>= 1/2`
    and a valid `upper` maximum document with bound `1/2` at `<= 1/3` each reject
    `Malformed` at `kind` with the same settlement. Identity/base mismatch retains
    its typed refusal; an invalid value rejects `InequalityFail
- id: FR-1043-AC-9-part-2
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: s` with `CertificateRejected`; a disagreeing witness policy settles `inconclusive`,
    `ReplayParity`. A raw engine verdict or unchecked certificate cannot settle `refuted`.
    An accepted partial `lower` maximum bound `2/5 > 1/3`, with unnamed successors
    counted as 0, refutes without an upper counterpart; cancellation or a live checking
    limit remains incomplete.
- id: FR-187-AC-3
  path: spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md
  role: examined
  excerpt: "A variant of `Health` (ADR-024 \xA77.1) whose `tick` postcondition leaves\
    \ a second Boolean field unconstrained: `weighted_steps` at the initial state\
    \ returns `NotMarkov` naming the state key, `tick`, the drawn `fault` value and\
    \ 2 post-states; `actions` returns one action whose draws each carry two successors.\
    \ A variant whose postcondition is unsatisfiable for `fault = true` returns `NotMarkov`\
    \ with 0 post-states from both."
- id: FR-192-AC-1-part-1
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: 'Each outcome row maps exactly as the table states: an Accepted, a Rejected
    and an Undecided (`IndifferenceRegion`) `Completed` outcome; a `Stopped` naming
    `max_samples`, settling `Incomplete(LimitReached{limit, value, setting})` with
    setting `statistical.max_samples`; an `Unsupported` outcome with cause `NotMarkov`;
    a `NoTest` refusal settling `inconclusive`, `no-initial-state`; a `ConfidenceBeyondPrecision`
    refusal settling `unsupported`, `confidence-beyond-precision`; an `Internal` refusal
    settling `failed`; a `Stopped` by an expansion naming'
- id: FR-192-AC-1-part-2
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: ' `EvaluationMeter` with exhausted counter `work_units`, under a meter
    budget whose `work_units` is 4096, settling `incomplete`, `limit-reached` with
    limit `work_units`, value 4096 and setting the `work_units` counter''s FR-255
    accounting setting name, and one naming `value_occurrences` under a budget whose
    `value_occurrences` is 7 settling with limit `value_occurrences`, value 7 and
    that counter''s setting name, and one naming `MaxCandidates` under `max_candidates`
    2 settling with limit `max_candidates`, value 2 and setting `expansion.max_candida'
- id: FR-192-AC-1-part-3
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: tes`, each equal to the record FR-203 writes for the same stop (each the
    name FR-255 registers for it, which the settings operation accepts to raise that
    bound; no setting `meter` and no bare `max_candidates`), one by an internal-failure
    expansion naming no limit settling `failed`, `runtime_invariant`/`established-invariant-broken`,
    and one by a non-internal expansion naming no limit settling `failed` with the
    stop's own `ExpansionStop.cause` (FSL FR-120) as its one diagnostic. None of the
    measured, stopped and unsupported records carries an FR
- id: FR-192-AC-1-part-4
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: -242 truth or an FR-243 basis.
- id: FR-203-AC-7-part-1
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: The build-result map is exhaustive over `ProductRefusal` and `ClauseOutcome`
    with no `_` arm. A build whose expansion stops at `max_candidates` 2 settles `incomplete`,
    `Incomplete(LimitReached{max_candidates, 2, expansion.max_candidates})`. A build
    whose `ProductRefusal`, asserted before settling, is `Expansion(stop)` naming
    `EvaluationMeter` (a subject whose successor expansion charges the meter before
    the claim reads any predicate, under an accounting bound `B` of `work_units` its
    first expansion exceeds) settles `Incomplete(LimitReached{work
- id: FR-203-AC-7-part-2
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: _units, B, accounting.work_units})`, naming the exhausted counter, `B`
    and the setting `accounting.<counter>` by FR-255 Behavior 9's rule (FSpec FR-461-AC-5);
    a build whose `ProductRefusal`, asserted before settling, is `PredicateRefused(Incomplete(i))`
    (the expansion fits under `B`, the predicate read's charge is the first one denied)
    settles the same way for `i`'s counter. A predicate refused `integer_out_of_domain`/`outside-domain`
    settles `failed` with that diagnostic; a constructed `PredicateRefused(NotBoolean)`,
    `PredicateRefused(KernelRe
- id: FR-203-AC-7-part-3
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: fused(CheckedInvariant))` and `Translation` error each settle `failed`,
    `runtime_invariant`/`established-invariant-broken`; an `Admission` failure and
    a `RequiresBound` refusal return no terminal record.
- id: FR-204-AC-3-part-1
  path: spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md
  role: examined
  excerpt: '`Retx` with `delay Msg::send ~ discrete { 1 ms: 1, 2 ms: 1 };`, under
    a workload `One` with weight 1 on `send`, is a DTMC over digital timed states;
    `probability >= 0.99 [eventually[0 ms, 4 ms] holds(m.delivered)]` settles `proved`,
    `ExactValue{159129/160000}`: after every reset `send` waits 1 or 2 ms with probability
    1/2 each, both inside its window `[1, 2]`, loses with `1/10`, and attempts at
    time stamps up to 4 ms inclusive count. The same claim `under every scheduler`
    is decided with the distribution applied (FR-204-AC-15); with `delay Msg:'
- id: FR-204-AC-3-part-2
  path: spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md
  role: examined
  excerpt: :send ~ exponential(1 per ms);` under `One` and exact evidence requested
    it settles `unsupported`, `DelayDistribution`, naming `send`.
- id: FR-741-AC-1
  path: spec/functional/FR-741-request-and-report-probability-items-through-analyze.md
  role: examined
  excerpt: 'Over FR-185-AC-1''s `Service` compiled from source, the items `{"claim":
    "P95", "probabilistic": {"evidence": "statistical", "method": {"type": T}, "seed":
    "7"}}` for `T` each of `okamoto`, `sprt` and `clopper-pearson`, and `{"claim":
    "LongRun", "probabilistic": {"evidence": "statistical", "method": {"type": "regenerative",
    "min_cycles": 100}, "seed": "7"}}`, each settle `measured` with a `verdict` naming
    the method and seed, and the same request gives equal item results with `workers`
    1 and 4; each witness it reports replays.'
- id: FR-741-AC-3
  path: spec/functional/FR-741-request-and-report-probability-items-through-analyze.md
  role: examined
  excerpt: An exact item for a claim whose exact value is below its threshold settles
    `refuted` with a `counterexample` and an item-level `replay` that is present and
    reproduced; FSpec FR-421-AC-1's timed claim settles through digital clocks with
    an accepted certificate; FR-203-AC-5's undefined claim settles `refuted` with
    an FSpec FR-413 `undefined-path` `counterexample` whose `replay` is present and
    reproduced, at either threshold.
```
