// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real bounded EN-5 documents checked by a separate qualified-core process.

use std::io::Write;
use std::process::{Command, Stdio};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use filament_checked::key::DeclarationKey;
use filament_exact::Cancel;
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines, Settlement};
use fsl_analyze::exact_probabilistic::{ExactProbLimits, ProductRequest};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, PopulationUniverse};
use fsl_foundation::{RequestIndex, SourceIdentity};
use fsl_replay::spine::{self, DependencyInput, ParseRequest, SpineLimits};
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::observation::{DocumentRef, ObservationLimits};
use ix_trace_rs::trace;
use serde_json::{json, Value};

const DOMAIN: &[u8] = include_bytes!("../fixtures/link.semantic-ir.json");
const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");
const SOURCE: &str = include_str!("../fixtures/link.native");
const COIN_DOMAIN: &[u8] = include_bytes!("../fixtures/coin.semantic-ir.json");
const COIN_SNAPSHOT: &[u8] = include_bytes!("../fixtures/coin.snapshot.json");
const COIN_SOURCE: &str = include_str!("../fixtures/coin.native");

fn model_source() -> String {
    SOURCE.replace(
        "filament.temporal.infinite-trace/v1",
        "filament.temporal.event-position.false-extension/v1",
    )
}

fn claim(scheduler: &str, operator: &str, threshold: &str, horizon: u64) -> String {
    format!("probabilistic Likely using inf on M over (m: M::Msg) under {scheduler} on origin {{ probability {operator} {threshold} [ eventually[0,{horizon}] holds(m.delivered) ] }}\n")
}

pub(super) fn compile_source(
    source: &str,
) -> (
    spine::CheckedUnit,
    fsl_semantics::model::intake::PackageInput,
) {
    compile_source_with(source, DOMAIN)
}

pub(super) fn compile_source_with(
    source: &str,
    domain: &[u8],
) -> (
    spine::CheckedUnit,
    fsl_semantics::model::intake::PackageInput,
) {
    compile_with_mode(source, domain, None)
}

pub(super) fn compile_kernel_source_with(
    source: &str,
    domain: &[u8],
) -> (
    spine::CheckedUnit,
    fsl_semantics::model::intake::PackageInput,
) {
    compile_with_mode(
        source,
        domain,
        Some(filament_semantic_value::checking::CheckMode::Kernel),
    )
}

fn compile_with_mode(
    source: &str,
    domain: &[u8],
    check_mode: Option<filament_semantic_value::checking::CheckMode>,
) -> (
    spine::CheckedUnit,
    fsl_semantics::model::intake::PackageInput,
) {
    try_compile_with_mode(source, domain, check_mode).unwrap()
}

pub(super) fn try_compile_source_with(
    source: &str,
    domain: &[u8],
) -> Result<
    (spine::CheckedUnit, fsl_semantics::model::intake::PackageInput),
    spine::FrontEndFailure,
> {
    try_compile_with_mode(source, domain, None)
}

fn try_compile_with_mode(
    source: &str,
    domain: &[u8],
    check_mode: Option<filament_semantic_value::checking::CheckMode>,
) -> Result<
    (spine::CheckedUnit, fsl_semantics::model::intake::PackageInput),
    spine::FrontEndFailure,
> {
    let identity = SourceIdentity::new("test", "bounded-certificate-source");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let packages = package_input([domain]).unwrap();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "bounded.native",
            bytes: source.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .unwrap()
    .into_value();
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .unwrap()
        .into_value();
    let checked = spine::check_with_execution_points_and_source_inputs(
        &parsed,
        "bounded-fixture".to_owned(),
        &spine::StateClauseExecutionPoints::default(),
        spine::SourceInputs {
            check_mode,
            ..spine::SourceInputs::default()
        },
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )?
    .into_value();
    Ok((checked, packages))
}

pub(super) fn request(
    source: &str,
    snapshot: &[u8],
    exact: ExactProbLimits,
) -> (Option<Value>, Settlement) {
    request_with(source, snapshot, exact, DOMAIN)
}

pub(super) fn request_with(
    source: &str,
    snapshot: &[u8],
    exact: ExactProbLimits,
    domain: &[u8],
) -> (Option<Value>, Settlement) {
    request_with_mode(source, snapshot, exact, domain, false)
}

fn request_with_mode(
    source: &str,
    snapshot: &[u8],
    exact: ExactProbLimits,
    domain: &[u8],
    kernel: bool,
) -> (Option<Value>, Settlement) {
    let (mut requests, settlement) = requests_with_mode(source, snapshot, exact, domain, kernel);
    let first = (!requests.is_empty()).then(|| requests.remove(0));
    (first, settlement)
}

/// One checker request for the settled record's certificate document and, for a
/// value-bounds proof, a second for its counterpart document.
fn requests_with_mode(
    source: &str,
    snapshot: &[u8],
    exact: ExactProbLimits,
    domain: &[u8],
    kernel: bool,
) -> (Vec<Value>, Settlement) {
    let identity = SourceIdentity::new("test", "bounded-certificate-source");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let (checked, packages) = if kernel {
        compile_kernel_source_with(source, domain)
    } else {
        compile_source_with(source, domain)
    };
    let package = checked.package();
    let claim = package
        .graph()
        .probabilistic_model()
        .claim("Likely")
        .unwrap();
    // The universe is the snapshot's first population and its first object.
    let shape: Value = serde_json::from_slice(snapshot).unwrap();
    let universe_package = shape["model"]["identity"].as_str().unwrap().to_owned();
    let universe_node = shape["populations"][0]["population"]
        .as_str()
        .unwrap()
        .to_owned();
    let universe_key = shape["populations"][0]["objects"][0]["key"]
        .as_str()
        .unwrap()
        .to_owned();
    let document = DocumentRef {
        authority: "test".to_owned(),
        identity: "s0".to_owned(),
    };
    let snapshots = [(document.clone(), snapshot.to_vec())]
        .into_iter()
        .collect();
    let simulation = ModelSimulationRequest {
        input: fsl_eval::simulation::model::ModelCheckInput::source(package),
        packages: &packages,
        normalization: limits.model,
        snapshots: &snapshots,
        initial: vec![document],
        universes: vec![PopulationUniverse {
            population: DeclarationKey {
                package: universe_package.clone(),
                node: universe_node.clone(),
            },
            keys: vec![universe_key.clone()],
        }],
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: 100_000,
        },
        meter: fsl_replay::AccountingLimits::default().0,
        emit: limits.package.emit(),
    };
    let system = fsl_eval::simulation::model::ModelSystem::new(simulation.clone()).unwrap();
    let binding = fsl_replay::claim_binding(&system, claim).unwrap().unwrap();
    if matches!(
        claim.form,
        fsl_semantics::check::ProbForm::Probability { .. }
    ) {
        let context =
            fsl_replay::probabilistic_formula_monitor_context(claim, Some(&binding.reference))
                .unwrap();
        assert_eq!(
            context.binding[0].parameter,
            *claim.over.as_ref().unwrap().node().unwrap()
        );
        assert!(
            matches!(&context.binding[0].value, fsl_eval::automaton::ContextBindingValue::Instance(key) if key == &binding.instance.key)
        );
    }
    let engines = Engines::fsl().unwrap();
    let mut records = analyze_items(
        &engines,
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            ProductRequest {
                subject: ModelSubject {
                    simulation,
                    proof_bounds: Vec::new(),
                },
                claim,
                limits: exact,
            },
        )],
        &cancel,
    );
    let record = records.remove(0);
    let Some(terminal) = record.terminal() else {
        return (Vec::new(), record.settlement);
    };
    if terminal.value() == &fsl_replay::TerminalValue::Refuted {
        let token = record
            .probabilistic_counterexample
            .as_ref()
            .expect("a native bounded refutation retains its reproduced path evidence");
        let evidence = token.evidence();
        assert_eq!(
            evidence.item,
            fsl_replay::probabilistic_claim_identity(claim).unwrap()
        );
        let fsl_replay::ProbabilityWitnessReplay::Reproduced(replayed) =
            fsl_replay::replay_probabilistic_witness(
                package,
                &system,
                &evidence.item,
                evidence,
                &exact,
                &cancel,
            )
            .expect("the retained native evidence independently replays")
        else {
            panic!("native refutation evidence must reproduce");
        };
        assert_eq!(replayed.mass(), token.mass());
    }
    let documents: Vec<&[u8]> = [
        terminal.exact_certificate_document(),
        terminal.exact_counterpart_document(),
    ]
    .into_iter()
    .flatten()
    .collect();
    if documents.is_empty() {
        return (Vec::new(), record.settlement);
    }
    let item = fsl_replay::probabilistic_claim_identity(claim).unwrap();
    let wire = |bytes: &[u8]| json!({
        "source": {"authority":identity.authority, "identity":identity.identity, "content":source},
        "package_name":"bounded-fixture",
        "item":{"node_id":fsl_foundation::node_wire::node_key_to_json(item.node()).unwrap(), "model":item.model().map(|model|model.as_str()), "ordinal":item.ordinal()},
        "byte_provision":[["test","bounded-certificate-source",source.as_bytes()]],
        "documents":[domain,snapshot],
        "initial":[{"authority":"test","identity":"s0"}],
        "universes":[{"population":{"package":universe_package,"node":universe_node},"keys":[universe_key]}],
        "certificate":String::from_utf8(bytes.to_vec()).unwrap(),
        "limits":{"bytes":1_000_000,"storage_bytes":8_000_000,"states":100,"edges":1000,"scalar_bits":128,"expansion":100_000,"stages":{}},
    });
    let requests = documents.into_iter().map(wire).collect();
    (requests, record.settlement)
}

pub(super) fn proof(source: &str, snapshot: &[u8]) -> Value {
    let (request, value) = request(source, snapshot, ExactProbLimits::default());
    request.unwrap_or_else(|| {
        panic!(
            "{}: bounded exact proof carried no document: {value:?}",
            source.lines().last().unwrap_or(source)
        )
    })
}

pub(super) fn kernel_proof(source: &str, snapshot: &[u8]) -> Value {
    let (request, settlement) =
        request_with_mode(source, snapshot, ExactProbLimits::default(), DOMAIN, true);
    request.unwrap_or_else(|| panic!("Kernel fixture carries no proof: {settlement:?}"))
}

fn document_limits(request: &Value) -> fsl_replay::ProbabilityDocumentLimits {
    fsl_replay::ProbabilityDocumentLimits {
        bytes: request["limits"]["bytes"].as_u64().unwrap(),
        storage_bytes: request["limits"]["storage_bytes"].as_u64().unwrap(),
        states: usize::try_from(request["limits"]["states"].as_u64().unwrap()).unwrap(),
        edges: usize::try_from(request["limits"]["edges"].as_u64().unwrap()).unwrap(),
        scalar_bits: request["limits"]["scalar_bits"].as_u64().unwrap(),
        accounting: fsl_replay::AccountingLimits::default().0,
    }
}

pub(super) fn check_kernel_document(
    request: &Value,
    cancel: &Cancel,
) -> Result<fsl_replay::ExactProbabilityCertificateCheck, fsl_replay::ProbabilityDocumentError> {
    let domain: Vec<u8> = serde_json::from_value(request["documents"][0].clone()).unwrap();
    let snapshot: Vec<u8> = serde_json::from_value(request["documents"][1].clone()).unwrap();
    let (checked, packages) =
        compile_kernel_source_with(request["source"]["content"].as_str().unwrap(), &domain);
    let initial = DocumentRef {
        authority: request["initial"][0]["authority"]
            .as_str()
            .unwrap()
            .to_owned(),
        identity: request["initial"][0]["identity"]
            .as_str()
            .unwrap()
            .to_owned(),
    };
    let snapshots = [(initial.clone(), snapshot)].into_iter().collect();
    let universe = &request["universes"][0];
    let stages = SpineLimits::default();
    let system = fsl_eval::simulation::model::ModelSystem::new(ModelSimulationRequest {
        input: fsl_eval::simulation::model::ModelCheckInput::source(checked.package()),
        packages: &packages,
        normalization: stages.model,
        snapshots: &snapshots,
        initial: vec![initial],
        universes: vec![PopulationUniverse {
            population: DeclarationKey {
                package: universe["population"]["package"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
                node: universe["population"]["node"].as_str().unwrap().to_owned(),
            },
            keys: universe["keys"]
                .as_array()
                .unwrap()
                .iter()
                .map(|key| key.as_str().unwrap().to_owned())
                .collect(),
        }],
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: request["limits"]["expansion"].as_u64().unwrap(),
        },
        meter: fsl_replay::AccountingLimits::default().0,
        emit: stages.package.emit(),
    })
    .unwrap();
    let claims = checked.package().graph().probabilistic_model().claims();
    let item = fsl_replay::probabilistic_claim_identity(
        checked
            .package()
            .graph()
            .probabilistic_model()
            .claim("Likely")
            .unwrap(),
    )
    .unwrap();
    fsl_replay::check_probability_certificate_document(
        &system,
        claims,
        &item,
        request["certificate"].as_str().unwrap().as_bytes(),
        document_limits(request),
        cancel,
    )
}

pub(super) fn run(request: &Value) -> (i32, Value) {
    run_with(request, &[])
}

pub(super) fn run_with(request: &Value, arguments: &[&str]) -> (i32, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_fsl-probability-check"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Err(error) = child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(request).unwrap())
    {
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }
    let output = child.wait_with_output().unwrap();
    let value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("child output: {output:?}"));
    (output.status.code().unwrap(), value)
}

pub(super) fn document(request: &Value) -> Value {
    serde_json::from_str(request["certificate"].as_str().unwrap()).unwrap()
}

pub(super) fn key(text: &Value) -> Value {
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(text.as_str().unwrap()).unwrap()).unwrap()
}

#[trace("TC-636", "FR-201-AC-14")]
#[test]
fn bounded_claim_over_is_a_real_s3_parameter_with_its_actual_model_type() {
    use fsl_semantics::check::{BodyTerm, LeafTerm, MemberTerm, NodeTag, Owner};
    let (checked, _) = compile_source(&cyclic_source(2, "0.75"));
    let package = checked.package();
    let claim = package
        .graph()
        .probabilistic_model()
        .claim("Likely")
        .unwrap();
    let parameter = claim.over.as_ref().unwrap().node().unwrap();
    let graph = package.graph().semantic_graph();
    let node = graph
        .node(parameter.clone())
        .expect("S3 retains the real parameter node");
    assert_eq!(node.node_tag(), NodeTag::Value);
    assert_eq!(node.semantic_form(), "parameter");
    assert_ne!(parameter, claim.occurrence().unwrap().node());
    let reference_type = graph.node(node.semantic_type().unwrap()).unwrap();
    assert_eq!(reference_type.semantic_form(), "reference");
    let BodyTerm::Aggregate(body) = reference_type.body() else {
        panic!("reference type has an aggregate body");
    };
    let [MemberTerm::Leaf(LeafTerm::Reference { target })] = body.members.as_slice() else {
        panic!("reference type names its model object");
    };
    let object = graph.node(target.admitted().unwrap().clone()).unwrap();
    let Some(Owner::Model(owner)) = object.owner() else {
        panic!("object type keeps its actual model owner");
    };
    assert_eq!(owner.identity(), "test/link");
    assert_eq!(owner.node(), "ix://test/link/Msg");
    let root = graph
        .node(claim.occurrence().unwrap().node().clone())
        .unwrap();
    assert!(matches!(root.body(), BodyTerm::Aggregate(body) if body.members.is_empty()));
}

pub(super) fn mutate(request: &Value, change: impl FnOnce(&mut Value)) -> Value {
    let mut changed = request.clone();
    let mut doc = document(request);
    change(&mut doc);
    changed["certificate"] = json!(serde_json::to_string(&doc).unwrap());
    changed
}

pub(super) fn alter_keys(document: &mut Value, change: impl Fn(&mut Value)) {
    let alter = |text: &mut Value| {
        let mut state = key(text);
        change(&mut state);
        *text = json!(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&state).unwrap()));
    };
    for member in ["values", "ranking"] {
        let rows = document[member].as_array_mut().unwrap();
        for row in rows.iter_mut() {
            alter(&mut row["state"]);
        }
        rows.sort_by_key(|row| {
            URL_SAFE_NO_PAD
                .decode(row["state"].as_str().unwrap())
                .unwrap()
        });
    }
    for member in ["targets", "failures"] {
        if let Some(targets) = document["objective"][member].as_array_mut() {
            for target in targets.iter_mut() {
                alter(target);
            }
            targets.sort_by_key(|target| URL_SAFE_NO_PAD.decode(target.as_str().unwrap()).unwrap());
        }
    }
}

#[trace("TC-636", "FR-201-AC-12", "FR-282-AC-13")]
#[test]
fn bounded_deliver_exact_minimum_maximum_and_workload_cross_the_process_boundary() {
    for (scheduler, operator, threshold, beyond, numerator, denominator, loss_n, loss_d) in [
        (
            "every scheduler",
            ">=",
            "0.96",
            "0.97",
            "24",
            "25",
            "4",
            "5",
        ),
        (
            "every scheduler",
            "<=",
            "0.99",
            "0.98",
            "99",
            "100",
            "9",
            "10",
        ),
        ("Even", ">=", "0.9775", "0.98", "391", "400", "17", "20"),
    ] {
        let source = format!(
            "{}{}",
            model_source(),
            claim(scheduler, operator, threshold, 2)
        );
        let request = proof(&source, SNAPSHOT);
        let (code, result) = run(&request);
        assert_eq!(code, 0, "{scheduler}: {result}");
        assert_eq!(
            result["value"],
            json!({"numerator":numerator,"denominator":denominator})
        );
        let doc = document(&request);
        let loss = doc["ranking"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["rank"] == 1)
            .unwrap();
        let value = doc["values"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["state"] == loss["state"])
            .unwrap();
        assert_eq!(
            value["value"],
            json!({"numerator":loss_n,"denominator":loss_d})
        );
        let mut stronger = request.clone();
        let source = source.replace(threshold, beyond);
        stronger["source"]["content"] = json!(source);
        stronger["byte_provision"][0][2] = json!(source.as_bytes());
        // The stronger bound is a different checked claim. Keep its actual
        // identity and monitor context while testing the same exact values.
        let (checked, _) = compile_source(&source);
        let claim = checked
            .package()
            .graph()
            .probabilistic_model()
            .claim("Likely")
            .unwrap();
        let item = fsl_replay::probabilistic_claim_identity(claim).unwrap();
        let identity = json!({"node_id":fsl_foundation::node_wire::node_key_to_json(item.node()).unwrap(), "model":item.model().map(|model| model.as_str()), "ordinal":item.ordinal()});
        stronger["item"] = identity.clone();
        let context = fsl_eval::automaton::StateContext {
            root: fsl_eval::automaton::ContextRoot::Clause(claim.occurrence().unwrap().clone()),
            profile: fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
                .unwrap(),
            activation: fsl_eval::automaton::ContextActivation::Origin,
            binding: vec![fsl_eval::automaton::ContextBinding::instance(
                claim.over.as_ref().unwrap().node().unwrap().clone(),
                stronger["universes"][0]["keys"][0].as_str().unwrap(),
            )],
        };
        let fsl_semantics::check::ProbForm::Probability {
            event: fsl_semantics::check::Event::Formula { formula, .. },
            ..
        } = &claim.form
        else {
            panic!("a bounded formula claim")
        };
        let monitor =
            fsl_eval::automaton::QualifiedBoundedMonitor::new(formula.formula(), &context, 100)
                .unwrap();
        let actual: Value =
            serde_json::from_slice(monitor.decision_key(true).unwrap().as_bytes()).unwrap();
        let limits = fsl_replay::ProbabilityDocumentLimits {
            bytes: stronger["limits"]["bytes"].as_u64().unwrap(),
            storage_bytes: stronger["limits"]["storage_bytes"].as_u64().unwrap(),
            states: usize::try_from(stronger["limits"]["states"].as_u64().unwrap()).unwrap(),
            edges: usize::try_from(stronger["limits"]["edges"].as_u64().unwrap()).unwrap(),
            scalar_bits: stronger["limits"]["scalar_bits"].as_u64().unwrap(),
            accounting: fsl_replay::AccountingLimits::default().0,
        };
        let cancel = Cancel::new();
        let mut doc = fsl_replay::read_probability_certificate(
            stronger["certificate"].as_str().unwrap().as_bytes(),
            limits,
            &cancel,
        )
        .unwrap();
        doc.identity = item;
        let rebind = |key: &mut fsl_replay::ProbabilityProductKey| {
            let mut state: Value = serde_json::from_slice(key.as_bytes()).unwrap();
            state[5][1][1] = actual[1].clone();
            *key = fsl_replay::ProbabilityProductKey::from_canonical_bytes(
                fsl_foundation::canonical_json::encode(&state).unwrap(),
            );
        };
        for row in &mut doc.values {
            rebind(&mut row.state);
        }
        for (key, _) in &mut doc.ranking {
            rebind(key);
        }
        for key in &mut doc.targets {
            rebind(key);
        }
        doc.values
            .sort_by(|left, right| left.state.cmp(&right.state));
        doc.ranking.sort_by(|left, right| left.0.cmp(&right.0));
        doc.targets.sort();
        let bytes = fsl_replay::write_probability_certificate(&doc, limits, &cancel).unwrap();
        fsl_replay::read_probability_certificate(&bytes, limits, &cancel)
            .expect("the rebound certificate is canonical before checking its threshold");
        stronger["certificate"] = json!(String::from_utf8(bytes).unwrap());
        let (code, result) = run(&stronger);
        assert_eq!(code, 1, "{result}");
        assert_eq!(result["rule"], "threshold-missed");
    }
}

fn cyclic_source(horizon: u64, threshold: &str) -> String {
    let model = model_source()
        .replace(" and self.attempts < 2", "")
        .replace(
            "self.attempts - pre(self.attempts) = 1",
            "self.attempts = pre(self.attempts)",
        )
        .replace("true: 1, false: 9", "true: 1, false: 1")
        .replace("true: 1, false: 4", "true: 1, false: 1")
        .replace("self.delivered or self.attempts = 2", "self.delivered");
    format!(
        "{model}{}",
        claim("every scheduler", ">=", threshold, horizon)
    )
}

fn always_source(scheduler: &str, operator: &str, threshold: &str, horizon: u64) -> String {
    cyclic_source(horizon, threshold)
        .replace("true: 1, false: 1", "true: 9, false: 1")
        .replace("under every scheduler", &format!("under {scheduler}"))
        .replace(
            &format!("probability >= {threshold}"),
            &format!("probability {operator} {threshold}"),
        )
        .replace("eventually[", "always[")
        .replace("holds(m.delivered)", "holds(not m.delivered)")
        .lines()
        .filter(|line| !line.starts_with("reward "))
        .collect::<Vec<_>>()
        .join("\n")
}

#[trace("TC-636", "FR-201-AC-56", "FSpec-FR-412-AC-21")]
#[test]
fn bounded_always_health_has_direct_accepting_values_and_real_remaining_ranks() {
    for terminal_fault in [false, true] {
        let mut source = always_source("Even", ">=", "0.75", 2);
        if !terminal_fault {
            source = source.replace("when { self.delivered }", "when { false }");
        }
        let request = proof(&source, SNAPSHOT);
        let (code, result) = run(&request);
        assert_eq!(code, 0, "{result}");
        assert_eq!(
            result["value"],
            json!({"numerator":"81","denominator":"100"})
        );
        let doc = document(&request);
        assert_eq!(doc.as_object().unwrap().len(), 5);
        assert!(doc.get("policy").is_none());
        assert_eq!(
            doc["objective"]["profile"],
            "filament.probability-objective/reachability/v1"
        );
        assert_eq!(doc["objective"]["fairness"], json!([]));
        for (rank, expected) in [(2, ("81", "100")), (1, ("9", "10"))] {
            let row = doc["ranking"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["rank"] == rank)
                .unwrap();
            let value = doc["values"]
                .as_array()
                .unwrap()
                .iter()
                .find(|value| value["state"] == row["state"])
                .unwrap();
            assert_eq!(
                value["value"],
                json!({"numerator":expected.0,"denominator":expected.1})
            );
            assert_eq!(key(&row["state"])[5][1][2][1][1], json!(rank.to_string()));
        }
        for row in doc["ranking"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["rank"] == 0)
        {
            let state = key(&row["state"]);
            let accepting = state[5][1][2] == json!(["accept"]);
            assert!(accepting || state[5][1][2] == json!(["reject"]));
            let value = doc["values"]
                .as_array()
                .unwrap()
                .iter()
                .find(|value| value["state"] == row["state"])
                .unwrap();
            assert_eq!(
                value["value"],
                json!({"numerator":if accepting {"1"} else {"0"},"denominator":"1"})
            );
            assert_eq!(
                doc["objective"]["targets"]
                    .as_array()
                    .unwrap()
                    .contains(&row["state"]),
                accepting
            );
        }
        let changed = mutate(&request, |doc| {
            let initial = doc["ranking"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["rank"] == 2)
                .unwrap()["state"]
                .clone();
            doc["values"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|row| row["state"] == initial)
                .unwrap()["value"] = json!({"numerator":"9","denominator":"10"});
        });
        let (code, result) = run(&changed);
        assert_eq!(code, 1, "{result}");
        assert_eq!(result["rule"], "inequality-fails");
    }
}

#[trace(
    "TC-636",
    "FR-201-AC-56",
    "FSpec-FR-412-AC-21",
    "FSpec-FR-412-AC-22",
    "FSpec-FR-412-AC-23"
)]
#[test]
fn bounded_always_extrema_workload_and_strict_policy_absence_are_independent() {
    for (scheduler, operator, threshold, numerator, denominator, optimization) in [
        ("every scheduler", ">=", "0.25", "1", "4", "minimum"),
        ("every scheduler", "<=", "0.5625", "9", "16", "maximum"),
        ("Even", ">=", "0.390625", "25", "64", "workload"),
        ("Even", "<=", "0.390625", "25", "64", "workload"),
    ] {
        let source = always_source(scheduler, operator, threshold, 2)
            .replacen("true: 9, false: 1", "true: 1, false: 1", 1)
            .replacen("true: 9, false: 1", "true: 3, false: 1", 1);
        let request = proof(&source, SNAPSHOT);
        let (code, result) = run(&request);
        assert_eq!(code, 0, "{result}");
        assert_eq!(
            result["value"],
            json!({"numerator":numerator,"denominator":denominator})
        );
        assert_eq!(
            document(&request)["objective"]["optimization"],
            optimization
        );
        let dual_operator = if operator == ">=" { "<=" } else { ">=" };
        let dual_threshold = if dual_operator == "<=" {
            "0.875"
        } else {
            "0.375"
        };
        let dual = source
            .replace("always[", "eventually[")
            .replace("holds(not m.delivered)", "holds(m.delivered)")
            .replace(
                &format!("probability {operator} {threshold}"),
                &format!("probability {dual_operator} {dual_threshold}"),
            );
        let (dual_code, dual_result) = run(&proof(&dual, SNAPSHOT));
        assert_eq!(dual_code, 0, "{dual_result}");
        let complement = match optimization {
            "minimum" => json!({"numerator":"3","denominator":"4"}),
            "maximum" => json!({"numerator":"7","denominator":"16"}),
            "workload" => json!({"numerator":"39","denominator":"64"}),
            _ => unreachable!("closed test cases"),
        };
        assert_eq!(
            dual_result["value"], complement,
            "Always exchanges extrema under complementation"
        );
        for policy in [
            json!([]),
            json!([{"choice":"send_a"}]),
            json!([{"choice":"send_b"}]),
        ] {
            let changed = mutate(&request, |doc| doc["policy"] = policy);
            let (code, result) = run(&changed);
            assert_eq!(code, 1, "{result}");
            assert_eq!(result["rule"], "malformed");
            assert_eq!(result["at"], json!({"member":"policy"}));
        }
        let missing = mutate(&request, |doc| {
            doc["ranking"]
                .as_array_mut()
                .unwrap()
                .retain(|row| row["rank"] != 2);
        });
        let (code, result) = run(&missing);
        assert_eq!(code, 1, "{result}");
        assert_eq!(result["rule"], "malformed");
        assert_eq!(result["at"], json!({"member":"ranking"}));
        if scheduler == "every scheduler" {
            let mut crossing = request.clone();
            let opposite = if operator == ">=" { "<=" } else { ">=" };
            let source = source.replace(
                &format!("probability {operator}"),
                &format!("probability {opposite}"),
            );
            crossing["source"]["content"] = json!(source);
            crossing["byte_provision"][0][2] = json!(source.as_bytes());
            let (code, result) = run(&crossing);
            assert_ne!(
                code, 0,
                "crossing objective cannot establish this claim: {result}"
            );
        }
    }
}

#[trace("TC-636", "FR-201-AC-56", "FR-201-AC-14")]
#[test]
fn bounded_always_endpoints_terminal_keys_ranks_and_controls_are_checked() {
    for (horizon, delivered, terminal, operator) in [
        (0, false, false, ">="),
        (0, true, false, "<="),
        (2, false, true, "<="),
    ] {
        let mut source = always_source("every scheduler", operator, "0.5", horizon);
        if terminal {
            source = source
                .replace("{ not self.delivered }", "{ false }")
                .replace("when { self.delivered }", "when { true }");
        }
        let mut snapshot: Value = serde_json::from_slice(SNAPSHOT).unwrap();
        snapshot["populations"][0]["objects"][0]["fields"]["delivered"]["boolean"] =
            json!(delivered);
        let request = proof(&source, &serde_json::to_vec(&snapshot).unwrap());
        let (code, result) = run(&request);
        assert_eq!(code, 0, "{result}");
        let numerator = if delivered || terminal { "0" } else { "1" };
        assert_eq!(
            result["value"],
            json!({"numerator":numerator,"denominator":"1"})
        );
        let mut ranks: Vec<_> = document(&request)["ranking"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["rank"].as_u64().unwrap())
            .collect();
        ranks.sort();
        assert_eq!(ranks, if terminal { vec![0, 2] } else { vec![0] });
    }
    let original = proof(&always_source("Even", ">=", "0.75", 2), SNAPSHOT);
    for alteration in [
        "rank", "horizon", "binding", "local-id", "absent", "targets",
    ] {
        let changed = mutate(&original, |doc| {
            if alteration == "rank" {
                doc["ranking"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|row| row["rank"] == 2)
                    .unwrap()["rank"] = json!(3);
            } else if alteration == "targets" {
                doc["objective"]["targets"] = json!([]);
            } else {
                alter_keys(doc, |state| {
                    if state[5][1][2][0] == "bounded" {
                        match alteration {
                            "horizon" => state[5][1][2][1][1] = json!("9"),
                            "binding" => state[5][1][1][3] = json!([]),
                            "local-id" => state[5][1] = json!(1),
                            "absent" => state[5] = Value::Null,
                            _ => unreachable!("closed test cases"),
                        }
                    }
                });
            }
        });
        let (code, result) = run(&changed);
        assert_ne!(code, 0, "{alteration}: {result}");
        if alteration == "rank" {
            assert_eq!(result["rule"], "rank-broken");
        }
    }
    for budget in [
        "bytes",
        "storage_bytes",
        "states",
        "edges",
        "scalar_bits",
        "expansion",
    ] {
        let mut request = original.clone();
        request["limits"][budget] = json!(0);
        let (code, result) = run(&request);
        assert_ne!(code, 0, "{budget}: {result}");
        assert!(result.get("value").is_none(), "{budget}: {result}");
    }
    let (code, result) = run_with(&original, &["--cancel"]);
    assert_ne!(code, 0, "{result}");
    assert!(result.get("value").is_none());
    let fair = always_source("every scheduler fair strong Msg::send_a", ">=", "0.75", 2);
    let (certificate, settlement) = request(&fair, SNAPSHOT, ExactProbLimits::default());
    assert!(certificate.is_none());
    assert!(
        !matches!(settlement, Settlement::Terminal(record) if matches!(record.value(), fsl_replay::TerminalValue::Proved { .. }))
    );
}

#[trace("TC-636", "FR-201-AC-13", "FR-282-AC-13")]
#[test]
fn bounded_raw_cycle_has_distinct_real_horizons_and_exact_ranks() {
    let request = proof(&cyclic_source(2, "0.75"), SNAPSHOT);
    let (code, result) = run(&request);
    assert_eq!(code, 0, "{result}");
    assert_eq!(result["value"], json!({"numerator":"3","denominator":"4"}));
    let doc = document(&request);
    let running: Vec<_> = doc["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| key(&row["state"])[5][1][2][0] == "bounded")
        .collect();
    assert_eq!(running.len(), 2);
    let first = key(&running[0]["state"]);
    let second = key(&running[1]["state"]);
    assert_eq!(
        first[4], second[4],
        "raw model state is the same on both visits"
    );
    assert_ne!(first[5], second[5]);
    let mut ranks: Vec<_> = running
        .iter()
        .map(|row| row["rank"].as_u64().unwrap())
        .collect();
    ranks.sort();
    assert_eq!(ranks, vec![1, 2]);
}

#[trace("TC-636", "FR-201-AC-13", "FR-282-AC-13")]
#[test]
fn bounded_zero_horizon_reads_position_zero_and_terminal_closes_by_false_extension() {
    for delivered in [false, true] {
        let mut snapshot: Value = serde_json::from_slice(SNAPSHOT).unwrap();
        snapshot["populations"][0]["objects"][0]["fields"]["delivered"]["boolean"] =
            json!(delivered);
        let source = if delivered {
            cyclic_source(0, "0.5")
        } else {
            cyclic_source(0, "0.5").replace("probability >= 0.5", "probability <= 0.5")
        };
        let mut request = proof(&source, &serde_json::to_vec(&snapshot).unwrap());
        request["limits"]["states"] = json!(1);
        let (code, result) = run(&request);
        assert_eq!(code, 0, "{result}");
        assert_eq!(
            result["value"],
            json!({"numerator":if delivered {"1"} else {"0"},"denominator":"1"})
        );
    }
    let source = cyclic_source(2, "0.5")
        .replace("when { self.delivered }", "when { true }")
        .replace("{ not self.delivered }", "{ false }")
        .replace("probability >= 0.5", "probability <= 0.5");
    let request = proof(&source, SNAPSHOT);
    let doc = document(&request);
    let mut ranks: Vec<_> = doc["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["rank"].as_u64().unwrap())
        .collect();
    ranks.sort();
    assert_eq!(
        ranks,
        vec![0, 2],
        "terminal closure keeps real debt rather than graph height"
    );
    let (code, result) = run(&request);
    assert_eq!(code, 0, "{result}");
    assert_eq!(result["value"], json!({"numerator":"0","denominator":"1"}));
}

#[trace(
    "TC-636",
    "FR-201-AC-13",
    "FR-201-AC-14",
    "FR-282-AC-12",
    "FR-282-AC-13"
)]
#[test]
fn bounded_graph_and_document_budget_stops_retain_limits_without_truth_or_evidence() {
    let original = proof(&cyclic_source(2, "0.75"), SNAPSHOT);
    for name in [
        "bytes",
        "storage_bytes",
        "states",
        "edges",
        "scalar_bits",
        "expansion",
    ] {
        let mut request = original.clone();
        request["limits"][name] = json!(0);
        let (code, result) = run(&request);
        assert_ne!(code, 0, "{name}: {result}");
        assert!(result.get("value").is_none(), "{name}: {result}");
    }
    let count = document(&original)["values"].as_array().unwrap().len();
    let mut exact = original.clone();
    exact["limits"]["states"] = json!(count);
    let (code, result) = run(&exact);
    assert_eq!(
        code, 0,
        "the exact product state ceiling suffices: {result}"
    );
    let mut limited = mutate(&original, |document| {
        document["values"].as_array_mut().unwrap().pop();
        document["ranking"].as_array_mut().unwrap().pop();
    });
    limited["limits"]["states"] = json!(count - 1);
    let (code, result) = run(&limited);
    assert_ne!(code, 0, "{result}");
    assert_eq!(result["limit"]["setting"], "exact_prob.max_states");
    assert_eq!(result["limit"]["bound"], json!(count - 1));
    assert_eq!(result["limit"]["actual"], json!(count));
    for arguments in [vec!["--cancel"], vec!["--max-input-bytes", "1"]] {
        let (code, result) = run_with(&original, &arguments);
        assert_ne!(code, 0, "{result}");
        assert!(result.get("value").is_none(), "{result}");
    }
}

#[trace("TC-636", "FR-201-AC-14", "FR-282-AC-13")]
#[test]
fn bounded_altered_horizon_binding_and_rank_never_certify_the_original_claim() {
    let request = proof(&cyclic_source(2, "0.75"), SNAPSHOT);
    let changed = mutate(&request, |doc| {
        let row = doc["ranking"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["rank"] == 2)
            .unwrap();
        row["rank"] = json!(3);
    });
    let (code, result) = run(&changed);
    assert_eq!(code, 1, "{result}");
    assert_eq!(result["rule"], "rank-broken");
    for alteration in ["horizon", "binding", "local-id", "absent"] {
        let changed = mutate(&request, |doc| {
            alter_keys(doc, |state| {
                if state[5][1][2][0] == "bounded" {
                    match alteration {
                        "binding" => state[5][1][1][3] = json!([]),
                        "horizon" => state[5][1][2][1][1] = json!("9"),
                        "local-id" => state[5][1] = json!(1),
                        "absent" => state[5] = Value::Null,
                        _ => unreachable!("closed test cases"),
                    }
                }
            });
        });
        let (code, result) = run(&changed);
        assert_ne!(code, 0, "{result}");
    }
}

#[trace("TC-636", "FR-201-AC-14", "FR-282-AC-13")]
#[test]
fn bounded_wrong_source_and_nonexact_rows_never_produce_an_accepted_document() {
    let (evidence, missed) = request(
        &cyclic_source(2, "0.9"),
        SNAPSHOT,
        ExactProbLimits::default(),
    );
    assert!(
        evidence.is_none(),
        "a replayed bounded refutation is not a proof certificate document"
    );
    assert!(
        matches!(&missed, Settlement::Terminal(terminal) if terminal.value() == &fsl_replay::TerminalValue::Refuted),
        "{missed:?}"
    );
    let mut original = proof(&cyclic_source(2, "0.75"), SNAPSHOT);
    original["package_name"] = json!("another-source-package");
    let (code, result) = run(&original);
    assert_ne!(code, 0, "{result}");
    for (threshold, precision, maximum, incomplete) in
        [("0.5", 64, 64, false), ("0.75", 1, 1, true)]
    {
        let limits = ExactProbLimits {
            max_rational_bits: 1,
            precision_bits: precision,
            max_precision_bits: maximum,
            ..ExactProbLimits::default()
        };
        let (document, terminal) = request(&cyclic_source(2, threshold), SNAPSHOT, limits);
        if !incomplete {
            // A decided interval proves with FR-1042's value-bounds documents, which
            // are never an `exact` one.
            let evidence = document.expect("a decided interval carries its documents");
            assert_eq!(self::document(&evidence)["kind"], "lower");
            assert!(
                matches!(&terminal, Settlement::Terminal(record) if matches!(record.value(), fsl_replay::TerminalValue::Proved { basis: fsl_replay::ProofBasis::ValueBounds { .. }, .. })),
                "{terminal:?}"
            );
            continue;
        }
        assert!(
            document.is_none(),
            "a precision-budget row never becomes evidence"
        );
        assert!(
            matches!(&terminal, Settlement::Terminal(record) if matches!(record.value(), fsl_replay::TerminalValue::Incomplete(fsl_replay::IncompleteCause::PrecisionBudget { .. }))),
            "{terminal:?}"
        );
    }
}

/// Keep the offered evidence while naming the actual edited checked claim.
/// The canonical writer preserves the profile and all numerical assertions.
pub(super) fn replace_kernel_source_and_rebind(request: &mut Value, source: &str) {
    replace_source_and_rebind_with_mode(
        request,
        source,
        Some(filament_semantic_value::checking::CheckMode::Kernel),
    );
}

pub(super) fn replace_source_and_rebind(request: &mut Value, source: &str) {
    replace_source_and_rebind_with_mode(request, source, None);
}

fn replace_source_and_rebind_with_mode(
    request: &mut Value,
    source: &str,
    check_mode: Option<filament_semantic_value::checking::CheckMode>,
) {
    let previous_source = request["source"]["content"].as_str().unwrap().to_owned();
    let domain: Vec<u8> = serde_json::from_value(request["documents"][0].clone()).unwrap();
    let (checked, packages) = compile_with_mode(source, &domain, check_mode);
    let claim = checked
        .package()
        .graph()
        .probabilistic_model()
        .claim("Likely")
        .unwrap();
    let item = fsl_replay::probabilistic_claim_identity(claim).unwrap();
    let limits = fsl_replay::ProbabilityDocumentLimits {
        bytes: request["limits"]["bytes"].as_u64().unwrap(),
        storage_bytes: request["limits"]["storage_bytes"].as_u64().unwrap(),
        states: usize::try_from(request["limits"]["states"].as_u64().unwrap()).unwrap(),
        edges: usize::try_from(request["limits"]["edges"].as_u64().unwrap()).unwrap(),
        scalar_bits: request["limits"]["scalar_bits"].as_u64().unwrap(),
        accounting: fsl_replay::AccountingLimits::default().0,
    };
    let cancel = Cancel::new();
    let mut proof = fsl_replay::read_probability_proof_document(
        request["certificate"].as_str().unwrap().as_bytes(),
        limits,
        &cancel,
    )
    .unwrap();
    let document = proof.document_mut();
    document.identity = item.clone();
    if let fsl_semantics::check::ProbForm::Probability {
        event: fsl_semantics::check::Event::Formula { formula, .. },
        ..
    } = &claim.form
    {
        let (previous, _) = compile_with_mode(&previous_source, &domain, check_mode);
        let previous_claim = previous
            .package()
            .graph()
            .probabilistic_model()
            .claim("Likely")
            .unwrap();
        let fsl_semantics::check::ProbForm::Probability {
            event:
                fsl_semantics::check::Event::Formula {
                    formula: previous_formula,
                    ..
                },
            ..
        } = &previous_claim.form
        else {
            panic!("the offered evidence names a bounded formula");
        };
        let initial = DocumentRef {
            authority: request["initial"][0]["authority"]
                .as_str()
                .unwrap()
                .to_owned(),
            identity: request["initial"][0]["identity"]
                .as_str()
                .unwrap()
                .to_owned(),
        };
        let snapshot: Vec<u8> = serde_json::from_value(request["documents"][1].clone()).unwrap();
        let snapshots = [(initial.clone(), snapshot)].into_iter().collect();
        let universe = &request["universes"][0];
        let simulation = ModelSimulationRequest {
            input: fsl_eval::simulation::model::ModelCheckInput::source(checked.package()),
            packages: &packages,
            normalization: SpineLimits::default().model,
            snapshots: &snapshots,
            initial: vec![initial],
            universes: vec![PopulationUniverse {
                population: DeclarationKey {
                    package: universe["population"]["package"]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                    node: universe["population"]["node"].as_str().unwrap().to_owned(),
                },
                keys: universe["keys"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|key| key.as_str().unwrap().to_owned())
                    .collect(),
            }],
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: request["limits"]["expansion"].as_u64().unwrap(),
            },
            meter: limits.accounting,
            emit: SpineLimits::default().package.emit(),
        };
        let system = fsl_eval::simulation::model::ModelSystem::new(simulation).unwrap();
        let binding = fsl_replay::claim_binding(&system, claim).unwrap().unwrap();
        let previous_context = fsl_replay::probabilistic_formula_monitor_context(
            previous_claim,
            Some(&binding.reference),
        )
        .unwrap();
        let context =
            fsl_replay::probabilistic_formula_monitor_context(claim, Some(&binding.reference))
                .unwrap();
        let mut previous_monitor = fsl_eval::automaton::QualifiedBoundedMonitor::new(
            previous_formula.formula(),
            &previous_context,
            u64::try_from(limits.states).unwrap(),
        )
        .unwrap();
        let mut monitor = fsl_eval::automaton::QualifiedBoundedMonitor::new(
            formula.formula(),
            &context,
            u64::try_from(limits.states).unwrap(),
        )
        .unwrap();
        let mut keys = std::collections::BTreeMap::new();
        for accepted in [false, true] {
            keys.insert(
                previous_monitor.decision_key(accepted).unwrap(),
                monitor.decision_key(accepted).unwrap(),
            );
        }
        let mut previous_state = previous_monitor.start(false).unwrap();
        let mut state = monitor.start(false).unwrap();
        loop {
            keys.insert(
                previous_monitor.key(previous_state).unwrap(),
                monitor.key(state).unwrap(),
            );
            if !matches!(
                previous_monitor.state(previous_state).unwrap(),
                fsl_eval::automaton::BoundedState::Open { .. }
            ) {
                break;
            }
            previous_state = previous_monitor.step(previous_state, false).unwrap();
            state = monitor.step(state, false).unwrap();
        }
        let io = fsl_replay::ProbabilityDocumentIoLimits {
            bytes: limits.bytes,
            storage_bytes: limits.storage_bytes,
        };
        let rebind = |key: &mut fsl_replay::ProbabilityProductKey| {
            let encoded: Value = serde_json::from_slice(key.as_bytes()).unwrap();
            let old = fsl_eval::automaton::AutomatonStateKey::imported(
                fsl_foundation::canonical_json::encode(&encoded[5][1]).unwrap(),
            );
            let monitor = keys
                .get(&old)
                .expect("the old key is reached by the actual checked monitor");
            let model = fsl_foundation::canonical_json::encode(&encoded[4][1]).unwrap();
            *key = fsl_replay::ProbabilityProductKey::for_formula_state(
                Some(&binding.reference),
                &model,
                monitor,
                io,
                &cancel,
            )
            .unwrap();
        };
        for row in &mut document.values {
            rebind(&mut row.state);
        }
        for (key, _) in &mut document.ranking {
            rebind(key);
        }
        for key in &mut document.targets {
            rebind(key);
        }
        for key in &mut document.failures {
            rebind(key);
        }
        document.values.sort_by(|a, b| a.state.cmp(&b.state));
        document.ranking.sort_by(|a, b| a.0.cmp(&b.0));
        document.targets.sort();
        document.failures.sort();
    }
    let bytes = fsl_replay::write_probability_proof_document(&proof, limits, &cancel).unwrap();
    request["source"]["content"] = json!(source);
    request["byte_provision"][0][2] = json!(source.as_bytes());
    request["item"] = json!({"node_id": fsl_foundation::node_wire::node_key_to_json(item.node()).unwrap(), "model": item.model().map(|model| model.as_str()), "ordinal": item.ordinal()});
    request["certificate"] = json!(String::from_utf8(bytes).unwrap());
}

#[trace("TC-3195", "FR-201-AC-57")]
#[test]
fn an_unwindowed_atom_claim_is_checked_exactly_as_eventually_at_horizon_zero() {
    // FR-201-AC-57's `Coin`: one coin, tails unless the snapshot says heads.
    for (heads, operator, value, targets) in [(false, "<=", "0", 0), (true, ">=", "1", 1)] {
        let mut snapshot: Value = serde_json::from_slice(COIN_SNAPSHOT).unwrap();
        snapshot["populations"][0]["objects"][0]["fields"]["heads"]["boolean"] = json!(heads);
        let snapshot = serde_json::to_vec(&snapshot).unwrap();
        let mut documents = Vec::new();
        for event in ["holds(c.heads)", "eventually[0,0] holds(c.heads)"] {
            let source = format!(
                "{}probabilistic Likely using inf on M over (c: M::Coin) under every scheduler on origin {{ probability {operator} 0.5 [ {event} ] }}\n",
                COIN_SOURCE.replace(
                    "filament.temporal.infinite-trace/v1",
                    "filament.temporal.event-position.false-extension/v1",
                )
            );
            let (request, settlement) =
                request_with(&source, &snapshot, ExactProbLimits::default(), COIN_DOMAIN);
            let request = request.unwrap_or_else(|| panic!("{event}: {settlement:?}"));
            let (code, result) = run(&request);
            assert_eq!(code, 0, "{event}: {result}");
            assert_eq!(
                result["value"],
                json!({"numerator": value, "denominator": "1"})
            );
            let doc = document(&request);
            assert_eq!(
                doc["objective"]["profile"],
                "filament.probability-objective/reachability/v1"
            );
            assert_eq!(
                doc["objective"]["targets"].as_array().unwrap().len(),
                targets
            );
            assert_eq!(doc["values"].as_array().unwrap().len(), 1);
            assert_eq!(doc["ranking"][0]["rank"], 0);
            let changed = mutate(&request, |doc| {
                doc["values"][0]["value"] = json!({"numerator":"1","denominator":"2"})
            });
            let (code, rejected) = run(&changed);
            assert_eq!(code, 1, "{event}: {rejected}");
            assert_eq!(rejected["rule"], "inequality-fails");
            assert_eq!(rejected["at"]["product_key"], doc["values"][0]["state"]);
            let values: Vec<Value> = doc["values"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["value"].clone())
                .collect();
            let ranks: Vec<Value> = doc["ranking"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["rank"].clone())
                .collect();
            documents.push((
                doc["objective"]["optimization"].clone(),
                doc["objective"]["targets"].as_array().unwrap().len(),
                values,
                ranks,
            ));
        }
        assert_eq!(
            documents[0], documents[1],
            "the two spellings certify identically"
        );
    }
}

/// FR-1042-AC-6 and FR-203-AC-1: past `max_rational_bits` the claim-side `lower`
/// document and its `upper` counterpart are each accepted by the separate
/// checker process, under the scalar bound the request's own limits give them
/// and no wider.
#[trace("TC-636", "FR-1042-AC-6")]
#[trace("TC-636", "FR-203-AC-1")]
#[test]
fn value_bounds_documents_are_accepted_by_the_separate_checker_under_the_request_limits() {
    let exact = ExactProbLimits {
        max_rational_bits: 4,
        ..ExactProbLimits::default()
    };
    let scalar_bits = fsl_replay::ProbabilityDocumentLimits::for_exact(
        &exact,
        fsl_replay::AccountingLimits::default().0,
    )
    .scalar_bits;
    let (mut requests, settlement) = requests_with_mode(
        &always_source("Even", ">=", "0.75", 2),
        SNAPSHOT,
        exact,
        DOMAIN,
        false,
    );
    assert_eq!(requests.len(), 2, "{settlement:?}");
    let kinds: Vec<String> = requests
        .iter()
        .map(|request| document(request)["kind"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(kinds, ["lower", "upper"]);
    for request in &mut requests {
        request["limits"]["scalar_bits"] = json!(scalar_bits);
        let (code, result) = run(request);
        assert_eq!(code, 0, "{result}");
    }
}
