// SPDX-License-Identifier: AGPL-3.0-or-later
//! The replay handoff supply admits one artifact per library identity
//! (FR-850 "Admission").

use super::{HandoffSupply, ReplayRefusal};
use crate::execute::PackageReadRefusal;
use filament_contract_model::{
    encode_checked_package_v2, CheckedDependencySelection, CheckedPackageSidecarsRef,
    CheckedPackageSourceCorrespondenceRef, CheckedPackageV2, NativeJoinBudget,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::{DependencyArtifact, Emission, EmitLimits, V2ReadLimits};
use fsl_semantics::check::PackageDeclarations;
use serde_json::Value;
use std::sync::Arc;

fn read_limits() -> V2ReadLimits {
    let mut limits = V2ReadLimits::bounded(1 << 40);
    limits.work = 1 << 40;
    limits
}

fn join_budget() -> NativeJoinBudget {
    let mut budget = read_limits()
        .native_join_limits()
        .budget()
        .with_read_storage_limit(u64::MAX);
    budget.resume();
    budget
}

/// The emission of the unit-declaring source `declarations` under `name`, and
/// its strictly admitted package.
fn emitted(name: &str, declarations: &str) -> (Emission, CheckedPackageV2) {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n{declarations}"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("fixture", name),
        "unit.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the source parses");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        name.to_owned(),
        fsl_forms::build_unit(&parsed).expect("the unit forms"),
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles");
    let checked = fsl_package::CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_semantics::check::test_support::ZeroRewardEvaluator,
            )
            .expect("the unit checks"),
    );
    let emission =
        fsl_package::emit_checked(&checked, EmitLimits::fixture()).expect("the declarations emit");
    let admitted = fsl_package::admit_checked_package(
        emission.package().bytes(),
        &Default::default(),
        read_limits(),
    )
    .expect("the emitted declarations admit");
    (emission, admitted)
}

/// Library `dep`, whose one source difference between calls is the ratio of
/// its dimension equivalence.
fn library(ratio: u8) -> DependencyArtifact {
    let (emission, package) = emitted(
        "dep",
        &format!(
            "dimension Time;\ndimension Period;\nunit s : Time = rational(1, 1);\n\
             unit sec : Period = rational(1, 1);\ndimension Period ~ Time by 1 sec = {ratio} s;\n"
        ),
    );
    let mut budget = join_budget();
    let identities = emission
        .identity_rows(&package, &mut budget)
        .expect("identity rows");
    let equality = emission
        .equality_witness(&package, &mut budget)
        .expect("equality rows");
    DependencyArtifact {
        identity: package.name().to_owned(),
        bytes: emission.package().bytes().into(),
        identity_bytes: identities
            .to_bytes(&package)
            .expect("identity bytes")
            .into(),
        equality_bytes: equality.to_bytes(&package).expect("equality bytes").into(),
        package: Arc::new(package),
        identities,
        equality,
    }
}

/// A root package whose lock selects `dep`, with its witness files.
fn root_selecting_dep() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let (emission, package) = emitted(
        "root",
        "dimension Beat;\nunit beat : Beat = rational(1, 1);\n",
    );
    let mut lock = package.lock().clone();
    lock.dependency_selections.push(CheckedDependencySelection {
        identity: "dep".into(),
    });
    let bytes = encode_checked_package_v2(
        package.name(),
        &lock,
        package.graph(),
        CheckedPackageSourceCorrespondenceRef {
            source_map: package.source_map(),
            refinement_step_rows: None,
        },
        CheckedPackageSidecarsRef {
            checked_term_data: package.checked_term_data(),
            temporal_bindings: package.temporal_bindings(),
            temporal_clause_names: package.temporal_clause_names(),
            claim_names: package.claim_names(),
            schedulability_claims: None,
            operation_effects: None,
            operation_bodies: None,
        },
        package.capability_report(),
        package.diagnostics(),
        quire_canonical::Limits::new(1 << 32),
    )
    .expect("the root encodes with its selection");
    let mut budget = join_budget();
    let identities = emission
        .identity_rows(&package, &mut budget)
        .expect("identity rows")
        .to_bytes(&package)
        .expect("identity bytes");
    let equality = emission
        .equality_witness(&package, &mut budget)
        .expect("equality rows")
        .to_bytes(&package)
        .expect("equality bytes");
    (bytes, identities, equality)
}

fn wire(artifact: &DependencyArtifact) -> Value {
    serde_json::from_slice(&artifact.bytes).expect("the library wire")
}

/// Handoff admission refuses a second artifact under one library identity by
/// its name, before reading it: byte-identical artifacts and two contents
/// differing only in an equivalence's `root_scale` refuse alike. No content is
/// compared here; the link diamond rule does that.
#[test]
fn a_second_supplied_artifact_of_one_name_refuses_whatever_its_content() {
    let one = wire(&library(1));
    let mut two = wire(&library(2));
    let position = one["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .position(|node| node["semantic_form"] == "dimension_equivalence")
        .expect("the equivalence node");
    let scale = format!("/semantic_graph/nodes/{position}/body/root_scale/numerator");
    assert_eq!(one.pointer(&scale), Some(&Value::from("1")));
    assert_eq!(two.pointer(&scale), Some(&Value::from("2")));
    *two.pointer_mut(&scale).expect("the ratio") = Value::from("1");
    assert_eq!(one, two, "the ratio is the only difference");
    assert_eq!(
        library(1).bytes,
        library(1).bytes,
        "the control pair is byte-identical"
    );

    let (root, identities, equality) = root_selecting_dep();
    let admit = |dependencies: &[DependencyArtifact]| {
        HandoffSupply {
            dependencies,
            identities: &identities,
            equality: Some(&equality),
        }
        .admit_package(
            &root,
            &Default::default(),
            read_limits(),
            &mut join_budget(),
        )
        .map(|(_, admitted)| admitted.len())
    };
    for ratio in [1, 2] {
        assert!(
            matches!(admit(&[library(ratio)]), Ok(1)),
            "each content admits alone"
        );
    }
    for ratios in [[1, 1], [1, 2], [2, 1]] {
        let refusal = admit(&ratios.map(library)).expect_err("a repeated name refuses");
        assert!(
            matches!(
                refusal,
                ReplayRefusal::PackageRead(PackageReadRefusal::Malformed)
            ),
            "{ratios:?}: {refusal:?}"
        );
    }
}
