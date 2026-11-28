// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-172 (TC-597): the aliases, object parameters, instances and `match`
//! conjuncts of ADR-023's checked clauses.

use fsl_eval::model_subject::ModelInstance;
use fsl_eval::simulation::model::ModelSystem;
use fsl_forms::QuantifierKind;
use fsl_semantics::check::{check_hyper_clauses, CheckedHyper};
use ix_trace_rs::trace;

use crate::hyper_world::{
    declarations, document, key, non_interference, opaque, other_document, unit_text, Vault, World,
    EQUAL_INPUTS, STARTS,
};

/// FR-172-AC-1: ADR-023 section 1's `NonInterference` checks with the one
/// alias `V`, two universal quantifiers over it, one instance for the
/// universe `{v}`, and both `match` conjuncts universal. The power comparison
/// checks with the aliases `S` and `N`, and section 8.2's `Opaque` with its
/// one conjunct existential.
#[trace("TC-597", "FR-172-AC-1")]
#[test]
fn the_adr_clauses_check_with_their_aliases_instances_and_match_conjuncts() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let clause = &world.clause;
    let aliases: Vec<&str> = clause
        .aliases()
        .iter()
        .map(|alias| alias.as_str())
        .collect();
    assert_eq!(aliases, ["V"]);
    let prefix: Vec<_> = clause
        .quantifiers()
        .iter()
        .map(|quantifier| {
            (
                quantifier.kind,
                quantifier.variable.as_str(),
                quantifier.alias.as_str(),
            )
        })
        .collect();
    assert_eq!(
        prefix,
        [
            (QuantifierKind::Forall, "a", "V"),
            (QuantifierKind::Forall, "b", "V"),
        ]
    );
    let [parameter] = clause.parameters() else {
        panic!("one object parameter");
    };
    assert_eq!(
        (parameter.name.as_str(), parameter.alias.as_str()),
        ("v", "V")
    );
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    assert_eq!(
        system
            .instances(&filament_exact::ValueType::Reference(
                parameter.object.clone()
            ))
            .expect("the vaults have a universe"),
        [ModelInstance {
            population: key("vaults"),
            key: "v".to_owned(),
        }]
    );
    let universal: Vec<Vec<&str>> = clause
        .mu_universal()
        .iter()
        .map(|conjunct| conjunct.variables().iter().map(String::as_str).collect())
        .collect();
    assert_eq!(universal, [["a", "b"], ["a", "b"]]);
    assert!(clause.mu_existential().is_empty());

    let first = document(false, false);
    let second = other_document(&first);
    let text = unit_text(
        Some(&second),
        Vault::Secure,
        false,
        "hyper SavesPower using inf over behaviours (d: S::Vault, e: N::Vault) \
         forall trace s of S forall trace n of N \
         match { s.step.Vault::step.i = n.step.Vault::step.i } \
         { always holds(d.l @ s <= e.l @ n) }\n",
    )
    .replace("model V = ", "model S = ")
    .replace("V::", "S::");
    let power = match check_hyper_clauses(&declarations(&[&first, &second], &text)).pop() {
        Some(Ok(CheckedHyper::Behaviours(clause))) => *clause,
        other => panic!("the power comparison checks: {other:?}"),
    };
    let aliases: Vec<&str> = power.aliases().iter().map(|alias| alias.as_str()).collect();
    assert_eq!(aliases, ["S", "N"]);

    let world = World::new(Vault::Secure, false, &STARTS, &opaque(""));
    assert!(world.clause.mu_universal().is_empty());
    let existential: Vec<Vec<&str>> = world
        .clause
        .mu_existential()
        .iter()
        .map(|conjunct| conjunct.variables().iter().map(String::as_str).collect())
        .collect();
    assert_eq!(existential, [["a", "b"]]);
}
