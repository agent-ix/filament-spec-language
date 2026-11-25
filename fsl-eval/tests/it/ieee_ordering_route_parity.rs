// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-397-AC-30, FR-397-AC-31 and FR-940-AC-15 (TC-3077): the source route
//! and the package route charge an IEEE ordering identically. Whether a
//! `numeric_equal` node is an ordering part is read from the expression's
//! shape on both routes (FR-397 "Ordering parts"), so the charge logs of a
//! function evaluated from its checked graph and from its written bytes are
//! equal entry for entry.

use filament_eval::{CallFailure, FamilyOutcome};
use filament_exact::{
    ChargePoint, FieldValue, IeeeFlag, IeeeValue, Integer, LimitKind, Meter, ObjectId,
    ObjectReference, ScalarLimits, UniverseId, Value, ValueType,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use filament_semantic_value::location::ExpressionOwner as LocationOwner;
use filament_semantic_value::loss::ValueLoss;
use filament_semantic_value::object_closure::ObjectClosure;
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::value::{DefinitionRole, admit_ieee_profile};
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

/// One route's outcome, loss records and stop location, written in their
/// debug forms, and charge log.
#[derive(Debug, PartialEq)]
struct Charged {
    outcome: String,
    losses: String,
    location: String,
    work_units: u64,
    charges: Vec<ChargePoint>,
}

fn charged(evaluation: &filament_eval::Evaluation, meter: &Meter) -> Charged {
    Charged {
        outcome: format!("{:?}", evaluation.outcome),
        losses: format!("{:?}", evaluation.losses),
        location: format!("{:?}", evaluation.location),
        work_units: meter.consumed(LimitKind::WorkUnits),
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
    unit_routes(
        &format!("function f using v({parameters}): Boolean pure {{ {body} }}\n"),
        "f",
        arguments,
    )
}

/// A unit holding `functions` evaluated at `arguments` through `entry` on the
/// source route and on the package route, whose call may be refused.
fn unit_routes(
    functions: &str,
    entry: &str,
    arguments: &[Value],
) -> (Charged, Result<Charged, CallFailure>) {
    let (package, emission) = checked_unit(functions);
    package_routes(&package, &emission, entry, arguments, UNLIMITED)
}

/// The checked package of a unit holding `functions`, and its emission.
fn checked_unit(functions: &str) -> (CheckedPackage, fsl_package::Emission) {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         {functions}"
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
            .unwrap_or_else(|refusal| panic!("{functions}: {refusal:?}")),
    );
    let emission = fsl_package::emit_checked(&package, fsl_package::EmitLimits::fixture())
        .expect("the checked package emits");
    (package, emission)
}

/// FR-255 AC-17/19: the evaluator's package route accepts maximum I2 identity
/// join limits and retains the first typed work refusal at a zero allowance.
#[trace("TC-721", "FR-255-AC-17", "FR-255-AC-19")]
#[test]
fn evaluator_identity_rows_accept_maximum_and_report_zero_work_refusal() {
    let (package, emission) =
        checked_unit("function f using v(x: Integer): Boolean pure { x >= 0 }\n");
    let no_models = package_input(std::iter::empty::<&[u8]>()).expect("no model imports");
    let read = fsl_package::read_evaluation_emission(
        &package,
        &emission,
        LibraryName::new("fixture").expect("library name"),
        &no_models,
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
        fsl_package::SourceWitnessSupplier::new(&emission),
    )
    .expect("the checked bytes are admitted");
    let admitted = read.admitted();

    let mut maximum = fsl_package::NativeJoinBudget::new(u64::MAX, u64::MAX);
    emission
        .identity_rows(admitted, &mut maximum)
        .expect("u64::MAX work and storage admit the evaluator identity join");
    let needed = maximum.consumed();
    assert!(
        needed.work > 0,
        "the I2 identity join charges positive work"
    );
    assert!(
        needed.storage > 0,
        "the I2 identity join charges positive storage"
    );

    let mut exact = fsl_package::NativeJoinBudget::new(needed.work, needed.storage);
    emission
        .identity_rows(admitted, &mut exact)
        .expect("the evaluator's measured W/S join allowance admits");
    assert_eq!(exact.consumed(), needed);

    let mut zero_work = fsl_package::NativeJoinBudget::new(0, u64::MAX);
    let error = match emission.identity_rows(admitted, &mut zero_work) {
        Err(fsl_package::IdentityFileRefusal::Rows(error)) => error,
        Err(other) => panic!("zero work returned the wrong refusal: {other:?}"),
        Ok(_) => panic!("zero work admitted a positive I2 identity join charge"),
    };
    assert_eq!(error.code(), fsl_package::NativeRowErrorCode::WorkLimit);
    let denied = error
        .denied()
        .expect("the evaluator refusal preserves the first denied charge");
    assert!(denied.amount > 0);
    assert_eq!(zero_work.first_unpaid(), Some(denied));
    assert_eq!(zero_work.first_unpaid_locus(), error.locus());
    assert_eq!(error.configured_bound(), Some(0));
}

fn signature_after<'a>(source: &'a str, marker: &str) -> &'a str {
    source
        .split_once(marker)
        .unwrap_or_else(|| panic!("missing public API marker {marker}"))
        .1
        .split_once('{')
        .expect("function signature has a body")
        .0
}

/// `entry` called at `arguments` on the source route over `package` and on the
/// package route over the bytes of `emission`.
fn package_routes(
    package: &CheckedPackage,
    emission: &fsl_package::Emission,
    entry: &str,
    arguments: &[Value],
    limits: ScalarLimits,
) -> (Charged, Result<Charged, CallFailure>) {
    let f = QualifiedName::unqualified(entry).expect("the entry is an identifier");
    let mut meter = Meter::new(limits);
    let source = package
        .call(
            &f,
            arguments.to_vec(),
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the source call runs");
    let source = charged(&source, &meter);

    let no_domains = package_input(std::iter::empty::<&[u8]>()).expect("empty model closure");
    let read = fsl_package::read_evaluation_emission(
        package,
        emission,
        LibraryName::new("fixture").expect("a library name"),
        &no_domains,
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
        fsl_package::SourceWitnessSupplier::new(emission),
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
    let mut meter = Meter::new(limits);
    let packaged = bare
        .call(
            &f,
            arguments.to_vec(),
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .map(|packaged| charged(&packaged, &meter));
    (source, packaged)
}

fn f64v(bits: u64) -> Value {
    Value::Float(IeeeValue::binary64(bits))
}

/// FR-397-AC-30, FR-940-AC-15: each ordering body charges the same log on
/// both routes, with ordinary operands and with `p` a quiet NaN. The AC's
/// `sqrt` body is covered by
/// `an_ordering_over_square_roots_charges_the_same_on_both_routes`, so this
/// test covers the AC's other bodies.
#[trace("TC-3077", "FR-397-AC-30")]
#[trace("TC-3077", "FR-940-AC-15")]
#[test]
fn an_ordering_charges_the_same_on_the_source_and_package_routes() {
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

const SQRTS: &str = "filament::value::ieee::sqrt(p) <= filament::value::ieee::sqrt(q)";

/// The unit of FR-531-AC-36: `f` orders two square roots and `g` calls `f`.
fn square_root_unit() -> String {
    format!(
        "function f using v(p: {F64}, q: {F64}, r: {F64}): Boolean pure {{ {SQRTS} }}\n\
         function g using v(p: {F64}, q: {F64}, r: {F64}): Boolean pure {{ f(p, q, r) }}\n"
    )
}

fn twos() -> [Value; 3] {
    let two = f64v(0x4000_0000_0000_0000);
    [two.clone(), two.clone(), two]
}

/// FR-531-AC-36: `f` and `g` complete on both routes with equal charge logs
/// and two inexact losses, located in `f`'s body at the checker's paths `[0]`
/// and `[1]` for both, never at `g`'s call site.
#[trace("TC-3133", "FR-531-AC-36")]
#[test]
fn a_call_through_a_function_locates_the_loss_in_the_callee_body() {
    let unit = square_root_unit();
    for entry in ["f", "g"] {
        let (source, packaged) = unit_routes(&unit, entry, &twos());
        let packaged = packaged.expect("the package call runs");
        assert_eq!(source, packaged, "{entry}");
        assert!(
            source.outcome.contains("true"),
            "{entry}: {}",
            source.outcome
        );
        assert_eq!(
            source.losses.matches("LocatedLoss").count(),
            2,
            "{}",
            source.losses
        );
        let first = source
            .losses
            .find("depth: 1, last_index: Some(0)")
            .expect("a loss at [0]");
        let second = source
            .losses
            .find("depth: 1, last_index: Some(1)")
            .expect("a loss at [1]");
        assert!(first < second, "{}", source.losses);
        assert!(
            !source.losses.contains("depth: 2") && !source.losses.contains("\"g\""),
            "{}",
            source.losses
        );
    }
}

/// FR-531-AC-36: the same bytes with the second `sqrt`'s row removed are
/// refused by the strict reader and evaluate nothing.
#[trace("TC-3133", "FR-531-AC-36")]
#[test]
fn a_function_body_without_its_location_rows_is_refused() {
    let (package, emission) = checked_unit(&square_root_unit());
    let mut wire: serde_json::Value =
        serde_json::from_slice(emission.package().bytes()).expect("the package is JSON");
    let rows = wire["checked_term_data"]["body_locations"]
        .as_array_mut()
        .expect("the body location rows");
    let removed = rows
        .iter()
        .position(|row| {
            row["location"]["up"] == serde_json::json!(1)
                && row["location"]["down"] == serde_json::json!([1])
        })
        .expect("the second sqrt's row");
    rows.remove(removed);
    let forged = emission
        .clone()
        .with_package_bytes(serde_json::to_vec(&wire).expect("the package encodes"));
    let no_domains = package_input(std::iter::empty::<&[u8]>()).expect("empty model closure");
    let refused = fsl_package::read_evaluation_emission(
        &package,
        &forged,
        LibraryName::new("fixture").expect("a library name"),
        &no_domains,
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
        fsl_package::SourceWitnessSupplier::new(&forged),
    )
    .err()
    .expect("the reader refuses the package");
    assert!(
        format!("{refused:?}").contains("/checked_term_data/body_locations"),
        "{refused:?}"
    );
}

/// One row of FR-531-AC-37: the parameters after `r`, the term `e` and its
/// arguments at the width `float`, and the flags the one loss holds.
struct FlagRow {
    parameters: String,
    term: String,
    arguments: Vec<Value>,
    flags: &'static [&'static str],
}

fn float_of(float: &str, value: f64) -> Value {
    if float == F32 {
        f32v((value as f32).to_bits())
    } else {
        f64v(value.to_bits())
    }
}

fn smallest_subnormal(float: &str) -> Value {
    if float == F32 { f32v(1) } else { f64v(1) }
}

fn largest_finite(float: &str) -> Value {
    if float == F32 {
        f32v(f32::MAX.to_bits())
    } else {
        f64v(f64::MAX.to_bits())
    }
}

fn rational(numerator: i64, denominator: i64) -> Value {
    Value::Rational(
        filament_exact::Rational::new(
            filament_exact::Integer::from(numerator),
            filament_exact::Integer::from(denominator),
        )
        .expect("a nonzero denominator"),
    )
}

fn flag_rows(float: &str) -> Vec<FlagRow> {
    let pq = format!("p: {float}, q: {float}");
    let row = |parameters: &str, term: &str, arguments: Vec<Value>, flags| FlagRow {
        parameters: parameters.to_owned(),
        term: term.to_owned(),
        arguments,
        flags,
    };
    let mut rows = vec![
        row(
            &pq,
            "p + q",
            vec![float_of(float, 1.0), float_of(float, 2f64.powi(-60))],
            &["Inexact"],
        ),
        row(
            &pq,
            "p - q",
            vec![float_of(float, 1.0), float_of(float, 2f64.powi(-60))],
            &["Inexact"],
        ),
        row(
            &pq,
            "p * q",
            vec![largest_finite(float), largest_finite(float)],
            &["Overflow", "Inexact"],
        ),
        row(
            &pq,
            "p * q",
            vec![smallest_subnormal(float), float_of(float, 0.5)],
            &["Underflow", "Inexact"],
        ),
        row(
            &pq,
            "p / q",
            vec![float_of(float, 1.0), float_of(float, 3.0)],
            &["Inexact"],
        ),
        row(
            &pq,
            "p / q",
            vec![float_of(float, 1.0), float_of(float, 0.0)],
            &["DivideByZero"],
        ),
        row(
            &format!("p: {float}"),
            "filament::value::ieee::sqrt(p)",
            vec![float_of(float, 2.0)],
            &["Inexact"],
        ),
        row(
            &format!("p: {float}"),
            "filament::value::ieee::sqrt(p)",
            vec![float_of(float, -1.0)],
            &["Invalid"],
        ),
        row(
            "a: Rational[-10, 10; 1, 10]",
            &format!("convert<{float}>(a)"),
            vec![rational(1, 10)],
            &["Inexact"],
        ),
    ];
    if float == F32 {
        rows.push(row(
            &format!("b: {F64}"),
            &format!("convert<{F32}>(b)"),
            vec![f64v(0.1f64.to_bits())],
            &["Inexact"],
        ));
        rows.push(row(
            &format!("b: {F64}"),
            &format!("convert<{F32}>(b)"),
            vec![f64v(f64::MAX.to_bits())],
            &["Overflow", "Inexact"],
        ));
    }
    rows
}

/// FR-531-AC-37: every flag-raising IEEE operation over both widths records
/// one loss with exactly its flags, located at `Body { f, 0 }` path `[1]` for
/// the right operand of `f`'s ordering and at `Body { g, 1 }` path `[]` for
/// `g`'s root, and the two routes agree outcome, loss and charge log.
#[trace("TC-3133", "FR-531-AC-37")]
#[test]
fn every_flag_raising_ieee_operation_is_located_alike_on_both_routes() {
    for float in [F32, F64] {
        for row in flag_rows(float) {
            let unit = format!(
                "function f using v(r: {float}, {parameters}): Boolean pure {{ r <= {term} }}\n\
                 function g using v({parameters}): {float} pure {{ {term} }}\n",
                parameters = row.parameters,
                term = row.term,
            );
            let (package, emission) = checked_unit(&unit);
            for (entry, arguments, location) in [
                (
                    "f",
                    [vec![float_of(float, 0.0)], row.arguments.clone()].concat(),
                    ("\"f\"", "index: 0", "depth: 1, last_index: Some(1)"),
                ),
                (
                    "g",
                    row.arguments.clone(),
                    ("\"g\"", "index: 1", "depth: 0, last_index: None"),
                ),
            ] {
                let label = format!("{float}: {} as {entry}", row.term);
                let (source, packaged) =
                    package_routes(&package, &emission, entry, &arguments, UNLIMITED);
                let packaged = packaged.unwrap_or_else(|failure| panic!("{label}: {failure:?}"));
                assert_eq!(source, packaged, "{label}");
                assert_eq!(
                    source.losses.matches("LocatedLoss").count(),
                    1,
                    "{label}: {}",
                    source.losses
                );
                let bits: u8 = row
                    .flags
                    .iter()
                    .map(|flag| match *flag {
                        "Invalid" => 1,
                        "DivideByZero" => 2,
                        "Overflow" => 4,
                        "Underflow" => 8,
                        "Inexact" => 16,
                        other => panic!("unknown flag {other}"),
                    })
                    .sum();
                assert!(
                    source.losses.contains(&format!("IeeeFlags({bits})")),
                    "{label}: {}",
                    source.losses
                );
                for fragment in [location.0, location.1, location.2] {
                    assert!(
                        source.losses.contains(fragment),
                        "{label}: {}",
                        source.losses
                    );
                }
            }
        }
    }
}

/// FR-531-AC-38: a call refused by a `work_units` limit from `0` to one below
/// what the completed call spends returns the same `Incomplete`, location and
/// charge log on both routes, and one limit stops at the second `sqrt` of
/// `f`'s body, `[1]`, not the lowered `[1, 0]`.
#[trace("TC-3133", "FR-531-AC-38")]
#[test]
fn a_denied_charge_in_a_body_is_located_alike_on_both_routes() {
    let (package, emission) = checked_unit(&square_root_unit());
    let (completed, _) = package_routes(&package, &emission, "f", &twos(), UNLIMITED);
    let spent = completed.work_units;
    assert!(spent > 0);
    let mut located = std::collections::BTreeSet::new();
    for limit in 0..spent {
        let limits = ScalarLimits {
            work_units: limit,
            ..UNLIMITED
        };
        let (source, packaged) = package_routes(&package, &emission, "f", &twos(), limits);
        let packaged = packaged.expect("the package call runs");
        assert!(
            source.outcome.contains("WorkUnits"),
            "{limit}: {}",
            source.outcome
        );
        assert_eq!(source, packaged, "work_units limit {limit}");
        located.insert(source.location);
    }
    assert!(
        located.iter().any(|location| location.contains("\"f\"")
            && location.contains("depth: 1, last_index: Some(1)")),
        "{located:?}"
    );
    assert!(
        located
            .iter()
            .all(|location| !location.contains("depth: 2")),
        "{located:?}"
    );
}

/// The source object unit of QSL-2701 item 1: `Order::measure` carries a
/// precondition holding two `sqrt` terms, so its clause function has body
/// location rows on the package route.
const SOURCE_OBJECT_UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.model.complete/v1\";\n\
    object Order { total: Int[0, 9]; operation measure(n: Integer): Integer; }\n\
    pre sqrtPre using v on Order::measure { \
    filament::value::ieee::sqrt(float64(bits: 0x4000000000000000)) <= \
    filament::value::ieee::sqrt(float64(bits: 0x4000000000000000)) }\n\
    body B using v on Order::measure(n) { n }\n\
    invariant Sized using v on Order at current { self.measure(0) >= 0 }\n";

/// QSL-2701 item 1 (FSpec FR-322): a source model's clause function is named
/// under its owning source's identity on the source route (the checked graph's
/// function name) and on the package route (the name its body location rows
/// carry), and the two names are equal.
#[trace("FSpec-TC-196")]
#[test]
fn a_source_models_clause_function_is_named_alike_on_both_routes() {
    let packages = package_input(std::iter::empty::<&[u8]>()).expect("source-only input");
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        SOURCE_OBJECT_UNIT.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = fsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("intake admits the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        models,
        Vec::new(),
    )
    .expect("the assembler admits the unit");
    declarations.ieee_profile =
        Some(admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).unwrap());
    declarations
        .link_dispatch(Default::default())
        .expect("dispatch links");
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .unwrap_or_else(|refusal| panic!("{refusal:?}"));
    let mut source_names = Vec::new();
    let mut index = 0;
    while let Some(state) = graph.function_state(index) {
        if state.name.ends_with("/Order/measure.precondition") {
            source_names.push(state.name.to_owned());
        }
        index += 1;
    }
    let package = CheckedPackage::link(graph);
    let emission = fsl_package::emit_checked(&package, fsl_package::EmitLimits::fixture())
        .expect("the checked package emits");
    let wire: serde_json::Value =
        serde_json::from_slice(emission.package().bytes()).expect("the package is JSON");
    let mut package_names: Vec<String> = wire["checked_term_data"]["body_locations"]
        .as_array()
        .expect("the body location rows")
        .iter()
        .filter_map(|row| row["location"]["origin"]["name"].as_str())
        .filter(|name| name.ends_with("/Order/measure.precondition"))
        .map(str::to_owned)
        .collect();
    package_names.dedup();
    let expected = "ix://u/Order/measure.precondition";
    assert_eq!(source_names, [expected]);
    assert_eq!(package_names, source_names);
}

/// FR-531-AC-40: the source-owned operation clause is selected by its
/// operation anchor on both routes, and both evaluators retain the source
/// identity and child paths across the full work-unit sweep. A label of
/// another operation's precondition selects nothing on either route.
#[trace("TC-3133", "FR-531-AC-40")]
#[test]
fn source_operation_precondition_is_selected_by_anchor_and_located_on_both_routes() {
    let text = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.model.complete/v1\";\n\
        object ConfigVersion { versionNumber: Integer; operation probe(): Integer; operation size(): Integer; }\n\
        object Sub extends ConfigVersion { operation size(): Integer redefines ConfigVersion::size; }\n\
        object Leaf extends Sub { operation size(): Integer redefines Sub::size; }\n\
        body sizeA using v on ConfigVersion::size() { 1 }\n\
        body sizeB using v on Sub::size() { 2 }\n\
        body sizeC using v on Leaf::size() { 3 }\n\
        pre rootCall using v on ConfigVersion::probe { self.size() > 0 }\n\
        pre sqrtPre using v on ConfigVersion::size { filament::value::ieee::sqrt(convert<Float64[nearest-even]>(float64(bits: 0x4000000000000000))) <= filament::value::ieee::sqrt(convert<Float64[nearest-even]>(float64(bits: 0x4000000000000000))) }\n";
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the source object unit");
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the source object unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("S3 admits the source object operations and clauses");
    declarations
        .link_dispatch(Default::default())
        .expect("rootCall reaches the source size operation");
    declarations.ieee_profile = Some(
        admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[])
            .expect("the IEEE value profile is admitted"),
    );
    let package = CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .unwrap_or_else(|refusal| panic!("{refusal:?}")),
    );
    let emission = fsl_package::emit_checked(&package, fsl_package::EmitLimits::fixture())
        .expect("S4 emits source-owner clause identities and body locations");
    let bytes = emission.package().bytes();
    assert!(
        !bytes
            .windows(b"ix://source/".len())
            .any(|window| window == b"ix://source/"),
        "the emitted package has no checker fallback identity"
    );

    let source_clause = package
        .graph()
        .state_clause("sqrtPre")
        .expect("the authored operation precondition is checked");
    let reference = ObjectReference::new(
        UniverseId::new(source_clause.identity()),
        source_clause.context(),
        ObjectId::new("size-instance").expect("non-empty object identity"),
    );
    let source_operation = source_clause
        .operation()
        .expect("a precondition names its operation");
    let source_anchor = package
        .graph()
        .operation_anchor_of(
            source_operation.declaring.clone(),
            source_operation.declaration.name(),
        )
        .expect("the checked size operation has an anchor");
    let source_objects = ObjectEnvironment::new(
        ObjectClosure::new(
            package.graph().scope().types(),
            [(
                reference.clone(),
                vec![(
                    "versionNumber",
                    FieldValue::Present(Value::Integer(Integer::from(1_i64))),
                )],
            )],
            &[],
        )
        .expect("the source-owned receiver admits in the checked package"),
    );
    let evaluate_source_clause = |name: &str, meter: &mut Meter| {
        fsl_eval::value::evaluate_source_operation_clause(
            &package,
            &source_anchor,
            &fsl_package::SourceOperationClause::Precondition {
                name: name.to_owned(),
            },
            vec![Value::Reference(reference.clone())],
            &source_objects,
            meter,
        )
    };
    let evaluate_source = |limits| {
        let mut meter = Meter::new(limits);
        let evaluation = evaluate_source_clause("sqrtPre", &mut meter)
            .expect("the checked source precondition evaluates");
        (evaluation, meter)
    };
    let (source, source_meter) = evaluate_source(UNLIMITED);

    let empty_models = package_input(std::iter::empty::<&[u8]>()).expect("no model imports");
    let read = fsl_package::read_evaluation_emission(
        &package,
        &emission,
        LibraryName::new("fixture").expect("a library name"),
        &empty_models,
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
        fsl_package::SourceWitnessSupplier::new(&emission),
    )
    .expect("the bytes-only reader admits the emitted package");
    let admitted = read.admitted();
    let mut supply_budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let identities = emission
        .identity_rows(admitted, &mut supply_budget)
        .expect("the source identity rows materialize");
    let bare = fsl_package::BarePackage::new(admitted)
        .with_supply(
            fsl_package::BareSupply {
                identities: &identities,
                equality: None,
            },
            &mut supply_budget,
        )
        .expect("the package joins the selected source operation");
    let operation = identities
        .source_operations()
        .iter()
        .find(|row| row.source_identity.as_ref() == "ConfigVersion::size")
        .expect("the admitted size operation identity row");
    let anchor = operation.anchor_id.clone();
    let selected_operations = identities
        .selected_source_operations(admitted, 1_000_000)
        .expect("the source-operation rows join to this admitted package");
    let selected_callable = admitted
        .source_operation_callable(&selected_operations, &anchor, 1_000_000)
        .expect("the size operation anchor resolves from its selected identity");
    assert!(
        selected_callable.callable().is_some(),
        "the admitted operation identity is exact"
    );
    let context = selected_callable
        .callable()
        .and_then(|callable| callable.context.as_ref())
        .expect("the selected source operation retains its type context");
    let context_owner = admitted
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| node.node_id == *context)
        .and_then(|node| node.owner.as_ref())
        .expect("the operation context has its source owner");
    let clause_candidates = admitted
        .resolved_graph()
        .nodes
        .iter()
        .filter_map(|node| {
            let declaration = node.declaration.as_ref()?;
            let names = &declaration.qualified_name;
            (names
                .first()
                .is_some_and(|name| name.as_ref() == "ConfigVersion")
                && names.get(1).is_some_and(|name| name.as_ref() == "size"))
            .then_some((node.owner.as_ref(), names))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        clause_candidates
            .iter()
            .filter(|(owner, names)| {
                *owner == Some(context_owner)
                    && names
                        .get(2)
                        .is_some_and(|name| name.as_ref() == "precondition")
            })
            .count(),
        1,
        "the exact owner/name clause join is visible: {clause_candidates:?}"
    );
    let bare_world = bare
        .world_with_source_objects()
        .expect("the source-object types project under their declaring-unit identities");
    let objects = ObjectEnvironment::new(
        ObjectClosure::new(bare_world.types(), [(reference.clone(), Vec::new())], &[])
            .expect("the source-owned receiver admits in the bare world"),
    );
    let evaluate_bytes = |limits| {
        let mut meter = Meter::new(limits);
        let mut native_budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
        let evaluation = fsl_eval::value::evaluate_bare_source_operation_clause(
            &bare,
            &anchor,
            fsl_package::SourceOperationClause::Precondition {
                name: "sqrtPre".to_owned(),
            },
            vec![Value::Reference(reference.clone())],
            &objects,
            &mut meter,
            &mut native_budget,
        )
        .expect("the selected bytes-only operation precondition evaluates");
        (evaluation, meter)
    };
    let mut projection_budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let debug_projection = bare
        .project_source_operation_clause(
            &anchor,
            fsl_package::SourceOperationClause::Precondition {
                name: "sqrtPre".to_owned(),
            },
            &mut projection_budget,
        )
        .expect("the source operation clause is selected by its admitted anchor");
    assert_eq!(
        debug_projection
            .selected_function()
            .and_then(|function| function.parameters.first())
            .map(|parameter| &parameter.value_type),
        Some(&ValueType::Reference(source_clause.context().clone())),
        "the bytes route preserves the source-owned receiver type"
    );
    let (packaged, package_meter) = evaluate_bytes(UNLIMITED);
    assert!(matches!(
        &source.outcome,
        FamilyOutcome::Evaluated(filament_exact::Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(matches!(
        &packaged.outcome,
        FamilyOutcome::Evaluated(filament_exact::Outcome::Completed(Value::Boolean(true)))
    ));

    let mut source_refusal_meter = Meter::new(UNLIMITED);
    let source_refusal = evaluate_source_clause("rootCall", &mut source_refusal_meter)
        .expect_err("rootCall is a precondition of probe, not of size");
    let mut package_refusal_meter = Meter::new(UNLIMITED);
    let mut refusal_budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let Err(fsl_eval::value::BareSourceOperationEvaluationFailure::Call(package_refusal)) =
        fsl_eval::value::evaluate_bare_source_operation_clause(
            &bare,
            &anchor,
            fsl_package::SourceOperationClause::Precondition {
                name: "rootCall".to_owned(),
            },
            vec![Value::Reference(reference.clone())],
            &objects,
            &mut package_refusal_meter,
            &mut refusal_budget,
        )
    else {
        panic!("the bytes route refuses another operation's precondition label");
    };
    assert_eq!(source_refusal, package_refusal);
    assert_eq!(source_refusal_meter.consumed(LimitKind::WorkUnits), 0);
    assert_eq!(package_refusal_meter.consumed(LimitKind::WorkUnits), 0);

    let wire: serde_json::Value = serde_json::from_slice(bytes).expect("canonical JSON");
    let rows = wire["checked_term_data"]["body_locations"]
        .as_array()
        .expect("body location rows");
    let clause_rows = rows
        .iter()
        .filter(|row| {
            row["location"]["origin"]["name"]
                .as_str()
                .is_some_and(|name| {
                    name.starts_with("ix://u/")
                        && name.ends_with("/ConfigVersion/size.precondition")
                })
        })
        .collect::<Vec<_>>();
    assert!(
        !clause_rows.is_empty(),
        "the clause body has retained locations"
    );
    let function_name = clause_rows[0]["location"]["origin"]["name"]
        .as_str()
        .expect("source-owned body function name");
    let function_index = clause_rows[0]["location"]["origin"]["index"]
        .as_u64()
        .expect("source-owned body function index") as usize;
    assert!(function_name.starts_with("ix://u/"), "{function_name}");
    for row in clause_rows {
        assert_eq!(row["location"]["origin"]["name"], function_name);
        assert_eq!(row["location"]["origin"]["index"], function_index);
    }
    for evaluation in [&source, &packaged] {
        assert_eq!(evaluation.losses.len(), 2);
        for (loss, path) in evaluation.losses.iter().zip([[0], [1]]) {
            assert!(matches!(
                loss.loss,
                ValueLoss::IeeeFlags(flags)
                    if flags.contains(IeeeFlag::Inexact)
                        && flags.iter().collect::<Vec<_>>() == [IeeeFlag::Inexact]
            ));
            assert_eq!(
                loss.location.origin,
                LocationOwner::Body {
                    function: function_name.to_owned(),
                    index: function_index,
                }
            );
            assert_eq!(loss.location.path(), path);
        }
    }
    assert_eq!(
        source_meter.consumed(LimitKind::WorkUnits),
        package_meter.consumed(LimitKind::WorkUnits)
    );
    for limit in 0..source_meter.consumed(LimitKind::WorkUnits) {
        let limits = ScalarLimits {
            work_units: limit,
            ..UNLIMITED
        };
        let (source, source_meter) = evaluate_source(limits);
        let (packaged, package_meter) = evaluate_bytes(limits);
        let FamilyOutcome::Evaluated(filament_exact::Outcome::Incomplete(source_stop)) =
            &source.outcome
        else {
            panic!(
                "source evaluator stops on the work limit: {:?}",
                source.outcome
            );
        };
        let FamilyOutcome::Evaluated(filament_exact::Outcome::Incomplete(package_stop)) =
            &packaged.outcome
        else {
            panic!(
                "bytes evaluator stops on the work limit: {:?}",
                packaged.outcome
            );
        };
        assert_eq!(source_stop, package_stop, "work limit {limit}");
        assert_eq!(source.location, packaged.location, "work limit {limit}");
        assert_eq!(
            source_meter.consumed(LimitKind::WorkUnits),
            package_meter.consumed(LimitKind::WorkUnits),
            "work limit {limit}"
        );
    }
}

/// TC-1331: public native joins have caller-owned finite budgets, the
/// bytes-only operation selector retains typed work-limit failure at its
/// package locus, and BarePackage has no copying constructor trait.
#[trace("TC-1331", "FR-531-AC-25")]
#[test]
fn bare_operation_selection_uses_a_finite_caller_budget_and_keeps_its_typed_limit() {
    let text = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.model.complete/v1\";\n\
        object ConfigVersion { versionNumber: Integer; operation probe(): Integer; operation size(): Integer; }\n\
        body sizeA using v on ConfigVersion::size() { 1 }\n\
        pre rootCall using v on ConfigVersion::probe { self.size() > 0 }\n\
        pre sqrtPre using v on ConfigVersion::size { true }\n";
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the source-operation unit");
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the source-operation unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("S3 admits the source operation");
    declarations
        .link_dispatch(Default::default())
        .expect("the source operation dispatch links");
    declarations.ieee_profile = Some(
        admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[])
            .expect("the value profile is admitted"),
    );
    let package = CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .expect("the source operation checks"),
    );
    let emission = fsl_package::emit_checked(&package, fsl_package::EmitLimits::fixture())
        .expect("the checked source operation emits");
    let no_models = package_input(std::iter::empty::<&[u8]>()).expect("no model imports");
    let read = fsl_package::read_evaluation_emission(
        &package,
        &emission,
        LibraryName::new("fixture").expect("library name"),
        &no_models,
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
        fsl_package::SourceWitnessSupplier::new(&emission),
    )
    .expect("the checked bytes are admitted");
    let admitted = read.admitted();
    let mut supply_budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let identities = emission
        .identity_rows(admitted, &mut supply_budget)
        .expect("the source identity rows materialize");
    let bare = fsl_package::BarePackage::new(admitted)
        .with_supply(
            fsl_package::BareSupply {
                identities: &identities,
                equality: None,
            },
            &mut supply_budget,
        )
        .expect("the finite supply budget admits its rows");
    let anchor = identities
        .source_operations()
        .iter()
        .find(|row| row.source_identity.as_ref() == "ConfigVersion::size")
        .expect("the exact source-operation row")
        .anchor_id
        .clone();
    let mut no_native_work = fsl_package::NativeJoinBudget::new(0, 0);
    let refusal = match bare.project_source_operation_clause(
        &anchor,
        fsl_package::SourceOperationClause::Precondition {
            name: "sqrtPre".to_owned(),
        },
        &mut no_native_work,
    ) {
        Err(refusal) => refusal,
        Ok(_) => panic!("zero native work refuses before source-operation selection"),
    };
    let fsl_package::BareSourceOperationFailure::NativeJoin(failure) = refusal else {
        panic!("zero native work is a typed native-join refusal: {refusal:?}");
    };
    assert_eq!(
        failure.setting(),
        Some(fsl_foundation::Setting::I2WorkUnits)
    );
    assert!(matches!(
        failure.native(),
        fsl_package::NativeJoinFailure::Rows(error)
            if error.code() == fsl_package::NativeRowErrorCode::WorkLimit
                && error.locus().is_some_and(|locus| locus.form == "source-operation-clause")
    ));
    assert_eq!(
        failure.limit().map(|limit| limit.setting()),
        Some(fsl_foundation::Setting::I2WorkUnits)
    );
    assert_eq!(
        failure.limit().map(|limit| limit.configured_bound()),
        Some(0)
    );

    let intake = include_str!("../../../fsl-semantics/src/model/intake.rs");
    let selected_models = include_str!("../../../fsl-semantics/src/model/intake/unit.rs");
    let checked_graph = include_str!("../../../fsl-semantics/src/check/mod.rs");
    let package_source = include_str!("../../../fsl-package/src/bare.rs");
    let evidence = include_str!("../../../fsl-eval/src/temporal/evidence/package.rs");
    let evaluator = include_str!("../../../fsl-eval/src/value/expression/bare.rs");
    for signature in [
        signature_after(intake, "pub fn selected_exact_identities_from_rows("),
        signature_after(selected_models, "pub fn exact_identities("),
        signature_after(checked_graph, "pub fn selected_exact_identities("),
        signature_after(package_source, "pub fn selected("),
        signature_after(package_source, "pub fn with_supply("),
        signature_after(evidence, "pub fn from_emission("),
        signature_after(evidence, "pub fn from_read("),
        signature_after(evidence, "pub fn from_package("),
    ] {
        assert!(signature.contains("budget: &mut"), "{signature}");
        assert!(signature.contains("NativeJoinBudget"), "{signature}");
    }
    let selector = include_str!("../../../fsl-package/src/bare_slot.rs");
    let selector = signature_after(selector, "pub fn project_source_operation_clause(");
    assert!(selector.contains("budget: &mut"));
    assert!(selector.contains("NativeJoinBudget"));
    assert!(!selector.contains("u64::MAX"));
    assert!(!selector.contains("NativeJoinBudget::new"));
    let evaluator_entry = evaluator
        .split_once("pub fn evaluate_bare_source_operation_clause(")
        .expect("the public bytes-only source-operation entry")
        .1
        .split_once("/// A selection or projection refusal")
        .expect("the entry body ends before the ordinary refusal mapper")
        .0;
    assert!(!evaluator_entry.contains("u64::MAX"));
    assert!(!evaluator_entry.contains("NativeJoinBudget::new"));
    assert!(!package_source.contains("u64::MAX"));
    assert!(!package_source.contains("NativeJoinBudget::new"));
    assert!(package_source.contains("#[derive(Debug)]\npub struct BarePackage"));
    assert!(!package_source.contains("impl Clone for BarePackage"));
    assert!(!package_source.contains("impl Copy for BarePackage"));
}
