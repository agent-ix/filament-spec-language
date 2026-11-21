// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original/effective correspondence reached from authored model units.

use filament_checked::key::DeclarationKey;
use fsl_foundation::SourceIdentity;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{
    admit_unit, compile_model_unit_with_causes, package_input, SelectedModels, UnitIntakeCause,
    UnitIntakeRefusal,
};
use fsl_semantics::model::key::{effective_id_of, RULE_INHERIT, RULE_QUALIFY, RULE_REDEFINE};
use fsl_semantics::model::normalize::{EffectiveView, ModelRefusalCause, ViewEntry};
use ix_trace_rs::trace;
use std::collections::BTreeSet;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\ndomain \"example/identity\";\n";

fn selected(declarations: &str) -> Result<SelectedModels, UnitIntakeRefusal> {
    let source = format!("{HEADER}{declarations}");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "identity-model"),
        "identity.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("model source metadata admits");
    compile_model_unit_with_causes(&parsed, ModelNormalizationLimits::default())
        .expect("authored declarations compile");
    let packages = package_input([source.as_bytes()]).expect("model unit enters package input");
    let selection = fsl_cst::parse(
        SourceIdentity::new("test", "identity-selection"),
        "selection.fsl",
        b"language \"ix:native\" edition \"1-draft\"; model M = \"example/identity\";",
        fsl_cst::Limits::default(),
    )
    .expect("selection parses");
    let built = fsl_forms::build_unit(&selection).expect("selection builds");
    admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
}

fn view(declarations: &str) -> EffectiveView {
    let selected = selected(declarations).expect("model normalizes at source selection");
    assert_eq!(selected.len(), 1);
    selected[0].view.clone()
}

fn key(view: &EffectiveView, name: &str) -> DeclarationKey {
    view.domain_package()
        .records
        .iter()
        .map(|record| record.key())
        .find(|key| key.node == format!("ix://example/identity/{name}"))
        .unwrap_or_else(|| panic!("source declaration {name} is retained"))
        .clone()
}

fn entry<'a>(view: &'a EffectiveView, owner: &str, original: &str) -> &'a ViewEntry {
    let owner = &view.type_identities()[&key(view, owner)];
    let original = key(view, original);
    view.declarations()
        .iter()
        .find(|entry| {
            entry.preimage.owner_effective_type.as_ref() == Some(owner)
                && entry.preimage.original == original
        })
        .expect("the owner/original correspondence exists")
}

#[test]
#[trace("FR-081-AC-1", "FR-081-AC-2", "FR-710-AC-3")]
fn inherited_source_member_keeps_original_and_changes_effective_owner() {
    let view = view("object Base { field: Integer; } object Child extends Base {}");
    let base = entry(&view, "Base", "Base/field");
    let child = entry(&view, "Child", "Base/field");
    assert_eq!(base.preimage.original, child.preimage.original);
    assert_ne!(base.effective_id, child.effective_id);
    for member in [base, child] {
        assert_eq!(
            member.effective_id,
            effective_id_of(
                member.preimage.owner_effective_type.clone(),
                &member.preimage.original
            )
        );
    }
    assert_eq!(base.preimage.derivation.len(), 1);
    assert_eq!(base.preimage.derivation[0].rule.identity, RULE_QUALIFY);
    assert_eq!(child.preimage.derivation.len(), 1);
    assert_eq!(child.preimage.derivation[0].rule.identity, RULE_INHERIT);
    assert_eq!(
        child.preimage.derivation[0].parent.as_ref(),
        Some(&base.effective_id)
    );
    let originals: BTreeSet<_> = view
        .domain_package()
        .records
        .iter()
        .map(|r| r.key())
        .collect();
    let referenced: BTreeSet<_> = view
        .declarations()
        .iter()
        .map(|e| &e.preimage.original)
        .collect();
    assert_eq!(referenced, originals);
    for entry in view.declarations() {
        for fact in &entry.preimage.derivation {
            assert!(fact.inputs.iter().all(|input| originals.contains(input)));
        }
    }
}

#[test]
#[trace("FR-081-AC-2")]
fn equally_shaped_source_fields_have_distinct_original_and_effective_keys() {
    let view = view("object Item { first: Integer; second: Integer; }");
    let first = entry(&view, "Item", "Item/first");
    let second = entry(&view, "Item", "Item/second");
    assert_eq!(
        first.preimage.owner_effective_type,
        second.preimage.owner_effective_type
    );
    assert_ne!(first.preimage.original, second.preimage.original);
    assert_ne!(first.effective_id, second.effective_id);
}

#[test]
#[trace("FR-081-AC-3", "FR-710-AC-3")]
fn most_specific_source_redefinition_retains_hidden_correspondence() {
    let view = view(
        "object Base { field: Integer; }
        object Mid extends Base { field: Integer redefines Base::field; }
        object Leaf extends Mid { field: Integer redefines Base::field; }",
    );
    let base = entry(&view, "Leaf", "Base/field");
    let mid = entry(&view, "Leaf", "Mid/field");
    let leaf = entry(&view, "Leaf", "Leaf/field");
    assert!(!base.visible);
    assert!(!mid.visible);
    assert!(leaf.visible);
    assert_ne!(mid.effective_id, leaf.effective_id);
    assert_ne!(base.effective_id, mid.effective_id);
    for hidden in [base, mid] {
        assert!(!hidden.preimage.derivation.is_empty());
        assert!(hidden
            .preimage
            .derivation
            .iter()
            .any(|fact| fact.rule.identity == RULE_REDEFINE));
        assert!(!view
            .links()
            .unwrap()
            .expand(&hidden.effective_id, 100)
            .unwrap()
            .is_empty());
    }
}

#[test]
#[trace("FR-081-AC-6", "FR-710-AC-3")]
fn sibling_source_redefinitions_refuse_without_a_selected_view() {
    let refusal = selected(
        "object Base { field: Integer; }
        object Left extends Base { field: Integer redefines Base::field; }
        object Right extends Base { field: Integer redefines Base::field; }
        object Join extends Left, Right {}",
    )
    .expect_err("no selected view escapes the conflict");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("a typed semantic refusal is required: {refusal:?}");
    };
    let conflict = refusals
        .iter()
        .find_map(|refusal| match &refusal.cause {
            ModelRefusalCause::DerivationConflict {
                type_,
                member,
                redefiners,
                ..
            } => Some((type_, member, redefiners)),
            _ => None,
        })
        .expect("normalization returns derivation-conflict");
    assert_eq!(conflict.0.node, "ix://example/identity/Join");
    assert_eq!(conflict.1.node, "ix://example/identity/Base/field");
    assert_eq!(
        conflict
            .2
            .iter()
            .map(|key| key.node.as_str())
            .collect::<Vec<_>>(),
        [
            "ix://example/identity/Left/field",
            "ix://example/identity/Right/field"
        ]
    );
}

#[test]
#[trace("FR-081-AC-9", "FSpec-FR-150-AC-6", "FR-710-AC-3")]
fn source_diamond_retains_two_direct_facts_and_both_expanded_paths() {
    let view = view(
        "object Base { field: Integer; }
        object Left extends Base {} object Right extends Base {}
        object Join extends Left, Right {}",
    );
    let join = entry(&view, "Join", "Base/field");
    assert_eq!(join.preimage.derivation.len(), 2);
    for (fact, parent) in join.preimage.derivation.iter().zip(["Left", "Right"]) {
        assert_eq!(fact.rule.identity, RULE_INHERIT);
        assert_eq!(
            fact.parent.as_ref(),
            Some(&entry(&view, parent, "Base/field").effective_id)
        );
    }
    let expanded = view
        .links()
        .unwrap()
        .expand(&join.effective_id, 100)
        .unwrap();
    let inherited: Vec<_> = expanded
        .iter()
        .filter(|fact| fact.rule.identity == RULE_INHERIT)
        .map(|fact| {
            fact.inputs
                .iter()
                .map(|key| key.node.as_str())
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(
        inherited,
        [
            vec![
                "ix://example/identity/Left",
                "ix://example/identity/Base",
                "ix://example/identity/Base/field"
            ],
            vec![
                "ix://example/identity/Right",
                "ix://example/identity/Base",
                "ix://example/identity/Base/field"
            ],
        ]
    );
    assert_eq!(
        view.declarations()
            .iter()
            .filter(|e| e.effective_id == join.effective_id)
            .count(),
        1
    );
}

#[test]
#[trace("FR-081-AC-5", "FR-081-AC-7")]
fn source_compilation_and_declaration_permutation_preserve_correspondence_order() {
    let first = view("object Base { field: Integer; } object Child extends Base {}");
    let repeated = view("object Base { field: Integer; } object Child extends Base {}");
    let permuted = view("object Child extends Base {} object Base { field: Integer; }");
    assert_eq!(first.declarations(), repeated.declarations());
    assert_eq!(first.declarations(), permuted.declarations());
    assert_eq!(first.type_identities(), permuted.type_identities());
    let keys = |view: &EffectiveView| {
        view.domain_package()
            .records
            .iter()
            .map(|r| r.key().clone())
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(keys(&first), keys(&permuted));
}
