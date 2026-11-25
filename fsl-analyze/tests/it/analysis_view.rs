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
    assert_eq!(source.requirement_items(), bytes.requirement_items());
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

/// Requirement rows preserve owner/occurrence identity and guarded scalar
/// facts; the same package calls charge the same operation ledger.
#[trace("FR-940-AC-13", "FR-940-AC-1")]
#[test]
fn value_record_views_and_native_charges_equal_the_source() {
    use filament_exact::{Cancel, Meter, Value};
    use fsl_eval::value::{ApplicationObservationOptions, CheckedPackageEvaluation, QualifiedName};
    use fsl_package::BareSupply;
    // Mixed requirement kinds must keep the Source map's absolute row order.
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
    let domain = fsl_semantics::model::intake::package_input([model.as_slice()]).unwrap();
    let mixed = fsl_package::read_evaluation_package(
        &package,
        fsl_semantics::library::LibraryName::new(UNIT).unwrap(),
        &domain,
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .unwrap();
    assert_eq!(
        mixed.requirement_items().len(),
        package.graph().requirements().len()
    );
    for ((claim, record), row) in package
        .graph()
        .requirements()
        .iter()
        .zip(mixed.requirement_items())
    {
        assert_eq!(row.occurrence.ordinal, claim.origin().ordinal());
        assert_eq!(
            serde_json::to_value(row.kind).unwrap(),
            serde_json::json!(record.requirements().kind().to_wire())
        );
    }
    let source = crate::value_validity_support::package_of(
        "ordered enum Status { ZED, ALPHA, MID }\n\
         function Guarded using v(x: Int[0, 9], p: Boolean): Integer pure { if p then x + 1 else x * 2 }\n\
         function Other using v(x: Int[0, 9]): Integer pure { x + 1 }\n\
         function Ready using v(s: Status): Boolean pure { s = Status::MID }",
    );
    let domain = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let read = fsl_package::read_evaluation_package(
        &source,
        fsl_semantics::library::LibraryName::new("package").unwrap(),
        &domain,
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .unwrap();
    let admitted = fsl_package::admit_checked_package(
        read.canonical_bytes(),
        &domain,
        V2ReadLimits::fixture(),
    )
    .unwrap();
    let mut budget = V2ReadLimits::fixture().native_join_budget();
    let identities = read
        .emission()
        .identity_rows(&admitted, &mut budget)
        .unwrap();
    let equality = read
        .emission()
        .equality_witness(&admitted, &mut budget)
        .unwrap();
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .unwrap();
    assert_eq!(read.requirement_items(), bare.requirement_items());
    let declarations = bare.declared_names();
    let owner = declarations
        .iter()
        .find(|(name, _)| name == "Guarded")
        .unwrap()
        .1;
    assert_eq!(
        read.value_validity_records_of(owner),
        bare.value_validity_records_of(owner)
    );
    let indices = bare.value_validity_records_of(owner);
    assert_eq!(indices.len(), 2);
    for &index in indices {
        let row = &bare.requirement_items()[index];
        let Some(filament_contract_model::RequirementRecord::ValueValidity(record)) =
            row.record.as_ref()
        else {
            panic!("a scalar record");
        };
        assert_eq!(&record.owner, owner);
        assert_eq!(record.application, row.claim);
        assert_eq!(record.occurrence, row.occurrence.ordinal);
        assert_eq!(read.package().path_condition(record).len(), 1);
        assert!(!record.extent_roots.is_empty());
        assert!(bare.node_key(&record.result_bound).is_some());
        let claim = fsl_replay::OccurrenceKey::new(
            bare.node_key(&row.claim).unwrap(),
            filament_exact::Origin::new(
                filament_exact::Role::new("expression"),
                row.occurrence.ordinal,
            ),
        );
        let source_target =
            fsl_replay::value_check::ValueTarget::select(&source, "Guarded", &claim).unwrap();
        let package_target =
            fsl_replay::value_check::ValueTarget::select_package(&bare, "Guarded", &claim).unwrap();
        assert_eq!(
            package_target.form(),
            fsl_replay::value_check::ClaimForm::ScalarOperation
        );
        for flag in [false, true] {
            let assignment = [Value::Integer(3_i64.into()), Value::Boolean(flag)];
            assert_eq!(
                package_target
                    .evaluate(&assignment, &Cancel::new())
                    .unwrap(),
                source_target.evaluate(&assignment, &Cancel::new()).unwrap()
            );
        }
    }
    let world = bare.world().unwrap();
    let fsl_replay::value_check::ParameterDomain::Values(enums) =
        fsl_replay::value_check::ValueTarget::select_package(
            &bare,
            "Ready",
            bare.function_body("Ready")
                .unwrap()
                .unwrap()
                .occurrences
                .iter()
                .find(|key| matches!(key.origin().role().as_str(), "claim" | "declaration"))
                .unwrap(),
        )
        .unwrap()
        .parameters()[0]
            .domain()
            .clone()
    else {
        panic!("enum parameter")
    };
    for (name, arguments) in [
        (
            "Guarded",
            vec![Value::Integer(3_i64.into()), Value::Boolean(false)],
        ),
        (
            "Guarded",
            vec![Value::Integer(3_i64.into()), Value::Boolean(true)],
        ),
        ("Other", vec![Value::Integer(3_i64.into())]),
        ("Ready", vec![enums[0].clone()]),
    ] {
        let name = QualifiedName::unqualified(name).unwrap();
        let objects = fsl_semantics::model::object_environment::ObjectEnvironment::default();
        let options = ApplicationObservationOptions {
            function_case_limit: 100,
            max_events: 10000,
            max_size_requests: 10000,
        };
        let left = source.call_observing_applications(
            &name,
            arguments.clone(),
            &objects,
            &mut Meter::new(fsl_replay::AccountingLimits::default().0),
            options,
        );
        let right = bare.call_observing_applications(
            &name,
            arguments,
            &objects,
            &mut Meter::new(fsl_replay::AccountingLimits::default().0),
            options,
        );
        assert_eq!(right.ledger, left.ledger);
        assert_eq!(right.meter_totals, left.meter_totals);
        assert_eq!(right.function_evaluations, left.function_evaluations);
    }
    assert!(world
        .enum_members()
        .resolve(match &enums[0] {
            Value::Enum(member) => member.variant().clone(),
            _ => panic!("enum value"),
        })
        .is_some());
}
