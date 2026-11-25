// SPDX-License-Identifier: AGPL-3.0-or-later
//! One package name holds one package, the root's name included: the bare-package
//! join refuses an artifact that repeats a name, and a source compile refuses
//! an import of the root's name.

use filament_contract_model::{CheckedPackageV2, NativeJoinBudget};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::{DependencyArtifact, Emission, EmitLimits, V2ReadLimits};
use fsl_replay::{
    compile_package, CompileUnit, DependencyInput, ReplayLimits, ReplayRefusal, StageLimits,
    SuppliedLibrary,
};
use fsl_semantics::check::PackageDeclarations;
use std::sync::Arc;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

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

/// One content of the package `name`, whose unit `ms` is `1/divisor` of a
/// second, with its witness files.
fn package(name: &str, divisor: u32) -> DependencyArtifact {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n\
         dimension Time;\nunit s : Time = rational(1, 1);\nunit ms : Time = rational(1, {divisor}) * s;\n\
         function in_ms using v(): Quantity<ms> pure {{ convert<Quantity<ms>>(3 s) }}\n\
         function in_ms_is_3000 using v(): Boolean pure {{ in_ms() = 3000 ms }}\n"
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
    let emission: Emission =
        fsl_package::emit_checked(&checked, EmitLimits::fixture()).expect("the unit emits");
    let admitted: CheckedPackageV2 = fsl_package::admit_checked_package(
        emission.package().bytes(),
        &Default::default(),
        read_limits(),
    )
    .expect("the emission admits");
    let mut budget = join_budget();
    let identity_bytes = emission
        .identity_rows(&admitted, &mut budget)
        .expect("identity rows")
        .to_bytes(&admitted)
        .expect("identity bytes");
    let equality_bytes = emission
        .equality_witness(&admitted, &mut budget)
        .expect("equality rows")
        .to_bytes(&admitted)
        .expect("equality bytes");
    DependencyArtifact {
        identity: admitted.name().to_owned(),
        identities: fsl_package::ReadIdentityRows::read(&identity_bytes, &admitted, &mut budget)
            .expect("the identity file reads"),
        equality: fsl_package::ReadEqualityRows::read(&equality_bytes, &admitted)
            .expect("the equality file reads"),
        package: Arc::new(admitted),
        bytes: emission.package().bytes().into(),
        identity_bytes: identity_bytes.into(),
        equality_bytes: equality_bytes.into(),
    }
}

/// The refusal of joining `root` with `dependencies` through the public
/// bare-package calls and building its world, or `None` when both are
/// admitted. A refusal names the step that gave it.
fn refusal(root: &DependencyArtifact, dependencies: &[DependencyArtifact]) -> Option<String> {
    let mut budget = join_budget();
    let joined = fsl_package::BarePackage::new(&root.package)
        .with_dependency_artifacts(dependencies, &mut budget)
        .and_then(|bare| {
            bare.with_supply(
                fsl_package::BareSupply {
                    identities: &root.identities,
                    equality: Some(&root.equality),
                },
                &mut budget,
            )
        });
    match joined {
        Err(refusal) => Some(format!("join: {refusal:?}")),
        Ok(bare) => bare
            .world()
            .err()
            .map(|refusal| format!("world: {refusal:?}")),
    }
}

/// The join itself refuses `dependencies`, before any world or call.
fn assert_join_refuses(root: &DependencyArtifact, dependencies: &[DependencyArtifact], case: &str) {
    let refusal = refusal(root, dependencies);
    assert!(
        refusal
            .as_deref()
            .is_some_and(|refusal| refusal.starts_with("join: ")),
        "{case}: {refusal:?}"
    );
}

/// An artifact named as the root refuses, with the root's own content or
/// another. The root compares a called conversion with `=`, so a second unit
/// table under its name has an application to disagree on.
#[test]
fn an_artifact_named_as_the_root_refuses_at_the_join() {
    let root = package("dep", 1000);
    assert_eq!(refusal(&root, &[]), None, "the root alone is admitted");
    assert_eq!(
        refusal(&root, &[package("lib", 2000)]),
        None,
        "the same content under another name is admitted"
    );
    assert_join_refuses(&root, &[package("dep", 2000)], "another content");
    assert_join_refuses(&root, &[package("dep", 1000)], "the root's own content");
}

/// A repeated dependency name refuses, with equal contents or differing ones.
#[test]
fn a_repeated_dependency_name_refuses_at_the_join() {
    let root = package("root", 1000);
    assert_eq!(
        refusal(&root, &[package("dep", 2000)]),
        None,
        "one artifact under the name is admitted"
    );
    for [first, second] in [[1000, 1000], [1000, 2000], [2000, 1000]] {
        assert_join_refuses(
            &root,
            &[package("dep", first), package("dep", second)],
            &format!("divisors {first} and {second}"),
        );
    }
}

fn library(identity: &str, body: &str) -> SuppliedLibrary {
    SuppliedLibrary {
        identity: identity.to_owned(),
        source: fsl_replay::SourceIdentity::new("fixture", identity),
        path: format!("{identity}.native"),
        bytes: format!("{HEADER}{body}").into_bytes(),
    }
}

/// Compile a root unit named `root_name` that imports `import` from
/// `libraries`.
fn compile(
    root_name: &str,
    import: &str,
    libraries: &[SuppliedLibrary],
) -> Result<(), ReplayRefusal> {
    let dependencies =
        DependencyInput::new(libraries.to_vec()).expect("the libraries are supplied");
    let text = format!(
        "{HEADER}import \"{import}\" as i;\nfunction f using v(): Boolean pure {{ true }}\n"
    );
    compile_package(
        CompileUnit {
            source: fsl_replay::SourceIdentity::new("fixture", "root"),
            package_name: root_name.to_owned(),
            path: "root.native",
            bytes: text.as_bytes(),
        },
        [],
        &dependencies,
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .map(|_| ())
}

/// The compile refused the import graph as cyclic.
fn assert_cycle_refuses(outcome: Result<(), ReplayRefusal>) {
    let Err(ReplayRefusal::Recompile { refusal, .. }) = &outcome else {
        panic!("expected a refused compile, got {outcome:?}");
    };
    assert_eq!(refusal.cause(), Some("definition-cycle"), "{refusal:?}");
}

#[test]
fn a_source_root_importing_its_own_name_refuses() {
    let libraries = [library(
        "dep",
        "function g using v(): Boolean pure { true }\n",
    )];
    let control = compile("root", "dep", &libraries);
    assert!(control.is_ok(), "{control:?}");
    assert_cycle_refuses(compile("dep", "dep", &libraries));
}

#[test]
fn a_source_library_importing_the_root_name_refuses() {
    let libraries = [
        library("dep", "function g using v(): Boolean pure { true }\n"),
        library(
            "mid",
            "import \"dep\" as d;\nfunction h using v(): Boolean pure { true }\n",
        ),
    ];
    let control = compile("root", "mid", &libraries);
    assert!(control.is_ok(), "{control:?}");
    assert_cycle_refuses(compile("dep", "mid", &libraries));
}
