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

/// The artifact of `declarations` emitted under `name`, with its witness files.
fn artifact(name: &str, declarations: &str) -> DependencyArtifact {
    let (emission, package) = emitted(name, declarations);
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

/// Library `dep`, whose one source difference between calls is the ratio of
/// its dimension equivalence.
fn library(ratio: u8) -> DependencyArtifact {
    artifact(
        "dep",
        &format!(
            "dimension Time;\ndimension Period;\nunit s : Time = rational(1, 1);\n\
             unit sec : Period = rational(1, 1);\ndimension Period ~ Time by 1 sec = {ratio} s;\n"
        ),
    )
}

/// A package `name` whose lock selects `selected`, with its witness files. The
/// selection is added to the emitted lock, so `package` stays the emission:
/// handoff admission reads `bytes` alone.
fn selecting(name: &str, selected: &str) -> DependencyArtifact {
    let mut artifact = artifact(
        name,
        "dimension Beat;\nunit beat : Beat = rational(1, 1);\n",
    );
    let package = &artifact.package;
    let mut lock = package.lock().clone();
    lock.dependency_selections.push(CheckedDependencySelection {
        identity: selected.into(),
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
    .expect("the package encodes with its selection");
    artifact.bytes = bytes.into();
    artifact
}

/// The number of dependencies handoff admission keeps for `root` over
/// `dependencies`, or its refusal.
fn admit(
    root: &DependencyArtifact,
    dependencies: &[DependencyArtifact],
) -> Result<usize, ReplayRefusal> {
    HandoffSupply {
        dependencies,
        identities: &root.identity_bytes,
        equality: Some(&root.equality_bytes),
    }
    .admit_package(
        &root.bytes,
        &Default::default(),
        read_limits(),
        &mut join_budget(),
    )
    .map(|(_, admitted)| admitted.len())
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

    let root = selecting("root", "dep");
    for ratio in [1, 2] {
        assert!(
            matches!(admit(&root, &[library(ratio)]), Ok(1)),
            "each content admits alone"
        );
    }
    for ratios in [[1, 1], [1, 2], [2, 1]] {
        let refusal = admit(&root, &ratios.map(library)).expect_err("a repeated name refuses");
        assert!(
            matches!(
                refusal,
                ReplayRefusal::PackageRead(PackageReadRefusal::Malformed)
            ),
            "{ratios:?}: {refusal:?}"
        );
    }
}

/// Handoff admission refuses with the code it gives a repeated name.
fn assert_refuses(outcome: Result<usize, ReplayRefusal>, case: &str) {
    assert!(
        matches!(
            outcome,
            Err(ReplayRefusal::PackageRead(PackageReadRefusal::Malformed))
        ),
        "{case}: {outcome:?}"
    );
}

/// Handoff admission refuses an artifact that carries the root package's name,
/// with the root's own content or another: that name already holds the root.
#[test]
fn an_artifact_named_as_the_root_refuses() {
    for (root_ratio, supplied_ratio) in [(1, 1), (1, 2), (2, 1)] {
        assert_refuses(
            admit(&library(root_ratio), &[library(supplied_ratio)]),
            &format!("root ratio {root_ratio}, supplied ratio {supplied_ratio}"),
        );
    }
}

/// A root whose lock selects its own name refuses.
#[test]
fn a_root_selecting_its_own_name_refuses() {
    let control = admit(&selecting("root", "dep"), &[library(2)]);
    assert!(matches!(control, Ok(1)), "{control:?}");
    assert_refuses(
        admit(&selecting("dep", "dep"), &[library(2)]),
        "the root selects its own name",
    );
}

/// The same refusal holds when a selected library selects the root's name.
#[test]
fn a_library_selecting_the_root_name_refuses() {
    let closure = [library(2), selecting("mid", "dep")];
    let control = admit(&selecting("root", "mid"), &closure);
    assert!(matches!(control, Ok(2)), "{control:?}");
    assert_refuses(
        admit(&selecting("dep", "mid"), &closure),
        "a library selects the root's name",
    );
}
