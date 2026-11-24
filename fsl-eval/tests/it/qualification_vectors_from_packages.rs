// SPDX-License-Identifier: AGPL-3.0-or-later
//! Published package-selection vectors whose unit graph spans domain packages.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use filament_specification::{QUALIFICATION_VECTORS, QualificationVector, TestCase};
use fsl_eval::value::S6aRewardEvaluator;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{PackageInput, admit_unit, package_input};
use fsl_semantics::value::{DefinitionRole, admit_integer_division};
use ix_trace_rs::trace;

const P_WITH_ROOT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    domain \"acme/p\";\n dimension L;\n unit m : L = rational(1, 1);\n\
    object Anchor { operation anchor(): Boolean; }\n";
const P_WITHOUT_ROOT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    domain \"acme/p\";\n dimension L;\n\
    object Anchor { operation anchor(): Boolean; }\n";
const P_WITH_TIME_UNITS: &str = "language \"ix:native\" edition \"1-draft\";\n\
    domain \"acme/p\";\n dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    object Anchor { operation anchor(): Boolean; }\n";
const P_WITH_RATE_AND_LENGTH: &str = "language \"ix:native\" edition \"1-draft\";\n\
    domain \"acme/p\";\n dimension Rate;\n\
    unit us : Rate = rational(1, 1);\n\
    dimension Length;\n\
    object Anchor { operation anchor(): Boolean; }\n";
const P_WITH_TIME_AND_LENGTH_ROOT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    domain \"acme/p\";\n dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    dimension Length;\n unit m : Length = rational(1, 1);\n\
    object Anchor { operation anchor(): Boolean; }\n";
const Q_WITH_ROOT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    domain \"acme/q\";\n dimension T;\n unit s : T = rational(1, 1);\n\
    object Anchor { operation anchor(): Boolean; }\n";
const Q_WITHOUT_ROOT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    domain \"acme/q\";\n dimension T;\n\
    object Anchor { operation anchor(): Boolean; }\n";
const G_WITHOUT_ROOT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    domain \"acme/g\";\n dimension Time;\n\
    object Anchor { operation anchor(): Boolean; }\n";
const G_WITH_MILLISECOND_ROOT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    domain \"acme/g\";\n dimension Time;\n\
    unit ms : Time = rational(1, 1);\n\
    object Anchor { operation anchor(): Boolean; }\n";
const Q_WITH_TIME_ROOT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    domain \"acme/q\";\n dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    object Anchor { operation anchor(): Boolean; }\n";

fn package_input_from(sources: &[&str]) -> PackageInput {
    let mut documents: Vec<&[u8]> = sources.iter().map(|source| source.as_bytes()).collect();
    if !sources
        .iter()
        .any(|source| source.contains("domain \"acme/q\""))
    {
        documents.push(Q_WITHOUT_ROOT.as_bytes());
    }
    if !sources
        .iter()
        .any(|source| source.contains("domain \"acme/g\""))
    {
        documents.push(G_WITHOUT_ROOT.as_bytes());
    }
    package_input(documents).expect("authored domain packages compile into intake documents")
}

/// Add a target edge to a package unit. The model-unit source adapter emits
/// roots only, while TC-187's imported multi-unit rows require this FCD wire
/// shape (`target` plus exact edge scale).
fn add_package_unit_edge(
    packages: &mut PackageInput,
    owner: &str,
    name: &str,
    dimension: &str,
    target: &str,
    scale: &str,
) {
    let bytes = packages.get(owner).expect("the selected package exists");
    let mut document: serde_json::Value = serde_json::from_slice(bytes).expect("package JSON");
    let origin = document["types"][0]["origin"].clone();
    document["units"]
        .as_array_mut()
        .expect("unit rows")
        .push(serde_json::json!({
            "identity": format!("ix://{owner}/unit/{name}"),
            "name": name,
            "dimension": format!("ix://{owner}/dimension/{dimension}"),
            "target": format!("ix://{owner}/unit/{target}"),
            "scale": scale,
            "offset": "0",
            "origin": origin,
        }));
    packages.insert(
        owner.to_owned(),
        serde_json::to_vec(&document).expect("package JSON"),
    );
}

/// Add one derived dimension directly to a package document. FSpec U11z1
/// needs an imported derived dimension; FR-710's model-unit parser emits base
/// dimensions only, so this uses the source-equivalent Semantic IR record.
fn add_derived_dimension(packages: &mut PackageInput, owner: &str, name: &str, base: &str) {
    let bytes = packages.get(owner).expect("the selected package exists");
    let mut document: serde_json::Value = serde_json::from_slice(bytes).expect("package JSON");
    let origin = document["types"][0]["origin"].clone();
    document["dimensions"]
        .as_array_mut()
        .expect("dimension rows")
        .push(serde_json::json!({
            "identity": format!("ix://{owner}/dimension/{name}"),
            "name": name,
            "base": false,
            "terms": [{
                "dimension": base,
                "exponent": "2"
            }],
            "origin": origin,
        }));
    packages.insert(
        owner.to_owned(),
        serde_json::to_vec(&document).expect("package JSON"),
    );
}

fn vector(id: &str) -> &'static QualificationVector {
    QUALIFICATION_VECTORS
        .iter()
        .find(|vector| vector.test_case == TestCase::Tc187 && vector.id == id)
        .expect("published TC-187 vector identity")
}

fn division_vector(id: &str) -> &'static QualificationVector {
    QUALIFICATION_VECTORS
        .iter()
        .find(|vector| vector.test_case == TestCase::Tc192 && vector.id == id)
        .expect("published TC-192 vector identity")
}

fn vector_contains(id: &str, authored_phrase: &str) {
    assert!(
        vector(id).expected.contains(authored_phrase),
        "TC-187 {id} expected outcome {:?} does not include {authored_phrase:?}",
        vector(id).expected
    );
}

fn division_vector_contains(id: &str, authored_phrase: &str) {
    assert!(
        division_vector(id).expected.contains(authored_phrase),
        "TC-192 {id} expected outcome {:?} does not include {authored_phrase:?}",
        division_vector(id).expected
    );
}

fn division_vector_input_contains(id: &str, authored_phrase: &str) {
    assert!(
        division_vector(id).inputs.contains(authored_phrase),
        "TC-192 {id} inputs {:?} do not include {authored_phrase:?}",
        division_vector(id).inputs
    );
}

/// FR-670-AC-2/3 and TC-192 DIV-03/DIV-09: package closure mutation must
/// reject an incompatible `mod` law, missing or duplicated `div`/`rem` law,
/// and a definition that is not an integer-division law.
#[trace("FR-670-AC-2", "FR-670-AC-3", "FSpec-TC-192")]
#[test]
fn integer_division_wire_closure_rows_match_div03_and_div09() {
    use DefinitionRole::{
        IntegerDivisionEuclidean as Euclidean, IntegerDivisionFloor as Floor,
        IntegerDivisionTruncating as Truncating,
    };

    division_vector_input_contains("DIV-03", "truncating or floor");
    division_vector_contains("DIV-03", "incompatible-definition");
    division_vector_contains("DIV-09", "invalid_package");
    division_vector_input_contains("DIV-09", "Omit the division definition");
    division_vector_input_contains("DIV-09", "select two laws");
    division_vector_input_contains("DIV-09", "another definition identity");
    for modulo in [Floor, Truncating] {
        let mismatch = admit_integer_division(&[Euclidean.reference()], Some(&modulo.reference()))
            .expect_err("DIV-03: modulo law must match the div/rem closure");
        assert_eq!(mismatch.code.as_str(), "invalid_package", "DIV-03");
        assert_eq!(mismatch.cause.as_str(), "incompatible-definition", "DIV-03");
    }

    let missing = admit_integer_division(&[], None).expect_err("DIV-09: empty closure has no law");
    assert_eq!(missing.code.as_str(), "invalid_package", "DIV-09");
    assert_eq!(missing.cause.as_str(), "missing-member", "DIV-09");

    let conflict = admit_integer_division(&[Euclidean.reference(), Truncating.reference()], None)
        .expect_err("DIV-09: multiple retained laws conflict");
    assert_eq!(conflict.code.as_str(), "invalid_package", "DIV-09");
    assert_eq!(conflict.cause.as_str(), "conflicting-definition", "DIV-09");

    let wrong_role = admit_integer_division(&[DefinitionRole::Root.reference()], None)
        .expect_err("DIV-09: a root definition is not an integer-division law");
    assert_eq!(wrong_role.code.as_str(), "invalid_package", "DIV-09");
    assert_eq!(
        wrong_role.cause.as_str(),
        "incompatible-definition",
        "DIV-09"
    );
}

/// TC-192 DIV-07/14 are backend negotiation vectors; their executable
/// behavior is covered by the FR-495 tests in the analysis integration suite.
/// These assertions keep the qualification rows tied to the owning ACs and
/// their literal negotiated outcomes.
#[trace("FR-495-AC-5", "FR-495-AC-7", "FSpec-TC-192")]
#[test]
fn backend_division_rows_match_div07_and_div14() {
    division_vector_input_contains("DIV-07", "omit operand, intermediate and result bounds");
    division_vector_contains("DIV-07", "requires-bound");
    division_vector_contains("DIV-07", "evaluation is not invoked");

    division_vector_input_contains("DIV-14", "one mathematical-integer division obligation");
    division_vector_contains("DIV-14", "disposition is not itself a verdict");
    division_vector_contains("DIV-14", "holding obligation to the verdict `proved`");
    division_vector_contains("DIV-14", "violated obligation to `refuted`");
    division_vector_contains("DIV-14", "replayable counterexample");
    division_vector_contains("DIV-14", "never narrowed");
}

/// Runs a unit against the authored package inputs and returns its first
/// assembly refusal as the stable code/cause pair, or `None` when it checks.
fn refusal(packages: PackageInput, unit_body: &str) -> Option<(String, String)> {
    let unit_source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         model P = \"acme/p\";\n\
         model Q = \"acme/q\";\n\
         model G = \"acme/g\";\n\
         {unit_body}"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("qualification", "package-vector"),
        "package-vector.native",
        unit_source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source intake");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("source forms");
    let models = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("selected authored domain packages admit");
    let declarations = match PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "qualification-package-vector".to_owned(),
        unit,
        models,
        Vec::new(),
    ) {
        Ok(declarations) => declarations,
        Err(refusal) => {
            let error = refusal.errors.first().expect("one assembly error");
            return Some((
                error.cause.code().as_str().to_owned(),
                error.cause.catalog_code().cause().to_owned(),
            ));
        }
    };
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &S6aRewardEvaluator,
        )
        .err()
        .map(|errors| {
            let error = errors.first().expect("one check error");
            (
                error.cause.code().as_str().to_owned(),
                error.cause.cause().unwrap_or_default().to_owned(),
            )
        })
}

/// FR-091-AC-55 and FSpec FR-142-AC-18 / TC-187 U11r-U11u: imported unit
/// graphs provide the root, cycle, second-root, and invalid-scale outcomes.
#[trace("FR-091-AC-55", "FSpec-FR-142-AC-18", "FSpec-TC-187")]
#[test]
fn imported_unit_rows_match_the_published_u11r_through_u11u_outcomes() {
    vector_contains("U11r", "admitted");
    vector_contains("U11s", "definition-cycle");
    vector_contains("U11t", "conflicting-definition");
    vector_contains("U11u", "invalid-value");
    assert_eq!(
        refusal(
            package_input_from(&[P_WITH_ROOT]),
            "unit cm : P::L = rational(1, 100) * P::m;\n"
        ),
        None,
        "U11r: the local unit may target the imported root"
    );
    assert_eq!(
        refusal(
            package_input_from(&[P_WITH_ROOT]),
            "unit a : P::L = rational(1, 1) * b;\n\
             unit b : P::L = rational(1, 1) * a;\n"
        ),
        Some(("invalid_package".to_owned(), "definition-cycle".to_owned())),
        "U11s: the imported root does not hide a local cycle"
    );
    assert_eq!(
        refusal(
            package_input_from(&[P_WITH_ROOT]),
            "unit m2 : P::L = rational(1, 1);\n"
        ),
        Some((
            "invalid_package".to_owned(),
            "conflicting-definition".to_owned()
        )),
        "U11t: a local root conflicts with the imported root"
    );
    assert_eq!(
        refusal(
            package_input_from(&[P_WITH_ROOT]),
            "unit z : P::L = rational(0, 1) * P::m;\n"
        ),
        Some(("invalid_package".to_owned(), "invalid-value".to_owned())),
        "U11u: zero scale is invalid even when the target is imported"
    );
}

/// FSpec FR-142-AC-16/18, FSL FR-091-AC-55 / TC-187 U11y/z1/z2: imported
/// dimension turns are visited in their required order before local units.
#[trace(
    "FR-091-AC-55",
    "FSpec-FR-142-AC-16",
    "FSpec-FR-142-AC-18",
    "FSpec-TC-187"
)]
#[test]
fn imported_dimension_turns_match_u11y_and_u11z1_u11z2() {
    for id in ["U11y", "U11z1", "U11z2"] {
        vector_contains(id, "missing_declaration");
    }
    assert_eq!(
        refusal(
            package_input_from(&[P_WITHOUT_ROOT]),
            "dimension Area = P::L ^ 2;\nunit m2 : Area = rational(1, 1);\n"
        ),
        Some(("missing_declaration".to_owned(), "missing-name".to_owned())),
        "U11y: the imported base dimension is visited before the derived dimension"
    );
    assert_eq!(
        refusal(
            {
                let mut packages = package_input_from(&[P_WITHOUT_ROOT]);
                add_derived_dimension(&mut packages, "acme/p", "Area", "ix://acme/p/dimension/L");
                packages
            },
            "unit m2 : P::Area = rational(1, 1);\n"
        ),
        Some(("missing_declaration".to_owned(), "missing-name".to_owned())),
        "U11z1: the imported derived dimension reaches its missing base dimension"
    );
    assert_eq!(
        refusal(
            package_input_from(&[P_WITHOUT_ROOT, Q_WITH_ROOT]),
            "dimension X = P::L * Q::T;\n\
             unit x : X = rational(1, 1);\n\
             unit t : Q::T = rational(0, 1) * Q::s;\n"
        ),
        Some(("missing_declaration".to_owned(), "missing-name".to_owned())),
        "U11z2: the P.L turn precedes the later zero-scale unit in Q"
    );
}

/// FSpec FR-142-AC-19 and TC-187 U34: same-spelled local and package units
/// keep distinct identities, and imported base dimensions take their turn.
#[trace("FSpec-FR-142-AC-19", "FSpec-TC-187")]
#[test]
fn same_spelled_package_units_keep_their_identity_in_u34() {
    vector_contains("U34", "missing_declaration");
    let mut package = package_input_from(&[P_WITH_RATE_AND_LENGTH]);
    add_package_unit_edge(&mut package, "acme/p", "ms", "Rate", "us", "1000");
    assert_eq!(
        refusal(
            package.clone(),
            "dimension Time;\n\
             unit s : Time = rational(1, 1);\n\
             unit ms : Time = rational(1, 1000) * s;\n\
             function f using v(d: Quantity<ms>): Quantity<ms> pure { d }\n\
             function g using v(d: Quantity<P::ms>): Quantity<P::ms> pure { d }\n\
             unit tick : P::Rate = rational(1, 2) * P::us;\n"
        ),
        None,
        "U34: local ms, package ms, and the imported dimension remain distinguishable"
    );
    assert_eq!(
        refusal(
            package.clone(),
            "dimension Time;\nunit tick : Rate = rational(1, 2) * P::us;\n"
        ),
        Some(("missing_declaration".to_owned(), "missing-name".to_owned())),
        "U34: an unqualified package dimension name is not in the local namespace"
    );
    assert_eq!(
        refusal(
            package,
            "dimension Area = P::Length ^ 2;\n\
             unit m2 : Area = rational(1, 1);\n"
        ),
        Some(("missing_declaration".to_owned(), "missing-name".to_owned())),
        "U34: an imported unitless base dimension is rejected at its turn"
    );
}

/// FSpec FR-142-AC-20 and TC-187 U35: source and imported dimensions remain
/// distinct, while unknown aliases and names refuse at the unresolved name.
#[trace("FSpec-FR-142-AC-20", "FSpec-TC-187")]
#[test]
fn imported_dimensions_from_two_packages_match_u35() {
    vector_contains("U35", "ambiguous_declaration");
    let packages = package_input_from(&[P_WITH_TIME_UNITS, Q_WITH_TIME_ROOT]);
    assert_eq!(
        refusal(
            packages.clone(),
            "dimension Time;\nunit s : Time = rational(1, 1);\n\
             unit tenth : P::Time = rational(1, 10) * P::s;\n\
             dimension Speed = P::Time ^ -1 * Q::Time;\n"
        ),
        None,
        "U35: source and imported dimension owners can compose"
    );
    assert_eq!(
        refusal(packages.clone(), "unit x : Time = rational(1, 1);\n"),
        Some(("missing_declaration".to_owned(), "missing-name".to_owned())),
        "U35: bare Time has no local declaration"
    );
    assert_eq!(
        refusal(packages.clone(), "unit x : Z::Time = rational(1, 1);\n"),
        Some(("missing_declaration".to_owned(), "missing-name".to_owned())),
        "U35: an undeclared model alias is refused"
    );
    assert_eq!(
        refusal(packages.clone(), "unit x : P::Speed = rational(1, 1);\n"),
        Some(("missing_declaration".to_owned(), "missing-name".to_owned())),
        "U35: an absent imported dimension is refused"
    );
    assert_eq!(
        refusal(packages, "dimension Time;\ndimension Time;\n"),
        Some((
            "ambiguous_declaration".to_owned(),
            "ambiguous-name".to_owned()
        )),
        "U35: duplicate source dimensions make the bare name ambiguous"
    );
}

/// FSpec FR-142-AC-21 and TC-187 U36: dimensions belonging to different
/// packages do not become interchangeable merely because their names match.
#[trace("FSpec-FR-142-AC-21", "FSpec-TC-187")]
#[test]
fn values_from_distinct_package_dimensions_match_u36() {
    vector_contains("U36", "incompatible dimensions");
    let packages = package_input_from(&[P_WITH_TIME_UNITS, Q_WITH_TIME_ROOT]);
    assert_eq!(
        refusal(
            packages.clone(),
            "function f using v(d: Quantity<P::s>): Boolean pure { true }\n\
             function g using v(): Boolean pure { f(3 Q::s) }\n"
        ),
        Some(("ill_typed".to_owned(), "type-mismatch".to_owned())),
        "U36: equal dimension names from distinct owners fail parameter typing"
    );
    assert_eq!(
        refusal(
            packages.clone(),
            "function f using v(): Quantity<P::s> pure { 3 P::s + 3 Q::s }\n"
        ),
        Some(("ill_typed".to_owned(), "type-mismatch".to_owned())),
        "U36: addition refuses quantities with incompatible dimensions"
    );
    assert_eq!(
        refusal(
            packages.clone(),
            "function f using v(): Boolean pure { 3 P::s < 3 Q::s }\n"
        ),
        Some(("ill_typed".to_owned(), "type-mismatch".to_owned())),
        "U36: comparison refuses quantities with incompatible dimensions"
    );
    assert_eq!(
        refusal(
            packages,
            "function f using v(): Quantity<Q::s> pure { convert<Quantity<Q::s>>(3 P::s) }\n"
        ),
        Some(("ill_typed".to_owned(), "type-mismatch".to_owned())),
        "U36: conversion refuses quantities with incompatible dimensions"
    );
}

/// FSpec FR-142-AC-22 and TC-187 U37: equivalence declarations preserve
/// package identity, reject cycles and inconsistent ratios, and require a
/// strictly positive root ratio.
#[trace("FSpec-FR-142-AC-22", "FSpec-TC-187")]
#[test]
fn dimension_equivalence_declarations_match_u37() {
    vector_contains("U37", "definition-cycle");
    let packages = package_input_from(&[P_WITH_TIME_UNITS, Q_WITH_TIME_ROOT]);
    assert_eq!(
        refusal(
            packages.clone(),
            "dimension P::Time ~ Q::Time by 1 P::s = 1 Q::s;\n"
        ),
        None,
        "U37: an equivalence between package dimensions admits its root equation"
    );
    assert_eq!(
        refusal(
            packages.clone(),
            "dimension Time;\nunit s : Time = rational(1, 1);\n\
             dimension P::Time ~ Time by 1 P::s = 1 s;\n"
        ),
        None,
        "U37: an equivalence may pair an imported and local dimension"
    );
    for (source, expected) in [
        (
            "dimension P::Time ~ Z::Time by 1 P::s = 1 Z::s;\n",
            "unknown model alias refuses",
        ),
        (
            "dimension P::Time ~ Nothing by 1 P::s = 1 P::s;\n",
            "unknown dimension refuses",
        ),
        (
            "dimension P::Time ~ Q::Time by 1 P::s = 1 Q::second;\n",
            "unknown package unit refuses",
        ),
    ] {
        assert_eq!(
            refusal(packages.clone(), source),
            Some(("missing_declaration".to_owned(), "missing-name".to_owned())),
            "U37: {expected}"
        );
    }
    assert_eq!(
        refusal(
            packages.clone(),
            "dimension P::Time ~ P::Time by 1 P::s = 1 P::s;\n"
        ),
        Some(("invalid_package".to_owned(), "definition-cycle".to_owned())),
        "U37: self-equivalence retains the cycle"
    );
    assert_eq!(
        refusal(
            packages.clone(),
            "dimension P::Time ~ Q::Time by 1 P::s = 1 Q::s;\n\
             dimension Q::Time ~ P::Time by 2 Q::s = 1 P::s;\n"
        ),
        Some((
            "invalid_package".to_owned(),
            "conflicting-definition".to_owned()
        )),
        "U37: inconsistent reciprocal root ratios conflict"
    );
    for (ratio, label) in [("0", "zero"), ("rational(-1, 1)", "negative")] {
        assert_eq!(
            refusal(
                packages.clone(),
                &format!("dimension P::Time ~ Q::Time by {ratio} P::s = 1 Q::s;\n")
            ),
            Some(("invalid_package".to_owned(), "invalid-value".to_owned())),
            "U37: {label} equivalence ratio refuses"
        );
    }
}

/// FSpec FR-142-AC-23/24 and TC-187 U38: admitted equivalences allow explicit
/// conversion, retain distinct unit typing, and do not equate unrelated base
/// dimensions.
#[trace("FSpec-FR-142-AC-23", "FSpec-FR-142-AC-24", "FSpec-TC-187")]
#[test]
fn conversions_over_equivalent_dimensions_match_u38() {
    vector_contains("U38", "3000");
    let mut packages = package_input_from(&[P_WITH_TIME_AND_LENGTH_ROOT, Q_WITH_TIME_ROOT]);
    add_package_unit_edge(&mut packages, "acme/q", "ms", "Time", "s", "1/1000");
    let equivalence = "dimension P::Time ~ Q::Time by 1 P::s = 1000 Q::ms;\n";
    assert_eq!(
        refusal(
            packages.clone(),
            &format!(
                "{equivalence}function f using v(): Quantity<Q::ms> pure {{ convert<Quantity<Q::ms>>(3 P::s) }}\n"
            )
        ),
        None,
        "U38: conversion between equivalent dimensions checks"
    );
    assert_eq!(
        refusal(
            packages.clone(),
            &format!(
                "{equivalence}function f using v(): Quantity<P::s> pure {{ 3 P::s + 3 Q::s }}\n"
            )
        ),
        Some(("ill_typed".to_owned(), "type-mismatch".to_owned())),
        "U38: equal dimension classes do not make distinct units interchangeable"
    );
    assert_eq!(
        refusal(
            packages,
            &format!(
                "{equivalence}function f using v(): Quantity<P::m> pure {{ convert<Quantity<P::m>>(3 Q::s) }}\n"
            )
        ),
        Some(("ill_typed".to_owned(), "type-mismatch".to_owned())),
        "U38: equivalence does not permit conversion between unrelated dimensions"
    );
}

/// FSpec FR-142-AC-26 and TC-187 U40: root ratios are symmetric by spelling,
/// keep their canonical magnitude, and detect inconsistent three-owner cycles.
#[trace("FSpec-FR-142-AC-26", "FSpec-TC-187")]
#[test]
fn root_ratios_across_three_packages_match_u40() {
    vector_contains("U40", "1000");
    let mut packages = package_input_from(&[
        P_WITH_TIME_AND_LENGTH_ROOT,
        Q_WITH_TIME_ROOT,
        G_WITH_MILLISECOND_ROOT,
    ]);
    add_package_unit_edge(&mut packages, "acme/q", "ms", "Time", "s", "1/1000");
    add_package_unit_edge(&mut packages, "acme/g", "s", "Time", "ms", "1000");
    for source in [
        "dimension P::Time ~ G::Time by 1 P::s = 1000 G::ms;\n",
        "dimension P::Time ~ G::Time by 1 P::s = 1 G::s;\n",
        "dimension G::Time ~ P::Time by 1000 G::ms = 1 P::s;\n",
    ] {
        assert_eq!(
            refusal(packages.clone(), source),
            None,
            "U40: equivalent root-ratio spelling admits: {source}"
        );
    }
    assert_eq!(
        refusal(
            packages.clone(),
            "dimension P::Time ~ G::Time by 1 P::s = 1000 G::ms;\n\
             dimension P::Time ~ Q::Time by 1 P::s = 1000 Q::ms;\n\
             dimension Q::Time ~ G::Time by 1 Q::s = 1001 G::ms;\n"
        ),
        Some((
            "invalid_package".to_owned(),
            "conflicting-definition".to_owned()
        )),
        "U40: inconsistent root ratios around a three-package cycle conflict"
    );
}
