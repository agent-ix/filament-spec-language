// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-571-AC-7, FR-517-AC-24 (TC-1411, TC-1312): a protocol package that
//! arrives only as a checked handoff directory reaches the realizability
//! engine through the package `checked_handoff::compile` compiles it back to.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use filament_checked::key::DeclarationKey;
use filament_exact::{
    Cancel, EffectiveId, NodeKey, ObjectId, ObjectReference, OriginalDeclarationKey, PathSegment,
    ScalarLimits, UniverseId,
};
use fsl_analyze::analyze::{analyze_items, Engines};
use fsl_analyze::protocol_analysis::{
    protocol_subject, realizability_item, RealizabilityRequest, SubjectBindings, SubjectPackage,
    SubjectRefusal,
};
use fsl_command::checked_handoff::{self, CheckedHandoffRequest, CompiledHandoff, Corpus};
use fsl_command::emit_checked_v2::Request;
use fsl_eval::model_subject::{ModelInstance, RoleBinding};
use fsl_eval::simulation::model::{ExpansionLimits, PopulationUniverse};
use fsl_eval::simulation::protocol::{MemorySelection, MemorySelectionRefusal};
use fsl_foundation::MemoryModelName;
use fsl_foundation::RequestIndex;
use fsl_replay::protocol_analysis::{AssessedRootDraft, BoundsDraft, RunBudget};
use fsl_replay::{
    Certification, Execution, ObservationIdentity, ProofBasis, ResultLabel, SettlementBasis,
    TerminalValue, Truth,
};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::meaning;
use fsl_semantics::model::observation::{
    document_reference, DocumentProvision, DocumentRef, ObservationLimits,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

const PACKAGE: &str = "test/flow";

/// An evaluation meter no fixture evaluation reaches.
const METER: ScalarLimits = ScalarLimits {
    integer_bits: 1 << 20,
    decimal_digits: 1 << 20,
    scale_expansion: 1 << 20,
    text_input_bytes: 1 << 20,
    text_scalars: 1 << 20,
    normalized_scalars: 1 << 20,
    unit_edges: 1 << 20,
    value_occurrences: 1 << 20,
    work_units: 1 << 30,
    result_units: 1 << 30,
};

fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

fn origin(identity: &str) -> Value {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

/// A domain package of one object type `Cell` with the field `v: Int[0, 3]`
/// and the closed population `cells` of it.
fn model() -> Vec<u8> {
    let constructs: Vec<Value> = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
        ("value_type", meaning::VALUE_TYPE),
    ]
    .into_iter()
    .map(|(name, meaning)| {
        json!({"kind": {"module": PACKAGE, "name": name}, "moduleVersion": "1.0.0",
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    })
    .collect();
    let level = ident("Level");
    let bound = |keyword: &str, value: &str| {
        json!({
            "identity": format!("{level}/constraints/{keyword}"), "keyword": keyword,
            "operands": {"value": value}, "appliesTo": "ix://filament/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"), "origin": origin(&level),
        })
    };
    let cell = ident("Cell");
    let field = ident("Cell/v");
    let population = ident("cells");
    let header = |identity: &str, kind: &str| {
        json!({
            "identity": identity, "displayName": identity,
            "kind": {"module": PACKAGE, "name": kind}, "roles": [],
            "origin": origin(identity), "extensions": [], "unknownPolicy": "reject",
        })
    };
    let mut level_type = header(&level, "value_type");
    level_type["scalar"] = "integer".into();
    level_type["constraints"] = json!([bound("min", "0"), bound("max", "3")]);
    let mut cell_type = header(&cell, "object_type");
    cell_type["constraints"] = json!([]);
    cell_type["supertypes"] = json!([]);
    cell_type["operations"] = json!([]);
    cell_type["relationships"] = json!([]);
    cell_type["fields"] = json!([{
        "identity": field, "name": "v", "typeRef": level,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(&field),
    }]);
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {"identity": PACKAGE, "version": "1.0.0",
            "mappingVersions": [], "profileVersions": []},
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": [level_type, cell_type],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [cell], "extent": "closed", "origin": origin(&population),
        }],
    })
    .to_string()
    .into_bytes()
}

fn channel(name: &str, from: &str, to: &str, carries: &str) -> String {
    format!(
        "channel {name} from {from} to {to} carries {carries} ordering unordered delivery \
         exactly-once-premise capacity 4 overflow reject;\n"
    )
}

/// ADR-036's worked example repaired by the `Cancelled` message: `Buyer`
/// places an order, `Seller` reads it and owns `ship | cancel`, and tells
/// `Warehouse` either way.
fn unit() -> String {
    let members = [
        "role Buyer on M::Cell;\nrole Seller on M::Cell;\nrole Warehouse on M::Cell;\n".to_owned(),
        channel("Order", "Buyer", "Seller", "Ack"),
        channel("Pick", "Seller", "Warehouse", "PickMsg"),
        channel("Cancelled", "Seller", "Warehouse", "CancelMsg"),
    ]
    .concat();
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile pp = \"filament.protocol.finite-global/v1\";\n\
         model M = \"{PACKAGE}\";\n\
         record Ack {{ ok: Boolean; }}\nrecord PickMsg {{ ok: Boolean; }}\n\
         record CancelMsg {{ ok: Boolean; }}\n\
         protocol Flow using pp over (k: M::Cell) on origin {{\n{members}\n\
         run sequence Main {{\n\
         send Placed via Order as (placed: Ack) {{ true }};\n\
         receive Got via Order of Placed as (got: Ack) {{ true }};\n\
         choice Decide by Seller visible (got.ok) {{\n\
           case no when {{ not got.ok }} sequence Cancel {{\n\
             send Cn via Cancelled as (c: CancelMsg) {{ true }};\n\
             receive Cn2 via Cancelled of Cn as (d: CancelMsg) {{ true }}; }}\n\
           case yes when {{ got.ok }} sequence Ship {{\n\
             send PickIt via Pick as (p: PickMsg) {{ true }};\n\
             receive PickedUp via Pick of PickIt as (q: PickMsg) {{ true }}; }}\n\
         }} }}\n\
         finish Done as (zz: Boolean) {{ true }};\n}}\n"
    )
}

/// The initialization snapshot `s0` of the one cell `c` with `v = 0`.
fn snapshots() -> (DocumentProvision, DocumentRef) {
    let document = json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "s0"},
        "observation": "current", "model": {"identity": PACKAGE},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": ident("cells"), "complete": true,
            "objects": [{"key": "c", "type": ident("Cell"),
                "fields": {"v": {"integer": "0"}}}]}],
    })
    .to_string()
    .into_bytes();
    let mut provision = BTreeMap::new();
    provision.insert(
        document_reference(&document).expect("a document names itself"),
        document,
    );
    let initial = DocumentRef {
        authority: "test".to_owned(),
        identity: "s0".to_owned(),
    };
    (provision, initial)
}

/// Write the handoff of [`unit`] under `holder` and return its directory.
fn written(holder: &Path) -> std::path::PathBuf {
    let source = holder.join("flow.native");
    let models = holder.join("model.json");
    std::fs::write(&source, unit()).expect("the unit is written");
    std::fs::write(&models, model()).expect("the model is written");
    let directory = holder.join("handoff");
    checked_handoff::write(
        &CheckedHandoffRequest {
            emit: Request {
                source,
                package_name: "flow".to_owned(),
                models: vec![models],
                libraries: Vec::new(),
                execution_points: None,
                temporal_selections: Vec::new(),
                out: None,
            },
            corpus: Corpus::None,
            native_join: fsl_package::NativeJoinLimits::new(1 << 20, 1 << 20),
        },
        &directory,
    )
    .expect("the handoff is written");
    directory
}

fn fixture_key(seed: &str) -> NodeKey {
    NodeKey::new("fixture", vec![PathSegment::Text(seed.to_owned())], 0)
        .expect("a fixture node identity is well formed")
}

fn assessed_root() -> AssessedRootDraft {
    AssessedRootDraft {
        observation: Some(ObservationIdentity {
            authority: "test".to_owned(),
            identity: "protocol-claim".to_owned(),
        }),
        object: Some(ObjectReference::new(
            UniverseId::new(fixture_key("u")),
            EffectiveId::new(None, OriginalDeclarationKey::new("fixture", "e")),
            ObjectId::new("flow-1").expect("an object id"),
        )),
    }
}

fn request<'a>(
    handoff: &'a CompiledHandoff,
    snapshots: &'a DocumentProvision,
    initial: DocumentRef,
) -> RealizabilityRequest<'a> {
    let cells = DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident("cells"),
    };
    let subject = protocol_subject(
        SubjectPackage {
            package: handoff.checked(),
            packages: handoff.packages(),
            snapshots,
        },
        SubjectBindings {
            protocol: "Flow".to_owned(),
            over: ModelInstance {
                population: cells.clone(),
                key: "c".to_owned(),
            },
            roles: ["Buyer", "Seller", "Warehouse"]
                .iter()
                .map(|role| RoleBinding {
                    role: (*role).to_owned(),
                    model: PACKAGE.to_owned(),
                })
                .collect(),
            capacities: Vec::new(),
            memory: Vec::new(),
            initial: vec![initial],
            universes: vec![PopulationUniverse {
                population: cells,
                keys: vec!["c".to_owned()],
            }],
            normalization: ModelNormalizationLimits::default(),
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: METER,
            emit: handoff.emit_limits(),
        },
    )
    .expect("the subject builds");
    let protocol = handoff
        .admitted()
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| &*node.semantic_form == "protocol_clause")
        .expect("one protocol clause")
        .node_id
        .clone();
    RealizabilityRequest {
        protocol,
        subject,
        definition_key: Some("ix://test/Flow".to_owned()),
        strategy_profile: Some("bounded-static".to_owned()),
        role_knowledge: Some(Some(BTreeSet::new())),
        environment: Some(Some(BTreeSet::new())),
        bounds: Some(BoundsDraft {
            max_depth: Some(64),
            max_history: Some(16),
        }),
        assessed_root: Some(assessed_root()),
        budget: RunBudget::default(),
    }
}

/// FR-571-AC-7, FR-517-AC-24: only the handoff directory reaches the
/// dispatch. It is compiled back, the subject is built from the package it
/// compiles to, and the item dispatched through the registered engines
/// settles realizable, exhaustive and uncertified.
#[trace("TC-1411", "FR-571-AC-7", "TC-1312", "FR-517-AC-24")]
#[test]
fn a_package_that_arrives_only_as_a_handoff_reaches_realizability() {
    let holder = tempfile::tempdir().expect("a temporary directory");
    let directory = written(holder.path());
    let handoff = checked_handoff::compile(&directory).expect("the directory compiles back");
    let (snapshots, initial) = snapshots();
    let records = analyze_items(
        &Engines::fsl().expect("they register"),
        vec![realizability_item(
            RequestIndex::new(0),
            handoff.admitted(),
            request(&handoff, &snapshots, initial),
        )],
        &Cancel::new(),
    );
    let settlement = records[0]
        .protocol_analysis
        .as_ref()
        .expect("the engine served the item")
        .settlement();
    assert_eq!(
        (
            settlement.label,
            settlement.basis,
            settlement.execution,
            settlement.truth
        ),
        (
            ResultLabel::Proved,
            SettlementBasis::ClosedScope,
            Execution::Completed,
            Truth::Satisfied
        )
    );
    assert_eq!(
        records[0].terminal().expect("a terminal record").value(),
        &TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Uncertified,
        }
    );
}

/// The refusal `protocol_subject` gives over `handoff` for the protocol
/// `protocol` with the memory selection `memory`.
fn refusal(
    handoff: &CompiledHandoff,
    protocol: &str,
    memory: Vec<MemorySelection>,
) -> Option<SubjectRefusal> {
    let (snapshots, initial) = snapshots();
    protocol_subject(
        SubjectPackage {
            package: handoff.checked(),
            packages: handoff.packages(),
            snapshots: &snapshots,
        },
        SubjectBindings {
            protocol: protocol.to_owned(),
            over: ModelInstance {
                population: DeclarationKey {
                    package: PACKAGE.to_owned(),
                    node: ident("cells"),
                },
                key: "c".to_owned(),
            },
            roles: Vec::new(),
            capacities: Vec::new(),
            memory,
            initial: vec![initial],
            universes: Vec::new(),
            normalization: ModelNormalizationLimits::default(),
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits { max_candidates: 1 },
            meter: METER,
            emit: handoff.emit_limits(),
        },
    )
    .err()
}

/// FR-571-AC-8: a protocol the package does not declare, and a memory
/// selection that names no outermost `parallel` of the protocol, build no
/// subject.
#[trace("TC-1411", "FR-571-AC-8")]
#[test]
fn an_undeclared_protocol_or_an_unresolved_memory_selection_builds_no_subject() {
    let holder = tempfile::tempdir().expect("a temporary directory");
    let handoff = checked_handoff::compile(&written(holder.path())).expect("compiles back");
    assert_eq!(
        refusal(&handoff, "Other", Vec::new()),
        Some(SubjectRefusal::UnknownProtocol("Other".to_owned()))
    );
    assert_eq!(
        refusal(
            &handoff,
            "Flow",
            vec![MemorySelection {
                parallel: "Flow::Nope".to_owned(),
                model: MemoryModelName::Tso,
            }]
        ),
        Some(SubjectRefusal::Memory(
            MemorySelectionRefusal::NotAParallel {
                entry: 0,
                parallel: "Flow::Nope".to_owned(),
            }
        ))
    );
}
