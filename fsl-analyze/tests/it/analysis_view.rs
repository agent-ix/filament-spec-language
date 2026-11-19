// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940 (TC-2620): the one sealed `AnalysisView` reads the same facts over
//! the `AdmittedPackageRead` of a compiled package and over the
//! `BarePackage` of the bytes it was written to.

use fsl_package::{AnalysisView, BarePackage, EmitLimits, V2ReadLimits};
use ix_trace_rs::trace;

use crate::counter::{document, package_with_clauses, CONTRACT};

/// The library name `CheckedPackage::link` gives the Counter unit.
const UNIT: &str = "package";

/// FR-940-AC-1 (accessor half): for the Counter package with `Bounded`,
/// `BelowThree` and `AtMostTwo`, every `AnalysisView` accessor returns equal
/// values over the compiled package's admitted read-back and over the bare
/// package of its emitted bytes.
#[trace("TC-2620", "FR-940-AC-1")]
#[test]
fn both_views_read_equal_facts_for_the_counter_package() {
    let model = document(false);
    let (package, _) = package_with_clauses(
        &model,
        CONTRACT,
        &[
            ("Bounded", "", "always holds(c.value <= 3)"),
            ("BelowThree", "", "always holds(c.value <= 2)"),
            ("AtMostTwo", "", "always holds(c.value <= 2)"),
        ],
    );
    let domain = fsl_semantics::model::intake::package_input([model.as_slice()])
        .expect("the Counter domain package admits");
    let read = fsl_package::read_evaluation_package(
        &package,
        fsl_semantics::library::LibraryName::new(UNIT).expect("a library name"),
        &domain,
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("the strict reader admits the package's own emission");
    let emission =
        fsl_package::emit_checked(&package, EmitLimits::fixture()).expect("the package emits");
    let admitted = fsl_package::admit_checked_package(
        emission.package().bytes(),
        &domain,
        V2ReadLimits::fixture(),
    )
    .expect("the strict reader admits the written bytes");
    let bare = BarePackage::new(&admitted);
    let views: [&dyn AnalysisView; 2] = [&read, &bare];
    let [source, bytes] = views;

    assert_eq!(source.identity(), bytes.identity());
    assert_eq!(source.identity().as_str(), UNIT);
    assert_eq!(source.model_selections(), bytes.model_selections());
    let names = |view: &dyn AnalysisView| -> Vec<String> {
        view.dependencies()
            .iter()
            .map(|dependency| dependency.name().to_owned())
            .collect()
    };
    assert_eq!(names(source), names(bytes));
    assert_eq!(source.activation_relation(), bytes.activation_relation());
    assert_eq!(source.clause_names(), bytes.clause_names());
    let mut clause_names: Vec<&str> = bytes
        .clause_names()
        .iter()
        .map(|row| row.name.as_ref())
        .collect();
    // The rows are in the package's row order, not declaration order.
    clause_names.sort_unstable();
    assert_eq!(clause_names, ["AtMostTwo", "BelowThree", "Bounded"]);
    assert_eq!(source.declared_names(), bytes.declared_names());
    assert!(bytes
        .declared_names()
        .iter()
        .any(|(name, _)| name == "Noop"));
    for row in bytes.clause_names() {
        let over_source = format!("{:?}", source.temporal_subject(&row.declaration));
        let over_bytes = format!("{:?}", bytes.temporal_subject(&row.declaration));
        assert!(
            bytes.temporal_subject(&row.declaration).is_ok(),
            "{over_bytes}"
        );
        assert_eq!(over_source, over_bytes, "{}", row.name);
    }
}
