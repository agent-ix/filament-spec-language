// SPDX-License-Identifier: AGPL-3.0-or-later
//! `convert` across a dimension equivalence class, from source text to its
//! evaluated value (FR-093 "Dimension equivalence", FR-093-AC-46; FSpec
//! FR-142-AC-26).

use filament_eval::{Evaluation, FamilyOutcome};
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
                    [refusal] if refusal.cause.as_ref() == &CheckCause::IllTyped(IllTypedCause::TypeMismatch)
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
        LibraryName::new(emission.package().name()).expect("the emitted package name"),
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

fn boundary_program() -> String {
    format!(
        "{}\
         unit shifted : Beat = rational(1, 1) * beat + rational(2, 1);\n\
         dimension TimeSquared = Time ^ 2;\n\
         unit s2 : TimeSquared = rational(1, 1);\n\
         dimension Dimensionless = Time / Time;\n\
         unit one : Dimensionless = rational(1, 1);\n\
         function product_value using v(): Quantity<s2> pure {{ convert<Quantity<s2>>(6 s * 3 beat) }}\n\
         function quotient_value using v(): Quantity<one> pure {{ convert<Quantity<one>>(6 s / 3 beat) }}\n\
         function power_s_value using v(): Quantity<s2> pure {{ convert<Quantity<s2>>(2 s ^ 2) }}\n\
         function power_beat_value using v(): Quantity<s2> pure {{ convert<Quantity<s2>>(2 beat ^ 2) }}\n\
         function cross using v(): Quantity<s> pure {{ convert<Quantity<s>>(1 beat) }}\n\
         function affine_class using v(): Quantity<s> pure {{ convert<Quantity<s>>(1 shifted) }}\n\
         function quotient using v(): Boolean pure {{ 6 s / 3 beat = 8 s / 1 s }}\n\
         function product using v(): Boolean pure {{ 6 s * 3 beat = rational(9, 2) s * 1 s }}\n\
         function power_s using v(): Boolean pure {{ 2 s ^ 2 = 4 s * 1 s }}\n\
         function power_beat using v(): Boolean pure {{ 2 beat ^ 2 = rational(1, 4) s * 1 s }}\n\
         dimension Length;\n\
         unit m : Length = rational(1, 1);\n\
         function ordinary_power using v(): Boolean pure {{ 2 m ^ 2 = 4 m * 1 m }}\n\
         function same using v(): Quantity<s> pure {{ convert<Quantity<s>>(1000 ms) }}\n\
         function affine_same using v(): Quantity<beat> pure {{ convert<Quantity<beat>>(1 shifted) }}\n",
        units(QUARTER)
    )
}

/// FR-533-AC-23: the admitted package reconstructs the same class graph,
/// quantities and result types as the source, including affine conversion.
#[trace("FR-533-AC-23", "TC-1333", "FR-093-AC-46", "FSpec-FR-142-AC-26")]
#[test]
fn package_class_conversions_and_compound_operations_match_source() {
    let package = package_with(&boundary_program()).expect("the unit checks");
    with_bare(&package, |bare| {
        for (name, expected) in [
            ("cross", fraction(1, 4)),
            ("affine_class", fraction(3, 4)),
            ("same", fraction(1, 1)),
            ("affine_same", fraction(3, 1)),
            ("product_value", fraction(9, 2)),
            ("quotient_value", fraction(8, 1)),
            ("power_s_value", fraction(4, 1)),
            ("power_beat_value", fraction(1, 4)),
        ] {
            let source = call(&package, name, &mut Meter::new(UNLIMITED));
            let bytes = call(bare, name, &mut Meter::new(UNLIMITED));
            assert_eq!(quantity(&source).magnitude(), &expected, "{name}");
            assert_eq!(
                quantity(&bytes),
                quantity(&source),
                "{name}: value and unit"
            );
            let expected_type = package.graph().callable(name).expect("the callable").result;
            assert_eq!(
                expected_type,
                &ValueType::Quantity(quantity(&bytes).unit().clone())
            );
        }
        for name in [
            "product",
            "quotient",
            "power_s",
            "power_beat",
            "ordinary_power",
        ] {
            for route in [&package as &dyn CheckedPackageEvaluation, bare] {
                let evaluation = route
                    .call(
                        &QualifiedName::unqualified(name).expect("an identifier"),
                        vec![],
                        &ObjectEnvironment::default(),
                        &mut Meter::new(UNLIMITED),
                    )
                    .expect("the function runs");
                assert!(
                    matches!(
                        evaluation.outcome,
                        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
                    ),
                    "{name}: {evaluation:?}"
                );
            }
        }
    });
}

fn limited(counter: LimitKind, bound: u64) -> filament_exact::ScalarLimits {
    let mut limits = UNLIMITED;
    match counter {
        LimitKind::WorkUnits => limits.work_units = bound,
        LimitKind::UnitEdges => limits.unit_edges = bound,
        LimitKind::IntegerBits => limits.integer_bits = bound,
        LimitKind::ValueOccurrences => limits.value_occurrences = bound,
        LimitKind::ResultUnits => limits.result_units = bound,
        _ => unreachable!("the selected quantity counters"),
    }
    limits
}

fn kernel_charges(meter: &Meter) -> Vec<&filament_exact::AdmittedCharge> {
    meter
        .admitted_charges()
        .iter()
        .filter(|charge| {
            matches!(
                charge.point,
                ChargePoint::UnitIdentityRead
                    | ChargePoint::UnitEdge
                    | ChargePoint::UnitRationalArithmetic
                    | ChargePoint::UnitTargetDomain
                    | ChargePoint::UnitResultRetain
            )
        })
        .collect()
}

/// FR-533-AC-23: both routes enter the same shared kernel, pay identical
/// charges and deny at the same exact boundary without a result quantity.
#[trace("FR-533-AC-23", "TC-1333", "FR-093-AC-46")]
#[test]
fn package_class_operations_share_kernel_charges_and_denial_boundaries() {
    let package = package_with(&boundary_program()).expect("the unit checks");
    with_bare(&package, |bare| {
        for name in [
            "cross",
            "affine_class",
            "product",
            "quotient",
            "power_s",
            "power_beat",
            "same",
            "affine_same",
            "ordinary_power",
            "product_value",
            "quotient_value",
            "power_s_value",
            "power_beat_value",
        ] {
            let mut source_meter = Meter::new(UNLIMITED);
            let mut bare_meter = Meter::new(UNLIMITED);
            call(&package, name, &mut source_meter);
            call(bare, name, &mut bare_meter);
            assert_eq!(
                kernel_charges(&bare_meter),
                kernel_charges(&source_meter),
                "{name}: exact kernel charges"
            );
            for counter in [
                LimitKind::WorkUnits,
                LimitKind::UnitEdges,
                LimitKind::IntegerBits,
                LimitKind::ValueOccurrences,
                LimitKind::ResultUnits,
            ] {
                let required = source_meter.consumed(counter);
                assert_eq!(
                    bare_meter.consumed(counter),
                    required,
                    "{name}: {counter:?}"
                );
                if required == 0 {
                    continue;
                }
                let mut source_tight = Meter::new(limited(counter, required - 1));
                let mut bare_tight = Meter::new(limited(counter, required - 1));
                let source = call(&package, name, &mut source_tight);
                let bytes = call(bare, name, &mut bare_tight);
                let FamilyOutcome::Evaluated(Outcome::Incomplete(expected)) = &source.outcome
                else {
                    panic!("{name}/{counter:?}: source denies: {source:?}");
                };
                let FamilyOutcome::Evaluated(Outcome::Incomplete(actual)) = &bytes.outcome else {
                    panic!("{name}/{counter:?}: bare denies without a quantity: {bytes:?}");
                };
                assert_eq!(actual, expected, "{name}/{counter:?}: exact denial");
                assert_eq!(
                    bytes.location, source.location,
                    "{name}/{counter:?}: retained location"
                );
                assert_eq!(
                    kernel_charges(&bare_tight),
                    kernel_charges(&source_tight),
                    "{name}/{counter:?}: denied charge is not paid"
                );
                for route in [&package as &dyn CheckedPackageEvaluation, bare] {
                    let evaluation = route
                        .call(
                            &QualifiedName::unqualified(name).expect("an identifier"),
                            vec![],
                            &ObjectEnvironment::default(),
                            &mut Meter::new(limited(counter, required)),
                        )
                        .expect("the exact-bound call runs");
                    assert!(
                        matches!(
                            evaluation.outcome,
                            FamilyOutcome::Evaluated(Outcome::Completed(_))
                        ),
                        "{name}/{counter:?}: exact budget completes: {evaluation:?}"
                    );
                }
            }
        }
    });
}
