---
id: SR-560
title: code-review review of QSL-2158 NotMarkov diagnostic
type: SpecReview
analysis: code-review
scope: agent-ix/filament-spec-language; QSL-2158 PR3336 diagnostic slice; Cargo.lock,
  Cargo.toml, fsl-analyze/src/exact_probabilistic/build.rs, fsl-analyze/src/exact_probabilistic/digital.rs,
  fsl-analyze/src/exact_probabilistic/settle.rs, fsl-analyze/src/statistical/tests.rs,
  fsl-analyze/tests/it/digital_clocks.rs, fsl-analyze/tests/it/exact_probabilistic.rs,
  fsl-analyze/tests/it/probability_from_source.rs, fsl-eval/src/simulation/model/frames.rs,
  fsl-eval/src/simulation/model/probabilistic.rs, fsl-eval/src/simulation/model/probabilistic/tests.rs,
  fsl-eval/src/simulation/model/restore.rs, fsl-eval/src/simulation/sampled.rs, fsl-eval/src/simulation/sampled/tests.rs,
  fsl-replay/src/exact_probabilistic.rs, fsl-replay/src/outcome.rs, fsl-replay/src/planner/entry.rs,
  fsl-replay/src/probability_certificate.rs, fsl-replay/src/probability_certificate/document/long_run.rs,
  fsl-replay/src/probability_certificate/document/measure.rs, fsl-replay/src/probability_certificate/reward.rs,
  fsl-replay/src/proof_result.rs, fsl-replay/src/statistical/settle.rs, fsl-replay/src/statistical/witness.rs,
  spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md,
  tools/fsl-probability-check/src/main.rs
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

The complete ModelTransition is constructed before successor enumeration in frames::apply, moved through EnabledVector into Draw, then retained on 0/multiple-post refusals. Existing canonical state ordering is used and only the first two actual distinct keys are exported; usize-to-u64 conversion is checked and does not saturate. Candidate generation visits state assignments once and stops after the first satisfying result; draw sorting establishes canonical StateKey order. The explicit duplicate check refuses malformed evidence instead of inventing distinct keys.

KeyPolicy work(1) and admit(size_of::<NotMarkov>()) occur before Box::new; KeyScope.finish preserves caller denial and cancellation. All seven changed consuming match arms forward the existing box. Untimed key evidence uses reserve/key/compare through the caller policy; legacy no-policy entry points retain their existing policy. Sampling borrows the successor slice instead of cloning all refused posts. Digital EnabledDraw borrows the Local draw identity, filters actual posts in owner order with one existing reset per transition, then captures keys from the filtered subset. No synthetic clock/product keys are introduced.

EN4/EN5, exact build, long-run, reward, certificate and measure owners retain typed payloads. Both TerminalRecord constructors validate count!=1, exact bounded evidence length and strict ascending distinct keys. EN4 keeps the diagnostic produced by constructor validation. Existing native ItemCause serializes state, complete transition, count and examples; planner's closed member table names all four members. Existing TaskSourceTransitionWire is reused. Native record.cause.kind and provider cause.type are different envelopes.

The shared EffectiveId/raw ObjectId reference conversion remains assigned to package-consumer-api under active QSL-2726: verified issue In Progress and actual comment a7962ba9-aaa1-4e68-8861-6bbb08f31c47. Current TaskSourceTransitionWire still preserves real reference values; this patch does not mask them with null or claim provider schema positivity. The actual native test exercises write_run/analyze_document_routed through real source requests for EN4 and EN5 at counts 0/2/4; payload equality is compared to actual ModelSystem output and independent Boolean argument/order/member assertions. Malformed synthetic records are limited to adverse constructor tests, not substituted for public source runs.

Restore's queued tasks borrow immutable admitted metadata from expected/system type registries whose lifetime encloses the work stack. Only completed owned values clone required type metadata. Vec capacity doubling's saturating_mul is equivalent to the prior checked_mul/usize::MAX fallback; byte multiplication remains checked and reservation occurs before try_reserve_exact. as_chunks::<2> rejects odd lengths; UTF-8/radix conversion rejects invalid hex. rust-version=1.98 supports this API. Existing provenance, noninitial successor parity, actual caller-work stops, cancellation and storage refusal assertions remain intact. No new Box or deep clone was introduced in restore.

Searched changed helpers, wire carriers and surrounding owner code for duplicates, stubs, TODO/todo!/unimplemented!, skipped tests and warning suppression; no introduced duplicate owner, live stub, hollow assertion or gate weakening found. The analogous existing private test fixtures are local fixtures, not a new production codec or imported schema. No async/Tokio task changes apply.

Current origin/main at inspection: 5f3800021f6c9ea002dd219145bf38689fa5af23. Independently examined accepted #3334/49f9e7a18c and #3340/9943d97241 overlap: walk_of/ExpansionSelection defaults retain FirstResult; AllResults is relation-specific; recommendation collection exits before post-state enumeration, separate from EnabledVector. Read-only git merge-tree using actual base/main/head produced no conflict markers. GitHub reports c084d6 MERGEABLE. No semantic incompatibility or speculative mandatory rebase was identified.

## Verification Evidence

Local read-only checks: git diff --check over accepted-base...HEAD exit 0; direct rustfmt --check --edition 2021 --config skip_children=true on all 24 changed Rust files exit 0; FR-187 Quire validation exit 0, 1/1 grammar-clean, no grammar findings. No CI workflow changes or weakened lints/gates. No local Cargo, compilation, runtime or Clippy execution by reviewer.

Remote evidence is reported in the dispatch brief, not relabeled as reviewer execution: final c084d6 seven-package --tests check exit 0 and narrow fsl-eval Clippy -D warnings exit 0; three exact focused selectors each selected 1/passed 1/failed 0/ignored 0: not_markov_box_admission_preserves_payload_and_caller_refusals, not_markov_retains_the_full_identity_with_interleaved_random_arguments, native_not_markov_preserves_actual_zero_two_and_four_post_payloads. Receipt logs are on titan at ~/arch-test-logs/pr3336-c084d6-c0.log,c5.log,c1.log,c2.log,c3.log; not independently opened from this local review session.

Historical evidence retains source attribution: old 7c4597a compiled and passed selectors 1-10 (owner identity/distinctness, sampled 0/2, malformed EN4/EN5/direct/planner, settlement controls and native EN4+EN5 0/2/4); its first digital selector failed before the engine and statistical selector was held. ded188bcce8782e7a18860d365d3da06f8f72bf4 compiled and passed the corrected 4-raw/2-filtered digital test, statistical budget control and two restore tests; its only narrow Clippy failure was large_enum_variant. c084d6 addresses that with admitted Box ownership and final green evidence above. No whole-workspace/whole-cause provider-schema/certificate campaign pass is claimed.

## Examined Units

The following is the exact machine-readable scope; clean units are listed deliberately.

```yaml
scope:
- id: Cargo.lock
  path: Cargo.lock
  role: examined
  excerpt: "source = \"git+https://github.com/agent-ix/filament-specification?rev=bca4311db3a4d9d15c6e4e8f9878ce3a3b9a60b1#bca4311db3a4d9d15c6e4e8f9878ce3a3b9a60b1\"\
    \n\n[[package]]\nname = \"filament-verification-contracts\"\nversion = \"0.2.1\"\
    \nsource = \"git+https://github.com/agent-ix/filament-verification-contracts?rev=cd0c8ef67e5d4263ce12773831cedd771246e5e5#cd0c8ef67e5d4263ce12773831cedd771246e5e5\"\
    \ndependencies = [\n \"jsonschema 0.55.1\",\n \"quire-canonical\",\n \"regress\"\
    ,\n \"schemars\",\n \"serde\",\n \"serde_json\",\n \"sha2\",\n \"thiserror\",\n\
    \ \"time\",\n \"typify\",\n]\n\n[[package]"
- id: Cargo.toml
  path: Cargo.toml
  role: examined
  excerpt: 'filament-specification-qualification = { git = "https://github.com/agent-ix/filament-specification",
    rev = "bca4311db3a4d9d15c6e4e8f9878ce3a3b9a60b1" }

    filament-specification = { package = "filament-specification-qualification", git
    = "https://github.com/agent-ix/filament-specification", rev = "bca4311db3a4d9d15c6e4e8f9878ce3a3b9a60b1"
    }

    # `default-features = false`: the encoder builds `no_std` + `alloc`, so the

    # layer-SV leaf `filament-semantic-value` can mint the compound-unit id

    # through it; a crate that writes through `io::Write` turns on'
- id: fsl-analyze/src/exact_probabilistic/build.rs
  path: fsl-analyze/src/exact_probabilistic/build.rs
  role: examined
  excerpt: "                return Err(ProductRefusal::NotMarkov(not_markov).into());\n\
    \            }\n            Err(Unsteppable::UndecidedSuccessor(undecided)) =>\
    \ {\n                return Err(ProductRefusal::UndecidedSuccessor(undecided).into());\n\
    \            }\n        };\n        let mut planned = Vec::with_capacity(steps.len());\n\
    \        for step in steps {\n            let Some(identity) = step.identity else\
    \ {\n                planned.push(self.terminal(at, model, step.rewards)?);\n\
    \                continue;\n            };\n            let taken = TraceStep\
    \ {\n "
- id: fsl-analyze/src/exact_probabilistic/digital.rs
  path: fsl-analyze/src/exact_probabilistic/digital.rs
  role: examined
  excerpt: "    taken: &Enabled<'_>,\n    delay: u64,\n) -> ProductAction {\n    ProductAction::WorkloadResolve\
    \ {\n        identity: representative.unwrap_or(&taken.identity).clone(),\n  \
    \      delay,\n    }\n}\n\nfn fault(invariant: &'static str) -> ProductRefusal\
    \ {\n    ProductRefusal::Internal(InternalFault::new(\"exact-probabilistic-digital\"\
    , invariant))\n}\n\nfn one() -> Rational {\n    Rational::from_integer(Integer::one())\n\
    }\n\nfn zero() -> Rational {\n    Rational::from_integer(Integer::zero())\n}\n\
    \n/// The claim forms the digital route admits (ADR-028 TA-4).\nenum Form"
- id: fsl-analyze/src/exact_probabilistic/settle.rs
  path: fsl-analyze/src/exact_probabilistic/settle.rs
  role: examined
  excerpt: "        ProductRefusal::NotMarkov(cause) => ExactProbBuildIssue::NotMarkov(cause),\n\
    \        ProductRefusal::UndecidedSuccessor(_) => ExactProbBuildIssue::UndecidedSuccessor,\n\
    \        ProductRefusal::PredicateRefused(outcome) => ExactProbBuildIssue::Predicate(outcome),\n\
    \        ProductRefusal::Translation(_) | ProductRefusal::Internal(_) => {\n \
    \           ExactProbBuildIssue::BrokenInvariant\n        }\n        ProductRefusal::Stopped(ProductStop::Limit\
    \ { limit, value }) => {\n            ExactProbBuildIssue::Limit { limit, value\
    \ }\n        }\n        Pr"
- id: fsl-analyze/src/statistical/tests.rs
  path: fsl-analyze/src/statistical/tests.rs
  role: examined
  excerpt: "/// A `tick` with no argument and no post-state.\nfn not_markov() -> NotMarkov\
    \ {\n    let transition = fsl_eval::simulation::model::ScheduledIdentity {\n \
    \       operation: r#\"\"tick\"\"#.to_owned(),\n        receiver: filament_exact::ObjectReference::new(\n\
    \            crate::test_ids::universe_id([1]),\n            crate::test_ids::effective_id([2]),\n\
    \            filament_exact::ObjectId::new(\"n\").expect(\"a key\"),\n       \
    \ ),\n        arguments: Vec::new(),\n    };\n    let drawn_transition = filament_semantic_value::model_transition::ModelTransition::new(\n\
    \ "
- id: fsl-analyze/tests/it/digital_clocks.rs
  path: fsl-analyze/tests/it/digital_clocks.rs
  role: examined
  excerpt: "/// A digital refusal reports its actual filtered model posts, rather\
    \ than\n/// the larger discrete successor set before the reset invariants are\
    \ applied.\n#[trace(\"TC-639\", \"FR-204-AC-3\", \"FR-187-AC-3\")]\n#[test]\n\
    fn not_markov_evidence_uses_the_actual_filtered_digital_posts() {\n    use fsl_eval::simulation::TransitionSystem;\n\
    \n    let mut document: serde_json::Value =\n        serde_json::from_slice(&retx_document(&[(\"\
    send\", &[\"late\", \"x\", \"phase\"])]))\n            .expect(\"the timed fixture\
    \ is JSON\");\n    document[\"types\"][1][\"fields\"]\n        .as_"
- id: fsl-analyze/tests/it/exact_probabilistic.rs
  path: fsl-analyze/tests/it/exact_probabilistic.rs
  role: examined
  excerpt: "    let mut expected_posts = product\n        .outgoing(healthy.to)\n\
    \        .iter()\n        .map(|edge| {\n            fsl_eval::simulation::StateKey::from_canonical_bytes(\n\
    \                product.states[edge.to.index()].model.clone(),\n            )\n\
    \        })\n        .collect::<Vec<_>>();\n    expected_posts.sort();\n    assert_eq!(loose.post_state_examples,\
    \ expected_posts);\n    assert_eq!(loose.post_state_examples.len(), 2);\n    assert_ne!(loose.post_state_examples[0],\
    \ loose.post_state_examples[1]);\n    assert!(loose.transition.operation.contains"
- id: fsl-analyze/tests/it/probability_from_source.rs
  path: fsl-analyze/tests/it/probability_from_source.rs
  role: examined
  excerpt: "/// Both native engines retain the actual zero/two/four-post refusal,\
    \ its\n/// complete interleaved argument vector and the first two real model keys.\n\
    #[trace(\"TC-622\", \"FR-187-AC-3\", \"TC-2066\", \"FR-741-AC-1\", \"FR-741-AC-3\"\
    )]\n#[test]\nfn native_not_markov_preserves_actual_zero_two_and_four_post_payloads()\
    \ {\n    use fsl_eval::simulation::model::Unsteppable;\n    use fsl_eval::simulation::TransitionSystem;\n\
    \    use fsl_replay::outcome::{state_key_wire, task_source_transition_wire};\n\
    \n    const CLAIM: &str =\n        \"probabilistic Broken using ev on M "
- id: fsl-eval/src/simulation/model/frames.rs
  path: fsl-eval/src/simulation/model/frames.rs
  role: examined
  excerpt: "                transition,\n                receiver: row.reference.clone(),\n\
    \                declaring: application.declaring.clone(),\n                name:\
    \ application.name.clone(),\n                operation: application.text.clone(),\n\
    \                args,\n                successors: first..successors.len(),\n\
    \            });\n        }\n    }\n    Ok(())\n}\n\n/// ADR-027 PD-4: the transition\
    \ identity of the first argument vector of\n/// `application` at `row`, in canonical\
    \ order, with the first of `clauses`'\n/// preconditions, in clause order, that\
    \ i"
- id: fsl-eval/src/simulation/model/probabilistic.rs
  path: fsl-eval/src/simulation/model/probabilistic.rs
  role: examined
  excerpt: "    /// The complete operation identity, with every argument in declared\
    \ order,\n    /// retained even when this draw has no successor.\n    pub transition:\
    \ ModelTransition,\n    /// One value per random parameter, in declared order.\n\
    \    pub random: Vec<Value>,\n    /// The product of the values' PM-1 probabilities:\
    \ the draws of an\n    /// action sum to exactly 1.\n    pub probability: Rational,\n\
    \    /// FR-120's post-states of the transition, in canonical order: exactly\n\
    \    /// one when the Markov condition holds (PM-3).\n    pub successors: Vec<(Mod"
- id: fsl-eval/src/simulation/model/probabilistic/tests.rs
  path: fsl-eval/src/simulation/model/probabilistic/tests.rs
  role: examined
  excerpt: "/// Caller admission must succeed before retaining the owned refusal box.\n\
    #[trace(\"TC-622\", \"FR-187-AC-3\")]\n#[test]\nfn not_markov_box_admission_preserves_payload_and_caller_refusals()\
    \ {\n    use super::super::{ModelKeyBudget, ModelKeyError, ModelKeyExpansionError};\n\
    \    use filament_exact::{\n        Cancel, CancelCause, IntegerDecimalBudget,\
    \ Reservation, ReservationDenied,\n    };\n\n    #[derive(Clone, Debug, Eq, PartialEq)]\n\
    \    enum Admission {\n        Storage(usize),\n        Work(u64),\n    }\n  \
    \  #[derive(Default)]\n    struct Budget<'a> {\n        "
- id: fsl-eval/src/simulation/model/restore.rs
  path: fsl-eval/src/simulation/model/restore.rs
  role: examined
  excerpt: "            let target = values.capacity().saturating_mul(2).max(1);\n\
    \            let bytes = target\n                .checked_mul(size_of::<T>())\n\
    \                .ok_or(ModelKeyError::SizeOverflow)?;\n            self.retain(bytes)?;\n\
    \            values\n                .try_reserve_exact(target - values.len())\n\
    \                .map_err(|_| {\n                    ModelKeyError::Encoding(crate::simulation::EncodingRefusal::Allocation\
    \ {\n                        requested: bytes,\n                    })\n     \
    \           })?;\n        }\n        values.push(v"
- id: fsl-eval/src/simulation/sampled.rs
  path: fsl-eval/src/simulation/sampled.rs
  role: examined
  excerpt: "    let [(transition, post)] = draw.successors.as_slice() else {\n   \
    \     return Err(SampleStop::NotMarkov(NotMarkov {\n            state: state.clone(),\n\
    \            transition: action.identity.clone(),\n            drawn_transition:\
    \ draw.transition.clone(),\n            random: random.clone(),\n            post_states:\
    \ u64::try_from(draw.successors.len())\n                .expect(\"supported targets\
    \ have at most 64-bit usize\"),\n            post_state_examples: system.not_markov_post_keys(draw)?,\n\
    \        }));\n    };\n    let identities = Rational::from"
- id: fsl-eval/src/simulation/sampled/tests.rs
  path: fsl-eval/src/simulation/sampled/tests.rs
  role: examined
  excerpt: "    assert_drawn_identity(&world, &not_markov);\n}\n\n/// A terminal state\
    \ continues on stutter steps (ADR-018 SM-4): `Link`\n/// samples to a terminal\
    \ state within its two attempts, then stutters with\n/// probability 1 and no\
    \ draw.\n#[trace(\"TC-623\")]\n#[test]\nfn a_terminal_state_continues_on_stutter_steps()\
    \ {\n    let world = World::new(&world::link());\n    let (result, _) = sample(&world,\
    \ \"Even\", &sampler(), (7, 0, 6), &Cancel::new());\n    let behaviour = result.expect(\"\
    Link samples\");\n    assert_eq!(behaviour.steps.len(), 5);\n    let last = behavi"
- id: fsl-replay/src/exact_probabilistic.rs
  path: fsl-replay/src/exact_probabilistic.rs
  role: examined
  excerpt: "    NotMarkov(Box<fsl_eval::simulation::model::NotMarkov>),\n    /// A\
    \ product successor's contract was undecided.\n    UndecidedSuccessor,\n    ///\
    \ State expansion failed or reached a subject-owned limit.\n    Expansion(ExpansionStop),\n\
    \    /// A claim predicate was refused or incomplete.\n    Predicate(ClauseOutcome),\n\
    \    /// An internal product invariant, automaton translation, or evaluator\n\
    \    /// invariant failed.\n    BrokenInvariant,\n    /// A refusal carrying its\
    \ own catalog diagnostic.\n    Failed(CatalogCode),\n    /// A timed-product form\
    \ the"
- id: fsl-replay/src/outcome.rs
  path: fsl-replay/src/outcome.rs
  role: examined
  excerpt: "    /// The model state named by FR-331's probabilistic refusal causes.\n\
    \    #[serde(skip_serializing_if = \"Option::is_none\")]\n    state: Option<TimedStateWire>,\n\
    \    /// The typed transition identity named by `not-stochastic`.\n    #[serde(skip_serializing_if\
    \ = \"Option::is_none\")]\n    identity: Option<TimedTransitionWire>,\n    ///\
    \ The complete drawn identity of a `not-markov` refusal.\n    #[serde(skip_serializing_if\
    \ = \"Option::is_none\")]\n    transition: Option<TaskSourceTransitionWire>,\n\
    \    /// The actual count of a `not-markov` draw's model post"
- id: fsl-replay/src/planner/entry.rs
  path: fsl-replay/src/planner/entry.rs
  role: examined
  excerpt: '    (`certification-mismatch`);

    //! 7. the `basis` or `cause`, then each remaining member in member-name

    //!    order, is read as a value.

    //!

    //! A plugin''s entry goes through FR-290''s readers

    //! ([`read_plugin_basis`], [`read_plugin_certification`],

    //! [`read_plugin_inconclusive_cause`]); every other entry through the

    //! first-party arms below. A value the planner''s types cannot hold (a

    //! certificate, a hybrid trace, a statistical verdict, ...) is refused

    //! `unknown_required_feature`/`unsupported-feature` at the point in the

    //! readin'
- id: fsl-replay/src/probability_certificate.rs
  path: fsl-replay/src/probability_certificate.rs
  role: examined
  excerpt: "                fsl_eval::simulation::model::Unsteppable::NotMarkov(cause)\
    \ => {\n                    crate::ExactProbBuildIssue::NotMarkov(cause)\n   \
    \             }\n                fsl_eval::simulation::model::Unsteppable::UndecidedSuccessor(_)\
    \ => {\n                    crate::ExactProbBuildIssue::UndecidedSuccessor\n \
    \               }\n            });\n            ExactProbabilityGraphError::ExpansionRefused\n\
    \        })?;\n        return workload_distribution(steps);\n    }\n    #[cfg(feature\
    \ = \"test-support\")]\n    observe(ProbabilityWitnessSemanticPhase"
- id: fsl-replay/src/probability_certificate/document/long_run.rs
  path: fsl-replay/src/probability_certificate/document/long_run.rs
  role: examined
  excerpt: "                        fsl_eval::simulation::model::Unsteppable::NotMarkov(cause)\
    \ => {\n                            crate::ExactProbBuildIssue::NotMarkov(cause)\n\
    \                        }\n                        fsl_eval::simulation::model::Unsteppable::UndecidedSuccessor(_)\
    \ => {\n                            crate::ExactProbBuildIssue::UndecidedSuccessor\n\
    \                        }\n                    });\n                    ProbabilityDocumentError::Graph(ExactProbabilityGraphError::ExpansionRefused)\n\
    \                })?;\n                if steps.i"
- id: fsl-replay/src/probability_certificate/document/measure.rs
  path: fsl-replay/src/probability_certificate/document/measure.rs
  role: examined
  excerpt: "                    .map_err(|stop| {\n                        ProbabilityDocumentError::Build(Box::new(match\
    \ stop {\n                            fsl_eval::simulation::model::Unsteppable::NotMarkov(cause)\
    \ => {\n                                crate::ExactProbBuildIssue::NotMarkov(cause)\n\
    \                            }\n                            fsl_eval::simulation::model::Unsteppable::UndecidedSuccessor(_)\
    \ => {\n                                crate::ExactProbBuildIssue::UndecidedSuccessor\n\
    \                            }\n                        }))\n "
- id: fsl-replay/src/probability_certificate/reward.rs
  path: fsl-replay/src/probability_certificate/reward.rs
  role: examined
  excerpt: "            Unsteppable::NotMarkov(cause) => crate::ExactProbBuildIssue::NotMarkov(cause),\n\
    \            Unsteppable::UndecidedSuccessor(_) => crate::ExactProbBuildIssue::UndecidedSuccessor,\n\
    \        }))\n    };\n    if let Some(workload) = workload {\n        let mut\
    \ admission =\n            ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes,\
    \ cancel);\n        let steps = system\n            .weighted_steps_with_budget(state,\
    \ workload, identity, &mut admission, cancel)\n            .map_err(|error| admission.expansion_refusal(error))?\n\
    \   "
- id: fsl-replay/src/proof_result.rs
  path: fsl-replay/src/proof_result.rs
  role: examined
  excerpt: "    /// FR-187: a drawn step of an EN-4 or EN-5 run broke the Markov condition\n\
    \    /// (ADR-024 PM-3). Wire cause `not-markov`.\n    NotMarkov(Box<fsl_eval::simulation::model::NotMarkov>),\n\
    \    /// FR-192: a choice's scaled weight total exceeds `2^256`, so no draw is\n\
    \    /// accepted (FSpec FR-181-AC-14). Wire cause `total-above-draw-range`,\n\
    \    /// `total` a decimal string.\n    TotalAboveDrawRange {\n        /// The\
    \ scaled total.\n        total: Integer,\n    },\n    /// A free delay raced in\
    \ a timed statistical sample (FR-254).\n    NotStochastic {\n"
- id: fsl-replay/src/statistical/settle.rs
  path: fsl-replay/src/statistical/settle.rs
  role: examined
  excerpt: "        let record = TerminalRecord::with_backend(request_index, backend,\
    \ self.value);\n        let diagnostic = self.diagnostic.or(record.diagnostic());\n\
    \        record.with_diagnostic(diagnostic)\n    }\n}\n\n/// FR-192's `NotStatistical`\
    \ row: the method does not decide the claim's\n/// form, so the request is a negotiation\
    \ `invalid-request` with no terminal\n/// record (FSpec FR-408 Admission, FR-331).\n\
    #[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]\n#[error(\"{method:?}\
    \ does not decide a {form:?} claim\")]\npub struct NotStatisticalReque"
- id: fsl-replay/src/statistical/witness.rs
  path: fsl-replay/src/statistical/witness.rs
  role: examined
  excerpt: "        let transition = ScheduledIdentity {\n            operation: r#\"\
    \"tick\"\"#.to_owned(),\n            receiver: ObjectReference::new(\n       \
    \         crate::fixture_universe(1),\n                crate::fixture_effective(2),\n\
    \                ObjectId::new(\"n\").expect(\"a key\"),\n            ),\n   \
    \         arguments: Vec::new(),\n        };\n        let drawn_transition = filament_semantic_value::model_transition::ModelTransition::new(\n\
    \            transition.operation.clone(),\n            Some(\n              \
    \  fsl_eval::simulation::model::reference_c"
- id: spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md
  path: spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md
  role: examined
  excerpt: "pub enum Unsteppable { NotMarkov(Box<NotMarkov>), UndecidedSuccessor(UndecidedSuccessor)\
    \ }\npub struct NotMarkov {\n    pub state: StateKey, // the canonical state key\
    \ (the RFC 8785 byte string of the state)\n    pub transition: ScheduledIdentity,\n\
    \    pub drawn_transition: ModelTransition, // the existing complete identity,\
    \ including random arguments in declared order\n    pub random: Vec<Value>,\n\
    \    pub post_states: u64, // the actual number of distinct model post-states,\
    \ checked without saturation\n    pub post_state_examples: Vec<StateKey>, // th"
- id: tools/fsl-probability-check/src/main.rs
  path: tools/fsl-probability-check/src/main.rs
  role: examined
  excerpt: "                fsl_replay::ExactProbBuildIssue::NotMarkov(_)\n      \
    \              | fsl_replay::ExactProbBuildIssue::Unsupported(_)\n           \
    \ ) =>\n        {\n            \"unsupported\"\n        }\n        ProbabilitySourceError::Compile(compile)\n\
    \            if matches!(\n                compile.as_ref(),\n               \
    \ fsl_foundation::diagnostic::StageFailure::Refused(_)\n                    |\
    \ fsl_foundation::diagnostic::StageFailure::RefusedWithPayload(_, _)\n       \
    \     ) =>\n        {\n            \"refused\"\n        }\n        ProbabilitySourceError"
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
