// SPDX-License-Identifier: AGPL-3.0-or-later
//! A scripted expansion for in-crate tests: the one state a test names ends
//! its expansion with a stop or a finding, every other state expands as the
//! subject does (FR-101 "Expansion", FR-120). FR-126-AC-29 and AC-30 are
//! stated over a subject whose expansion of one state ends so, which no
//! authored model reaches: a `pre` or `post` reads only the receiver's own
//! fields, so the expansion that precedes it meets the same values.

use std::cell::RefCell;
use std::rc::Rc;

use fsl_eval::simulation::model::{ModelFinding, ModelSimState};
use fsl_eval::simulation::{Cut, Expansion, ExpansionStop};

/// How a scripted state ends its expansion.
pub(crate) enum Script {
    /// The expansion stops.
    Stop(ExpansionStop),
    /// The expansion records one more finding.
    Finding(ModelFinding),
}

type Rule = Rc<dyn Fn(&ModelSimState) -> Option<Script>>;

thread_local! {
    static RULE: RefCell<Option<Rule>> = const { RefCell::new(None) };
}

/// The script of a test, in force until it drops.
pub(crate) struct Scripted;

impl Scripted {
    /// Run every expansion on this thread through `rule`.
    pub(crate) fn install(rule: impl Fn(&ModelSimState) -> Option<Script> + 'static) -> Self {
        RULE.with(|slot| *slot.borrow_mut() = Some(Rc::new(rule)));
        Self
    }
}

impl Drop for Scripted {
    fn drop(&mut self) {
        RULE.with(|slot| slot.borrow_mut().take());
    }
}

/// `result`, the expansion of `state`, as the script ends it.
pub(crate) fn apply<T, S>(
    state: &ModelSimState,
    result: Result<(Expansion<T, S, ModelFinding>, Cut), ExpansionStop>,
) -> Result<(Expansion<T, S, ModelFinding>, Cut), ExpansionStop> {
    let rule = RULE.with(|slot| slot.borrow().clone());
    let Some(script) = rule.and_then(|rule| rule(state)) else {
        return result;
    };
    match script {
        Script::Stop(stop) => Err(stop),
        Script::Finding(finding) => result.map(|(mut expansion, cut)| {
            expansion.findings.push(finding);
            (expansion, cut)
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filament_exact::{Cancel, Meter, Value};
    use filament_semantic_value::catalog::CatalogCode;
    use filament_semantic_value::checking::CheckMode;
    use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
    use fsl_eval::simulation::model::{ClauseOutcome, ModelSystem};
    use fsl_eval::simulation::{ExpansionLimit, TransitionSystem};
    use fsl_eval::temporal::{AtomReading, UndefinedCause};
    use fsl_replay::{
        CounterexampleKind, ModelCheckItem, ModelCheckLimits, ModelCheckOutcome,
        TemporalCounterexample, UndefinedLocus,
    };
    use fsl_semantics::check::FormulaNodeId;
    use ix_trace_rs::trace;

    use crate::counter::UNLIMITED;
    use crate::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest};
    use crate::units::{clause, config_object, unit, Domain, World};

    /// The example unit's post clause: the version advances modulo 3.
    const CYCLES: &str = "post Cycles using v on Config::ConfigVersion::attemptUpdate { \
         (pre(self.versionNumber) < 2 and self.versionNumber = pre(self.versionNumber) + 1) \
         or (pre(self.versionNumber) = 2 and self.versionNumber = 0) }\n";
    const SAFETY: &str = "always holds(6 mod (2 - c.versionNumber) >= 0)";
    const LIVE: &str = "always eventually holds(6 mod (2 - c.versionNumber) >= 0)";

    /// The example world (FR-126-AC-11): objects `a` and `b` at `versions`,
    /// `formula` over `c: Config::ConfigVersion`, `mod_defined` checked under
    /// `Kernel`.
    fn world(formula: &str, versions: (i64, i64)) -> World {
        let domain = Domain::config_version().with_versions_up_to(2);
        let formula = formula.replace(
            "6 mod (2 - c.versionNumber) >= 0",
            "mod_defined(c.versionNumber)",
        );
        let clause = clause("c: Config::ConfigVersion", "", &formula);
        let function =
            "function mod_defined using v(n: Int[0, 2]): Boolean pure { 6 mod (2 - n) >= 0 }\n";
        let text = unit(&domain, "Config", &format!("{CYCLES}{function}{clause}"));
        World::new_with_mode(
            domain,
            &text,
            &[
                ("a", config_object(versions.0)),
                ("b", config_object(versions.1)),
            ],
            CheckMode::Kernel,
        )
    }

    /// The versions of `a` and `b` at a state, read without the world.
    fn versions_of(world: &World) -> impl Fn(&ModelSimState) -> (i64, i64) + 'static {
        let population = world.domain.key(world.domain.population);
        let field = world
            .domain
            .key(&format!("{}/versionNumber", world.domain.object));
        move |state| {
            let read = |object: &str| match state.state().populations[&population][object]
                .fields
                .get(&field)
            {
                Some(Value::Integer(version)) => version.to_string().parse().expect("small"),
                other => panic!("{object}.versionNumber is not an integer: {other:?}"),
            };
            (read("a"), read("b"))
        }
    }

    /// The letters of the clause's one atom: `6 mod (2 - version) >= 0`,
    /// undefined at version 2.
    struct Letters<'w> {
        world: &'w World,
        atom: FormulaNodeId,
    }

    impl<'w> Letters<'w> {
        fn new(world: &'w World) -> Self {
            let atoms = fsl_eval::automaton::translate(&world.clauses[0], 1 << 20)
                .expect("the clause translates")
                .atoms()
                .to_vec();
            let [atom] = atoms.as_slice() else {
                panic!("one atom, got {atoms:?}");
            };
            Self {
                world,
                atom: *atom,
            }
        }
    }

    impl ModelLetters for Letters<'_> {
        crate::units::unguarded!();

        fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
            crate::units::unread()
        }

        fn atom(
            &self,
            atom: FormulaNodeId,
            instance: &ModelInstance,
            state: &ModelSimState,
            _meter: &mut Meter,
        ) -> Result<AtomReading, AtomStop> {
            assert_eq!(atom, self.atom);
            let version = self.world.integer(state, &instance.key, "versionNumber");
            Ok(if version == 2 {
                AtomReading::Undefined(UndefinedCause::Kernel(
                    filament_exact::Undefined::DivisionByZero,
                ))
            } else {
                AtomReading::True
            })
        }
    }

    fn check(
        world: &World,
        max_depth: Option<usize>,
        limits: ModelCheckLimits,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        let letters = Letters::new(world);
        check_model(
            ModelCheckRequest {
                workers: 1,
                temporal_clock_selections: &[],
                subject: world.subject(UNLIMITED),
                item: ModelCheckItem::Clause {
                    clause: &world.clauses[0],
                    letters: &letters,
                },
                max_depth,
                limits,
                smt_encoding: Default::default(),
                smt_run: Default::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
    }

    fn violated(outcome: &Result<ModelCheckOutcome, ModelCheckRefusal>) -> &TemporalCounterexample {
        match outcome {
            Ok(ModelCheckOutcome::Violated { counterexample, .. }) => counterexample,
            other => panic!("expected a violation, got {other:?}"),
        }
    }

    /// AC-27's finding: bound to `b`, the prefix `(0,1) -upd(b)-> (0,2)`,
    /// undefined at position 1 by a division by zero.
    fn is_the_finding_for_b(counterexample: &TemporalCounterexample) {
        let bound = counterexample.over_binding.as_ref().expect("a binding");
        assert_eq!(bound.key, "b");
        let receivers: Vec<_> = counterexample
            .prefix
            .iter()
            .map(|step| step.transition.receiver().expect("a receiver").to_owned())
            .collect();
        assert_eq!(receivers.len(), 1);
        assert!(receivers[0].contains("\"identity\":\"b\""), "{receivers:?}");
        let CounterexampleKind::UndefinedEvaluation(found) = &counterexample.kind else {
            panic!("expected an undefined evaluation: {:?}", counterexample.kind);
        };
        assert_eq!(found.site.position, 1);
        assert!(matches!(found.site.locus, UndefinedLocus::Atom(_)), "{found:?}");
    }

    /// One transition of the world, to name in a scripted finding.
    fn some_transition(world: &World) -> filament_semantic_value::model_transition::ModelTransition {
        let system = ModelSystem::new(world.subject(UNLIMITED).simulation.clone())
            .expect("the world admits");
        let initial = system.try_initial().expect("an initial state");
        let expansion = system.successors(&initial[0]).expect("expands");
        expansion.successors[0].0.clone()
    }

    /// FR-126-AC-29 (TC-1085): over AC-27's subject with the expansion of
    /// `(1,1)` recording FR-120's `ContractUndetermined`, `a`'s search ends
    /// undecided at `(1,1)`, the first state after the initial one in
    /// breadth-first order, while `b`'s undefined letter was read at `(0,2)`,
    /// a successor of the initial state: the item returns `b`'s violation.
    #[trace("TC-1085", "FR-126-AC-29")]
    #[test]
    fn a_later_safety_refutation_is_reported_past_an_earlier_undecided_binding() {
        let world = world(SAFETY, (0, 1));
        let versions = versions_of(&world);
        let transition = some_transition(&world);
        let _script = Scripted::install(move |state| {
            (versions(state) == (1, 1)).then(|| {
                Script::Finding(ModelFinding::ContractUndetermined {
                    transition: transition.clone(),
                    candidate: None,
                    clause: "Guard".to_owned(),
                    evaluation: ClauseOutcome::NotBoolean,
                })
            })
        });
        let outcome = check(&world, None, ModelCheckLimits::default());
        is_the_finding_for_b(violated(&outcome));
    }

    /// FR-126-AC-30 (TC-1085): AC-29's subject with the expansion of `(1,1)`
    /// ending on FR-120's `max_candidates`, `always eventually
    /// holds(mod_defined(c.versionNumber))` with no fairness and no
    /// `max_depth` returns `b`'s violation: `a` has no finding, and `b`'s
    /// `(0,2)` was discovered by the expansion of `(0,1)`.
    #[trace("TC-1085", "FR-126-AC-30")]
    #[test]
    fn a_tp4_binding_that_ends_on_max_candidates_does_not_end_the_item() {
        let world = world(LIVE, (0, 1));
        let versions = versions_of(&world);
        let _script = Scripted::install(move |state| {
            (versions(state) == (1, 1)).then(|| {
                Script::Stop(ExpansionStop {
                    cause: CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
                    limit: Some(ExpansionLimit::MaxCandidates),
                })
            })
        });
        let outcome = check(&world, None, ModelCheckLimits::default());
        is_the_finding_for_b(violated(&outcome));
    }
}
