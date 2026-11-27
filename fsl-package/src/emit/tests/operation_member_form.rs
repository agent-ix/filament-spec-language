// SPDX-License-Identifier: AGPL-3.0-or-later
//! Contract IR FR-038-AC-644 and FR-038-AC-646 (cross-repo, `IR-` tags): an
//! operation member FSL builds from its typed members is written by the IR's
//! `from_form` and read back by the strict reader as the same operation, and
//! FR-105-AC-12's state clause carries its execution point.

use filament_contract_model::{CheckedApplicationOperation, ExecutionPoint, ExecutionPointName};
use filament_exact::Identifier;
use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use fsl_forms::StateClauseKind;
use fsl_foundation::bound::{TemporalInterval, TemporalUpper};
use fsl_semantics::value::{AcceptedPoint, FairnessTarget, Member, TypeArgumentAlias};

use super::golden::float_add_of;
use super::*;

fn id(name: &str) -> Identifier {
    Identifier::new(name).expect("identifier")
}

/// The operation the emitter writes for `member` alone, read back strictly,
/// and the wire member it wrote.
fn written(ids: &StructuralIds, member: &Member) -> Value {
    let (typed, point) = member_value(ids, member).expect("typed member");
    let operation = CheckedApplicationOperation::from_form(
        &filament_contract_model::CheckedApplicationOperationForm {
            identity: "filament.op.test.member".into(),
            laws: Vec::new(),
            mode: None,
            member: Some(typed),
            execution_point: point,
            leaves: Vec::new(),
        },
    )
    .expect("the form writes");
    let wire = serde_json::to_value(&operation).expect("operation wire");
    let read: CheckedApplicationOperation =
        serde_json::from_value(wire.clone()).expect("strict read of the write");
    assert_eq!(read, operation, "the strict read is the written operation");
    wire["member"].clone()
}

/// Every member kind the emitter builds writes its closed wire member and
/// reads back as the same operation, including the declaration-carrying kinds
/// (a field with its population, a relationship end, a fairness target).
#[trace("IR-FR-038-AC-644", "IR-FR-038-AC-646", "FR-105-AC-12")]
#[test]
fn emitted_operation_members_write_then_strictly_read_back() {
    let ids = StructuralIds::default();
    let declaration = fixture_type_key("decl");
    let population = fixture_type_key("pop");
    let destination = fixture_type_key("dest");
    let at = |key: &NodeKey| serde_json::to_value(ids.id(key)).expect("node id");
    let cases = [
        (
            Member::ProfileOperator {
                operator: id("add"),
            },
            json!({"kind":"profile_operator","operator":"add"}),
        ),
        (
            Member::StateClause {
                clause: StateClauseKind::Precondition,
                execution_point: Some(AcceptedPoint::new(ExecutionPoint::Pre {
                    operation: ExecutionPointName::new("go").expect("name"),
                })),
            },
            json!({"kind":"state_clause","clause":"precondition",
                "execution_point":{"kind":"pre","operation":"go"}}),
        ),
        (
            Member::TemporalInterval {
                interval: Some(TemporalInterval::new(1, TemporalUpper::Finite(3)).expect("ok")),
            },
            json!({"kind":"temporal_interval","interval":{"lower":"1","upper":"3"}}),
        ),
        (
            Member::TemporalInterval { interval: None },
            json!({"kind":"temporal_interval","interval":null}),
        ),
        (
            Member::Field {
                declaration: declaration.clone(),
                name: id("f"),
                explicit_deref: Some(true),
                population: Some(population.clone()),
            },
            json!({"kind":"field","declaration":at(&declaration),"name":"f",
                "population":at(&population),"read_form":"explicit_deref"}),
        ),
        (
            Member::Field {
                declaration: declaration.clone(),
                name: id("f"),
                explicit_deref: None,
                population: None,
            },
            json!({"kind":"field","declaration":at(&declaration),"name":"f"}),
        ),
        (
            Member::Position {
                declaration: declaration.clone(),
                position: 2,
            },
            json!({"kind":"position","declaration":at(&declaration),"position":2}),
        ),
        (
            Member::Element {
                declaration: declaration.clone(),
            },
            json!({"kind":"element","declaration":at(&declaration)}),
        ),
        (
            Member::RelationshipEnd {
                declaration: declaration.clone(),
                name: id("end"),
                destination_type: destination.clone(),
                population: Some(population.clone()),
            },
            json!({"kind":"relationship_end","declaration":at(&declaration),"name":"end",
                "destination_type":at(&destination),"population":at(&population)}),
        ),
        (
            Member::Operation {
                declaration: declaration.clone(),
                name: id("op"),
            },
            json!({"kind":"operation","declaration":at(&declaration),"name":"op"}),
        ),
        (
            Member::History {
                declaration: declaration.clone(),
                name: id("h"),
            },
            json!({"kind":"history","declaration":at(&declaration),"name":"h"}),
        ),
        (
            Member::TypeArgument {
                declaration: declaration.clone(),
                alias: Some(TypeArgumentAlias {
                    package: "p".to_owned(),
                    node: "ix://p/a".to_owned(),
                }),
            },
            json!({"kind":"type_argument","declaration":at(&declaration),
                "alias":{"package":"p","node":"ix://p/a"}}),
        ),
        (
            Member::Fairness {
                kind: FairnessKind::Strong,
                granularity: FairnessGranularity::Each,
                target: FairnessTarget::ParallelBranch {
                    parallel: declaration.clone(),
                    branch: id("left"),
                },
            },
            json!({"kind":"fairness","fairness_kind":"strong","granularity":"each",
                "target":{"kind":"parallel_branch","parallel":at(&declaration),"branch":"left"}}),
        ),
        (
            Member::Fairness {
                kind: FairnessKind::Weak,
                granularity: FairnessGranularity::Whole,
                target: FairnessTarget::Operation {
                    declaration: declaration.clone(),
                    name: id("op"),
                },
            },
            json!({"kind":"fairness","fairness_kind":"weak","granularity":"whole",
                "target":{"kind":"operation","declaration":at(&declaration),"name":"op"}}),
        ),
    ];
    for (member, wire) in cases {
        assert_eq!(written(&ids, &member), wire, "wire bytes of {member:?}");
    }
}

/// `operation_term` writes an application's laws and mode through the same
/// typed form: an emitted float addition carries its profile law and rounding
/// mode, and IR's strict reader admits the package.
#[trace("IR-FR-038-AC-644", "IR-FR-038-AC-646")]
#[test]
fn an_emitted_application_operation_carries_its_laws_and_mode() {
    let emission = emit(&float_add_of(BuiltinType::Float64, "nearest-even"));
    let wire = wire(&emission);
    let operation = nodes(&wire)
        .iter()
        .map(|node| &node["body"]["operation"])
        .find(|operation| operation["mode"].is_object())
        .expect("an application with a rounding mode");
    assert_eq!(operation["mode"]["kind"], "rounding");
    assert!(operation["mode"]["value"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(operation["laws"].as_array().is_some_and(|laws| !laws.is_empty()));
    assert!(operation["leaves"].is_array());
    assert!(matches!(read_back(&emission), Read::Verified { .. }));
}
