// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-produced model witnesses use actual simulated states on the native wire.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use filament_checked::key::DeclarationKey;
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelCheckInput, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use fsl_eval::simulation::{StateKey, TransitionSystem};
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{
    read_native_run_result, run_clause, write_clause_run_result, write_clause_run_result_for,
    ClauseRunRequest, ClauseRunSelection, NativeRunResult, OverObject, TemporalRecord, TraceInput,
};
use fsl_replay::ModelStep;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::support::config_version_spine::{spine, Case};

const PACKAGE: &str = "example/config-version";
const POPULATION: &str = "ix://example/config-version/config_history";
const OBJECT_TYPE: &str = "ix://example/config-version/ConfigVersion";

fn source(file: &str) -> Value {
    json!({"file": file, "authority": "test", "identity": file,
        "document": "ModelNative", "formal_revision": 1})
}

fn unit(body: &str, activation: &str, advances: bool) -> String {
    let post = if advances {
        "pre(self.versionNumber) < 3 and self.versionNumber = pre(self.versionNumber) + 1"
    } else {
        "false"
    };
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile t = \"filament.temporal.infinite-trace/v1\";\n\
         model Config = \"{PACKAGE}\";\n\
         post Bump using v on Config::ConfigVersion::attemptUpdate {{ {post} }}\n\
         temporal Low using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         {activation} {{ {body} }}\n"
    )
}

struct Fixture {
    directory: tempfile::TempDir,
    request: ClauseRunRequest,
    steps: Vec<ModelStep>,
    state_keys: Vec<StateKey>,
}

fn model_system<'a>(
    package: &'a fsl_package::CheckedPackage,
    request: &'a ClauseRunRequest,
    missing_b: bool,
) -> ModelSystem<'a> {
    let initial = request.snapshots.keys().next().unwrap().clone();
    ModelSystem::new(ModelSimulationRequest {
        input: ModelCheckInput::source(package),
        packages: &request.packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: &request.snapshots,
        initial: vec![initial],
        universes: vec![PopulationUniverse {
            population: DeclarationKey {
                package: PACKAGE.to_owned(),
                node: POPULATION.to_owned(),
            },
            keys: if missing_b {
                vec!["a".to_owned(), "b".to_owned()]
            } else {
                vec!["a".to_owned()]
            },
        }],
        observation: request.observation_limits,
        expansion: ExpansionLimits {
            max_candidates: 1_000_000,
        },
        meter: request.accounting,
        emit: fsl_package::EmitLimits::fixture(),
    })
    .expect("the independent model simulator admits the source package")
}

impl Fixture {
    fn new(body: &str, activation: &str, advances: bool) -> Self {
        Self::with_universe(body, activation, advances, false)
    }

    fn with_universe(body: &str, activation: &str, advances: bool, missing_b: bool) -> Self {
        Self::with_options(body, activation, advances, missing_b, false)
    }

    fn with_options(
        body: &str,
        activation: &str,
        advances: bool,
        missing_b: bool,
        reference_argument: bool,
    ) -> Self {
        let directory = tempfile::tempdir().unwrap();
        spine::write(directory.path(), Case::Healthy).unwrap();
        let model_path = directory.path().join("spine-model.semantic-ir.json");
        let model = std::fs::read_to_string(&model_path)
            .unwrap()
            .replace("\"value\": \"1000\"", "\"value\": \"3\"");
        assert!(!model.contains("\"1000\""));
        let mut model: Value = serde_json::from_str(&model).unwrap();
        if reference_argument {
            let owner = model["types"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|node| node["identity"] == OBJECT_TYPE)
                .unwrap();
            let identity = "ix://example/config-version/ConfigVersion/attemptUpdate/refs";
            owner["operations"][0]["params"] = json!([{
                "identity":identity,"name":"refs","typeRef":OBJECT_TYPE,
                "presence":"required","nullable":false,"defaultKind":"none",
                "multiplicity":{"lower":1,"upper":2,"ordered":true,"unique":false},
                "origin":{"generated":{"generatorIdentity":identity,
                    "generatorVersion":"1.0.0","inputIdentities":[identity]}}
            }]);
        }
        std::fs::write(model_path, model.to_string()).unwrap();
        let mut snapshot: Value = serde_json::from_slice(
            &std::fs::read(directory.path().join("spine-snapshot.json")).unwrap(),
        )
        .unwrap();
        snapshot["identity"] = json!({"authority":"test", "identity":"model-initial"});
        snapshot["anchor"] = json!({"kind": "initialization", "name": "start"});
        snapshot["populations"][0]["complete"] = json!(true);
        let mut object = snapshot["populations"][0]["objects"][0].clone();
        object["key"] = json!("a");
        object["fields"]["versionNumber"] = json!({"integer": "0"});
        object["fields"]["parent"] = json!({"absent": {}});
        snapshot["populations"][0]["objects"] = json!([object]);
        std::fs::write(directory.path().join("s.json"), snapshot.to_string()).unwrap();
        if missing_b {
            let mut universe = snapshot.clone();
            universe["identity"]["identity"] = json!("model-universe-b");
            universe["populations"][0]["objects"][0]["key"] = json!("b");
            std::fs::write(directory.path().join("universe.json"), universe.to_string()).unwrap();
        }
        let text = unit(body, activation, advances);
        std::fs::write(directory.path().join("unit.native"), &text).unwrap();
        let path = directory.path().join("request.json");
        std::fs::write(
            &path,
            json!({"format":"native-run/1", "request":{
                "models":[{"format":"semantic-ir/2.0.0",
                    "source":source("spine-model.semantic-ir.json")}],
                "program":{"source":source("unit.native")},
                "clause":{"selection":{"temporal":{"name":"Low", "trace":{"finite":[]},
                    "over":{"reference":{"population":POPULATION,"key":"a"}}}},
                    "observations":{"s0":"s.json"}}}})
            .to_string(),
        )
        .unwrap();
        if missing_b {
            let mut wire: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            wire["request"]["clause"]["observations"]["s1"] = json!("universe.json");
            std::fs::write(&path, wire.to_string()).unwrap();
        }
        let request = fsl_command::command::read_clause_run_request(&path)
            .expect("the real source request is admitted");
        let (steps, state_keys) = {
            let checked = crate::support::front_end::checked(
                SourceIdentity::new("test", "unit.native"),
                "unit.native",
                text.as_bytes(),
                &request.packages,
                &request.dependencies,
            )
            .expect("the source model and temporal clause check");
            let system = model_system(checked.package(), &request, missing_b);
            let mut state = system.initial().pop().expect("one authored initial state");
            let mut keys = vec![system.state_key(&state).unwrap().1];
            let mut steps = Vec::new();
            for _ in 0..4 {
                let expansion = system.successors(&state).expect("the model expands");
                let Some((transition, successor)) = expansion.successors.into_iter().next() else {
                    break;
                };
                let key = system.state_key(&successor).unwrap().1;
                keys.push(key.clone());
                steps.push(ModelStep {
                    transition,
                    post_state: key,
                    delay: None,
                });
                state = successor;
            }
            assert_eq!(steps.len(), if advances { 3 } else { 0 });
            assert!(system.successors(&state).unwrap().successors.is_empty());
            (steps, keys)
        };
        Self {
            directory,
            request,
            steps,
            state_keys,
        }
    }

    fn model_request(&self) -> ClauseRunRequest {
        let mut request = fsl_command::command::read_clause_run_request(
            &self.directory.path().join("request.json"),
        )
        .unwrap();
        let initial = self.request.snapshots.keys().next().unwrap().clone();
        request.selection = ClauseRunSelection::Temporal {
            name: "Low".to_owned(),
            trace: TraceInput::Model {
                initial: initial.clone(),
                universes: self.request.snapshots.keys().cloned().collect(),
                prefix: self.steps.clone(),
                repeat: Vec::new(),
                terminal_stutter: true,
                max_candidates: fsl_replay::spine::DEFAULT_MODEL_MAX_CANDIDATES,
            },
            over: OverObject {
                population: POPULATION.to_owned(),
                key: "a".to_owned(),
            },
            ledger: None,
        };
        request
    }

    fn result(&self) -> Value {
        let request = self.model_request();
        let selection = request.selection.clone();
        let report = run_clause(request).expect("the source request reports");
        if matches!(
            report.counterexample,
            Some(fsl_replay::ClauseRunCounterexample::Model(_))
        ) {
            assert!(matches!(
                write_clause_run_result(&report),
                Err(fsl_replay::spine::NativeRunResultError::ModelSelectionRequired)
            ));
            let mut wrong = selection.clone();
            if let ClauseRunSelection::Temporal { trace, .. } = &mut wrong {
                *trace = TraceInput::Finite(Vec::new());
            }
            assert!(matches!(
                write_clause_run_result_for(&report, Some(&wrong)),
                Err(fsl_replay::spine::NativeRunResultError::ModelSelectionRequired)
            ));
        }
        let bytes = write_clause_run_result_for(&report, Some(&selection))
            .expect("the actual model report writes");
        read_native_run_result(&bytes).expect("the produced document strictly reads");
        serde_json::from_slice(&bytes).unwrap()
    }
}

fn replay_read_witness(
    fixture: &Fixture,
    document: &Value,
) -> Result<fsl_replay::ModelTraceEnd, fsl_replay::ReplayRefusal> {
    let request = fixture.model_request();
    let text = std::fs::read(fixture.directory.path().join("unit.native")).unwrap();
    let checked = crate::support::front_end::checked(
        SourceIdentity::new("test", "unit.native"),
        "unit.native",
        &text,
        &request.packages,
        &request.dependencies,
    )
    .unwrap();
    let system = model_system(checked.package(), &request, false);
    let node = checked
        .package()
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == "Low")
        .unwrap();
    let letters = system.clause_letters(node).unwrap();
    let read = read_native_run_result(&serde_json::to_vec(document).unwrap()).unwrap();
    let NativeRunResult::Clause(record) = read else {
        panic!("a clause record")
    };
    let Some(TemporalRecord::Violated {
        counterexample: carried,
        ..
    }) = record.temporal()
    else {
        panic!("a violated model record")
    };
    let instance = fsl_eval::model_subject::ModelInstance {
        population: DeclarationKey {
            package: PACKAGE.to_owned(),
            node: POPULATION.to_owned(),
        },
        key: "a".to_owned(),
    };
    assert_eq!(
        letters.instance_reference(&instance).as_ref(),
        Some(&carried.model_over_binding().unwrap().typed())
    );
    let prefix = carried
        .model_prefix()
        .map(|step| {
            let fsl_replay::spine::ModelTransitionRead::Transition(transition) = &step.transition
            else {
                panic!("the prefix takes real operations")
            };
            ModelStep {
                transition: transition.clone(),
                post_state: StateKey::from_canonical_bytes(
                    URL_SAFE_NO_PAD.decode(step.post_state.key()).unwrap(),
                ),
                delay: None,
            }
        })
        .collect::<Vec<_>>();
    let loop_steps = carried.model_loop().collect::<Vec<_>>();
    assert_eq!(loop_steps.len(), 1);
    assert!(matches!(
        loop_steps[0].transition,
        fsl_replay::spine::ModelTransitionRead::TerminalStutter
    ));
    let payload = fsl_replay::TemporalCounterexample {
        initial_state: usize::try_from(carried.model_initial_state().unwrap()).unwrap(),
        lasso: Some(fsl_replay::Lasso {
            entry: prefix.len(),
            terminal_stutter: true,
            terminal_stutter_delay: None,
        }),
        prefix,
        over_binding: Some(instance),
        fairness: Vec::new(),
        kind: fsl_replay::CounterexampleKind::Formula,
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
        closure_stutters: Vec::new(),
    };
    fsl_replay::evaluate_model_trace(
        &system,
        &payload,
        node.clause(),
        &letters,
        request.accounting,
    )
}

fn counterexample(document: &Value) -> &Value {
    &document["disposition"]["temporal"]["counterexample"]
}

fn assert_native_binding(binding: &Value) {
    assert_eq!(binding["type"], "reference");
    assert_eq!(binding["identity"], "a");
    assert_eq!(
        binding["object_type"],
        json!({"owner_effective_type":null,
        "original":{"package":PACKAGE,"node":OBJECT_TYPE}})
    );
    assert_eq!(binding.as_object().unwrap().len(), 4);
    let universe = binding["universe"]
        .as_object()
        .expect("a structural node identity");
    assert_eq!(universe.len(), 3);
    assert_eq!(universe["package"], PACKAGE);
    assert!(universe["path"].is_array());
    assert!(universe["ordinal"].is_u64());
}

fn assert_member_refused(document: &Value, path: &str, value: Option<Value>) {
    let mut malformed = document.clone();
    if let Some(value) = value {
        *malformed
            .pointer_mut(path)
            .expect("the authentic member exists") = value;
    } else {
        let (parent, name) = path.rsplit_once('/').unwrap();
        malformed
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(name);
    }
    let error = read_native_run_result(&serde_json::to_vec(&malformed).unwrap())
        .err()
        .expect("the closed native reader refuses the mutation");
    // Custom typed transition decoding retains its relative pointer in the
    // decoder cause, while direct fields use the serde path. Both must name
    // the actual offending member rather than only its enclosing witness.
    let name = path.rsplit('/').next().unwrap();
    let reported = error.to_string();
    if let Some((_, relative)) = path.split_once("/transition") {
        assert!(
            reported.contains("transition") && reported.contains(relative),
            "expected transition member {relative}, found {reported}"
        );
    }
    assert!(
        reported.contains(name),
        "expected offending {path}, found {reported}"
    );
}

/// The unchecked request key is discarded; the producer records the actual initial state.
#[test]
#[trace("TC-1160", "FR-267-AC-30", "TC-840", "FR-330-AC-19")]
fn source_model_terminal_witness_uses_the_reconstructed_state_key() {
    let fixture = Fixture::new("holds(c.versionNumber > 2)", "on origin", false);
    let mut request: Value = serde_json::from_slice(
        &std::fs::read(fixture.directory.path().join("request.json")).unwrap(),
    )
    .unwrap();
    request["request"]["clause"]["selection"]["temporal"]["trace"] = json!({"model":{
        "initial":"s0", "universes":["s0"], "prefix":[],
        "loop":[{"transition":{"type":"terminal-stutter"},"post_state":{"key":"W10"}}]}});
    let path = fixture.directory.path().join("terminal-request.json");
    std::fs::write(&path, request.to_string()).unwrap();
    let result = fsl_command::command::run(&path, &crate::negotiation::Fr290)
        .unwrap_or_else(|error| panic!("source model result: {}", error.value().unwrap()));
    assert_eq!(result.exit_code, 10);
    let document = result.value.as_value();
    let witness = counterexample(document);
    assert_eq!(witness["trace"], "model");
    assert_eq!(witness["initial_state"], 0);
    assert_eq!(witness["prefix"], json!([]));
    assert_eq!(
        witness["loop"][0]["transition"],
        json!({"type":"terminal-stutter"})
    );
    let expected = URL_SAFE_NO_PAD.encode(fixture.state_keys[0].as_bytes());
    assert_ne!(expected, "W10");
    assert_eq!(witness["loop"][0]["post_state"]["key"], expected);
    assert_native_binding(&witness["over_binding"]);
    read_native_run_result(&serde_json::to_vec(document).unwrap()).unwrap();
}

/// A failing later activation keeps its real position and each taken model successor.
#[test]
#[trace("TC-1160", "FR-267-AC-30", "TC-840", "FR-330-AC-19")]
fn source_model_later_violation_keeps_taken_transitions_and_state_keys() {
    let fixture = Fixture::new(
        "holds(c.versionNumber <= 1)",
        "on each (d: Config::ConfigVersion)",
        true,
    );
    let document = fixture.result();
    let witness = counterexample(&document);
    assert_eq!(document["disposition"]["temporal"]["kind"], "violated");
    assert_eq!(document["disposition"]["temporal"]["position"], 2);
    assert_eq!(witness["trace_position"], 2);
    assert_eq!(witness["initial_state"], 0);
    assert_native_binding(&witness["over_binding"]);
    let prefix = witness["prefix"].as_array().unwrap();
    assert_eq!(prefix.len(), 3);
    for (index, step) in prefix.iter().enumerate() {
        assert_eq!(
            step["post_state"]["key"],
            URL_SAFE_NO_PAD.encode(fixture.state_keys[index + 1].as_bytes())
        );
        assert_eq!(
            step["transition"]["operation"],
            json!("ix://example/config-version/ConfigVersion/attemptUpdate")
        );
        assert_native_binding(&step["transition"]["receiver"]);
        assert_eq!(step["transition"]["arguments"], json!([]));
    }
    assert_eq!(
        witness["loop"][0]["post_state"]["key"],
        URL_SAFE_NO_PAD.encode(fixture.state_keys[3].as_bytes())
    );
    let read = read_native_run_result(&serde_json::to_vec(&document).unwrap()).unwrap();
    let NativeRunResult::Clause(record) = read else {
        panic!("a clause record")
    };
    let Some(TemporalRecord::Violated {
        position,
        counterexample,
    }) = record.temporal()
    else {
        panic!("a typed violated model record")
    };
    assert_eq!(*position, 2);
    assert!(counterexample.is_model());
    assert_eq!(counterexample.trace_position(), 2);
    assert_eq!(counterexample.model_prefix().count(), 3);
    let replay = replay_read_witness(&fixture, &document)
        .expect("decoded native steps replay over the authenticated source model");
    let fsl_replay::ModelTraceEnd::Evaluated(replay) = replay else {
        panic!("the actual replay completes")
    };
    assert!(
        matches!(replay.outcome, Ok(fsl_eval::temporal::TemporalOutcome::Violated { at }) if at.index() == 2)
    );
    let mut stale = document.clone();
    stale["disposition"]["temporal"]["counterexample"]["prefix"][0]["post_state"]["key"] =
        json!(URL_SAFE_NO_PAD.encode(b"{}"));
    assert!(
        matches!(replay_read_witness(&fixture, &stale), Err(fsl_replay::ReplayRefusal::ModelTrace(ref error)) if matches!(**error, fsl_replay::ModelTraceRefusal::PostState { step: 0, .. }))
    );
    stale = document.clone();
    stale["disposition"]["temporal"]["counterexample"]["prefix"][0]["transition"]["operation"] =
        json!("ix://example/config-version/ConfigVersion/missing");
    assert!(
        matches!(replay_read_witness(&fixture, &stale), Err(fsl_replay::ReplayRefusal::ModelTrace(ref error)) if matches!(**error, fsl_replay::ModelTraceRefusal::NotEnabled { step: 0, .. }))
    );
}

/// Closed native reference members are checked on documents produced by a real source run.
#[test]
#[trace("TC-1160", "FR-267-AC-31", "FR-267-AC-32")]
fn source_model_witness_reference_mutations_name_the_offending_member() {
    let fixture = Fixture::new("always holds(c.versionNumber <= 1)", "on origin", true);
    let document = fixture.result();
    for reference in [
        "/disposition/temporal/counterexample/over_binding",
        "/disposition/temporal/counterexample/prefix/0/transition/receiver",
    ] {
        for (name, value) in [
            ("identity", Some(json!(17))),
            ("universe", Some(json!([]))),
            (
                "object_type",
                Some(json!({"package":PACKAGE,"path":["ConfigVersion"],"ordinal":0})),
            ),
            ("identity", None),
        ] {
            assert_member_refused(&document, &format!("{reference}/{name}"), value);
        }
    }
    assert_member_refused(&document, "/disposition/temporal/counterexample", None);
    assert_member_refused(
        &document,
        "/disposition/temporal/counterexample/trace_position",
        Some(json!(1)),
    );
}

/// A genuinely undefined reconstructed model condition may omit its witness.
#[test]
#[trace("TC-1160", "FR-267-AC-30", "FR-267-AC-31")]
fn source_model_undefined_can_omit_its_counterexample() {
    let fixture = Fixture::with_universe(
        "holds(deref(Config::ConfigVersion#\"b\").versionNumber >= 0)",
        "on origin",
        false,
        true,
    );
    let document = fixture.result();
    assert_eq!(document["disposition"]["temporal"]["kind"], "undefined");
    assert_eq!(document["disposition"]["temporal"]["position"], 0);
    assert!(document["disposition"]["temporal"]
        .get("counterexample")
        .is_none());
    let read = read_native_run_result(&serde_json::to_vec(&document).unwrap()).unwrap();
    let NativeRunResult::Clause(record) = read else {
        panic!("a clause record")
    };
    assert!(matches!(
        record.temporal(),
        Some(TemporalRecord::Undefined {
            position: 0,
            counterexample: None,
        })
    ));
    let mut malformed = document;
    malformed["disposition"]["temporal"]["counterexample"] = Value::Null;
    assert!(read_native_run_result(&serde_json::to_vec(&malformed).unwrap()).is_err());
}

/// References nested in real operation arguments keep the same closed native shape.
#[test]
#[trace("TC-1160", "FR-267-AC-30", "FR-267-AC-32")]
fn source_model_witness_checks_references_inside_sequence_arguments() {
    let fixture = Fixture::with_options(
        "always holds(c.versionNumber <= 1)",
        "on origin",
        true,
        false,
        true,
    );
    let document = fixture.result();
    let arguments = counterexample(&document)["prefix"][0]["transition"]["arguments"]
        .as_array()
        .expect("the actual taken operation's arguments");
    assert_eq!(arguments.len(), 1);
    assert_eq!(arguments[0]["type"], "sequence");
    let elements = arguments[0]["elements"].as_array().unwrap();
    assert!((1..=2).contains(&elements.len()));
    for element in elements {
        assert_native_binding(element);
    }
    let base = "/disposition/temporal/counterexample/prefix/0/transition/arguments/0/elements/0";
    for (name, value) in [
        ("identity", Some(json!(false))),
        ("universe", Some(json!([]))),
        (
            "object_type",
            Some(json!({"package":PACKAGE,"path":["ConfigVersion"],"ordinal":0})),
        ),
        ("identity", None),
    ] {
        assert_member_refused(&document, &format!("{base}/{name}"), value);
    }
}
