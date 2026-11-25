// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1111, NFR-015-AC-5: the `fsl-package` surface that exists with `std`
//! disabled. This file builds in both configurations; the `std`-only exports
//! are checked as compile-fail cases in `fsl_package`'s crate docs.

use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::checking::CheckingLimits;
use fsl_package::{
    export_bindings, BoundItem, CheckedPackage, ExportRefusal, ResolvedDependency, UnboundElement,
};
use fsl_semantics::check::{CheckedGraph, PackageDeclarations};
use ix_trace_rs::trace;

fn empty_graph() -> CheckedGraph {
    let source = fsl_foundation::Source::read(
        fsl_foundation::SourceIdentity::new("a", "u"),
        "u".to_owned(),
        b"",
        0,
    )
    .expect("an empty source reads")
    .reference()
    .clone();
    let mut declarations =
        PackageDeclarations::new(source, "pkg", fsl_foundation::IdentityLimits::default());
    declarations.types = filament_semantic_value::declaration::TypeEnvironment::new(
        [filament_semantic_value::declaration::CompositeDeclaration::new(
            filament_exact::NodeKey::new(
                "pkg",
                vec![filament_exact::PathSegment::Text(
                    "FixturePlaceholder".to_owned(),
                )],
                0,
            )
            .expect("a fixture node key"),
            "FixturePlaceholder",
            filament_semantic_value::declaration::CompositeShape::Record(vec![]),
        )],
        [],
    )
    .unwrap();
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .expect("an empty package checks")
}

#[trace("TC-1111", "NFR-015-AC-5")]
#[test]
fn link_builds_a_package_with_no_dependencies_or_selections() {
    let package = CheckedPackage::link(empty_graph());
    assert!(package.dependencies().is_empty());
    assert!(package.dependency_selections().is_empty());
    assert!(package.abstraction().is_none());
    let _: &CheckedGraph = package.graph();
    let _shared: std::sync::Arc<CheckedGraph> = package.shared_graph();
    let _: Option<&ResolvedDependency> = package.dependency_selections().values().next();
}

#[trace("TC-1111", "NFR-015-AC-5")]
#[test]
fn bind_abstraction_with_no_bindings_gives_the_package_a_relation() {
    let mut package = CheckedPackage::link(empty_graph());
    package
        .bind_abstraction(&[])
        .expect("no bindings check against an empty package");
    assert!(package.abstraction().is_some());
    package
        .bind_abstraction_at(&[], Vec::new())
        .expect("no bindings check against an empty package");
    assert!(package.abstraction().is_some());
}

#[trace("TC-1111", "NFR-015-AC-5")]
#[test]
fn export_surface_resolves_without_std() {
    let package = CheckedPackage::link(empty_graph());
    let none: Vec<Result<BoundItem, ExportRefusal>> = export_bindings(&package, &[]);
    assert!(none.is_empty());
    let _: Option<UnboundElement> = None; // the type resolves
}
