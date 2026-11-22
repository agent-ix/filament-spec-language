// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 Inputs and FR-338 Inputs: the item an EN-1 run decides and the
//! core's closure checker checks, and the canonical safety translation of
//! each of its instances (ADR-018 LA-2).
//!
//! A TP-1 clause binds one instance per key of each population of the
//! subject that admits its `over` type, derived from the subject's universes
//! at request time (FR-125 Instancing). Each instance has its own
//! translation, under a key context that binds the typed reference of its
//! key: two admitting populations that hold the same key give two instances
//! with one key context. The deadlock-freedom and race-freedom items have one
//! instance with the binding `[]`.
//!
//! A clause's key contexts name its identity, which the item does not carry:
//! [`ClauseIdentity::resolve`] reads it from the request's package
//! (FR-337-AC-5), so no caller names an occurrence or an `over` node.

use filament_exact::{Cancel, ValueType};
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::fairness::PropertyForm;
use fsl_eval::automaton::{
    closure_shape, invariant_atom, is_safety_translated, translate, translate_counted,
    AutomatonError, AutomatonKind, BoundedTranslation, BuchiStates, ContextBinding, NotTranslated,
    PropertyAutomaton, SafetyStates, SafetyTranslation, StateContext, SubjectKind,
    TranslationRefusal, MAX_ATOMS,
};
use fsl_eval::model_subject::{ModelInstance, ModelLetters, ModelSubject};
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::NotSimulated;
use fsl_semantics::check::{Activation, CheckedStateClause, CheckedTemporalClause, ProtocolNodeId};

use crate::model_check::identity::{ClauseIdentity, ClauseRootRefusal};

/// What an EN-1 run decides (FR-126 Inputs) and a certificate request
/// checks (FR-338 Inputs).
#[derive(Clone, Copy)]
pub enum ModelCheckItem<'a> {
    /// An authored clause with its property form, its atoms read through
    /// `letters`.
    Clause {
        /// The checked clause.
        clause: &'a CheckedTemporalClause,
        /// The seam to the one clause evaluator for its `holds` atoms.
        letters: &'a dyn ModelLetters,
    },
    /// FR-124's derived deadlock-freedom item.
    DeadlockFreedom,
    /// FR-224's derived race-freedom item, `always holds(not raced)`, over a
    /// protocol subject with a weak `parallel`.
    RaceFreedom,
    /// FR-951's invariant item: the `invariant` state clause `clause` holds
    /// at every reachable state of a model subject.
    Invariant {
        /// The `invariant` state clause, by name and claim occurrence.
        clause: InvariantClause<'a>,
    },
    /// FR-730's invariant obligation for one checked protocol repeat.
    LoopInvariant {
        /// The checked repeat node.
        repeat: ProtocolNodeId,
    },
    /// FR-730's termination obligation for one checked protocol repeat.
    LoopTermination {
        /// The checked repeat node.
        repeat: ProtocolNodeId,
    },
}

/// The `invariant` state clause an invariant item decides (FR-951): its
/// declared name, which keys its evaluation, and its node and `claim`
/// occurrence, which key the proof's key-context root. The source route
/// reads them from its checked clause, the package route from the clause's
/// `claim_names` row (FR-940).
#[derive(Clone, Copy, Debug)]
pub struct InvariantClause<'a> {
    /// The declared name.
    pub name: &'a str,
    /// The clause's `state_clause` node.
    pub node: &'a filament_exact::NodeKey,
    /// The declaration's `claim` occurrence of the node.
    pub claim: &'a filament_exact::Origin,
}

impl<'a> InvariantClause<'a> {
    /// The source route's invariant clause.
    pub fn of(clause: &'a CheckedStateClause) -> Self {
        Self {
            name: clause.name(),
            node: clause.node(),
            claim: clause.claim(),
        }
    }
}

/// One instance of an item: the bound population key, `None` for the
/// deadlock-freedom item, and the translation under its context.
#[derive(Debug)]
pub struct Instance {
    /// The population key the instance binds.
    pub binding: Option<ModelInstance>,
    /// The instance's canonical translation.
    pub translation: Translation,
}

/// The canonical translation of one instance (FR-338): a safety item's, a
/// bounded clause's, or a safety monitor's.
#[derive(Debug)]
pub enum Translation {
    /// A TP-1 clause or the deadlock-freedom item.
    Safety(SafetyTranslation),
    /// A TP-3 clause over an ordinary subject whose product states are the
    /// bad-prefix monitor's `["safety", branches]` keys: obligations, past
    /// memory, interval windows and acceptance membership.
    Monitored(SafetyStates),
    /// A bounded `eventually[0,u]` or `always[0,u]` clause on origin (TP-2).
    Bounded(BoundedTranslation),
    /// A TP-4 liveness clause, built only for the closure checker.
    Liveness(Box<LivenessTranslation>),
}

/// The TP-4 engine automaton and its FR-338 state-key context.
#[derive(Debug)]
pub struct LivenessTranslation {
    pub(crate) automaton: PropertyAutomaton,
    pub(crate) keys: BuchiStates,
}

impl LivenessTranslation {
    /// The key context of the instance's automaton states: it gives the
    /// canonical key of the structural state (`PropertyAutomaton::buchi_state`)
    /// of any automaton over the same clause.
    pub const fn keys(&self) -> &BuchiStates {
        &self.keys
    }
}

impl Translation {
    /// Whether `octets` are a key of this translation's context, admitted
    /// before the key is used for membership.
    pub fn admits(&self, octets: &[u8]) -> bool {
        match self {
            Self::Safety(translation) => translation.admit(octets).is_ok(),
            Self::Monitored(states) => states.admit(octets).is_ok(),
            Self::Bounded(translation) => translation.admit(octets).is_ok(),
            Self::Liveness(translation) => translation.keys.admit(octets).is_ok(),
        }
    }
}

/// The TP-4 request of a closure check: the subject's key algebra and the
/// automaton state bound.
#[derive(Clone, Copy)]
struct Tp4 {
    subject_kind: SubjectKind,
    max_automaton_states: u64,
}

/// Why an item has no canonical translation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ItemRefusal {
    /// The clause is neither a TP-1 `always holds(A)` under infinite-trace
    /// nor a bounded single-interval clause with a canonical translation.
    #[error(transparent)]
    Translation(#[from] TranslationRefusal),
    /// The clause binds more instances than one letter holds atoms.
    #[error("the clause binds {count} instances; a letter holds at most {max}")]
    TooManyBindings {
        /// The number of instances.
        count: usize,
        /// The atoms one letter holds.
        max: usize,
    },
    /// The package gives the clause no root (`clause-root-unresolved`).
    #[error(transparent)]
    ClauseRoot(#[from] ClauseRootRefusal),
    /// The deadlock-freedom item's context did not encode.
    #[error("the deadlock-freedom context does not encode")]
    DeadlockContext,
    /// The race-freedom item's context did not encode.
    #[error("the race-freedom context does not encode")]
    RaceContext,
    /// The invariant item's context did not encode.
    #[error("the invariant context does not encode")]
    InvariantContext,
    /// A loop-proof item was sent to a model subject.
    #[error("a loop-proof item is decided over a protocol subject only")]
    LoopProofOverModel,
    /// A protocol loop-proof item's context did not encode.
    #[error("the protocol loop-proof context does not encode")]
    LoopProofContext,
    /// FR-125: a population that admits the `over` type has no universe
    /// (`RequiresBound`), or that check could not classify it.
    #[error("the clause's instances are not finite: {0:?}")]
    Unbounded(NotSimulated),
    /// An invariant of the subject broke.
    #[error("instancing broke invariant {0:?}")]
    Fault(InternalFault),
    /// The requested TP-4 property automaton could not be materialized.
    #[error(transparent)]
    Automaton(#[from] AutomatonError),
}

impl ModelCheckItem<'_> {
    /// Each instance of the item with its translation, in FR-125's
    /// canonical instance order, derived from `system`'s universes. A
    /// clause's identity is resolved from `subject`'s package; `system` is
    /// `subject`'s admitted model system.
    ///
    /// # Errors
    ///
    /// [`ItemRefusal`] when an admitting population has no universe, the
    /// package gives the clause no root, or the item has no canonical
    /// translation.
    pub fn instances(
        &self,
        subject: &ModelSubject<'_>,
        system: &ModelSystem<'_>,
        temporal_clock_selections: &[fsl_package::TemporalClockSelection],
        cancel: &Cancel,
    ) -> Result<Vec<Instance>, ItemRefusal> {
        self.build_instances(subject, system, temporal_clock_selections, cancel, None)
    }

    /// Instances whose TP-4 liveness clauses carry a subject-specific key
    /// algebra of `subject_kind` and an automaton of at most
    /// `max_automaton_states` states: the closure checker's, and the key
    /// contexts EN-1 writes a component certificate's product states in
    /// (FR-339).
    ///
    /// # Errors
    ///
    /// [`ItemRefusal`] as [`Self::instances`] returns it.
    pub fn instances_for(
        &self,
        subject: &ModelSubject<'_>,
        system: &ModelSystem<'_>,
        temporal_clock_selections: &[fsl_package::TemporalClockSelection],
        cancel: &Cancel,
        subject_kind: SubjectKind,
        max_automaton_states: u64,
    ) -> Result<Vec<Instance>, ItemRefusal> {
        self.build_instances(
            subject,
            system,
            temporal_clock_selections,
            cancel,
            Some(Tp4 {
                subject_kind,
                max_automaton_states,
            }),
        )
    }

    fn build_instances(
        &self,
        subject: &ModelSubject<'_>,
        system: &ModelSystem<'_>,
        temporal_clock_selections: &[fsl_package::TemporalClockSelection],
        cancel: &Cancel,
        tp4: Option<Tp4>,
    ) -> Result<Vec<Instance>, ItemRefusal> {
        match self {
            Self::DeadlockFreedom => Ok(vec![Instance {
                binding: None,
                translation: Translation::Safety(
                    SafetyTranslation::deadlock_freedom()
                        .map_err(|_| ItemRefusal::DeadlockContext)?,
                ),
            }]),
            Self::RaceFreedom => Ok(vec![Instance {
                binding: None,
                translation: Translation::Safety(
                    SafetyTranslation::race_freedom().map_err(|_| ItemRefusal::RaceContext)?,
                ),
            }]),
            Self::Invariant { clause } => {
                // The source route checks the root against its own checked
                // clauses; the package route's root is the clause's
                // `claim_names` row, which only an invariant row selects
                // (FR-940).
                let occurrence = match subject.simulation.source_package() {
                    Some(package) => invariant_root(package, *clause)?,
                    None => fsl_foundation::source::provenance::OccurrenceKey::new(
                        clause.node.clone(),
                        clause.claim.clone(),
                    ),
                };
                Ok(vec![Instance {
                    binding: None,
                    translation: Translation::Safety(
                        SafetyTranslation::state_invariant(occurrence)
                            .map_err(|_| ItemRefusal::InvariantContext)?,
                    ),
                }])
            }
            Self::LoopInvariant { repeat } if tp4.is_some() => Ok(vec![Instance {
                binding: None,
                translation: Translation::Safety(
                    SafetyTranslation::loop_invariant(*repeat)
                        .map_err(|_| ItemRefusal::LoopProofContext)?,
                ),
            }]),
            Self::LoopTermination { repeat } if tp4.is_some() => Ok(vec![Instance {
                binding: None,
                translation: Translation::Safety(
                    SafetyTranslation::loop_termination(*repeat)
                        .map_err(|_| ItemRefusal::LoopProofContext)?,
                ),
            }]),
            Self::LoopInvariant { .. } | Self::LoopTermination { .. } => {
                Err(ItemRefusal::LoopProofOverModel)
            }
            Self::Clause { clause, .. } => {
                // The source route reads the clause's identity from its own
                // emission, the package route from its admitted bytes (FR-940).
                let identity = match (
                    subject.simulation.source_package(),
                    subject.simulation.admitted_package(),
                ) {
                    (Some(package), _) => ClauseIdentity::resolve(
                        package,
                        subject.simulation.packages,
                        clause,
                        temporal_clock_selections,
                        cancel,
                        subject.simulation.emit,
                    )?,
                    (None, Some(admitted)) => ClauseIdentity::of_admitted(admitted, clause)?,
                    (None, None) => {
                        return Err(ItemRefusal::Fault(InternalFault::new(
                            "model-check-item",
                            "the-item-reads-a-package",
                        )))
                    }
                };
                let over = &clause.over().1;
                let bindings = system.instances(over).map_err(ItemRefusal::Unbounded)?;
                if bindings.len() > MAX_ATOMS {
                    return Err(ItemRefusal::TooManyBindings {
                        count: bindings.len(),
                        max: MAX_ATOMS,
                    });
                }
                bindings
                    .into_iter()
                    .map(|instance| {
                        let binding = match (identity.over(), over) {
                            (Some(parameter), ValueType::Reference(object_type)) => {
                                system
                                    .bound_reference(object_type.clone(), &instance)
                                    .map_err(ItemRefusal::Fault)?;
                                vec![ContextBinding::instance(parameter, instance.key.clone())]
                            }
                            _ => Vec::new(),
                        };
                        let context =
                            StateContext::clause(clause, identity.occurrence().clone(), binding);
                        let translation = if closure_shape(clause).is_some() {
                            // FR-338: a protocol subject's steps do not all
                            // count, so its bounded `on each` has no
                            // canonical translation (the producer refuses it
                            // too); its `Idle` key is no state there.
                            if matches!(clause.activation(), Activation::Each { .. })
                                && tp4.is_some_and(|tp4| tp4.subject_kind == SubjectKind::Protocol)
                            {
                                return Err(TranslationRefusal::NotTranslated(NotTranslated).into());
                            }
                            Translation::Bounded(BoundedTranslation::clause(clause, &context)?)
                        } else if let Some(tp4) =
                            tp4.filter(|_| clause.property_form() == Some(PropertyForm::Liveness))
                        {
                            let automaton = match tp4.subject_kind {
                                SubjectKind::Ordinary => {
                                    translate(clause, tp4.max_automaton_states)
                                }
                                SubjectKind::Protocol => {
                                    translate_counted(clause, tp4.max_automaton_states)
                                }
                            }
                            .map_err(ItemRefusal::Automaton)?;
                            if automaton.kind() == AutomatonKind::Liveness {
                                let keys = BuchiStates::clause(clause, &context, tp4.subject_kind)
                                    .map_err(ItemRefusal::Translation)?;
                                Translation::Liveness(Box::new(LivenessTranslation {
                                    automaton,
                                    keys,
                                }))
                            } else {
                                // Counted protocol progress can lower a
                                // liveness-form formula into the safety
                                // fragment; its replay form is the safety one.
                                Translation::Safety(SafetyTranslation::reachable_invariant(
                                    clause, &context,
                                )?)
                            }
                        } else if let Some(tp4) = tp4.filter(|_| {
                            invariant_atom(clause).is_none() && is_safety_translated(clause)
                        }) {
                            // An infinite-trace TP-3 clause under either
                            // activation, over either subject kind.
                            Translation::Monitored(
                                SafetyStates::clause(clause, &context, tp4.subject_kind)
                                    .map_err(ItemRefusal::Translation)?,
                            )
                        } else {
                            Translation::Safety(SafetyTranslation::reachable_invariant(
                                clause, &context,
                            )?)
                        };
                        Ok(Instance {
                            binding: Some(instance),
                            translation,
                        })
                    })
                    .collect()
            }
        }
    }
}

/// FR-338's invariant root of `clause` in `package`: its `state`/
/// `state_clause` node and `claim` occurrence, resolved to a checked state
/// clause of kind `invariant` with the same node and occurrence. A node that
/// is no such clause, a clause of another kind (a `constraint` included) or a
/// foreign occurrence is refused as a foreign clause root is refused.
///
/// # Errors
///
/// [`ClauseRootRefusal::Undeclared`] naming the clause.
pub fn invariant_root(
    package: &fsl_package::CheckedPackage,
    clause: InvariantClause<'_>,
) -> Result<fsl_foundation::source::provenance::OccurrenceKey, ClauseRootRefusal> {
    package
        .graph()
        .state_clauses()
        .iter()
        .find(|candidate| {
            candidate.is_invariant()
                && candidate.node() == clause.node
                && candidate.claim() == clause.claim
        })
        .map(|found| {
            fsl_foundation::source::provenance::OccurrenceKey::new(
                found.identity(),
                found.claim().clone(),
            )
        })
        .ok_or_else(|| ClauseRootRefusal::Undeclared {
            name: clause.name.to_owned(),
        })
}
