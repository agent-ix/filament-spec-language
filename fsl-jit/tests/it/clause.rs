// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560: clause entries are translated when their checked expressions lie
//! in coverage; clauses with outside constructs are listed unsupported.

use filament_checked::key::DeclarationKey;
use filament_eval::ExecutionBackendChoice;
use filament_eval::{FamilyOutcome, UnsupportedCause, UntranslatableConstruct};
use filament_exact::{
    Cancel, Meter, NodeKey, ObjectId, ObjectReference, Outcome, PathSegment, Presence, UniverseId,
    Value, ValueType,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_conformance::{
    run_parity_with_reference, ParityClauseCase, ParityInputs, ParityLimits, SourceFile,
};
use fsl_eval::execution::{
    EntryKind, EntrySelection, ExecutionBackend, Interpreter, PrepareLimits, PreparedEntries,
};
use fsl_eval::value::S6aRewardEvaluator;
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_jit::Jit;
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, FieldMemberRecord, Multiplicity,
    ObjectTypeRecord, PopulationRecord, RelationshipDirection, RelationshipEnd,
    RelationshipRecord, ScalarTypeRecord, ScalarValueDomain, ValueTypeRef,
};
use fsl_semantics::model::intake::SelectedModels;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::{AdmittedObservations, DocumentRef, Observation, SelfKind};
use ix_trace_rs::trace;
use std::collections::BTreeMap;
use std::sync::Arc;

fn key(node: &str) -> DeclarationKey {
    DeclarationKey::fixture(format!("ix://test/orders/{node}"))
}

fn housing() -> DeclarationKey {
    key("relationship/Pump-housing-Tank")
}

fn one() -> Multiplicity {
    Multiplicity {
        lower: 1,
        upper: Some(1),
        ordered: false,
        unique: true,
    }
}

fn object_type(name: &str) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: key(name),
        interface_features: None,
        abstract_type: false,
        supertypes: Vec::new(),
    })
}

fn orders_package() -> CheckedPackage {
    orders_package_with_clause("self.home.level < 50")
}

fn orders_package_with_clause(clause_body: &str) -> CheckedPackage {
    orders_package_with_clause_source(clause_body).0
}

fn orders_package_with_clause_source(clause_body: &str) -> (CheckedPackage, Vec<u8>) {
    orders_package_with_population_clause_source(clause_body, &["Pump"])
}

fn orders_package_with_population_clause_source(
    clause_body: &str,
    population_members: &[&str],
) -> (CheckedPackage, Vec<u8>) {
    let end = |object: &str, role: &str| RelationshipEnd {
        type_identity: key(object),
        role: Some(role.to_owned()),
        multiplicity: Some(one()),
    };
    let records = vec![
        object_type("Pump"),
        object_type("Tank"),
        DomainPackageRecord::ScalarType(ScalarTypeRecord {
            key: key("Level"),
            domain: ScalarValueDomain::Integer {
                lower: 0,
                upper: 100,
            },
        }),
        DomainPackageRecord::FieldMember(FieldMemberRecord {
            key: key("Tank/level"),
            owner: key("Tank"),
            value_type: ValueTypeRef::Package(key("Level")),
            multiplicity: one(),
            presence: Presence::Required,
            subsets: Vec::new(),
            redefines: None,
            position: 0,
        }),
        DomainPackageRecord::Relationship(RelationshipRecord {
            key: housing(),
            declared_at: None,
            source: end("Pump", "home"),
            target: end("Tank", "pump"),
            direction: RelationshipDirection::Undirected,
        }),
        DomainPackageRecord::Population(PopulationRecord {
            key: key("pumps"),
            member_types: population_members
                .iter()
                .map(|member| key(*member))
                .collect(),
            extent: Extent::Closed,
            symmetric: false,
        }),
    ];
    let package = DomainPackage::new(DomainPackageRef::fixture("bundle.orders"), records);
    let identity = package.model_selection.identity.clone();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {identity:?};\n\
         bound B on M::pumps [1]\n\
         invariant TankLevel using v on M::Pump at current {{ {clause_body} }}\n"
    );
    let selected = SelectedModels::fixture(
        "M",
        fsl_foundation::Span { start: 0, end: 1 },
        package,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the model admits");
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "tc-1040"),
        "unit.native",
        unit.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        built,
        selected,
        vec![],
    )
    .expect("the model assembles");
    let package = CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &S6aRewardEvaluator,
            )
            .expect("the orders model checks"),
    );
    (package, unit.into_bytes())
}

#[trace("FR-560-AC-7", "TC-1381")]
#[test]
fn a_covered_state_clause_is_compiled_and_matches_the_interpreter_evaluation() {
    let (package, source) = orders_package_with_clause_source("true");
    let package = Arc::new(package);
    let clause = package
        .graph()
        .state_clause("TankLevel")
        .expect("the invariant is checked");
    let selection = EntrySelection {
        kind: EntryKind::Clause,
        name: crate::common::name("TankLevel"),
    };
    let prepared = Jit
        .prepare(
            &package,
            std::slice::from_ref(&selection),
            &PrepareLimits::default(),
            &Cancel::new(),
        )
        .unwrap_or_else(|failure| panic!("the JIT prepares: {failure:?}"));
    assert!(
        prepared.unsupported().is_empty(),
        "{:?}",
        prepared.unsupported()
    );

    let observations = AdmittedObservations {
        usage: Default::default(),
        clause: clause.identity(),
        current: Some(Observation {
            identity: DocumentRef {
                authority: "test".to_owned(),
                identity: "current".to_owned(),
            },
            environment: ObjectEnvironment::default(),
            populations: Default::default(),
            out_of_range: Vec::new(),
        }),
        pre: None,
        post: None,
        self_object: ObjectReference::new(
            UniverseId::new(
                NodeKey::new("test", vec![PathSegment::Text("root".to_owned())], 9)
                    .expect("universe root key"),
            ),
            clause.context(),
            ObjectId::new("p1".to_owned()).expect("object id"),
        ),
        self_population: None,
        invocation: None,
        self_kind: SelfKind::Object,
        receiver_pre: None,
        receiver_post: None,
        parameters: Vec::new(),
        value_origins: Default::default(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    };
    let accounting = fsl_semantics::check::SCALAR_LIMITS_UNLIMITED;
    let expected = package
        .evaluate_clause(
            &QualifiedName::unqualified("TankLevel").expect("qualified name"),
            &observations,
            &mut Meter::new(accounting),
        )
        .expect("the interpreter evaluates the clause");
    let actual = prepared
        .evaluate(
            &QualifiedName::unqualified("TankLevel").expect("qualified name"),
            &observations,
            &accounting,
            &Cancel::new(),
        )
        .expect("the JIT evaluates the clause");
    assert!(matches!(
        (&actual.outcome, &expected.evaluation.outcome),
        (
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(actual))),
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(expected)))
        ) if actual == expected
    ));
    assert_eq!(actual.location, expected.evaluation.location);
    assert_eq!(actual.losses, expected.evaluation.losses);

    let corpus = tempfile::tempdir().expect("a corpus directory");
    std::fs::write(corpus.path().join("complete-v1.json"), br#"{"vectors":[]}"#)
        .expect("the empty corpus writes");
    let mut inputs = ParityInputs::new(corpus.path());
    inputs.count = 0;
    inputs.clauses.push(ParityClauseCase {
        id: "covered-state-clause".to_owned(),
        source: source.clone(),
        source_files: vec![SourceFile {
            path: "orders.native".to_owned(),
            bytes: source,
        }],
        package: Arc::clone(&package),
        clause: "TankLevel".to_owned(),
        observations,
    });
    let jit = Jit;
    let backends = [(
        ExecutionBackendChoice::Jit,
        &jit as &dyn fsl_eval::execution::SuppliedBackend,
        Some(fsl_jit::coverage()),
    )];
    let report = run_parity_with_reference(
        &Interpreter,
        &backends,
        &inputs,
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the clause parity case runs");
    assert_eq!(report.exit_code(), 0, "{:#?}", report.backends[0].findings);
    assert_eq!(report.backends[0].compared, 1);
    assert!(report.backends[0].unsupported.is_empty());
    assert!(report.backends[0].findings.is_empty());

    let (multi_member_package, _) = orders_package_with_population_clause_source(
        "only(pump in M::pumps: 1) = 1",
        &["Pump", "Tank"],
    );
    let multi_member_package = Arc::new(multi_member_package);
    let multi_member_selection = EntrySelection {
        kind: EntryKind::Clause,
        name: crate::common::name("TankLevel"),
    };
    let multi_member_prepared = Jit
        .prepare(
            &multi_member_package,
            std::slice::from_ref(&multi_member_selection),
            &PrepareLimits::default(),
            &Cancel::new(),
        )
        .unwrap_or_else(|failure| panic!("the JIT prepares: {failure:?}"));
    let unsupported = multi_member_prepared.unsupported();
    assert_eq!(unsupported.len(), 1, "{unsupported:?}");
    assert!(matches!(
        &unsupported[0].1.cause,
        UnsupportedCause::Untranslatable {
            construct: UntranslatableConstruct::ValueType(value_type),
            ..
        } if matches!(value_type.as_ref(), ValueType::Population(_))
    ));
}

#[trace("TC-1380", "FR-560-AC-1", "FR-560-AC-7")]
#[test]
fn population_only_query_uses_the_admitted_population_members() {
    let (package, source) = orders_package_with_clause_source("only(pump in M::pumps: 1) = 1");
    let package = Arc::new(package);
    let clause = package.graph().state_clause("TankLevel").expect("the clause checks");
    let selection = EntrySelection {
        kind: EntryKind::Clause,
        name: crate::common::name("TankLevel"),
    };
    let prepared = Jit
        .prepare(
            &package,
            std::slice::from_ref(&selection),
            &PrepareLimits::default(),
            &Cancel::new(),
        )
        .unwrap_or_else(|failure| panic!("the JIT prepares: {failure:?}"));
    assert!(prepared.unsupported().is_empty(), "{:?}", prepared.unsupported());

    let self_object = ObjectReference::new(
        UniverseId::new(
            NodeKey::new("test", vec![PathSegment::Text("root".to_owned())], 9)
                .expect("universe root key"),
        ),
        clause.context(),
        ObjectId::new("p1".to_owned()).expect("object id"),
    );
    let environment = ObjectEnvironment::default()
        .with_population_members(BTreeMap::from([(
            key("pumps").node.clone(),
            vec![self_object.clone()],
        )]))
        .with_population_completeness(BTreeMap::from([(key("pumps").node, true)]));
    let observations = AdmittedObservations {
        usage: Default::default(),
        clause: clause.identity(),
        current: Some(Observation {
            identity: DocumentRef {
                authority: "test".to_owned(),
                identity: "current".to_owned(),
            },
            environment,
            populations: Default::default(),
            out_of_range: Vec::new(),
        }),
        pre: None,
        post: None,
        self_object,
        self_population: None,
        invocation: None,
        self_kind: SelfKind::Object,
        receiver_pre: None,
        receiver_post: None,
        parameters: Vec::new(),
        value_origins: Default::default(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    };
    let accounting = fsl_semantics::check::SCALAR_LIMITS_UNLIMITED;
    let expected = package
        .evaluate_clause(
            &QualifiedName::unqualified("TankLevel").expect("qualified name"),
            &observations,
            &mut Meter::new(accounting),
        )
        .expect("the interpreter evaluates the clause");
    let actual = prepared
        .evaluate(
            &QualifiedName::unqualified("TankLevel").expect("qualified name"),
            &observations,
            &accounting,
            &Cancel::new(),
        )
        .expect("the JIT evaluates the clause");
    assert!(matches!(
        (&actual.outcome, &expected.evaluation.outcome),
        (
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(actual))),
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(expected)))
        ) if *actual && *expected
    ));
    assert_eq!(actual.location, expected.evaluation.location);
    assert_eq!(actual.losses, expected.evaluation.losses);

    let corpus = tempfile::tempdir().expect("a corpus directory");
    std::fs::write(corpus.path().join("complete-v1.json"), br#"{"vectors":[]}"#)
        .expect("the empty corpus writes");
    let mut inputs = ParityInputs::new(corpus.path());
    inputs.count = 0;
    inputs.clauses.push(ParityClauseCase {
        id: "population-only-clause".to_owned(),
        source: source.clone(),
        source_files: vec![SourceFile {
            path: "orders.native".to_owned(),
            bytes: source,
        }],
        package: Arc::clone(&package),
        clause: "TankLevel".to_owned(),
        observations,
    });
    let jit = Jit;
    let backends = [(
        ExecutionBackendChoice::Jit,
        &jit as &dyn fsl_eval::execution::SuppliedBackend,
        Some(fsl_jit::coverage()),
    )];
    let report = run_parity_with_reference(
        &Interpreter,
        &backends,
        &inputs,
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the population clause parity case runs");
    assert_eq!(report.exit_code(), 0, "{:#?}", report.backends[0].findings);
    assert_eq!(report.backends[0].compared, 1);
    assert!(report.backends[0].unsupported.is_empty());
    assert!(report.backends[0].findings.is_empty());
}

#[trace("TC-1381")]
#[test]
fn a_selected_state_clause_is_listed_unsupported_with_its_first_outside_construct() {
    let package = orders_package();
    let selection = EntrySelection {
        kind: EntryKind::Clause,
        name: crate::common::name("TankLevel"),
    };
    let prepared = Jit
        .prepare(
            &package,
            std::slice::from_ref(&selection),
            &PrepareLimits::default(),
            &Cancel::new(),
        )
        .unwrap_or_else(|failure| panic!("the JIT prepares: {failure:?}"));
    assert_eq!(prepared.unsupported().len(), 1);
    let (listed, entry) = &prepared.unsupported()[0];
    assert_eq!(*listed, selection);
    let UnsupportedCause::Untranslatable { construct, .. } = &entry.cause else {
        panic!("expected Untranslatable, got {:?}", entry.cause);
    };
    assert!(matches!(
        construct,
        UntranslatableConstruct::ExpressionForm(form) if form == "deref"
    ), "{construct:?}");
}
