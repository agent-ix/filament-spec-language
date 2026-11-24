// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-114-AC-5: bind process-owned protocol attempts through checked-package linking.

use filament_checked::key::DeclarationKey;
use filament_exact::Identifier;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, FieldMemberRecord, Multiplicity,
    NativeValueType, OperationEffect, OperationMemberRecord, PopulationRecord, ProcessRecord,
    ProcessStepKind, ProcessStepRecord, ValueTypeRef,
};
use fsl_semantics::model::normalize::{NormalizeOutcome, normalize};
use ix_trace_rs::trace;

fn flow_process_model() -> fsl_semantics::model::intake::SelectedModel {
    let key = |name: &str| DeclarationKey {
        package: "example/flow".to_owned(),
        node: format!("ix://example/flow/{name}"),
    };
    let flow = key("Flow");
    let n = key("Flow/n");
    let process = ProcessRecord {
        key: flow.clone(),
        identity_fields: vec![key("Flow/x")],
        identity_fields_declared: true,
        supertypes: Vec::new(),
        steps: vec![ProcessStepRecord {
            key: key("Flow/start"),
            name: "start".to_owned(),
            step_kind: ProcessStepKind::Command,
            consumes: Vec::new(),
            emits: Vec::new(),
        }],
    };
    let one = Multiplicity {
        lower: 1,
        upper: Some(1),
        ordered: false,
        unique: true,
    };
    let records = vec![
        DomainPackageRecord::Process(process),
        DomainPackageRecord::FieldMember(FieldMemberRecord {
            key: key("Flow/x"),
            owner: flow.clone(),
            value_type: ValueTypeRef::Native(NativeValueType::Uuid),
            multiplicity: one,
            presence: filament_exact::Presence::Required,
            subsets: Vec::new(),
            redefines: None,
            position: 0,
        }),
        DomainPackageRecord::FieldMember(FieldMemberRecord {
            key: n.clone(),
            owner: flow.clone(),
            value_type: ValueTypeRef::Native(NativeValueType::Integer),
            multiplicity: one,
            presence: filament_exact::Presence::Required,
            subsets: Vec::new(),
            redefines: None,
            position: 1,
        }),
        DomainPackageRecord::OperationMember(OperationMemberRecord {
            key: key("Flow/go"),
            owner: flow.clone(),
            parameters: Vec::new(),
            result: None,
            effect: OperationEffect {
                modifies: vec![n],
                ..Default::default()
            },
            requires: Vec::new(),
            ensures: Vec::new(),
            has_body: false,
            redefines: None,
        }),
        DomainPackageRecord::Population(PopulationRecord {
            key: key("FlowPopulation"),
            member_types: vec![flow],
            extent: Extent::Closed,
            symmetric: false,
        }),
    ];
    let package = DomainPackage::new(
        DomainPackageRef {
            identity: "example/flow".to_owned(),
        },
        records,
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("the Flow process fixture normalizes");
    };
    fsl_semantics::model::intake::SelectedModel {
        alias: "Config".to_owned(),
        span: fsl_foundation::Span { start: 0, end: 0 },
        view,
        operation_clauses: Vec::new(),
        type_invariants: Vec::new(),
    }
}

fn flow_attempt_unit(contracts: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile p = \"filament.protocol.finite-global/v1\";\n\
         model Config = \"example/flow\";\n\
         pre P using v on Config::Flow::go {{ self.n < 3 }}\n\
         post Q using v on Config::Flow::go {{ self.n = pre(self.n) + 1 }}\n\
         invariant I using v on Config::Flow at current {{ self.n >= 0 }}\n\
         protocol FlowProtocol using p over (input: Config::Flow) on origin {{\n\
         role R on Config::Flow;\n\
         run sequence Main {{\n\
         attempt U by R on Config::Flow::go contracts [{contracts}] as (tried: Boolean) {{ true }};\n\
         }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}"
    )
}

fn check_flow_attempt_unit(
    source: &str,
) -> Result<fsl_semantics::check::CheckedGraph, Vec<fsl_semantics::check::CheckRefusal>> {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:qsl-2090-fr114"),
        "program.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 admits the process attempt unit");
    assert_eq!(parsed.diagnostics(), []);
    let source = parsed.source().reference().clone();
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the process attempt unit");
    fsl_semantics::check::PackageDeclarations::assemble(
        source,
        "pkg".to_owned(),
        unit,
        vec![flow_process_model()],
        Vec::new(),
    )
    .expect("the process attempt unit assembles")
    .check(
        filament_semantic_value::checking::CheckMode::Linked,
        filament_semantic_value::checking::CheckingLimits::default(),
        &fsl_semantics::check::test_support::ZeroRewardEvaluator,
    )
}

/// FR-114-AC-5 (TC-513): the process-owned `go` attempt binds `Q` and shares
/// the process's single anchor and frame with `P` and `Q`; invalid entries
/// retain their specified typed refusal causes.
#[trace("TC-513", "FR-114-AC-5")]
#[test]
fn a_process_attempt_binds_the_process_anchor_frame_and_contract() {
    let unit = flow_attempt_unit("Q");
    let graph = check_flow_attempt_unit(&unit)
        .unwrap_or_else(|refusals| panic!("process attempt checks: {refusals:?}"));
    let [protocol] = graph.protocols() else {
        panic!("one protocol: {:?}", graph.protocols());
    };
    let [attempt] = protocol.attempts.as_slice() else {
        panic!("one attempt: {:?}", protocol.attempts);
    };
    let clauses = graph.state_clauses();
    for name in ["P", "Q", "I"] {
        assert!(
            clauses.iter().any(|clause| clause.name() == name),
            "the FR-104-AC-10 clause {name} is checked"
        );
    }
    let q = clauses
        .iter()
        .find(|clause| clause.name() == "Q")
        .expect("the postcondition Q is checked");
    assert_eq!(attempt.contracts, [q.identity()]);
    assert_eq!(attempt.operation.declaration.name(), "go");
    assert_eq!(attempt.operation.declaring, attempt.receiver);
    let attempt_receiver = attempt.receiver.clone();
    let attempt_anchor = attempt.anchor.clone();
    let attempt_frame = attempt.frame.clone();
    let frame_records = graph
        .requirements()
        .keys()
        .filter(|key| key.node() == &attempt_frame)
        .count();
    assert_eq!(frame_records, 1);

    let checked = fsl_package::CheckedPackage::link(graph);
    let graph = checked.graph();
    let anchor_count = graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.semantic_form() == "operation_anchor")
        .count();
    let frame_count = graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.semantic_form() == "frame")
        .count();
    assert_eq!(anchor_count, 1, "one operation_anchor node for go");
    assert_eq!(frame_count, 1, "one frame node for go");
    let name = |text: &str| Identifier::new(text).expect("a model identifier");
    let selection = graph
        .resolve_operation(&name("Config"), &name("Flow"), &name("go"))
        .expect("the process operation resolves");
    let (context, operation_frame) = graph
        .operation_frame(&selection)
        .expect("the checked graph has the process operation frame");
    assert_eq!(context, attempt_receiver);
    assert_eq!(operation_frame.anchor(), attempt_anchor);
    assert_eq!(operation_frame.frame(), attempt_frame);
    assert_eq!(operation_frame.operation().declaring, attempt_receiver);
    assert_eq!(operation_frame.operation().declaration.name(), "go");
    assert_eq!(
        graph.operation_anchor_of(attempt_receiver, "go"),
        Some(attempt_anchor.clone()),
        "the P/Q clauses and attempt share one anchor identity"
    );
    assert_eq!(
        graph.operation_frame_by_anchor(attempt_anchor),
        Some(operation_frame),
        "the process context resolves exactly the attempt's frame"
    );
    assert_eq!(
        graph.operation_frame_by_identity(attempt_frame),
        Some(operation_frame),
        "the frame identity resolves to the same unique operation frame"
    );

    let missing = check_flow_attempt_unit(&flow_attempt_unit("Absent"))
        .expect_err("an absent contracts entry refuses");
    let [missing] = missing.as_slice() else {
        panic!("one missing-contract refusal: {missing:?}");
    };
    assert_eq!(missing.cause.code().as_str(), "missing_declaration");
    assert_eq!(missing.cause.cause(), Some("missing-name"));
    let missing_span = contracts_entry_span(&flow_attempt_unit("Absent"), "Absent");
    match missing.cause.as_ref() {
        fsl_semantics::check::CheckCause::ProtocolAnchor(cause) => match cause.as_ref() {
            fsl_semantics::check::ProtocolAnchorCause::MissingContract { entry, span } => {
                assert_eq!(entry, "Absent");
                assert_eq!(*span, missing_span, "the refusal locus is the entry token");
            }
            other => panic!("typed missing-contract refusal: {other:?}"),
        },
        other => panic!("a protocol-anchor refusal: {other:?}"),
    }

    let wrong_anchor = check_flow_attempt_unit(&flow_attempt_unit("I"))
        .expect_err("an invariant contracts entry refuses");
    let [wrong_anchor] = wrong_anchor.as_slice() else {
        panic!("one wrong-anchor refusal: {wrong_anchor:?}");
    };
    assert_eq!(wrong_anchor.cause.code().as_str(), "wrong_snapshot");
    assert_eq!(wrong_anchor.cause.cause(), Some("wrong-anchor"));
    let wrong_anchor_span = contracts_entry_span(&flow_attempt_unit("I"), "I");
    match wrong_anchor.cause.as_ref() {
        fsl_semantics::check::CheckCause::ProtocolAnchor(cause) => match cause.as_ref() {
            fsl_semantics::check::ProtocolAnchorCause::WrongContractAnchor {
                entry,
                attempt_anchor,
                clause_anchor,
                span,
            } => {
                assert_eq!(entry, "I");
                assert_eq!(attempt_anchor, "Config::Flow::go");
                assert_eq!(clause_anchor, "Config::Flow");
                assert_eq!(
                    *span, wrong_anchor_span,
                    "the refusal locus is the entry token"
                );
            }
            other => panic!("typed wrong-anchor refusal: {other:?}"),
        },
        other => panic!("a protocol-anchor refusal: {other:?}"),
    }
}

fn contracts_entry_span(source: &str, entry: &str) -> fsl_foundation::Span {
    let prefix = "contracts [";
    let start = source
        .find(prefix)
        .expect("the fixture has a contracts list")
        + prefix.len();
    let token_start = start
        + source[start..]
            .find(entry)
            .expect("the contracts entry is present");
    fsl_foundation::Span {
        start: token_start,
        end: token_start + entry.len(),
    }
}
