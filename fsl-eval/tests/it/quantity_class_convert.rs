// SPDX-License-Identifier: AGPL-3.0-or-later
//! `convert` across a dimension equivalence class, from source text to its
//! evaluated value (FR-093 "Dimension equivalence", FR-093-AC-46; FSpec
//! FR-142-AC-26).

use filament_eval::{Evaluation, FamilyOutcome, FamilyResult};
use filament_exact::{
    ChargePoint, Integer, LimitKind, Meter, Outcome, Quantity as ExactQuantity, Rational, Value,
    ValueType,
};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::{BodyTerm, LeafTerm, MemberTerm, OperationMode, Operator};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::value::Member;
use ix_trace_rs::trace;

use super::quantity_literals::{package_with, UNLIMITED};

/// FR-091-AC-44's `Time` units, `Beat` units and the equivalence
/// `equivalence` declares between them.
fn units(equivalence: &str) -> String {
    format!(
        "dimension Time;\n\
         unit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         dimension Beat;\n\
         unit beat : Beat = rational(1, 1);\n\
         unit bar : Beat = rational(4, 1) * beat;\n\
         {equivalence}\n"
    )
}

const QUARTER: &str = "dimension Beat ~ Time by 1 beat = rational(1, 4) s;";
const EQUAL: &str = "dimension Beat ~ Time by 1 beat = 1 s;";

fn fraction(numerator: i64, denominator: i64) -> Rational {
    Rational::from_integer(Integer::from(numerator))
        .div(&Rational::from_integer(Integer::from(denominator)))
        .expect("a nonzero denominator")
}

/// The Boolean function `name` over its one quantity parameter =
/// `magnitude` of that parameter's unit.
fn truth_at(package: &CheckedPackage, name: &str, magnitude: Rational) -> bool {
    let callable = package.graph().callable(name).expect("a callable");
    let ValueType::Quantity(unit) = &callable.parameters[0].1 else {
        panic!("{name} takes a quantity");
    };
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            vec![Value::Quantity(ExactQuantity::new(magnitude, unit.clone()))],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .unwrap_or_else(|error| panic!("the {name} call runs: {error:?}"));
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(truth))) => truth,
        other => panic!("a completed Boolean, not {other:?}"),
    }
}

const FUNCTIONS: &str = "\
    function u1 using v(d: Quantity<beat>): Boolean pure { convert<Quantity<s>>(d) = rational(1, 4) s }\n\
    function u2 using v(d: Quantity<s>): Boolean pure { convert<Quantity<beat>>(d) = 12 beat }\n\
    function u3 using v(d: Quantity<bar>): Boolean pure { convert<Quantity<ms>>(d) = 1000 ms }\n";

/// FR-093-AC-46 (root ratio 1/4, then 1): a conversion across the class
/// evaluates to the exact value through the class root.
#[trace("FR-093-AC-46", "FSpec-FR-142-AC-26")]
#[test]
fn a_conversion_across_an_equivalence_class_is_exact() {
    let quarter = package_with(&format!("{}{FUNCTIONS}", units(QUARTER))).expect("checks");
    let graph = quarter.graph().semantic_graph();
    let conversions: Vec<_> = graph
        .nodes()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(application)
                if application.operation.identity() == "filament.op.quantity.convert" =>
            {
                Some(application)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        conversions.len(),
        3,
        "u1, u2 and u3 each lower a conversion"
    );
    let mut targets = Vec::new();
    for conversion in conversions {
        assert_eq!(conversion.operator, Operator::Convert);
        assert_eq!(
            conversion.operation.mode(),
            Some(&OperationMode::Rounding(
                filament_exact::RoundingMode::Exact
            ))
        );
        let Some(Member::TypeArgument { declaration, .. }) = conversion.operation.member() else {
            panic!("the conversion names its target unit");
        };
        assert_eq!(conversion.result_type.admitted(), Some(declaration));
        let [MemberTerm::Leaf(LeafTerm::Reference { target })] = conversion.arguments.as_slice()
        else {
            panic!("the conversion takes one referenced quantity operand");
        };
        let operand = graph
            .node(target.admitted().expect("an admitted operand").clone())
            .expect("the operand node");
        assert_ne!(operand.semantic_type().as_ref(), Some(declaration));
        let target = graph
            .node(declaration.clone())
            .expect("the target unit node");
        assert_eq!(target.semantic_form(), "unit");
        targets.push(
            target
                .declaration()
                .expect("a declared target unit")
                .iter()
                .map(|segment| segment.as_str())
                .collect::<Vec<_>>()
                .join("::"),
        );
    }
    targets.sort();
    assert_eq!(targets, ["beat", "ms", "s"]);
    assert!(truth_at(&quarter, "u1", fraction(1, 1)));
    assert!(!truth_at(&quarter, "u1", fraction(2, 1)));
    assert!(truth_at(&quarter, "u2", fraction(3, 1)));
    assert!(!truth_at(&quarter, "u2", fraction(4, 1)));
    assert!(truth_at(&quarter, "u3", fraction(1, 1)));
    assert!(!truth_at(&quarter, "u3", fraction(2, 1)));

    let equal = package_with(&format!("{}{FUNCTIONS}", units(EQUAL))).expect("checks");
    assert!(!truth_at(&equal, "u1", fraction(1, 1)));
    assert!(truth_at(&equal, "u1", fraction(1, 4)));
    assert!(truth_at(&equal, "u2", fraction(12, 1)));
}

/// FR-093-AC-46 u5 and FR-093-AC-44: addition, subtraction, equality and
/// ordering of distinct units of one class still refuse
/// `ill_typed`/`type-mismatch` under either root ratio.
#[trace("FR-093-AC-46")]
#[test]
fn arithmetic_and_comparison_across_a_class_require_conversion() {
    use filament_exact::IllTypedCause;
    use fsl_semantics::check::CheckCause;
    for equivalence in [QUARTER, EQUAL] {
        for expression in ["d + p = d + p", "d - p = d - p", "d = p", "d < p"] {
            let refusals = package_with(&format!(
                "{}function u5 using v(d: Quantity<s>, p: Quantity<beat>): Boolean pure \
             {{ {expression} }}\n",
                units(equivalence)
            ))
            .expect_err("the distinct-unit operation refuses");
            assert!(
                matches!(
                    refusals.as_slice(),
                    [refusal] if refusal.cause == CheckCause::IllTyped(IllTypedCause::TypeMismatch)
                ),
                "{expression} under {equivalence}: {refusals:?}"
            );
        }
    }
}

/// FR-093-AC-46 u4: a quotient takes each operand in the class root, so
/// both quotients of `6 s` are the same dimensionless value.
#[trace("FR-093-AC-46")]
#[test]
fn a_quotient_across_an_equivalence_class_takes_the_class_root() {
    let package = package_with(&format!(
        "{}function u4 using v(d: Quantity<s>): Boolean pure \
         {{ d / 3 beat = d / rational(3, 4) s }}\n",
        units(QUARTER)
    ))
    .expect("checks");
    let graph = package.graph().semantic_graph();
    let quotients: Vec<_> = graph
        .nodes()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(application)
                if application.operation.identity() == "filament.op.quantity.div" =>
            {
                Some(application)
            }
            _ => None,
        })
        .collect();
    let [left, right] = quotients.as_slice() else {
        panic!("u4 lowers two quantity divisions");
    };
    assert_eq!(left.result_type, right.result_type);
    let result = graph
        .node(left.result_type.admitted().expect("a result type").clone())
        .expect("the compound unit");
    assert_eq!(result.semantic_form(), "compound_unit");
    let BodyTerm::Aggregate(terms) = result.body() else {
        panic!("the compound unit carries its terms");
    };
    assert!(terms.members.is_empty(), "the quotient is dimensionless");
    assert!(truth_at(&package, "u4", fraction(6, 1)));
}

fn call(package: &impl CheckedPackageEvaluation, name: &str, meter: &mut Meter) -> Evaluation {
    package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            vec![],
            &ObjectEnvironment::default(),
            meter,
        )
        .unwrap_or_else(|error| panic!("{name} runs: {error:?}"))
}

fn quantity(evaluation: &Evaluation) -> &ExactQuantity {
    match &evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Quantity(quantity))) => quantity,
        other => panic!("a completed quantity, got {other:?}"),
    }
}

/// FR-093-AC-46: class conversion traverses the shared unit graph under
/// the evaluation's meter, including the class identification edge.
#[trace("FR-093-AC-46")]
#[test]
fn a_class_conversion_obeys_the_callers_meter() {
    let program = format!(
        "{}function cross using v(): Quantity<s> pure {{ convert<Quantity<s>>(1 beat) }}\n",
        units(QUARTER)
    );
    let package = package_with(&program).expect("the conversion checks");
    let mut measured = Meter::new(UNLIMITED);
    let evaluation = call(&package, "cross", &mut measured);
    assert_eq!(quantity(&evaluation).magnitude(), &fraction(1, 4));
    let points: Vec<_> = measured
        .admitted_charges()
        .iter()
        .map(|charge| charge.point)
        .collect();
    assert_eq!(
        points
            .iter()
            .filter(|point| **point == ChargePoint::UnitEdge)
            .count(),
        1,
        "one identification edge, even when the representative is Beat"
    );
    assert!(points.contains(&ChargePoint::UnitRationalArithmetic));
    assert!(points.contains(&ChargePoint::UnitTargetDomain));
    assert!(points.contains(&ChargePoint::UnitResultRetain));

    for counter in [
        LimitKind::WorkUnits,
        LimitKind::UnitEdges,
        LimitKind::IntegerBits,
        LimitKind::ValueOccurrences,
        LimitKind::ResultUnits,
    ] {
        let required = measured.consumed(counter);
        assert!(required > 0, "the conversion measures {counter:?}");
        let bound = required - 1;
        let mut limits = UNLIMITED;
        match counter {
            LimitKind::WorkUnits => limits.work_units = bound,
            LimitKind::UnitEdges => limits.unit_edges = bound,
            LimitKind::IntegerBits => limits.integer_bits = bound,
            LimitKind::ValueOccurrences => limits.value_occurrences = bound,
            LimitKind::ResultUnits => limits.result_units = bound,
            _ => unreachable!("only the five selected counters"),
        }
        let mut meter = Meter::new(limits);
        let evaluation = call(&package, "cross", &mut meter);
        let FamilyOutcome::Evaluated(Outcome::Incomplete(record)) = &evaluation.outcome else {
            panic!("{counter:?}={bound} returns no converted value: {evaluation:?}");
        };
        assert_eq!(record.limit_kind, counter);
        assert_eq!(record.limit, bound);
        assert_eq!(record.consumed, meter.consumed(counter));
        if counter == LimitKind::UnitEdges {
            assert_eq!(record.charge_point, ChargePoint::UnitEdge);
            assert_eq!(record.consumed, 0, "the denied edge is not charged");
            let location = evaluation
                .location
                .as_ref()
                .expect("the conversion is located");
            assert_eq!(location.depth(), 0, "the function's conversion root");
            let region = package.graph().region(location).expect("the source span");
            let expression = "convert<Quantity<s>>(1 beat)";
            assert_eq!(
                region.end() - region.start(),
                u64::try_from(expression.len()).expect("a source length")
            );
        }
    }
}

fn with_bare<T>(
    package: &CheckedPackage,
    run: impl FnOnce(&fsl_package::BarePackage<'_>) -> T,
) -> T {
    use fsl_semantics::library::LibraryName;
    use fsl_semantics::model::intake::package_input;

    let emission = fsl_package::emit_checked(package, fsl_package::EmitLimits::fixture())
        .expect("the checked unit emits");
    let no_models = package_input(std::iter::empty::<&[u8]>()).expect("no model inputs");
    let read = fsl_package::read_evaluation_emission(
        package,
        &emission,
        LibraryName::new("fixture").expect("a package name"),
        &no_models,
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
        fsl_package::SourceWitnessSupplier::new(&emission),
    )
    .expect("the bytes are admitted");
    let budget = || fsl_package::V2ReadLimits::fixture().native_join_budget();
    let identities = emission
        .identity_rows(read.admitted(), &mut budget())
        .expect("identity rows");
    let equality = emission
        .equality_witness(read.admitted(), &mut budget())
        .expect("equality rows");
    let bare = fsl_package::BarePackage::new(read.admitted())
        .with_supply(
            fsl_package::BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget(),
        )
        .expect("the package supply joins");
    run(&bare)
}

/// FR-533-AC-23: until the IR reader exposes equivalence definitions, the
/// bytes-only route reports the bounded unsupported construct at the
/// conversion, while ordinary and affine conversions still evaluate.
#[trace("FR-533-AC-23")]
#[test]
fn a_package_class_conversion_is_typed_unsupported_until_reconstruction() {
    use filament_semantic_value::catalog::CatalogCode;

    let program = format!(
        "{}\
         unit shifted : Beat = rational(1, 1) * beat + rational(2, 1);\n\
         function cross using v(): Quantity<s> pure {{ convert<Quantity<s>>(1 beat) }}\n\
         function affine_class using v(): Quantity<s> pure {{ convert<Quantity<s>>(1 shifted) }}\n\
         function quotient using v(): Boolean pure {{ 6 s / 3 beat = 8 s / 1 s }}\n\
         function same using v(): Quantity<s> pure {{ convert<Quantity<s>>(1000 ms) }}\n\
         function affine_same using v(): Quantity<beat> pure {{ convert<Quantity<beat>>(1 shifted) }}\n",
        units(QUARTER)
    );
    let package = package_with(&program).expect("the unit checks");
    assert_eq!(
        quantity(&call(&package, "affine_class", &mut Meter::new(UNLIMITED))).magnitude(),
        &fraction(3, 4)
    );
    with_bare(&package, |bare| {
        for (name, expression) in [
            ("cross", "convert<Quantity<s>>(1 beat)"),
            ("affine_class", "convert<Quantity<s>>(1 shifted)"),
            ("quotient", "6 s / 3 beat"),
        ] {
            let mut meter = Meter::new(UNLIMITED);
            let evaluation = call(bare, name, &mut meter);
            let FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) = &evaluation.outcome
            else {
                panic!("a typed unsupported conversion, got {evaluation:?}");
            };
            assert_eq!(
                cause.catalog_code(),
                CatalogCode::new("unsupported_construct", "not-yet-implemented")
            );
            let fields = cause
                .catalog_fields()
                .expect("the unsupported construct payload");
            assert_eq!(
                fields.get("what").map(String::as_str),
                Some("dimension-equivalence-package-reconstruction")
            );
            let location = evaluation
                .location
                .as_ref()
                .expect("the conversion's location");
            let depth = if name == "quotient" { 1 } else { 0 };
            assert_eq!(location.depth(), depth);
            let region = package
                .graph()
                .region(location)
                .expect("the source conversion span");
            let prefix = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";
            let full = format!("{prefix}{program}");
            let start = usize::try_from(region.start()).expect("the source start fits");
            let end = usize::try_from(region.end()).expect("the source end fits");
            assert_eq!(&full[start..end], expression);
            assert!(!meter
                .admitted_charges()
                .iter()
                .any(|charge| charge.point == ChargePoint::UnitRationalArithmetic));
        }
        assert_eq!(
            quantity(&call(bare, "same", &mut Meter::new(UNLIMITED))).magnitude(),
            &fraction(1, 1)
        );
        assert_eq!(
            quantity(&call(bare, "affine_same", &mut Meter::new(UNLIMITED))).magnitude(),
            &fraction(3, 1)
        );
    });
}
