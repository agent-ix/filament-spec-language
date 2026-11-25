// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330's source-to-temporal-evaluation integration cases.

use std::collections::BTreeMap;

use filament_exact::Cancel;
use filament_semantic_value::catalog::Category;
use filament_semantic_value::checking::CheckMode;
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{
    self, default_accounting, CallOutcome, CallRefusal, ClauseDisposition, ClauseRunReport,
    ClauseRunRequest, ClauseRunSelection, ClauseRunSource, ClauseRunStage, DependencyInput,
    OverObject, PackageTemporalRequest, ParseRequest, SpineLimits, StateClauseExecutionPoints,
    TraceInput,
};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{meaning, package_input, PackageInput};
use fsl_semantics::model::observation::{
    document_reference, DocumentProvision, DocumentRef, ObservationLimits,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

const PACKAGE: &str = "test/temporal-counter";
const PATH: &str = "temporal.native";
/// `InUnitAny` narrows its `Int[0, 3]` argument into `InUnit`'s `Int[0, 1]`:
/// an obligation a Kernel function body leaves to the run.
const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.value.complete/v1\";\n\
profile t = \"filament.temporal.event-position.false-extension/v1\";\n\
model M = \"test/temporal-counter\";\n\
function InUnit using v(x: Int[0, 1]): Boolean pure { x >= 0 }\n\
function InUnitAny using v(x: Int[0, 3]): Boolean pure { InUnit(x) }\n\
temporal Bounded using t over (c: M::Counter) clock \"model-steps\" on origin { always[0,1] holds(InUnitAny(c.value)) }\n";
/// The same narrowing written in the `holds` condition of a declared clause.
const NARROWING_CLAUSE: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.value.complete/v1\";\n\
profile t = \"filament.temporal.event-position.false-extension/v1\";\n\
model M = \"test/temporal-counter\";\n\
function InUnit using v(x: Int[0, 1]): Boolean pure { x >= 0 }\n\
temporal Bounded using t over (c: M::Counter) clock \"model-steps\" on origin { always[0,1] holds(InUnit(c.value)) }\n";

fn ident(name: &str) -> String {
    format!("ix://{PACKAGE}/{name}")
}

fn origin(identity: &str) -> Value {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0", "inputIdentities": [identity]}})
}

fn wire_type(identity: &str, kind: &str, extra: Value) -> Value {
    let mut node = json!({
        "identity": identity,
        "displayName": identity,
        "kind": {"module": PACKAGE, "name": kind},
        "roles": [],
        "origin": origin(identity),
        "constraints": [],
        "extensions": [],
        "unknownPolicy": "reject",
    });
    for (name, value) in extra.as_object().expect("extra is an object") {
        node[name] = value.clone();
    }
    node
}

fn model() -> Vec<u8> {
    let counter = ident("Counter");
    let level = ident("Level");
    let population = ident("counters");
    let bound = |keyword: &str, value: &str| {
        json!({
            "identity": format!("{level}/constraints/{keyword}"),
            "keyword": keyword,
            "operands": {"value": value},
            "appliesTo": "ix://filament/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"),
            "origin": origin(&level),
        })
    };
    let constructs: Vec<Value> = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
        ("value_type", meaning::VALUE_TYPE),
    ]
    .into_iter()
    .map(|(name, meaning)| {
        json!({
            "kind": {"module": PACKAGE, "name": name},
            "moduleVersion": "1.0.0",
            "construct": {"identity": "none", "shape": "record", "members": {}, "meaning": meaning},
        })
    })
    .collect();
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {"identity": PACKAGE, "version": "1.0.0", "mappingVersions": [], "profileVersions": []},
        "occurrences": [],
        "extensions": [],
        "constructs": constructs,
        "types": [
            wire_type(&level, "value_type", json!({"scalar": "integer", "constraints": [bound("min", "0"), bound("max", "3")]})),
            wire_type(&counter, "object_type", json!({
                "supertypes": [],
                "fields": [{
                    "identity": ident("Counter/value"), "name": "value", "typeRef": level,
                    "presence": "required", "nullable": false, "defaultKind": "none",
                    "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                    "origin": origin(&ident("Counter/value")),
                }],
                "operations": [],
                "relationships": [],
            })),
        ],
        "populations": [{
            "identity": population,
            "displayName": population,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [counter],
            "extent": "closed",
            "origin": origin(&population),
        }],
    })
    .to_string()
    .into_bytes()
}

fn snapshot(label: &str, value: &str) -> (DocumentRef, Vec<u8>) {
    let bytes = json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": label},
        "observation": "current",
        "anchor": {"kind": "initialization", "name": "start"},
        "model": {"identity": PACKAGE},
        "populations": [{
            "population": ident("counters"),
            "complete": true,
            "objects": [{"key": "c", "type": ident("Counter"), "fields": {"value": {"integer": value}}}],
        }],
    })
    .to_string()
    .into_bytes();
    let reference = document_reference(&bytes).expect("the snapshot has an identity");
    (reference, bytes)
}

struct Inputs {
    packages: PackageInput,
    snapshots: DocumentProvision,
    trace: TraceInput,
    over: OverObject,
}

/// The `Counter` package, one snapshot per value and the trace over them.
fn inputs(values: &[(&str, &str)]) -> Inputs {
    let package = model();
    let packages = package_input([package.as_slice()]).expect("the Counter package is valid");
    let documents: Vec<(DocumentRef, Vec<u8>)> = values
        .iter()
        .map(|(label, value)| snapshot(label, value))
        .collect();
    let trace = TraceInput::Finite(
        documents
            .iter()
            .map(|(reference, _)| reference.clone())
            .collect(),
    );
    Inputs {
        packages,
        snapshots: documents.into_iter().collect(),
        trace,
        over: OverObject {
            population: ident("counters"),
            key: "c".to_owned(),
        },
    }
}

fn identity() -> SourceIdentity {
    SourceIdentity::new("agent-ix", "qsl-2094-fr330-ac7")
}

/// A source run of `source`, which compiles it under `Linked`.
fn run_source(source: &str, values: &[(&str, &str)]) -> ClauseRunReport {
    let inputs = inputs(values);
    let request = ClauseRunRequest {
        source: ClauseRunSource::Program {
            identity: identity(),
            path: PATH.to_owned(),
            bytes: source.as_bytes().to_vec(),
        },
        package_name: "temporal-counter".to_owned(),
        packages: inputs.packages,
        dependencies: DependencyInput::default(),
        snapshots: inputs.snapshots,
        invocations: BTreeMap::new(),
        selection: ClauseRunSelection::Temporal {
            name: "Bounded".to_owned(),
            trace: inputs.trace,
            over: inputs.over,
            ledger: None,
        },
        limits: SpineLimits::default(),
        observation_limits: ObservationLimits::default(),
        accounting: default_accounting(1_000_000),
        clocks: Default::default(),
    };
    spine::run_clause(request).expect("the request returns an evaluation report")
}

/// A run of `SOURCE`'s checked package, compiled with its function bodies
/// under `Kernel`.
fn run_kernel_package(values: &[(&str, &str)]) -> ClauseRunReport {
    let inputs = inputs(values);
    let identity = identity();
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: PATH,
            bytes: SOURCE.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("the unit parses")
    .into_value();
    let models = spine::select(&parsed, &inputs.packages, limits.model, &cancel)
        .expect("the model selects")
        .into_value();
    let checked = spine::check_with_execution_points_and_source_inputs(
        &parsed,
        "temporal-counter".to_owned(),
        &StateClauseExecutionPoints::new(),
        spine::SourceInputs {
            check_mode: Some(CheckMode::Kernel),
            ..spine::SourceInputs::default()
        },
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .expect("the unit checks with Kernel function bodies")
    .into_value();
    spine::run_package_temporal(&PackageTemporalRequest {
        package: checked.package(),
        packages: &inputs.packages,
        model_limits: ModelNormalizationLimits::default(),
        snapshots: &inputs.snapshots,
        observation_limits: ObservationLimits::default(),
        accounting: default_accounting(1_000_000),
        name: "Bounded",
        trace: &inputs.trace,
        over: Some(&inputs.over),
        ledger: None,
        sources: std::slice::from_ref(checked.source()),
        authentication: None,
        cancel: &cancel,
    })
}

#[trace("TC-1160", "FR-330-AC-7")]
#[test]
fn temporal_holds_refusal_preserves_kernel_record_position_and_locus() {
    let linked = run_source(NARROWING_CLAUSE, &[("s0", "0"), ("s1", "2")]);
    assert_eq!(linked.disposition.stage(), ClauseRunStage::Compile);
    let ClauseDisposition::Compile(refusal) = &linked.disposition else {
        panic!("the clause's own unproved narrowing is refused: {linked:?}");
    };
    assert_eq!(refusal.code().as_str(), "undefined_expression");
    assert_eq!(refusal.cause(), Some("unproved-range"));
    assert_eq!(linked.disposition.exit_code(), 20);

    let zero = run_kernel_package(&[("s0", "0"), ("s1", "0")]);
    assert_eq!(zero.disposition.category(), Category::Success);
    assert_eq!(zero.disposition.category_label(), "success");
    assert_eq!(zero.disposition.exit_code(), 0);
    assert_eq!(zero.disposition.truth(), Some(true));

    let report = run_kernel_package(&[("s0", "0"), ("s1", "2")]);
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(report.disposition.category(), Category::Refusal);
    assert_eq!(report.disposition.category_label(), "refusal");
    assert_eq!(report.disposition.exit_code(), 20);
    assert_eq!(report.refused_at, Some(1));
    assert_eq!(report.disposition.truth(), None);
    let ClauseDisposition::Evaluate(CallOutcome::Refused(CallRefusal::Record {
        code,
        fields,
        locus: Some(locus),
        location: Some(_),
    })) = &report.disposition
    else {
        panic!("the temporal condition returns its typed refusal: {report:?}");
    };
    assert_eq!(code.code(), "integer_out_of_domain");
    assert_eq!(code.cause(), "outside-domain");
    assert_eq!(fields.len(), 1);
    assert_eq!(
        fields.get("expected").map(String::as_str),
        Some("Int[0, 1]")
    );
    assert_eq!(locus.file, PATH);
    let narrowing = "InUnit(x)";
    let argument = SOURCE
        .find(narrowing)
        .expect("the narrowing call is in source")
        + "InUnit(".len();
    assert_eq!(locus.span.start.byte, argument);
    assert_eq!(locus.span.end.byte, argument + "x".len());
    assert!(report.decisive.is_none());
    assert!(report.counterexample.is_none());
}
