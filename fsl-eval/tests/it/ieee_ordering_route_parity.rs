// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-397-AC-30, FR-397-AC-31 and FR-940-AC-15 (TC-3077): the source route
//! and the package route charge an IEEE ordering identically. Whether a
//! `numeric_equal` node is an ordering part is read from the expression's
//! shape on both routes (FR-397 "Ordering parts"), so the charge logs of a
//! function evaluated from its checked graph and from its written bytes are
//! equal entry for entry.

use filament_eval::CallFailure;
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
    both_routes_of(F64, body, arguments)
}

/// `both_routes` over parameters of the type `float`.
fn both_routes_of(float: &str, body: &str, arguments: &[Value]) -> (Charged, Charged) {
    let parameters = format!("p: {float}, q: {float}, r: {float}");
    let (source, packaged) = routes(&parameters, body, arguments);
    (source, packaged.expect("the package call runs"))
}

/// `function f using v(parameters): Boolean pure { body }` evaluated at
/// `arguments` on the source route and on the package route, whose call may
/// be refused.
fn routes(
    parameters: &str,
    body: &str,
    arguments: &[Value],
) -> (Charged, Result<Charged, CallFailure>) {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function f using v({parameters}): Boolean pure {{ {body} }}\n"
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
        .map(|packaged| charged(format!("{:?}", packaged.outcome), &meter));
    (source, packaged)
}

fn f64v(bits: u64) -> Value {
    Value::Float(IeeeValue::binary64(bits))
}

/// FR-397-AC-30, FR-940-AC-15: each ordering body charges the same log on
/// both routes, with ordinary operands and with `p` a quiet NaN. The AC's
/// `sqrt` body is under the ignored
/// `an_ordering_over_square_roots_charges_the_same_on_both_routes` (QSL-2625),
/// so this test covers the AC's other bodies only.
#[trace("TC-3077", "FR-397-AC-30")]
#[trace("TC-3077", "FR-940-AC-15")]
#[test]
fn an_ordering_charges_the_same_on_the_source_and_package_routes_except_square_roots() {
    let one = f64v(0x3ff0_0000_0000_0000);
    let two = f64v(0x4000_0000_0000_0000);
    let nan = f64v(0x7ff8_0000_0000_0001);
    for body in ["p < q", "p <= q", "p > q", "p >= q", "-p < -q"] {
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

/// FR-397-AC-30 and FR-940-AC-15 for an ordering over `sqrt` operands, at
/// AC-30's inputs. The source route records the inexact flag of `sqrt(2.0)`
/// as a value loss at the node's location, and a package function body has no
/// location for it, so the package route faults (`value-loss-site-is-located`,
/// FR-390). QSL-2625 specifies the location.
#[trace("TC-3077", "FR-397-AC-30")]
#[trace("TC-3077", "FR-940-AC-15")]
#[test]
#[ignore = "QSL-2625: a package function body has no location for a flag-raising IEEE operation's value loss"]
fn an_ordering_over_square_roots_charges_the_same_on_both_routes() {
    let one = f64v(0x3ff0_0000_0000_0000);
    let two = f64v(0x4000_0000_0000_0000);
    let nan = f64v(0x7ff8_0000_0000_0001);
    let body = "filament::value::ieee::sqrt(p) <= filament::value::ieee::sqrt(q)";
    for p in [&one, &nan] {
        let (source, packaged) = both_routes(body, &[p.clone(), two.clone(), two.clone()]);
        assert_eq!(source, packaged, "{body} at {p:?}");
    }
}

const F32: &str = "Float32[nearest-even]";

fn f32v(bits: u32) -> Value {
    Value::Float(IeeeValue::binary32(bits))
}

/// The IEEE applications the package reader projects (FR-390), at flag-free
/// operands, write the source route's log on the package route, for both
/// widths.
#[trace("TC-3077", "FR-940-AC-15")]
#[test]
fn flag_free_ieee_operations_charge_the_same_on_both_routes() {
    let bodies = [
        "p + q = r",
        "r - q = p",
        "p * q = q",
        "q / p = q",
        "-p < q",
        "filament::value::ieee::bitIdentical(p, p)",
        "filament::value::ieee::bitIdentical(p, q)",
        "filament::value::ieee::sqrt(r) = q",
    ];
    let wide = [
        f64v(0x3ff0_0000_0000_0000),
        f64v(0x4000_0000_0000_0000),
        f64v(0x4010_0000_0000_0000),
    ];
    let narrow = [f32v(0x3f80_0000), f32v(0x4000_0000), f32v(0x4080_0000)];
    // `p + q = r` needs 1 + 2 = 3, and the square root needs sqrt(4) = 2.
    let sum_wide = [
        wide[0].clone(),
        wide[1].clone(),
        f64v(0x4008_0000_0000_0000),
    ];
    let sum_narrow = [narrow[0].clone(), narrow[1].clone(), f32v(0x4040_0000)];
    for (float, arguments, sums) in [(F64, &wide, &sum_wide), (F32, &narrow, &sum_narrow)] {
        for body in bodies {
            let arguments = if body.starts_with("p + q") || body.starts_with("r - q") {
                sums
            } else {
                arguments
            };
            let (source, packaged) = both_routes_of(float, body, arguments);
            assert_eq!(source, packaged, "{float}: {body}");
        }
    }
}

/// QSL-2625: the flag-raising IEEE operations on the package route.
#[trace("TC-3077", "FR-940-AC-15")]
#[test]
#[ignore = "QSL-2625: a package function body has no location for a flag-raising IEEE operation's value loss"]
fn flag_raising_ieee_operations_charge_the_same_on_both_routes() {
    for float in [F64, F32] {
        let bits = |one: u64, three: u64| match float {
            F64 => [f64v(one), f64v(three), f64v(three)],
            _ => [
                f32v((one >> 32) as u32),
                f32v((three >> 32) as u32),
                f32v((three >> 32) as u32),
            ],
        };
        let arguments = bits(0x3ff0_0000_0000_0000, 0x4008_0000_0000_0000);
        for body in ["p / q = r", "filament::value::ieee::sqrt(q) = r"] {
            let (source, packaged) = both_routes_of(float, body, &arguments);
            assert_eq!(source, packaged, "{float}: {body}");
        }
    }
}

/// A `Float64[mode]` nested in a parameter's option type is refused like the
/// sibling scalar forms, not typed as the bare float (FR-390).
#[test]
fn a_float_nested_in_an_option_parameter_is_refused_on_the_package_route() {
    let float = filament_exact::ValueType::Float(filament_exact::FloatType::new(
        filament_exact::IeeeWidth::Binary64,
        filament_exact::RoundingMode::NearestEven,
    ));
    let (_, packaged) = routes(
        &format!("p: Option<{F64}>"),
        "true",
        &[filament_exact::OptionValue::none(float)],
    );
    let refusal = packaged.expect_err("the package route refuses the nested float");
    assert!(
        format!("{refusal:?}").contains("unknown_required_feature"),
        "{refusal:?}"
    );
}
