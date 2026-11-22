// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-530, FR-940: the planner declaration of an authenticated composition.

use filament_contract_model::{
    NativeJoinBudget, NativeJoinChargeKind, NativeJoinLocus, NativeJoinLocusSource,
};
use fsl_semantics::check::CompositionRule;

use super::{ItemRole, PackageComposition, PackageCompositionRefusal, PlannerDeclaration};

/// Why an authenticated composition yields no planner declaration.
#[derive(Debug, thiserror::Error)]
pub enum AdmittedDeclarationRefusal {
    /// The actual rule is routed to a composition-capable provider instead.
    #[error("the rule `{0}` is not settled by the planner")]
    NotPlannerLocal(&'static str),
    /// Original cyclic, authenticated mismatch or native projection refusal.
    #[error(transparent)]
    Composition(#[from] PackageCompositionRefusal),
    /// A prepaid declaration output buffer could not reserve storage.
    #[error(transparent)]
    Allocation(#[from] std::collections::TryReserveError),
}

fn charge(
    budget: &mut NativeJoinBudget,
    kind: NativeJoinChargeKind,
    amount: u128,
    slot: Option<usize>,
) -> Result<(), AdmittedDeclarationRefusal> {
    let locus = NativeJoinLocus {
        source: slot.map_or(
            NativeJoinLocusSource::Package,
            NativeJoinLocusSource::AdapterRow,
        ),
        form: "admitted_planner_declaration",
    };
    budget
        .accounted_charge(kind, amount, locus)
        .map_err(|denied| {
            PackageCompositionRefusal::Native {
                denied,
                locus: budget.first_unpaid_locus(),
                configured_bound: match denied.kind {
                    NativeJoinChargeKind::Work => budget.work_limit(),
                    NativeJoinChargeKind::Storage => budget.storage_limit(),
                },
            }
            .into()
        })
}

impl PlannerDeclaration {
    /// Move authenticated package composition facts into their planner declaration.
    ///
    /// Like the source factory, this refuses a non-`assume-guarantee` rule
    /// before a cycle. It then recomputes the mismatch from the composition's
    /// actual retained clause and supply facts, before moving any field. No
    /// provider witness or caller-supplied mismatch is an authority.
    ///
    /// The package projection already paid for its node, package identity,
    /// environment, supplies and slot payloads; these are moved without copying.
    /// This call pays its guard visits, new declaration row, rule spelling and
    /// derived-slot buffer before inspecting, allocating or moving them. It
    /// never creates or resumes an allowance.
    ///
    /// # Errors
    ///
    /// The original rule/cycle refusal, authenticated mismatch refusal, native
    /// denial with its unchanged first-unpaid metadata, or paid allocation failure.
    pub fn from_admitted(
        composition: PackageComposition<'_, '_>,
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, AdmittedDeclarationRefusal> {
        charge(budget, NativeJoinChargeKind::Work, 1, None)?;
        if composition.rule() != CompositionRule::AssumeGuarantee {
            return Err(AdmittedDeclarationRefusal::NotPlannerLocal(
                composition.rule().as_str(),
            ));
        }
        charge(budget, NativeJoinChargeKind::Work, 1, None)?;
        if !composition.cycles().is_empty() {
            return Err(PackageCompositionRefusal::Cyclic.into());
        }
        let mismatch = composition.interface_mismatch_with_budget(budget)?;
        charge(budget, NativeJoinChargeKind::Work, 1, None)?;
        charge(budget, NativeJoinChargeKind::Storage, 1, None)?;

        let spelling = composition.rule().as_str();
        charge(
            budget,
            NativeJoinChargeKind::Work,
            spelling.len() as u128,
            None,
        )?;
        charge(
            budget,
            NativeJoinChargeKind::Storage,
            1 + spelling.len() as u128,
            None,
        )?;
        let mut rule = String::new();
        rule.try_reserve_exact(spelling.len())?;
        rule.push_str(spelling);

        let count = composition.slots.len();
        let mut slots = Vec::new();
        if count != 0 {
            charge(
                budget,
                NativeJoinChargeKind::Storage,
                1 + count as u128,
                None,
            )?;
            slots.try_reserve_exact(count)?;
        }
        charge(budget, NativeJoinChargeKind::Work, count as u128, None)?;
        for slot in composition.slots {
            slots.push(slot.derived);
        }
        Ok(Self {
            role: ItemRole::Composition,
            node: composition.node,
            package: composition.package,
            rule: Some(rule),
            clause: None,
            relation_sources: Vec::new(),
            relation_node: None,
            environment: composition.environment,
            slots,
            supplies: composition.supplies,
            mismatch,
        })
    }
}

#[cfg(test)]
mod admitted_declaration_tests {
    use super::super::admitted_fixture::{AdmittedCompositionFixture, CompositionCase};
    use super::super::{DeclarationRefusal, EnvSupplies, MismatchReason};
    use super::*;
    use fsl_package::{AnalysisView, BarePackage};

    fn ledger(work: u64, storage: u64) -> NativeJoinBudget {
        let mut budget = NativeJoinBudget::new(work, storage);
        budget.resume();
        budget
    }

    fn projection<'a, 'p>(
        bare: &'a BarePackage<'p>,
        budget: &mut NativeJoinBudget,
    ) -> PackageComposition<'a, 'a> {
        let named = bare.claim_name("Sys").expect("the actual composition name");
        let view = bare
            .composition_with_budget(&named.declaration, budget)
            .expect("bounded authenticated composition");
        PackageComposition::from_view(bare, &view, budget).expect("actual paid projection")
    }

    fn same_declaration(actual: &PlannerDeclaration, expected: &PlannerDeclaration) {
        assert_eq!(actual.role, expected.role);
        assert_eq!(actual.node, expected.node);
        assert_eq!(actual.package, expected.package);
        assert_eq!(actual.rule, expected.rule);
        assert!(actual.clause.is_none() && expected.clause.is_none());
        assert_eq!(actual.relation_sources, expected.relation_sources);
        assert_eq!(actual.relation_node, expected.relation_node);
        assert_eq!(actual.supplies, expected.supplies);
        assert_eq!(actual.mismatch, expected.mismatch);
        assert_eq!(actual.environment.len(), expected.environment.len());
        for (actual, expected) in actual.environment.iter().zip(&expected.environment) {
            assert_eq!(actual.name, expected.name);
            assert_eq!(actual.supplier, expected.supplier);
            match (&actual.supplies, &expected.supplies) {
                (EnvSupplies::Fairness(actual), EnvSupplies::Fairness(expected)) => {
                    assert_eq!(actual.constraint, expected.constraint);
                    assert_eq!(actual.operation, expected.operation);
                }
                (EnvSupplies::Assumption(actual), EnvSupplies::Assumption(expected)) => {
                    assert_eq!(actual, expected)
                }
                _ => panic!("the original environment supply kind is retained"),
            }
        }
        assert_eq!(actual.slots.len(), expected.slots.len());
        for (actual, expected) in actual.slots.iter().zip(&expected.slots) {
            assert_eq!(actual.role, expected.role);
            assert_eq!(actual.key, expected.key);
            assert_eq!(actual.backing, expected.backing);
            assert_eq!(actual.constraint, expected.constraint);
            assert_eq!(actual.operation, expected.operation);
            assert_eq!(actual.premise_subject, expected.premise_subject);
            assert_eq!(actual.peer_clause, expected.peer_clause);
            assert_eq!(actual.source, expected.source);
        }
    }

    /// Trace: FR-530-AC-1, FR-940-AC-1, FR-1044-AC-14
    #[test]
    fn admitted_declarations_move_paid_facts_equal_to_actual_source_declarations() {
        for (case, expected_mismatch) in [
            (CompositionCase::Acyclic, None),
            (CompositionCase::UnusedSupply, None),
            (CompositionCase::StepBounded, None),
            (
                CompositionCase::MissingInterface,
                Some((4, MismatchReason::Absent)),
            ),
            (
                CompositionCase::MissingSupply,
                Some((5, MismatchReason::Weaker)),
            ),
        ] {
            let fixture = AdmittedCompositionFixture::new(case);
            let mut budget = ledger(10_000_000, 10_000_000);
            let bare = fixture.bare(&mut budget).unwrap();
            let projected = projection(&bare, &mut budget);
            let expected =
                PlannerDeclaration::from_package(&fixture.source, projected.node()).unwrap();
            let package_buffer = projected.package.as_ptr();
            let supply_buffer = projected.supplies.as_ptr();
            let actual = PlannerDeclaration::from_admitted(projected, &mut budget).unwrap();
            same_declaration(&actual, &expected);
            assert_eq!(actual.role, ItemRole::Composition);
            assert_eq!(actual.rule.as_deref(), Some("assume-guarantee"));
            assert_eq!(actual.package, "test/composition");
            assert_eq!(
                actual.package.as_ptr(),
                package_buffer,
                "paid package identity is moved, not copied"
            );
            assert_eq!(
                actual.supplies.as_ptr(),
                supply_buffer,
                "paid supplied constraints keep their owned buffer"
            );
            assert_eq!(
                actual
                    .mismatch
                    .as_ref()
                    .map(|mismatch| (mismatch.premise, mismatch.reason)),
                expected_mismatch
            );
            if let Some(mismatch) = &actual.mismatch {
                assert_eq!(mismatch.component, "Producer");
                let source_clause = fixture.source.graph().compositions()[0]
                    .clauses()
                    .iter()
                    .find(|clause| {
                        clause.reference.component == "Producer" && clause.reference.name == "Sends"
                    })
                    .unwrap();
                assert_eq!(mismatch.interface, source_clause.reference.node);
            }
        }
    }

    /// Trace: FR-530-AC-1, FR-1044-AC-6
    #[test]
    fn admitted_declaration_preserves_nonlocal_rule_before_cycle_refusal() {
        for (case, nonlocal) in [
            (CompositionCase::Cyclic, false),
            (CompositionCase::InterfaceCyclic, true),
        ] {
            let fixture = AdmittedCompositionFixture::new(case);
            let mut budget = ledger(10_000_000, 10_000_000);
            let bare = fixture.bare(&mut budget).unwrap();
            let projected = projection(&bare, &mut budget);
            assert_eq!(projected.cycles().len(), 1);
            assert!(projected.slots().is_empty());
            assert!(projected
                .clauses()
                .iter()
                .all(|clause| clause.form.is_none()));
            let source = PlannerDeclaration::from_package(&fixture.source, projected.node());
            let before = budget.consumed();
            let actual = PlannerDeclaration::from_admitted(projected, &mut budget);
            if nonlocal {
                assert!(matches!(
                    source,
                    Err(DeclarationRefusal::NotPlannerLocal("interface"))
                ));
                assert!(matches!(
                    actual,
                    Err(AdmittedDeclarationRefusal::NotPlannerLocal("interface"))
                ));
                assert_eq!(budget.consumed().work, before.work + 1);
            } else {
                assert!(matches!(source, Err(DeclarationRefusal::Cyclic)));
                assert!(matches!(
                    actual,
                    Err(AdmittedDeclarationRefusal::Composition(
                        PackageCompositionRefusal::Cyclic
                    ))
                ));
                assert_eq!(budget.consumed().work, before.work + 2);
            }
            assert_eq!(
                budget.consumed().storage,
                before.storage,
                "guards do not prepare mismatch facts or outputs"
            );
        }
    }

    /// Trace: FR-530-AC-1, FR-940-AC-1
    #[test]
    fn admitted_declaration_exact_and_short_output_budgets_keep_native_denial_metadata() {
        let fixture = AdmittedCompositionFixture::new(CompositionCase::Acyclic);
        let mut measured = ledger(10_000_000, 10_000_000);
        let bare = fixture.bare(&mut measured).unwrap();
        let projected = projection(&bare, &mut measured);
        let count = projected.slots().len();
        assert!(count > 0);
        let expected = PlannerDeclaration::from_admitted(projected, &mut measured).unwrap();
        let paid = measured.consumed();
        let mut exact = ledger(paid.work, paid.storage);
        let bare = fixture.bare(&mut exact).unwrap();
        let projected = projection(&bare, &mut exact);
        let actual = PlannerDeclaration::from_admitted(projected, &mut exact).unwrap();
        same_declaration(&actual, &expected);
        assert_eq!(exact.consumed(), paid);

        for kind in [NativeJoinChargeKind::Work, NativeJoinChargeKind::Storage] {
            let mut short = ledger(
                paid.work - u64::from(kind == NativeJoinChargeKind::Work),
                paid.storage - u64::from(kind == NativeJoinChargeKind::Storage),
            );
            let bare = fixture.bare(&mut short).unwrap();
            let projected = projection(&bare, &mut short);
            let refusal = PlannerDeclaration::from_admitted(projected, &mut short).unwrap_err();
            let AdmittedDeclarationRefusal::Composition(PackageCompositionRefusal::Native {
                denied,
                locus,
                configured_bound,
            }) = refusal
            else {
                panic!("the new output's original native denial is retained");
            };
            assert_eq!(denied.kind, kind);
            assert_eq!(Some(denied), short.first_unpaid());
            assert_eq!(locus, short.first_unpaid_locus());
            assert_eq!(locus.unwrap().form, "admitted_planner_declaration");
            assert_eq!(
                configured_bound,
                match kind {
                    NativeJoinChargeKind::Work => paid.work - 1,
                    NativeJoinChargeKind::Storage => paid.storage - 1,
                }
            );
            assert_eq!(
                denied.amount,
                count as u128 + u128::from(kind == NativeJoinChargeKind::Storage)
            );
        }
    }

    /// Trace: FR-530-AC-1, FR-940-AC-1
    #[test]
    fn admitted_declaration_zero_remaining_allowance_and_prior_denial_do_not_reset_paid_input() {
        let fixture = AdmittedCompositionFixture::new(CompositionCase::Acyclic);
        let mut measured = ledger(10_000_000, 10_000_000);
        let bare = fixture.bare(&mut measured).unwrap();
        let _projected = projection(&bare, &mut measured);
        let prefix = measured.consumed();
        for kind in [NativeJoinChargeKind::Work, NativeJoinChargeKind::Storage] {
            let mut budget = ledger(
                if kind == NativeJoinChargeKind::Work {
                    prefix.work
                } else {
                    10_000_000
                },
                if kind == NativeJoinChargeKind::Storage {
                    prefix.storage
                } else {
                    10_000_000
                },
            );
            let bare = fixture.bare(&mut budget).unwrap();
            let projected = projection(&bare, &mut budget);
            assert_eq!(budget.consumed(), prefix);
            let refusal = PlannerDeclaration::from_admitted(projected, &mut budget).unwrap_err();
            let AdmittedDeclarationRefusal::Composition(PackageCompositionRefusal::Native {
                denied,
                locus,
                configured_bound,
            }) = refusal
            else {
                panic!("the same ledger stops preparation at its first unpaid operation");
            };
            assert_eq!(denied.kind, kind);
            assert_eq!(Some(denied), budget.first_unpaid());
            assert_eq!(locus, budget.first_unpaid_locus());
            assert_eq!(
                configured_bound,
                match kind {
                    NativeJoinChargeKind::Work => prefix.work,
                    NativeJoinChargeKind::Storage => prefix.storage,
                }
            );
            assert!(budget.consumed().work >= prefix.work);
            assert_eq!(budget.consumed().storage, prefix.storage);
        }
        let mut prior = ledger(10_000_000, prefix.storage);
        let bare = fixture.bare(&mut prior).unwrap();
        let projected = projection(&bare, &mut prior);
        let before = prior.consumed();
        let original_locus = NativeJoinLocus {
            source: NativeJoinLocusSource::ReachedRow(37),
            form: "caller_after_projection",
        };
        let original = prior
            .accounted_charge(NativeJoinChargeKind::Storage, 1, original_locus)
            .unwrap_err();
        let refusal = PlannerDeclaration::from_admitted(projected, &mut prior).unwrap_err();
        assert!(
            matches!(refusal, AdmittedDeclarationRefusal::Composition(PackageCompositionRefusal::Native {
            denied, locus: Some(actual), configured_bound,
        }) if denied == original && actual == original_locus && configured_bound == prefix.storage)
        );
        assert_eq!(prior.consumed(), before);
    }
}
