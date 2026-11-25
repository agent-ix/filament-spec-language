// SPDX-License-Identifier: AGPL-3.0-or-later
//! One package name holds one package, the root's name included: the
//! bare-package join and a package replay refuse an artifact that repeats a
//! name, and a source compile refuses an import of the root's name.

use filament_contract_model::{CheckedPackageV2, NativeJoinBudget};
use filament_exact::{NodeKey, PathSegment, ScalarLimits};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::simulation::model::ExpansionLimits;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, Code};
use fsl_package::{
    BarePackage, DependencyArtifact, DependencyJoinRefusal, Emission, EmitLimits,
    ObligationIdentity, V2ReadLimits,
};
use fsl_replay::spine::{ImportRefusal, SpineStage};
use fsl_replay::{
    compile_package, replay_model_graph_from_package, CompileRefusal, CompileUnit, DependencyInput,
    GraphEvidence, GraphEvidenceEnvelope, HandoffSupply, ModelTraceSubject, PackageReadRefusal,
    ReplayLimits, ReplayPackage, ReplayRefusal, ReplayRequestWire, ReplaySource, RequestTargetWire,
    StageLimits, StateEnvironment, SuppliedLibrary,
};
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::library::LibraryName;
use ix_trace_rs::trace;
use std::collections::BTreeMap;
use std::sync::Arc;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

/// A function that converts a quantity, and one that compares a call's result
/// with a quantity literal.
const FUNCTIONS: &str = "function three_seconds_in_ms using v(): Quantity<ms> pure { convert<Quantity<ms>>(3 s) }\n\
     function three_seconds_is_3000_ms using v(): Boolean pure { three_seconds_in_ms() = 3000 ms }\n";

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
        "{HEADER}dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, {divisor}) * s;\n{FUNCTIONS}"
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

/// `artifact`, supplied under the identity `identity` with its bytes unchanged.
fn supplied_as(identity: &str, mut artifact: DependencyArtifact) -> DependencyArtifact {
    artifact.identity = identity.to_owned();
    artifact
}

/// The join itself refuses `dependencies` for the package name `name`, before
/// it charges anything.
fn assert_name_refuses(
    root: &DependencyArtifact,
    dependencies: &[DependencyArtifact],
    name: &str,
    case: &str,
) {
    let mut budget = join_budget();
    let refusal = BarePackage::new(&root.package)
        .with_dependency_artifacts(dependencies, &mut budget)
        .expect_err(case);
    assert_eq!(
        refusal,
        DependencyJoinRefusal::Name {
            package: name.to_owned()
        },
        "{case}"
    );
    assert_eq!(refusal.code(), Some(Code::InvalidPackage), "{case}");
    assert_eq!(budget.consumed(), join_budget().consumed(), "{case}");
}

/// An artifact whose package name is the root's refuses, with the root's own
/// content or another, under whatever identity it is supplied.
#[trace("TC-1331", "FR-531-AC-61")]
#[test]
fn an_artifact_named_as_the_root_refuses_at_the_join() {
    let root = package("dep", 1000);
    for (artifact, case) in [
        (package("dep", 1000), "the root's own content"),
        (package("dep", 2000), "another content"),
        (
            supplied_as("lib", package("dep", 2000)),
            "a root-named package supplied as `lib`",
        ),
    ] {
        assert_name_refuses(&root, &[artifact], "dep", case);
    }
}

/// A repeated dependency name refuses, with equal contents or differing ones;
/// distinct names, none the root's, join.
#[trace("TC-1331", "FR-531-AC-61")]
#[test]
fn a_repeated_dependency_name_refuses_at_the_join() {
    let root = package("root", 1000);
    let distinct = [package("dep", 2000), package("lib", 2000)];
    let joined = BarePackage::new(&root.package)
        .with_dependency_artifacts(&distinct, &mut join_budget())
        .expect("distinct names join");
    assert_eq!(joined.dependency_views().len(), 2);
    let repeated = [package("dep", 2000)];
    let refusal = joined
        .with_dependency_artifacts(&repeated, &mut join_budget())
        .expect_err("a name already attached refuses");
    assert_eq!(
        refusal,
        DependencyJoinRefusal::Name {
            package: "dep".to_owned()
        }
    );

    for [first, second] in [[1000, 1000], [1000, 2000], [2000, 1000]] {
        assert_name_refuses(
            &root,
            &[package("dep", first), package("dep", second)],
            "dep",
            &format!("divisors {first} and {second}"),
        );
    }
}

/// A model-graph replay of `root`'s package bytes with `dependencies` as its
/// handoff supply. The claim names no node of `root`, so a replay whose supply
/// is admitted refuses later, on the claim.
fn replay(root: &DependencyArtifact, dependencies: &[DependencyArtifact]) -> ReplayRefusal {
    let node = NodeKey::new(
        root.package.name(),
        vec![PathSegment::Text("claim".into())],
        0,
    )
    .expect("a node key");
    let obligation = ObligationIdentity::new(node.clone(), None, 0);
    let limits = ScalarLimits {
        integer_bits: 1 << 20,
        decimal_digits: 1 << 20,
        scale_expansion: 1 << 20,
        text_input_bytes: 1 << 20,
        text_scalars: 1 << 20,
        normalized_scalars: 1 << 20,
        unit_edges: 1 << 20,
        value_occurrences: 1 << 20,
        work_units: 1 << 20,
        result_units: 1 << 20,
    };
    let envelope = GraphEvidenceEnvelope::new(
        obligation.clone(),
        OccurrenceKey::declaration(node.clone()),
        node,
        Vec::new(),
        limits,
        Vec::new(),
        BackendId::new("test-backend".to_owned()),
        GraphEvidence::Witness {
            paths: Vec::new(),
            over: None,
        },
    );
    let wire = ReplayRequestWire {
        certificate: None,
        package: ReplayPackage::read(&root.bytes, u64::MAX).expect("the package document reads"),
        profile_selections: Vec::new(),
        dependencies: Vec::new(),
        replay_target: RequestTargetWire::default(),
        source: ReplaySource::ModelGraph(Box::new(envelope)),
        obligation_identity: obligation,
        backend: "test-backend".to_owned(),
        state_environment: StateEnvironment::new(Vec::new()),
        accounting_limits: limits,
        stage_limits: BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: Vec::new(),
        documents: Vec::new(),
    };
    let subject = ModelTraceSubject {
        initial: Vec::new(),
        universes: Vec::new(),
        expansion: ExpansionLimits { max_candidates: 64 },
        timed: None,
        stutter_closure: false,
    };
    replay_model_graph_from_package(
        wire,
        HandoffSupply {
            dependencies,
            identities: &root.identity_bytes,
            equality: Some(&root.equality_bytes),
        },
        &subject,
        ReplayLimits::default(),
    )
    .expect_err("the claim names no claim of the package")
}

fn assert_supply_refused(refusal: &ReplayRefusal, case: &str) {
    assert!(
        matches!(
            refusal,
            ReplayRefusal::PackageRead(PackageReadRefusal::Malformed)
        ),
        "{case}: {refusal:?}"
    );
    assert_eq!(refusal.code(), Code::InvalidPackage, "{case}");
    assert_eq!(refusal.cause(), None, "{case}");
}

fn assert_supply_admitted(refusal: &ReplayRefusal, case: &str) {
    assert!(
        !matches!(refusal, ReplayRefusal::PackageRead(_)),
        "{case}: {refusal:?}"
    );
}

/// A package replay refuses an artifact whose package name is the root's before
/// it reads the package's functions or claim.
#[trace("TC-1331", "FR-531-AC-59")]
#[test]
fn a_package_replay_refuses_an_artifact_named_as_the_root() {
    let root = package("dep", 1000);
    assert_supply_admitted(&replay(&root, &[]), "no artifact");
    assert_supply_admitted(
        &replay(&root, &[package("lib", 2000)]),
        "another package name",
    );
    for (artifact, case) in [
        (package("dep", 1000), "the root's own content"),
        (package("dep", 2000), "another content"),
        (
            supplied_as("lib", package("dep", 2000)),
            "a root-named package supplied as `lib`",
        ),
    ] {
        assert_supply_refused(&replay(&root, &[artifact]), case);
    }
}

/// A package replay refuses two artifacts of one package name, in either order.
#[trace("TC-1331", "FR-531-AC-60")]
#[test]
fn a_package_replay_refuses_a_repeated_artifact_name() {
    let root = package("root", 1000);
    assert_supply_admitted(&replay(&root, &[package("dep", 2000)]), "one artifact");
    for [first, second] in [[1000, 1000], [2000, 2000], [1000, 2000], [2000, 1000]] {
        assert_supply_refused(
            &replay(&root, &[package("dep", first), package("dep", second)]),
            &format!("divisors {first} and {second}"),
        );
    }
}

/// A library supplied as `identity` from the source owner `owner`.
fn library(identity: &str, owner: &str, body: &str) -> SuppliedLibrary {
    SuppliedLibrary {
        identity: identity.to_owned(),
        source: fsl_replay::SourceIdentity::new("fixture", owner),
        path: format!("{owner}.native"),
        bytes: format!("{HEADER}{body}").into_bytes(),
    }
}

const G: &str = "function g using v(): Boolean pure { true }\n";

/// The source of a unit that imports `import` and declares `g`.
fn importing(import: &str) -> String {
    format!("import \"{import}\" as i;\n{G}")
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
    let text = format!("{HEADER}{}", importing(import));
    compile_package(
        CompileUnit {
            source: fsl_replay::SourceIdentity::new("fixture", "unit"),
            package_name: root_name.to_owned(),
            path: "unit.native",
            bytes: text.as_bytes(),
        },
        [],
        &dependencies,
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .map(|_| ())
}

/// The compile refused, unwrapped, at the import of `root_name` in the source
/// owned by `owner`, naming the identity path `path`.
fn assert_cycle_refuses(
    outcome: Result<(), ReplayRefusal>,
    owner: &str,
    source: &str,
    path: &[&str],
    case: &str,
) {
    let Err(ReplayRefusal::Recompile { refusal, .. }) = &outcome else {
        panic!("{case}: expected a refused compile, got {outcome:?}");
    };
    assert_eq!(refusal.stage(), SpineStage::Intake, "{case}");
    assert_eq!(refusal.code(), Code::InvalidPackage, "{case}");
    assert_eq!(refusal.cause(), Some("definition-cycle"), "{case}");
    let CompileRefusal::Import {
        refusal: ImportRefusal::Cycle { path: found },
        region,
    } = refusal.as_ref()
    else {
        panic!("{case}: expected an unwrapped cycle, got {refusal:?}");
    };
    let found: Vec<_> = found.iter().map(LibraryName::as_str).collect();
    assert_eq!(found, path, "{case}");
    let region = region.as_ref().expect("the cycle is located");
    assert_eq!(region.source().identity(), owner, "{case}");
    let start = usize::try_from(region.start()).expect("a source offset");
    let end = usize::try_from(region.end()).expect("a source offset");
    let named = format!("\"{}\"", path[path.len() - 1]);
    assert_eq!(&source[start..end], named, "{case}");
}

/// An import of the compile's own package name is a cycle, directly or through
/// a library, whether or not a library is supplied under that name and
/// whatever it declares.
#[trace("TC-446", "FR-099-AC-10")]
#[test]
fn a_source_closure_importing_the_root_name_is_a_cycle() {
    const R: &str = "test/own";
    let through = library("test/a", "a", &importing(R));
    let control = compile(
        "test/root",
        "test/a",
        &[through.clone(), library(R, "r", G)],
    );
    assert!(control.is_ok(), "{control:?}");

    for imported in [R, "test/a"] {
        let namesakes = [
            ("no library as R", None),
            (
                "one declaring what the unit declares",
                Some(importing(imported)),
            ),
            (
                "one that differs",
                Some("function other using v(): Boolean pure { false }\n".to_owned()),
            ),
        ];
        for (case, namesake) in namesakes {
            let mut libraries = vec![through.clone()];
            libraries.extend(namesake.map(|body| library(R, "r", &body)));
            // The refused import is the unit's own, or the one in `test/a`.
            let (owner, path): (_, &[&str]) = if imported == R {
                ("unit", &[R, R])
            } else {
                ("a", &[R, "test/a", R])
            };
            assert_cycle_refuses(
                compile(R, imported, &libraries),
                owner,
                &format!("{HEADER}{}", importing(R)),
                path,
                case,
            );
        }
    }
}
