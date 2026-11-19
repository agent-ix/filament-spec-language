// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-397-AC-30, FR-397-AC-31 and FR-940-AC-15 (TC-3077): the source route
//! and the package route charge an IEEE ordering identically. Whether a
//! `numeric_equal` node is an ordering part is read from the expression's
//! shape on both routes (FR-397 "Ordering parts"), so the charge logs of a
//! function evaluated from its checked graph and from its written bytes are
//! equal entry for entry.

use filament_exact::{ChargePoint, IeeeValue, Meter, ScalarLimits, Value};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::value::{admit_ieee_profile, DefinitionRole};
use ix_trace_rs::trace;

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

const F64: &str = "Float64[nearest-even]";

/// One route's outcome, written in its debug form, and charge log.
#[derive(Debug, PartialEq)]
struct Charged {
    outcome: String,
    charges: Vec<ChargePoint>,
}

fn charged(outcome: String, meter: &Meter) -> Charged {
    Charged {
        outcome,
        charges: meter
            .admitted_charges()
            .iter()
            .map(|charge| charge.point)
            .collect(),
    }
}

/// `function f using v(p, q, r: Float64[nearest-even]): Boolean pure { body }`
/// evaluated at `arguments` on the source route and, from the package's
/// written bytes, on the package route.
fn both_routes(body: &str, arguments: &[Value]) -> (Charged, Charged) {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function f using v(p: {F64}, q: {F64}, r: {F64}): Boolean pure {{ {body} }}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler admits the unit");
    declarations.ieee_profile =
        Some(admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).unwrap());
    let package = CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .unwrap_or_else(|refusal| panic!("{body}: {refusal:?}")),
    );
    let f = QualifiedName::unqualified("f").expect("f is an identifier");
    let mut meter = Meter::new(UNLIMITED);
    let source = package
        .call(
            &f,
            arguments.to_vec(),
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the source call runs");
    let source = charged(format!("{:?}", source.outcome), &meter);

    let emission = fsl_package::emit_checked(&package, fsl_package::EmitLimits::fixture())
        .expect("the checked package emits");
    let no_domains = package_input(std::iter::empty::<&[u8]>()).expect("empty model closure");
    let read = fsl_package::read_evaluation_emission(
        &package,
        &emission,
        LibraryName::new("fixture").expect("a library name"),
        &no_domains,
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
        fsl_package::SourceWitnessSupplier::new(&emission),
    )
    .expect("the reader admits the package");
    let v2 = read.admitted();
    let budget = || fsl_package::V2ReadLimits::fixture().native_join_budget();
    let identities = emission
        .identity_rows(v2, &mut budget())
        .expect("the identity rows materialize");
    let equality = emission
        .equality_witness(v2, &mut budget())
        .expect("the equality witness materializes");
    let bare = fsl_package::BarePackage::new(v2)
        .with_supply(
            fsl_package::BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget(),
        )
        .expect("the fixture budget admits the supply");
    let mut meter = Meter::new(UNLIMITED);
    let packaged = bare
        .call(
            &f,
            arguments.to_vec(),
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the package call runs");
    (source, charged(format!("{:?}", packaged.outcome), &meter))
}

fn f64v(bits: u64) -> Value {
    Value::Float(IeeeValue::binary64(bits))
}

/// FR-397-AC-30, FR-940-AC-15: each ordering body charges the same log on
/// both routes, with ordinary operands and with `p` a quiet NaN.
#[trace("TC-3077", "FR-397-AC-30")]
#[trace("TC-3077", "FR-940-AC-15")]
#[test]
#[ignore = "QSL-2566: the package-route call refuses a Float64 argument (unknown_required_feature), and its reader reads no ordering part, so it would charge N(p), N(q) and numericEqual(p, q)"]
fn an_ordering_charges_the_same_on_the_source_and_package_routes() {
    let one = f64v(0x3ff0_0000_0000_0000);
    let two = f64v(0x4000_0000_0000_0000);
    let nan = f64v(0x7ff8_0000_0000_0001);
    for body in [
        "p < q",
        "p <= q",
        "p > q",
        "p >= q",
        "-p < -q",
        "filament::value::ieee::sqrt(p) <= filament::value::ieee::sqrt(q)",
    ] {
        for p in [&one, &nan] {
            let (source, packaged) = both_routes(body, &[p.clone(), two.clone(), two.clone()]);
            assert_eq!(source, packaged, "{body} at {p:?}");
        }
    }
}

/// FR-397-AC-31: a hand-written composition in the ordering's shape charges
/// as `p < q`; near-miss shapes and a standalone `numericEqual` beside an
/// ordering charge each `numeric_equal` they reach, on both routes alike.
#[trace("TC-3077", "FR-397-AC-31")]
#[test]
#[ignore = "QSL-2566: the package-route call refuses a Float64 argument, and the ordering part is a source-only node flag, not the FR-397 shape rule"]
fn the_ordering_part_rule_reads_the_shape_on_both_routes() {
    let args = [
        f64v(0x3ff0_0000_0000_0000),
        f64v(0x4000_0000_0000_0000),
        f64v(0x4008_0000_0000_0000),
    ];
    let ieee = "filament::value::ieee::";
    let operands = |log: &[ChargePoint]| {
        log.iter()
            .filter(|point| matches!(point, ChargePoint::IeeeOperands))
            .count()
    };
    let (ordering, ordering_package) = both_routes("p < q", &args);
    assert_eq!(ordering, ordering_package);
    let by_hand = format!(
        "{ieee}numericEqual(p, p) and {ieee}numericEqual(q, q) and \
         ({ieee}totalOrder(p, q) and not {ieee}numericEqual(p, q))"
    );
    let (source, packaged) = both_routes(&by_hand, &args);
    assert_eq!(source.charges, ordering.charges, "{by_hand}");
    assert_eq!(packaged.charges, ordering.charges, "{by_hand}");
    for near_miss in [
        format!(
            "{ieee}numericEqual(p, p) and {ieee}numericEqual(q, q) and \
             ({ieee}totalOrder(p, q) and not {ieee}numericEqual(q, p))"
        ),
        format!(
            "{ieee}numericEqual(p, p) and {ieee}numericEqual(r, r) and \
             ({ieee}totalOrder(p, q) and not {ieee}numericEqual(p, q))"
        ),
    ] {
        let (source, packaged) = both_routes(&near_miss, &args);
        assert_eq!(source, packaged, "{near_miss}");
        // Three numeric_equal nodes and one total_order, each charged.
        assert_eq!(operands(&source.charges), 4, "{near_miss}");
    }
    let beside = format!("p < q and {ieee}numericEqual(p, p)");
    let (source, packaged) = both_routes(&beside, &args);
    assert_eq!(source, packaged, "{beside}");
    assert_eq!(
        operands(&source.charges),
        operands(&ordering.charges) + 1,
        "{beside}"
    );
}
