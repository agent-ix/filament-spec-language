---
id: SR-563
title: spec-review/integrity review of QSL-2158 NotMarkov diagnostic
type: SpecReview
analysis: integrity
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

CONDITIONAL for the scoped method. One low documentation mismatch requires disposition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-187 Outputs still declares Draw without the required transition: ModelTransition field added at probabilistic.rs:82. A consumer following this public type example cannot construct the new Draw, and the retained zero-post draw identity is absent from the stated output contract. Add the field with its complete declared-argument-order meaning to the existing Draw declaration. | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md:56 |

## Coverage

Checked public structure consistency and the edited count/examples obligations against the actual source and accepted FSpec bca4311 FR-331 cause contract. Zero count implies empty evidence; count >=2 requires first two distinct canonical model keys; count1 is excluded. Public native and provider envelopes remain separately owned. The amendment changes neither US/FR dependency edges nor external tool lookup/credential/pagination/concurrency responsibilities.

One low structural completeness mismatch exists in the Outputs block: public Draw gained transition in code, while the adjacent declaration still lists only random/probability/successors/rewards. Add the actual complete identity field to that existing declaration. The bounded evidence and Box ownership declarations otherwise agree with implementation. This is the same correction recorded by base/code-review; there is no second count/identity defect.

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
- id: FR-187-AC-3
  path: spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md
  role: examined
  excerpt: "A variant of `Health` (ADR-024 \xA77.1) whose `tick` postcondition leaves\
    \ a second Boolean field unconstrained: `weighted_steps` at the initial state\
    \ returns `NotMarkov` naming the state key, `tick`, the drawn `fault` value and\
    \ 2 post-states; `actions` returns one action whose draws each carry two successors.\
    \ A variant whose postcondition is unsatisfiable for `fault = true` returns `NotMarkov`\
    \ with 0 post-states from both."
```
