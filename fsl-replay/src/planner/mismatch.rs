// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-492 "Interface mismatch": the refutation of a composition by a
//! declared interface that is absent or weaker than a premise needs.
//!
//! The mismatch is decided from the checked declaration alone, never from a
//! result a request supplies, so the planner settles it and an analyze run
//! recomputes it to admit or refuse a provider's `interface-mismatch`
//! counterexample (FSpec FR-331). One order decides both, reason-major: the
//! first `absent` mismatch over the `rely` and `liveness-dependency` slots in
//! ascending slot order, and only when there is none the first `weaker` one
//! over the same slots.

use std::convert::Infallible;

use filament_contract_model::{
    NativeJoinBudget, NativeJoinChargeKind, NativeJoinDeniedCharge, NativeJoinLocus,
    NativeJoinLocusSource,
};
use filament_exact::{Identifier, NodeKey};
use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use fsl_foundation::node_wire::node_key_to_json;
use fsl_semantics::check::{CheckedComposition, ClauseConstraint, ClauseRef, SlotRole};
use serde_json::{json, Value};

use super::declaration::strength_implies;
use super::key::KeyWireError;

/// Why a declared interface does not meet a premise (FSpec FR-492 "Interface
/// mismatch").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MismatchReason {
    /// The declaration has no peer guarantee at the clause the premise names.
    Absent,
    /// The peer's declared guarantee is conditional on a fairness constraint
    /// the composition supplies nothing that implies.
    Weaker,
}

impl MismatchReason {
    /// The `reason` spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Weaker => "weaker",
        }
    }
}

/// A declared interface mismatch: FSpec FR-331's `interface-mismatch`
/// counterexample, and the planner result's `mismatch` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterfaceMismatch {
    /// The peer component `D`.
    pub component: String,
    /// The node of the peer guarantee's clause `H`.
    pub interface: NodeKey,
    /// The index, in the item's `premises`, of the slot that needs the peer.
    pub premise: usize,
    /// Why the interface does not meet it.
    pub reason: MismatchReason,
}

impl InterfaceMismatch {
    /// The mismatch's members, as the planner result's `mismatch` member
    /// spells them (FSpec FR-492 "Planner result").
    ///
    /// # Errors
    ///
    /// [`KeyWireError`] when the interface node cannot be written.
    pub fn to_members(&self) -> Result<Value, KeyWireError> {
        Ok(json!({
            "component": self.component,
            "interface": node_key_to_json(&self.interface)?,
            "premise": self.premise,
            "reason": self.reason.as_str(),
        }))
    }

    /// The mismatch as FSpec FR-331 spells the `interface-mismatch`
    /// counterexample.
    ///
    /// # Errors
    ///
    /// [`KeyWireError`] when the interface node cannot be written.
    pub fn to_wire(&self) -> Result<Value, KeyWireError> {
        let mut wire = self.to_members()?;
        if let Some(members) = wire.as_object_mut() {
            members.insert("type".to_owned(), json!("interface-mismatch"));
        }
        Ok(wire)
    }
}

/// The first interface mismatch `composition` declares, in FSpec FR-492's
/// reason-major order; `None` when no premise names an absent or weaker
/// interface.
pub fn interface_mismatch(composition: &CheckedComposition) -> Option<InterfaceMismatch> {
    let slots = composition.slots();
    let needs: Vec<_> = slots
        .iter()
        .filter(|slot| matches!(slot.role, SlotRole::Rely | SlotRole::LivenessDependency))
        .filter_map(|slot| {
            Some(MismatchNeed {
                premise: slot.index,
                peer: slot.on.as_ref()?,
            })
        })
        .collect();
    let declared: Vec<_> = composition
        .guarantees()
        .iter()
        .map(|row| &row.reference)
        .collect();
    let clauses: Vec<_> = composition
        .clauses()
        .iter()
        .map(|row| MismatchClauseFairness {
            peer: &row.reference,
            needed: &row.fairness,
        })
        .collect();
    let supplies: Vec<_> = composition
        .supplies()
        .iter()
        .map(|row| MismatchSupply {
            context: &row.context,
            operation: &row.operation,
            kind: row.kind,
            granularity: row.granularity,
        })
        .collect();
    let facts = MismatchFacts {
        needs: &needs,
        declared: &declared,
        clauses: &clauses,
        supplies: &supplies,
    };
    let decision = match first_mismatch(facts, &mut Unmetered) {
        Ok(decision) => decision,
        Err(never) => match never {},
    };
    decision.map(|decision| InterfaceMismatch {
        component: decision.peer.component.clone(),
        interface: decision.peer.node.clone(),
        premise: decision.premise,
        reason: decision.reason,
    })
}

/// An authenticated peer interface needed by an original full premise slot.
#[derive(Clone, Copy)]
pub struct MismatchNeed<'a> {
    /// Index in the complete derived slot list, before filtering its roles.
    pub premise: usize,
    /// Complete component, clause name and structural clause identity.
    pub peer: &'a ClauseRef,
}

/// Actual fairness facts read for a peer clause.
#[derive(Clone, Copy)]
pub struct MismatchClauseFairness<'a> {
    /// Complete identity of the clause whose fairness was read.
    pub peer: &'a ClauseRef,
    /// The clause's own admitted constraints, in their retained order.
    pub needed: &'a [ClauseConstraint],
}

/// One actual composition `supplies` member, without source-only span data.
#[derive(Clone, Copy)]
pub struct MismatchSupply<'a> {
    /// Original structural identity of the operation's declaring context.
    pub context: &'a NodeKey,
    /// Original admitted operation identifier.
    pub operation: &'a Identifier,
    /// Supplied fairness kind.
    pub kind: FairnessKind,
    /// Supplied fairness granularity.
    pub granularity: FairnessGranularity,
}

/// Borrowed declaration facts, independent of requests or provider answers.
///
/// Adapters supply needs in original full-slot order, retain complete clause
/// identities, and include only actual `supplies` members, never environment
/// records or pool results. Package adapters must read complete authenticated
/// clause facts or return their typed projection refusal before calling this
/// API. An unread, unsupported or refused fact is never an empty fairness set.
/// The source checker's legacy absent-clause-facts case remains vacuously met.
#[derive(Clone, Copy)]
pub struct MismatchFacts<'a> {
    /// Peer needs in ascending original premise-slot order.
    pub needs: &'a [MismatchNeed<'a>],
    /// Actual declared guarantee interfaces.
    pub declared: &'a [&'a ClauseRef],
    /// Actual peer clause fairness rows.
    pub clauses: &'a [MismatchClauseFairness<'a>],
    /// Actual composition supplies, with original context and operation identities.
    pub supplies: &'a [MismatchSupply<'a>],
}

/// A bounded interface decision stopped before an unpaid operation or copy.
#[derive(Debug, thiserror::Error)]
pub enum MismatchRefusal {
    /// The original cumulative native ledger denial, without a diagnostic allocation.
    #[error(
        "native mismatch charge denied: {denied:?} (bound {configured_bound}, locus {locus:?})"
    )]
    Native {
        /// Original first unpaid resource and attempted amount.
        denied: NativeJoinDeniedCharge,
        /// Original non-owning denial locus, including an earlier caller phase.
        locus: Option<NativeJoinLocus>,
        /// Original configured ceiling for the denied resource.
        configured_bound: u64,
    },
    /// The prepaid winning component string could not reserve storage.
    #[error(transparent)]
    Allocation(#[from] std::collections::TryReserveError),
}

/// Recompute the first declared mismatch from authenticated neutral facts.
///
/// Shares the source wrapper's reason-major decision: all absent interfaces
/// precede weaker fairness interfaces. Full clause identities and original
/// operation context/identifier pairs are compared before `strength_implies`.
/// This is independent of any canonical supplied-constraint fallback.
///
/// The caller pays its owned fact rows, names and structural-key conversions
/// before passing borrowed facts here. This function pays row visits and actual
/// identity/operation/strength comparisons, then only the winning output row
/// and component UTF-8 copy. The interface key is a shallow existing `NodeKey`
/// handle. No allowance is created or resumed.
///
/// # Errors
///
/// Returns the original native denial or paid string reservation failure.
/// Package read failures must remain typed adapter refusals, never missing facts
/// or `None`. Provider admission must compare the full independently recomputed
/// witness; this function accepts no claimed witness or authority Boolean.
pub fn interface_mismatch_over(
    facts: MismatchFacts<'_>,
    budget: &mut NativeJoinBudget,
) -> Result<Option<InterfaceMismatch>, MismatchRefusal> {
    let mut provision = Metered { budget };
    let Some(decision) = first_mismatch(facts, &mut provision)? else {
        return Ok(None);
    };
    provision.work(1, Some(decision.premise))?;
    provision.work(
        decision.peer.component.len() as u128,
        Some(decision.premise),
    )?;
    provision.charge(NativeJoinChargeKind::Storage, 1, Some(decision.premise))?;
    if !decision.peer.component.is_empty() {
        provision.charge(
            NativeJoinChargeKind::Storage,
            1 + decision.peer.component.len() as u128,
            Some(decision.premise),
        )?;
    }
    let mut component = String::new();
    component.try_reserve_exact(decision.peer.component.len())?;
    component.push_str(&decision.peer.component);
    Ok(Some(InterfaceMismatch {
        component,
        interface: decision.peer.node.clone(),
        premise: decision.premise,
        reason: decision.reason,
    }))
}

struct Decision<'a> {
    peer: &'a ClauseRef,
    premise: usize,
    reason: MismatchReason,
}

trait Provision {
    type Error;
    fn work(&mut self, amount: u128, premise: Option<usize>) -> Result<(), Self::Error>;
}

struct Unmetered;

impl Provision for Unmetered {
    type Error = Infallible;
    fn work(&mut self, _: u128, _: Option<usize>) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct Metered<'a> {
    budget: &'a mut NativeJoinBudget,
}

impl Metered<'_> {
    fn charge(
        &mut self,
        kind: NativeJoinChargeKind,
        amount: u128,
        premise: Option<usize>,
    ) -> Result<(), MismatchRefusal> {
        let locus = NativeJoinLocus {
            source: premise.map_or(
                NativeJoinLocusSource::Package,
                NativeJoinLocusSource::AdapterRow,
            ),
            form: "composition_interface_mismatch",
        };
        self.budget
            .accounted_charge(kind, amount, locus)
            .map_err(|denied| MismatchRefusal::Native {
                denied,
                locus: self.budget.first_unpaid_locus(),
                configured_bound: match denied.kind {
                    NativeJoinChargeKind::Work => self.budget.work_limit(),
                    NativeJoinChargeKind::Storage => self.budget.storage_limit(),
                },
            })
    }
}

impl Provision for Metered<'_> {
    type Error = MismatchRefusal;
    fn work(&mut self, amount: u128, premise: Option<usize>) -> Result<(), Self::Error> {
        self.charge(NativeJoinChargeKind::Work, amount, premise)
    }
}

fn same_clause<P: Provision>(
    left: &ClauseRef,
    right: &ClauseRef,
    premise: usize,
    provision: &mut P,
) -> Result<bool, P::Error> {
    provision.work(1, Some(premise))?;
    if left.component != right.component {
        return Ok(false);
    }
    provision.work(1, Some(premise))?;
    if left.name != right.name {
        return Ok(false);
    }
    provision.work(1, Some(premise))?;
    Ok(left.node == right.node)
}

fn first_mismatch<'a, P: Provision>(
    facts: MismatchFacts<'a>,
    provision: &mut P,
) -> Result<Option<Decision<'a>>, P::Error> {
    // Even empty facts preserve an earlier caller denial before inspection.
    provision.work(1, None)?;
    for need in facts.needs {
        provision.work(1, Some(need.premise))?;
        let mut declared = false;
        for peer in facts.declared {
            provision.work(1, Some(need.premise))?;
            if same_clause(peer, need.peer, need.premise, provision)? {
                declared = true;
                break;
            }
        }
        if !declared {
            return Ok(Some(Decision {
                peer: need.peer,
                premise: need.premise,
                reason: MismatchReason::Absent,
            }));
        }
    }
    for need in facts.needs {
        provision.work(1, Some(need.premise))?;
        let mut clause = None;
        for row in facts.clauses {
            provision.work(1, Some(need.premise))?;
            if same_clause(row.peer, need.peer, need.premise, provision)? {
                clause = Some(row);
                break;
            }
        }
        // Preserve the source wrapper's legacy missing-clause-facts behavior.
        let Some(clause) = clause else {
            continue;
        };
        for needed in clause.needed {
            provision.work(1, Some(need.premise))?;
            let mut supplied = false;
            for candidate in facts.supplies {
                provision.work(1, Some(need.premise))?;
                provision.work(1, Some(need.premise))?;
                if *candidate.context != needed.context {
                    continue;
                }
                provision.work(1, Some(need.premise))?;
                if *candidate.operation != needed.operation {
                    continue;
                }
                provision.work(1, Some(need.premise))?;
                if strength_implies(
                    (candidate.kind, candidate.granularity),
                    (needed.constraint.kind(), needed.constraint.granularity()),
                ) {
                    supplied = true;
                    break;
                }
            }
            if !supplied {
                return Ok(Some(Decision {
                    peer: need.peer,
                    premise: need.premise,
                    reason: MismatchReason::Weaker,
                }));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod mismatch_budget_tests {
    use super::super::admitted_fixture::{AdmittedCompositionFixture, CompositionCase};
    use super::super::tests::{constraint, node_in};
    use super::*;

    fn ledger(work: u64, storage: u64) -> NativeJoinBudget {
        let mut budget = NativeJoinBudget::new(work, storage);
        budget.resume();
        budget
    }

    fn peer(name: &str) -> ClauseRef {
        ClauseRef {
            component: "Producer".into(),
            name: name.into(),
            node: node_in("producer", name),
        }
    }

    fn needed(
        context: &NodeKey,
        kind: FairnessKind,
        granularity: FairnessGranularity,
    ) -> ClauseConstraint {
        ClauseConstraint {
            context: context.clone(),
            operation: Identifier::new("inc").unwrap(),
            constraint: constraint(kind, granularity),
        }
    }

    fn witness(
        peer: &ClauseRef,
        premise: usize,
        reason: MismatchReason,
    ) -> Option<InterfaceMismatch> {
        Some(InterfaceMismatch {
            component: peer.component.clone(),
            interface: peer.node.clone(),
            premise,
            reason,
        })
    }

    /// Trace: FR-1044-AC-14
    #[test]
    fn mismatch_reason_major_oracle_prefers_later_absent_to_earlier_weaker() {
        let fenced = peer("Fenced");
        let pings = peer("Pings");
        let context = node_in("producer", "Counter");
        let required = [needed(
            &context,
            FairnessKind::Strong,
            FairnessGranularity::Each,
        )];
        let needs = [
            MismatchNeed {
                premise: 3,
                peer: &fenced,
            },
            MismatchNeed {
                premise: 4,
                peer: &pings,
            },
        ];
        let clauses = [
            MismatchClauseFairness {
                peer: &fenced,
                needed: &required,
            },
            MismatchClauseFairness {
                peer: &pings,
                needed: &[],
            },
        ];
        let declared = [&fenced];
        let facts = MismatchFacts {
            needs: &needs,
            declared: &declared,
            clauses: &clauses,
            supplies: &[],
        };
        assert_eq!(
            interface_mismatch_over(facts, &mut ledger(10_000, 10_000)).unwrap(),
            witness(&pings, 4, MismatchReason::Absent)
        );
        assert_eq!(
            interface_mismatch_over(
                MismatchFacts {
                    declared: &[],
                    ..facts
                },
                &mut ledger(10_000, 10_000)
            )
            .unwrap(),
            witness(&fenced, 3, MismatchReason::Absent)
        );
        let both = [&fenced, &pings];
        assert_eq!(
            interface_mismatch_over(
                MismatchFacts {
                    declared: &both,
                    ..facts
                },
                &mut ledger(10_000, 10_000)
            )
            .unwrap(),
            witness(&fenced, 3, MismatchReason::Weaker)
        );
        // Equal full peer identities still report their first original slot.
        let repeated = [
            MismatchNeed {
                premise: 3,
                peer: &fenced,
            },
            MismatchNeed {
                premise: 4,
                peer: &fenced,
            },
        ];
        assert_eq!(
            interface_mismatch_over(
                MismatchFacts {
                    needs: &repeated,
                    ..facts
                },
                &mut ledger(10_000, 10_000)
            )
            .unwrap(),
            witness(&fenced, 3, MismatchReason::Weaker)
        );
    }

    /// Trace: FR-1044-AC-14
    #[test]
    fn mismatch_compares_complete_clause_and_original_operation_identities() {
        let original = peer("Shared");
        let needs = [MismatchNeed {
            premise: 7,
            peer: &original,
        }];
        let mut component = original.clone();
        component.component = "Other".into();
        let mut name = original.clone();
        name.name = "Different".into();
        let mut owner = original.clone();
        owner.node = node_in("other-owner", "Shared");
        for changed in [component, name, owner] {
            let declared = [&changed];
            let facts = MismatchFacts {
                needs: &needs,
                declared: &declared,
                clauses: &[],
                supplies: &[],
            };
            assert_eq!(
                interface_mismatch_over(facts, &mut ledger(10_000, 10_000)).unwrap(),
                witness(&original, 7, MismatchReason::Absent)
            );
        }
        let context = node_in("producer", "Counter");
        let foreign = node_in("other-owner", "Counter");
        let operation = Identifier::new("inc").unwrap();
        let other_operation = Identifier::new("other").unwrap();
        let required = [needed(
            &context,
            FairnessKind::Weak,
            FairnessGranularity::Whole,
        )];
        let clauses = [MismatchClauseFairness {
            peer: &original,
            needed: &required,
        }];
        let declared = [&original];
        for (context, operation) in [(&foreign, &operation), (&context, &other_operation)] {
            let supplies = [MismatchSupply {
                context,
                operation,
                kind: FairnessKind::Strong,
                granularity: FairnessGranularity::Each,
            }];
            let facts = MismatchFacts {
                needs: &needs,
                declared: &declared,
                clauses: &clauses,
                supplies: &supplies,
            };
            assert_eq!(
                interface_mismatch_over(facts, &mut ledger(10_000, 10_000)).unwrap(),
                witness(&original, 7, MismatchReason::Weaker)
            );
        }
    }

    /// Trace: FR-1044-AC-14
    #[test]
    fn mismatch_strength_matrix_matches_the_existing_fairness_law() {
        let peer = peer("Sends");
        let context = node_in("producer", "Counter");
        let operation = Identifier::new("inc").unwrap();
        let needs = [MismatchNeed {
            premise: 3,
            peer: &peer,
        }];
        let declared = [&peer];
        let strengths = [
            (FairnessKind::Weak, FairnessGranularity::Whole),
            (FairnessKind::Weak, FairnessGranularity::Each),
            (FairnessKind::Strong, FairnessGranularity::Whole),
            (FairnessKind::Strong, FairnessGranularity::Each),
        ];
        // Rows are supplies, columns needs. This frozen oracle is not computed
        // by strength_implies or by the new decision core.
        let implies = [
            [true, false, false, false],
            [true, true, false, false],
            [true, false, true, false],
            [true, true, true, true],
        ];
        for (row, &(kind, granularity)) in strengths.iter().enumerate() {
            let supplies = [MismatchSupply {
                context: &context,
                operation: &operation,
                kind,
                granularity,
            }];
            for (column, &(kind, granularity)) in strengths.iter().enumerate() {
                let required = [needed(&context, kind, granularity)];
                let clauses = [MismatchClauseFairness {
                    peer: &peer,
                    needed: &required,
                }];
                let facts = MismatchFacts {
                    needs: &needs,
                    declared: &declared,
                    clauses: &clauses,
                    supplies: &supplies,
                };
                let expected = if implies[row][column] {
                    None
                } else {
                    witness(&peer, 3, MismatchReason::Weaker)
                };
                assert_eq!(
                    interface_mismatch_over(facts, &mut ledger(10_000, 10_000)).unwrap(),
                    expected,
                    "supply row {row}, need column {column}"
                );
            }
        }
    }

    /// Trace: FR-1044-AC-14
    #[test]
    fn mismatch_keeps_empty_and_legacy_missing_clause_facts_vacuously_met() {
        let peer = peer("Sends");
        let needs = [MismatchNeed {
            premise: 3,
            peer: &peer,
        }];
        let declared = [&peer];
        let empty = [MismatchClauseFairness {
            peer: &peer,
            needed: &[],
        }];
        for clauses in [&[][..], &empty[..]] {
            let facts = MismatchFacts {
                needs: &needs,
                declared: &declared,
                clauses,
                supplies: &[],
            };
            let mut budget = ledger(10_000, 0);
            assert_eq!(interface_mismatch_over(facts, &mut budget).unwrap(), None);
            assert_eq!(
                budget.consumed().storage,
                0,
                "no owned output means no output allocation charge"
            );
        }
        let facts = MismatchFacts {
            needs: &[],
            declared: &[],
            clauses: &[],
            supplies: &[],
        };
        let mut prior = ledger(11, 5);
        let locus = NativeJoinLocus {
            source: NativeJoinLocusSource::ReachedRow(37),
            form: "caller_selection",
        };
        let original = prior
            .accounted_charge(NativeJoinChargeKind::Storage, 6, locus)
            .unwrap_err();
        let before = prior.consumed();
        assert!(matches!(interface_mismatch_over(facts, &mut prior),
            Err(MismatchRefusal::Native { denied, locus: Some(actual), configured_bound: 5 })
                if denied == original && actual == locus));
        assert_eq!(prior.consumed(), before);
    }

    /// Trace: FR-1044-AC-14
    #[test]
    fn mismatch_exact_short_and_zero_budgets_preserve_original_native_metadata() {
        let mut peer = peer("Sends");
        peer.component = "Produçer".into();
        let context = node_in("producer", "Counter");
        let operation = Identifier::new("inc").unwrap();
        let required = [needed(
            &context,
            FairnessKind::Strong,
            FairnessGranularity::Each,
        )];
        let clauses = [MismatchClauseFairness {
            peer: &peer,
            needed: &required,
        }];
        let supplies = [MismatchSupply {
            context: &context,
            operation: &operation,
            kind: FairnessKind::Weak,
            granularity: FairnessGranularity::Whole,
        }];
        let needs = [MismatchNeed {
            premise: 3,
            peer: &peer,
        }];
        let declared = [&peer];
        let facts = MismatchFacts {
            needs: &needs,
            declared: &declared,
            clauses: &clauses,
            supplies: &supplies,
        };
        let mut measured = ledger(10_000, 10_000);
        let expected = witness(&peer, 3, MismatchReason::Weaker);
        assert_eq!(
            interface_mismatch_over(facts, &mut measured).unwrap(),
            expected
        );
        let paid = measured.consumed();
        assert_eq!(
            paid.storage,
            2 + u64::try_from(peer.component.len()).unwrap()
        );
        let mut exact = ledger(paid.work, paid.storage);
        assert_eq!(
            interface_mismatch_over(facts, &mut exact).unwrap(),
            expected
        );
        assert_eq!(exact.consumed(), paid);
        for kind in [NativeJoinChargeKind::Work, NativeJoinChargeKind::Storage] {
            for bound in [
                0,
                match kind {
                    NativeJoinChargeKind::Work => paid.work - 1,
                    NativeJoinChargeKind::Storage => paid.storage - 1,
                },
            ] {
                let mut short = ledger(
                    if kind == NativeJoinChargeKind::Work {
                        bound
                    } else {
                        paid.work
                    },
                    if kind == NativeJoinChargeKind::Storage {
                        bound
                    } else {
                        paid.storage
                    },
                );
                let refusal = interface_mismatch_over(facts, &mut short).unwrap_err();
                let MismatchRefusal::Native {
                    denied,
                    locus,
                    configured_bound,
                } = refusal
                else {
                    panic!("the caller's original native ledger denies before copying");
                };
                assert_eq!(denied.kind, kind);
                assert_eq!(Some(denied), short.first_unpaid());
                assert_eq!(locus, short.first_unpaid_locus());
                assert_eq!(configured_bound, bound);
                assert_eq!(locus.unwrap().form, "composition_interface_mismatch");
                assert!(denied.amount > 0);
                if kind == NativeJoinChargeKind::Work && bound == 0 {
                    assert_eq!(denied.amount, 1);
                    assert_eq!(short.consumed().storage, 0);
                }
            }
        }
        let mut reused = ledger(paid.work * 2, paid.storage * 2 - 1);
        interface_mismatch_over(facts, &mut reused).unwrap();
        assert!(matches!(
            interface_mismatch_over(facts, &mut reused),
            Err(MismatchRefusal::Native {
                denied: NativeJoinDeniedCharge {
                    kind: NativeJoinChargeKind::Storage,
                    ..
                },
                ..
            })
        ));
    }

    /// Trace: FR-530-AC-1, FR-1044-AC-14
    #[test]
    fn mismatch_source_wrapper_and_bounded_facts_preserve_genuine_supplied_interfaces() {
        let fixture = AdmittedCompositionFixture::new(CompositionCase::Acyclic);
        let [composition] = fixture.source.graph().compositions() else {
            panic!("one real checked source composition");
        };
        let slots = composition.slots();
        let needs: Vec<_> = slots
            .iter()
            .filter(|slot| matches!(slot.role, SlotRole::Rely | SlotRole::LivenessDependency))
            .map(|slot| MismatchNeed {
                premise: slot.index,
                peer: slot.on.as_ref().unwrap(),
            })
            .collect();
        assert_eq!(
            needs.iter().map(|need| need.premise).collect::<Vec<_>>(),
            [4, 5]
        );
        assert_eq!(
            needs
                .iter()
                .map(|need| need.peer.to_string())
                .collect::<Vec<_>>(),
            ["Producer::Shared", "Producer::Sends"]
        );
        let declared: Vec<_> = composition
            .guarantees()
            .iter()
            .map(|row| &row.reference)
            .collect();
        let clauses: Vec<_> = composition
            .clauses()
            .iter()
            .map(|row| MismatchClauseFairness {
                peer: &row.reference,
                needed: &row.fairness,
            })
            .collect();
        let supplies: Vec<_> = composition
            .supplies()
            .iter()
            .map(|row| MismatchSupply {
                context: &row.context,
                operation: &row.operation,
                kind: row.kind,
                granularity: row.granularity,
            })
            .collect();
        let sends = clauses.iter().find(|row| row.peer.name == "Sends").unwrap();
        assert_eq!(sends.needed.len(), 1);
        assert_eq!(supplies.len(), 1);
        assert_eq!(sends.needed[0].context, *supplies[0].context);
        assert_eq!(sends.needed[0].operation, *supplies[0].operation);
        assert_eq!(
            (
                sends.needed[0].constraint.kind(),
                sends.needed[0].constraint.granularity()
            ),
            (FairnessKind::Weak, FairnessGranularity::Each)
        );
        assert_eq!(
            (supplies[0].kind, supplies[0].granularity),
            (FairnessKind::Weak, FairnessGranularity::Each)
        );
        // Both interfaces are declared; Shared has no fairness, and the actual
        // Sends weak/each operation is supplied. None is an independent oracle.
        assert_eq!(interface_mismatch(composition), None);
        let facts = MismatchFacts {
            needs: &needs,
            declared: &declared,
            clauses: &clauses,
            supplies: &supplies,
        };
        assert_eq!(
            interface_mismatch_over(facts, &mut ledger(10_000, 10_000)).unwrap(),
            None
        );
    }
}
