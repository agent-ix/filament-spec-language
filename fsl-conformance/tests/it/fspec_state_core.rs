// SPDX-License-Identifier: AGPL-3.0-or-later
//! The standard's state-core fixtures run through FSL (QSL-2048): the typing,
//! state, boundary, reference and invocation case files, with their
//! snapshots, frames, invocations and the two `.native` sources.
//!
//! Each case is compiled and run through the public spine (parse, select,
//! check, admit, evaluate) against the model its fixture names, and the
//! disposition and refusal code are compared with the fixture's. The
//! fixtures are proposals written before FSL's `1-draft` edition, so three
//! adapters stand between a fixture and the compiler, each one explicit:
//!
//! * the sources are `edition "0-draft"` with the `state-finite/0-draft`
//!   profile; [`modern`] rewrites those header lines and nothing else, and
//!   a case named `source/<file>` checks the verbatim source as published.
//!   FSL has no `state-finite` profile, so every executed case runs under
//!   `filament.model.complete/v1` instead: a refusal the state-finite profile
//!   would add is not observable here. The run prints this substitution and
//!   the number of cases it applies to;
//! * the domain package `example/config` is FSL's ConfigVersion example
//!   renamed, with the frame of each invocation taken from its frame file;
//! * a fixture snapshot is written in the compiler's snapshot format.
//!
//! A case that does not match, or that no adapter can execute, fails the
//! run unless the checked-in registry keys it to an open ticket. The
//! expected result of a case is never changed to match the compiler.

use std::collections::BTreeMap;

use filament_exact::Cancel;
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{
    self, default_accounting, CallOutcome, ClauseArgument, ClauseArgumentValue, ClauseDisposition,
    ClauseRunRequest, ClauseRunSelection, ClauseRunSource, DependencyInput, ParseRequest,
    SpineLimits,
};
use fsl_semantics::model::observation::{
    AdmissionFailure, AnchorKind, ClauseSelection, ClauseSelectionInput, DocumentProvision,
    DocumentRef, ObservationLimits, SelectedAnchor, SelectedObject,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::fspec_standard::{self, Settled};

const RUNNER: &str = "state-core";
const ROOT: &str = "proposals/state-core/fixtures/";
const FILES: [&str; 5] = [
    "typing-cases.json",
    "state-cases.json",
    "boundary-cases.json",
    "reference-cases.json",
    "invocation-cases.json",
];
const PACKAGE: &str = "example/config";
const CONFIG_VERSION: &str = "ix://example/config/ConfigVersion";
const POPULATION: &str = "ix://example/config/config_history";
const FIXTURE_TYPE: &str = "ix://example/config/type/ConfigVersion";
const WORK: u64 = 1_000_000;

/// One case file: its short name and its cases.
struct File {
    name: &'static str,
    cases: Vec<Value>,
}

fn load_file(name: &'static str) -> Result<File, String> {
    let document = fspec_standard::json(&format!("{ROOT}{name}"))?;
    let cases = document["cases"]
        .as_array()
        .ok_or_else(|| format!("{ROOT}{name} has no cases array"))?
        .clone();
    if cases.is_empty() {
        return Err(format!("{ROOT}{name} lists no cases"));
    }
    Ok(File {
        name: name.trim_end_matches("-cases.json"),
        cases,
    })
}

fn load() -> Result<Vec<File>, String> {
    FILES.into_iter().map(load_file).collect()
}

fn text(path: &str) -> Result<String, String> {
    let bytes = fspec_standard::read(&format!("{ROOT}{path}"))?;
    String::from_utf8(bytes).map_err(|error| format!("{path} is not UTF-8: {error}"))
}

fn id_of(case: &Value) -> Result<&str, String> {
    case["id"]
        .as_str()
        .ok_or_else(|| format!("a case has no id: {case}"))
}

/// What running a case produced, in the fixtures' words.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Observed {
    Completed(bool),
    Refused(String),
    Incomplete(String),
    Other(String),
}

fn code_of(failure: &AdmissionFailure) -> (bool, String) {
    match failure {
        AdmissionFailure::Refused(record) => (false, record.catalog.code().to_owned()),
        AdmissionFailure::Incomplete(record) => (true, record.catalog.code().to_owned()),
        AdmissionFailure::Fault(fault) => (
            false,
            format!(
                "InternalFault stage={} invariant={}",
                fault.stage(),
                fault.invariant()
            ),
        ),
    }
}

fn observe(disposition: &ClauseDisposition) -> Observed {
    match disposition {
        ClauseDisposition::Compile(refusal) => {
            Observed::Refused(refusal.code().as_str().to_owned())
        }
        ClauseDisposition::Admit(failure)
        | ClauseDisposition::SnapshotAdmission { failure, .. } => match code_of(failure) {
            (true, code) => Observed::Incomplete(code),
            (false, code) => Observed::Refused(code),
        },
        // The compiler's own outcome document (`fsl-replay` outcome.rs, the
        // `filament-outcome/1` classification) reports every incomplete
        // evaluation, whichever accounting limit it hit, as the code
        // `resource_exhausted`; this is its classification, not the runner's.
        ClauseDisposition::Evaluate(CallOutcome::Incomplete(_)) => {
            Observed::Incomplete("resource_exhausted".to_owned())
        }
        // A fault is pinned by its stable stage and invariant, not by Debug text.
        ClauseDisposition::EvaluateFault(fault) => Observed::Other(format!(
            "EvaluateFault/InternalFault stage={} invariant={}",
            fault.stage(),
            fault.invariant()
        )),
        other => match other.truth() {
            Some(value) => Observed::Completed(value),
            None => Observed::Other(format!("{other:?}")),
        },
    }
}

/// The result a fixture's `expected` object states.
fn expected_observation(expected: &Value) -> Result<Observed, String> {
    let diagnostic = || {
        expected["diagnostic"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("expected result without a diagnostic: {expected}"))
    };
    match expected["execution"].as_str() {
        Some("completed") => expected["value"]
            .as_bool()
            .map(Observed::Completed)
            .ok_or_else(|| format!("completed result without a value: {expected}")),
        Some("refused") => diagnostic().map(Observed::Refused),
        Some("incomplete") => diagnostic().map(Observed::Incomplete),
        other => Err(format!("unknown execution {other:?}")),
    }
}

/// The disposition a boundary fixture's diagnostic implies. The standard
/// (boundary.md, "Proposed codes") says invalid input and unsupported
/// capability refuse a requested operation, while missing observations and
/// exhausted resources leave it incomplete.
fn boundary_wanted(code: &str) -> Observed {
    match code {
        "resource_exhausted"
        | "unavailable_observation"
        | "cancelled"
        | "incomplete_population" => Observed::Incomplete(code.to_owned()),
        _ => Observed::Refused(code.to_owned()),
    }
}

fn compare(wanted: &Observed, actual: &Observed) -> Settled {
    if wanted == actual {
        Settled::Matched
    } else {
        Settled::Mismatch(format!("expected {wanted:?}, actual {actual:?}"))
    }
}

/// Rewrites the `0-draft` header lines of a published source to the edition
/// and profile FSL accepts, and gives each clause the profile it names.
/// Nothing else of the source changes.
fn modern(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let line = if line.starts_with("language ") {
            line.replace("\"0-draft\"", "\"1-draft\"")
        } else if line.starts_with("profile \"state-finite/0-draft\"") {
            "profile v = \"filament.model.complete/v1\";".to_owned()
        } else if let Some((head, rest)) = clause_head(line) {
            format!("{head} using v on {rest}")
        } else {
            line.to_owned()
        };
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn clause_head(line: &str) -> Option<(&str, &str)> {
    ["invariant ", "pre ", "post "]
        .iter()
        .any(|keyword| line.starts_with(keyword))
        .then(|| line.split_once(" on "))
        .flatten()
}

/// The body between the braces of the clause named `name` in a source.
fn clause_body(source: &str, name: &str) -> Result<String, String> {
    let marker = format!(" {name} using v on ");
    let start = source
        .find(&marker)
        .ok_or_else(|| format!("the source has no clause {name}"))?;
    let open = source[start..]
        .find('{')
        .map(|at| start + at + 1)
        .ok_or("clause without a brace")?;
    let close = source[open..]
        .find('}')
        .map(|at| open + at)
        .ok_or("clause without a closing brace")?;
    Ok(source[open..close].trim().to_owned())
}

/// The domain package `example/config`, with the frame of the operation.
fn model<S: AsRef<str>>(modifies: &[S], may_delete: bool) -> Result<String, String> {
    let mut document: Value = serde_json::from_str(
        &include_str!("../../../examples/config-version/model.semantic-ir.json")
            .replace("example/config-version", PACKAGE),
    )
    .map_err(|error| format!("the ConfigVersion example is not JSON: {error}"))?;
    let fields: Vec<Value> = modifies
        .iter()
        .map(|field| json!(format!("{CONFIG_VERSION}/{}", field.as_ref())))
        .collect();
    let frame = document
        .pointer_mut("/types/1/operations/0/frame")
        .and_then(Value::as_object_mut)
        .ok_or("the ConfigVersion example has no operation frame at types[1].operations[0]")?;
    frame.insert("modifies".to_owned(), Value::Array(fields));
    if may_delete {
        frame.insert("deletes".to_owned(), json!([CONFIG_VERSION]));
    }
    Ok(document.to_string())
}

fn label(authority: &str, identity: &str) -> DocumentRef {
    DocumentRef {
        authority: authority.to_owned(),
        identity: identity.to_owned(),
    }
}

/// A fixture snapshot in the compiler's snapshot document.
fn snapshot(snapshot: &Value, observation: &str, identity: &DocumentRef) -> Vec<u8> {
    let objects: Vec<Value> = snapshot["objects"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|object| {
            let parent = match &object["fields"]["parent"] {
                Value::Null => json!({"absent": {}}),
                reference => json!({"present": {"reference": {
                    "population": POPULATION, "key": reference["object"],
                }}}),
            };
            json!({
                "key": object["id"], "type": CONFIG_VERSION,
                "fields": {
                    "versionNumber": {"integer": object["fields"]["versionNumber"].to_string()},
                    "parent": parent,
                },
            })
        })
        .collect();
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": identity.authority, "identity": identity.identity},
        "observation": observation,
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {"identity": PACKAGE},
        "populations": [{
            "population": POPULATION,
            "complete": snapshot["complete"].as_bool().unwrap_or(true),
            "objects": objects,
        }],
    })
    .to_string()
    .into_bytes()
}

/// What to run.
struct Run {
    unit: String,
    model: Option<String>,
    snapshots: DocumentProvision,
    invocations: DocumentProvision,
    selection: ClauseRunSelection,
    work: u64,
}

fn run(run: Run) -> Result<Observed, String> {
    let packages = match &run.model {
        Some(model) => fsl_semantics::model::intake::package_input([model.as_bytes()])
            .map_err(|refusal| format!("the model package: {refusal:?}"))?,
        None => BTreeMap::new(),
    };
    let request = ClauseRunRequest {
        clocks: Default::default(),
        package_name: "state-core".to_owned(),
        source: ClauseRunSource::Program {
            identity: SourceIdentity::new("fspec", "ix://example/config/source/state-core"),
            path: "state-core.native".to_owned(),
            bytes: run.unit.into_bytes(),
        },
        packages,
        dependencies: DependencyInput::default(),
        snapshots: run.snapshots,
        invocations: run.invocations,
        selection: run.selection,
        limits: SpineLimits::default(),
        observation_limits: ObservationLimits::default(),
        accounting: default_accounting(run.work),
    };
    let report = spine::run_clause(request).map_err(|error| format!("{error:?}"))?;
    Ok(observe(&report.disposition))
}

fn current_selection(
    clause: &str,
    identity: &DocumentRef,
    self_key: Option<&str>,
) -> ClauseRunSelection {
    ClauseRunSelection::Clause(ClauseSelection {
        name: clause.to_owned(),
        input: ClauseSelectionInput::Current {
            snapshot: identity.clone(),
            anchor: SelectedAnchor {
                kind: AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: self_key.map(|key| SelectedObject {
                population: POPULATION.to_owned(),
                key: key.to_owned(),
            }),
        },
    })
}

/// The unit of a state case: the published invariant, any expression the
/// case adds as clause `Case`, and the identity function.
fn state_unit(case: &Value) -> Result<(String, &'static str), String> {
    let mut unit = modern(&text("parent-order.native")?);
    unit.push_str(
        "function sameIdentity using v(a: M::ConfigVersion, b: M::ConfigVersion): Boolean pure { a = b }\n",
    );
    let clause = match case["expression"].as_str() {
        Some(expression) => {
            unit.push_str(&format!(
                "invariant Case using v on M::ConfigVersion at current {{ {expression} }}\n"
            ));
            "Case"
        }
        None => "ParentPrecedes",
    };
    Ok((unit, clause))
}

fn run_state(case: &Value) -> Result<Settled, String> {
    let id = id_of(case)?;
    let (unit, clause) = state_unit(case)?;
    let identity = label("agent-ix", &format!("ix://example/config/snapshot/{id}"));
    let bytes = snapshot(&case["snapshot"], "current", &identity);
    let selection = match case["primitive"].as_str() {
        None => current_selection(clause, &identity, case["self"].as_str()),
        Some("identity-equality") => {
            let [first, second] = case["operands"]
                .as_array()
                .map(Vec::as_slice)
                .ok_or("identity-equality needs operands")?
            else {
                return Err(format!(
                    "{id}: identity-equality needs exactly two operands"
                ));
            };
            let argument = |parameter: &str, key: &Value| ClauseArgument {
                parameter: parameter.to_owned(),
                value: ClauseArgumentValue::Reference {
                    population: POPULATION.to_owned(),
                    key: key.as_str().unwrap_or_default().to_owned(),
                },
            };
            ClauseRunSelection::Function {
                name: "sameIdentity".to_owned(),
                arguments: vec![argument("a", first), argument("b", second)],
                snapshot: identity.clone(),
                snapshot_path: None,
            }
        }
        Some(other) => return Ok(Settled::Unsupported(format!("{id}: primitive {other}"))),
    };
    let actual = run(Run {
        unit,
        model: Some(model(&["versionNumber"], false)?),
        snapshots: BTreeMap::from([(identity, bytes)]),
        invocations: BTreeMap::new(),
        selection,
        work: WORK,
    })?;
    Ok(compare(&expected_observation(&case["expected"])?, &actual))
}

fn artifact(reference: &Value) -> DocumentRef {
    label(
        reference["authority"].as_str().unwrap_or_default(),
        reference["identity"].as_str().unwrap_or_default(),
    )
}

fn run_invocation(case: &Value) -> Result<Settled, String> {
    let id = id_of(case)?;
    let path = case["input"]["path"]
        .as_str()
        .ok_or("an invocation case names its file")?;
    let input = serde_json::from_str::<Value>(&text(path)?).map_err(|error| error.to_string())?;
    let frame = serde_json::from_str::<Value>(&text(
        input["frameBinding"]["path"]
            .as_str()
            .ok_or("an invocation names its frame")?,
    )?)
    .map_err(|error| error.to_string())?;
    let names = |key: &str| -> Vec<String> {
        frame["frame"][key]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(|iri| iri.rsplit('/').next().unwrap_or(iri).to_owned())
            .collect()
    };
    let changed = names("allowedChangedFields");
    let deleted = names("allowedDeletedTypes");
    let mut snapshots = BTreeMap::new();
    let mut foreign: Vec<String> = Vec::new();
    let mut side = |name: &str| -> Result<Option<DocumentRef>, String> {
        let Some(file) = input[name]["path"].as_str() else {
            return Ok(None);
        };
        let document = serde_json::from_str::<Value>(&text(file)?).map_err(|e| e.to_string())?;
        if document["invocation"] != input["invocation"] {
            foreign.push(name.to_owned());
        }
        let identity = artifact(&input[name]["artifactRef"]);
        snapshots.insert(identity.clone(), snapshot(&document, name, &identity));
        Ok(Some(identity))
    };
    let pre = side("pre")?;
    let post = side("post")?;
    if !foreign.is_empty() {
        return Ok(Settled::Unsupported(format!(
            "{id}: the snapshot wire carries no invocation member, so the {} snapshot cannot name another invocation",
            foreign.join(" and ")
        )));
    }
    let invocation = label("agent-ix", input["invocation"].as_str().unwrap_or_default());
    let reference = |identity: &Option<DocumentRef>| match identity {
        Some(identity) => json!({"identity": {
            "authority": identity.authority, "identity": identity.identity,
        }}),
        None => json!({"identity": {"authority": "agent-ix", "identity": "unavailable"}}),
    };
    let key = |name: &str| input[name].as_array().cloned().unwrap_or_default();
    let document = json!({
        "format": "filament.state.invocation/v1",
        "identity": {"authority": invocation.authority, "identity": invocation.identity},
        "model": {"identity": PACKAGE},
        "context": CONFIG_VERSION,
        "operation": "attemptUpdate",
        "self": {"population": POPULATION, "key": input["self"]},
        "pre": reference(&pre),
        "post": reference(&post),
        "parameters": {},
        "result": {"boolean": input["result"]},
        "created": key("created").iter().map(|k| json!({"population": POPULATION, "key": k})).collect::<Vec<_>>(),
        "deleted": key("deleted").iter().map(|k| json!({"population": POPULATION, "key": k})).collect::<Vec<_>>(),
    });
    let clause = input["subject"]["clause"]
        .as_str()
        .unwrap_or("VersionUnchanged");
    let actual = run(Run {
        unit: modern(&text("version-update.native")?),
        model: Some(model(&changed, !deleted.is_empty())?),
        snapshots,
        invocations: BTreeMap::from([(invocation.clone(), document.to_string().into_bytes())]),
        selection: ClauseRunSelection::Clause(ClauseSelection {
            name: clause.to_owned(),
            input: ClauseSelectionInput::Invocation { invocation },
        }),
        work: WORK,
    })?;
    Ok(compare(&expected_observation(&case["expected"])?, &actual))
}

/// Parses and checks a unit against the model; the observation is a
/// refusal's code or `Completed(true)` for a unit that checks.
fn check_unit(unit: &str, model: Option<String>) -> Result<Observed, String> {
    let packages = match &model {
        Some(model) => fsl_semantics::model::intake::package_input([model.as_bytes()])
            .map_err(|refusal| format!("the model package: {refusal:?}"))?,
        None => BTreeMap::new(),
    };
    let cancel = Cancel::new();
    let limits = SpineLimits::default();
    let identity = SourceIdentity::new("fspec", "ix://example/config/source/boundary");
    let parsed = match spine::parse(
        &ParseRequest {
            source: &identity,
            path: "boundary.native",
            bytes: unit.as_bytes(),
        },
        limits.source,
        &cancel,
    ) {
        Ok(parsed) => parsed.into_value(),
        Err(error) => return Ok(refusal_of(error)),
    };
    let models = match spine::select(&parsed, &packages, limits.model, &cancel) {
        Ok(models) => models.into_value(),
        Err(error) => return Ok(refusal_of(error)),
    };
    match spine::check(
        &parsed,
        "boundary".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    ) {
        Ok(_) => Ok(Observed::Completed(true)),
        Err(error) => Ok(refusal_of(error)),
    }
}

/// A stage failure of the front end, as the refusal it is.
fn refusal_of(failure: spine::FrontEndFailure) -> Observed {
    match spine::refusal_or_fault(failure) {
        Ok(refusal) => Observed::Refused(refusal.code().as_str().to_owned()),
        Err(fault) => Observed::Other(format!(
            "InternalFault stage={} invariant={}",
            fault.stage(),
            fault.invariant()
        )),
    }
}

fn run_boundary(case: &Value) -> Result<Settled, String> {
    let id = id_of(case)?;
    let wanted = boundary_wanted(
        case["expectedDiagnostic"]
            .as_str()
            .ok_or("a boundary case names its diagnostic")?,
    );
    let published = text("parent-order.native")?;
    let good = modern(&published);
    let model_json = model(&["versionNumber"], false)?;
    let actual = match id {
        "unknown-language-profile" => {
            return Ok(Settled::Unsupported(format!(
                "{id}: the fixture sets profile.version of subject.json to 999 and no FSL reader accepts the subject wire; renaming the source profile is a different input"
            )))
        }
        "missing-model" => check_unit(&good, None)?,
        "unsupported-collect" => check_unit(
            &good.replace(
                &clause_body(&good, "ParentPrecedes")?,
                "collect(self.children)",
            ),
            Some(model_json),
        )?,
        "malformed-expression" => check_unit(good.trim_end().trim_end_matches('}'), Some(model_json))?,
        "undefined-option" => check_unit(
            &good.replace(
                &clause_body(&good, "ParentPrecedes")?,
                "deref(value(self.parent)).versionNumber < self.versionNumber",
            ),
            Some(model_json),
        )?,
        "foreign-import" => {
            // The fixture sets the parent target to a foreign type: only the
            // `parent` field's typeRef changes, no other reference.
            let mut document: Value =
                serde_json::from_str(&model_json).map_err(|error| error.to_string())?;
            let parent = document
                .pointer_mut("/types/1/fields")
                .and_then(Value::as_array_mut)
                .and_then(|fields| fields.iter_mut().find(|field| field["name"] == "parent"))
                .ok_or("the ConfigVersion example has no parent field")?;
            parent["typeRef"] = json!("ix://foreign/config/type/Hidden");
            checked_then_run(&good, document.to_string())?
        }
        "ambiguous-export" => {
            let mut document: Value =
                serde_json::from_str(&model_json).map_err(|error| error.to_string())?;
            // Two exported type identities under the name ConfigVersion.
            let duplicate: Value = serde_json::from_str(
                &document
                    .pointer("/types/1")
                    .ok_or("the model has no second type")?
                    .to_string()
                    .replace(CONFIG_VERSION, "ix://example/config/other/ConfigVersion"),
            )
            .map_err(|error| error.to_string())?;
            document["types"]
                .as_array_mut()
                .ok_or("types")?
                .push(duplicate);
            checked_then_run(&good, document.to_string())?
        }
        "unbounded-integer" => {
            let mut document: Value =
                serde_json::from_str(&model_json).map_err(|error| error.to_string())?;
            document
                .pointer_mut("/types/0/constraints")
                .and_then(Value::as_array_mut)
                .ok_or("the first type has no constraints")?
                .pop();
            checked_then_run(&good, document.to_string())?
        }
        "deleted-object" => {
            return Ok(Settled::Unsupported(format!(
                "{id}: the fixture dereferences a removed object in the post snapshot of a future operation; a dangling parent in a current snapshot is a different input"
            )))
        }
        "wrong-invocation" | "exhausted-evaluation" => return run_future_input(id, &wanted),
        "unavailable-observation" => {
            return Ok(Settled::Unsupported(format!(
                "{id}: the snapshot wire has no unavailable field value; only a whole missing document is unavailable"
            )))
        }
        other => {
            return Ok(Settled::Unsupported(format!(
                "{id}: no source adapter for the mutation of {} ({other})",
                case["artifact"]
            )))
        }
    };
    Ok(compare(&wanted, &actual))
}

/// A model mutation is refused when the unit is checked against it or, if
/// the check accepts it, when a healthy snapshot is admitted and run over it.
fn checked_then_run(unit: &str, model: String) -> Result<Observed, String> {
    let checked = check_unit(unit, Some(model.clone()))?;
    if checked != Observed::Completed(true) {
        return Ok(checked);
    }
    let identity = label("agent-ix", "ix://example/config/snapshot/mutated-model");
    let object = |key: &str, n: i64, parent: Value| json!({"id": key, "type": FIXTURE_TYPE, "fields": {"versionNumber": n, "parent": parent}});
    let document = json!({"complete": true, "objects": [
        object("root", 1, Value::Null),
        object("child", 2, json!({"type": FIXTURE_TYPE, "object": "root"})),
    ]});
    run(Run {
        unit: unit.to_owned(),
        model: Some(model),
        snapshots: BTreeMap::from([(identity.clone(), snapshot(&document, "current", &identity))]),
        invocations: BTreeMap::new(),
        selection: current_selection("ParentPrecedes", &identity, Some("child")),
        work: WORK,
    })
}

/// The boundary cases over inputs the file calls future: each is the
/// state or invocation case that carries the same defect.
fn run_future_input(id: &str, wanted: &Observed) -> Result<Settled, String> {
    let (unit, clause) = state_unit(&Value::Null)?;
    let identity = label("agent-ix", &format!("ix://example/config/snapshot/{id}"));
    let objects = |complete: bool, parent: Value| {
        json!({"complete": complete, "objects": [{
            "id": "a", "type": FIXTURE_TYPE,
            "fields": {"versionNumber": 1, "parent": parent},
        }]})
    };
    let (document, work) = match id {
        "exhausted-evaluation" => (objects(true, Value::Null), 0),
        _ => {
            return Ok(Settled::Unsupported(format!(
                "{id}: the snapshot wire carries no invocation member, so pre and post cannot name different invocations"
            )))
        }
    };
    let actual = run(Run {
        unit,
        model: Some(model(&["versionNumber"], false)?),
        snapshots: BTreeMap::from([(identity.clone(), snapshot(&document, "current", &identity))]),
        invocations: BTreeMap::new(),
        selection: current_selection(clause, &identity, Some("a")),
        work,
    })?;
    Ok(compare(wanted, &actual))
}

/// Names the ConfigVersion model does not declare: a typing case that uses
/// one needs a richer model than this runner writes.
const FOREIGN_NAMES: [&str; 11] = [
    "signed", "den", "wide", "count", "distance", "duration", "items", "peer", "reaches", "size(",
    "collect",
];

/// The typing cases whose expressions use only the `n` and `parent` fields
/// of the typing model's `Node`, which the ConfigVersion model declares as
/// `versionNumber` and `parent`. The judgment is the compile result; an
/// accepted case is then evaluated over a snapshot of its stated
/// hypotheses.
fn run_typing(case: &Value) -> Result<Settled, String> {
    let id = id_of(case)?;
    let expression = case["expression"]
        .as_str()
        .ok_or("a typing case has an expression")?;
    let unsupported = |reason: &str| Ok(Settled::Unsupported(format!("{id}: {reason}")));
    if case["anchor"] != "current" {
        return unsupported("a post anchor needs an operation invocation adapter");
    }
    if let Some(name) = FOREIGN_NAMES
        .iter()
        .find(|name| expression.contains(**name))
    {
        return unsupported(&format!(
            "the model has no `{name}`; the typing model's Node is not written as a domain package"
        ));
    }
    if case.get("budget").is_some() {
        return unsupported("an expression or graph budget has no request adapter");
    }
    let hypotheses = case["hypotheses"].as_object().ok_or("hypotheses")?;
    if hypotheses
        .keys()
        .any(|key| !["self.n", "self.parent", "other.n"].contains(&key.as_str()))
        || case["inputValidation"] != "valid-under-hypotheses"
    {
        return unsupported("the hypotheses are not a plain snapshot of self and other");
    }
    let source = expression.replace(".n", ".versionNumber");
    let mut unit = modern(&text("parent-order.native")?);
    unit.push_str(&format!(
        "invariant Case using v on M::ConfigVersion at current {{ {source} }}\n"
    ));
    let expected = &case["expected"];
    let compiled = check_unit(&unit, Some(model(&["versionNumber"], false)?))?;
    let wanted_typing = expected["typing"].as_str().unwrap_or_default();
    if wanted_typing != "accepted-under-hypotheses" {
        let wanted = Observed::Refused(
            expected["diagnostic"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        );
        return Ok(compare(&wanted, &compiled));
    }
    if compiled != Observed::Completed(true) {
        return Ok(Settled::Mismatch(format!(
            "{id}: expected the expression to be accepted, actual {compiled:?}"
        )));
    }
    let number = |key: &str| hypotheses.get(key).and_then(Value::as_i64).unwrap_or(0);
    let parent = match hypotheses.get("self.parent") {
        Some(parent) if parent["case"] == "present" => {
            json!({"type": FIXTURE_TYPE, "object": parent["target"]})
        }
        _ => Value::Null,
    };
    let object = |key: &str, n: i64, parent: Value| {
        json!({"id": key, "type": FIXTURE_TYPE,
            "fields": {"versionNumber": n, "parent": parent}})
    };
    let mut objects = vec![object("self", number("self.n"), parent)];
    if hypotheses.contains_key("other.n") || hypotheses.contains_key("self.parent") {
        objects.push(object("other", number("other.n"), Value::Null));
    }
    let identity = label(
        "agent-ix",
        &format!("ix://example/config/snapshot/typing/{id}"),
    );
    let bytes = snapshot(
        &json!({"complete": true, "objects": objects}),
        "current",
        &identity,
    );
    let actual = run(Run {
        unit,
        model: Some(model(&["versionNumber"], false)?),
        snapshots: BTreeMap::from([(identity.clone(), bytes)]),
        invocations: BTreeMap::new(),
        selection: current_selection("Case", &identity, Some("self")),
        work: WORK,
    })?;
    let evaluation = &expected["evaluation"];
    let wanted = match evaluation["state"].as_str() {
        Some("completed-under-hypotheses") => Observed::Completed(
            evaluation["value"]
                .as_bool()
                .ok_or("a completed evaluation has a value")?,
        ),
        Some("incomplete-under-hypotheses") => Observed::Incomplete(
            expected["diagnostic"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        ),
        other => return unsupported(&format!("evaluation state {other:?}")),
    };
    Ok(compare(&wanted, &actual))
}

fn run_reference(case: &Value) -> Result<Settled, String> {
    let id = id_of(case)?;
    Ok(Settled::Unsupported(format!(
        "{id}: FSL has no reader that computes a semantic reference from an artifact reference"
    )))
}

type Runner = fn(&Value) -> Result<Settled, String>;

fn runner_for(file: &str) -> Runner {
    match file {
        "typing" => run_typing,
        "state" => run_state,
        "boundary" => run_boundary,
        "reference" => run_reference,
        _ => run_invocation,
    }
}

/// Every case of every file, keyed `file/id`, and what running it gave.
/// The verbatim published sources are two more cases, keyed `source/<file>`.
fn run_all(files: &[File]) -> Vec<(String, Settled)> {
    let mut results = Vec::new();
    for file in files {
        let run = runner_for(file.name);
        for case in &file.cases {
            let id = id_of(case).unwrap_or("?");
            let settled = run(case).unwrap_or_else(Settled::Error);
            results.push((format!("{}/{id}", file.name), settled));
        }
    }
    results
}

/// The published `.native` sources, as published, through parse and check.
fn run_sources() -> Vec<(String, Settled)> {
    ["parent-order.native", "version-update.native"]
        .into_iter()
        .map(|file| {
            let settled = text(file)
                .and_then(|source| check_unit(&source, Some(model(&["versionNumber"], false)?)))
                .map_or_else(Settled::Error, |observed| match observed {
                    Observed::Completed(true) => Settled::Matched,
                    other => Settled::Mismatch(format!(
                        "source/{file}: the published source should parse and check, actual {other:?}"
                    )),
                });
            (format!("source/{file}"), settled)
        })
        .collect()
}

/// The cases per file the published standard holds. The test below derives
/// the same counts from the fixture bytes and requires them to be equal, so
/// a changed package fails loudly instead of running fewer cases.
const PUBLISHED_COUNTS: [(&str, usize); 5] = [
    ("typing", 51),
    ("state", 9),
    ("boundary", 17),
    ("reference", 10),
    ("invocation", 7),
];

/// One full run: the case ids each file lists, and what every case and
/// published source produced. Computed once and shared by the tests.
struct Outcome {
    listed: Vec<String>,
    counts: Vec<(&'static str, usize)>,
    results: Vec<(String, Settled)>,
}

fn full_run() -> Result<Outcome, String> {
    let files = load()?;
    let mut listed = Vec::new();
    for file in &files {
        for case in &file.cases {
            let id = format!("{}/{}", file.name, id_of(case)?);
            if listed.contains(&id) {
                return Err(format!("the fixtures list the case {id} twice"));
            }
            listed.push(id);
        }
    }
    let counts = files.iter().map(|f| (f.name, f.cases.len())).collect();
    let mut results = run_all(&files);
    results.extend(run_sources());
    Ok(Outcome {
        listed,
        counts,
        results,
    })
}

fn outcome() -> &'static Outcome {
    static OUTCOME: std::sync::OnceLock<Result<Outcome, String>> = std::sync::OnceLock::new();
    OUTCOME
        .get_or_init(full_run)
        .as_ref()
        .unwrap_or_else(|error| panic!("{error}"))
}

/// AC gap: QSL-3074. FR-350 has no acceptance criterion for the state-core
/// runner or the ticket-key registry (AC-20 and AC-21 are about the
/// finite-window `window` group only), so the tests below carry the
/// requirement id `FR-350` and no AC id.
///
/// Provenance: QSL-2048. Every state-core case runs through the compiler.
#[trace("FR-350")]
#[test]
fn conformance_fspec_state_core_source_cases() {
    let outcome = outcome();
    let tally = fspec_standard::settle(RUNNER, outcome.results.clone(), fspec_standard::registry());
    for id in &tally.matched {
        println!("passed: {id}");
    }
    println!(
        "adapter: sources run under filament.model.complete/v1 in place of state-finite/0-draft (FSL has no state-finite profile); applies to every case that executes",
    );
    if let Err(error) = fspec_standard::report(RUNNER, &tally) {
        panic!("{error}");
    }
    println!(
        "conformance: {} state-core source cases matched",
        tally.matched.len()
    );
}

/// AC gap: QSL-3074. Provenance: QSL-2048. The cases run equal the cases the
/// fixtures list, once each, and equal the published package's shape.
#[trace("FR-350")]
#[test]
fn executed_and_reported_cases_equal_the_cases_in_the_fixtures() {
    let outcome = outcome();
    let mut derived = Vec::new();
    for (file, _) in PUBLISHED_COUNTS {
        let document = fspec_standard::json(&format!("{ROOT}{file}-cases.json"))
            .unwrap_or_else(|error| panic!("{error}"));
        derived.push((file, document["cases"].as_array().map_or(0, Vec::len)));
    }
    assert_eq!(
        derived, PUBLISHED_COUNTS,
        "the published fixtures changed shape; the count is the standard's"
    );
    assert_eq!(outcome.counts, derived);
    // Every listed case has exactly one result, and nothing else ran but
    // the two published sources.
    let mut ran: Vec<&str> = outcome
        .results
        .iter()
        .map(|(id, _)| id.as_str())
        .filter(|id| !id.starts_with("source/"))
        .collect();
    let mut listed: Vec<&str> = outcome.listed.iter().map(String::as_str).collect();
    ran.sort_unstable();
    listed.sort_unstable();
    assert_eq!(ran, listed, "the cases run are not the cases listed");
    assert_eq!(outcome.results.len(), listed.len() + 2);
    for (id, settled) in &outcome.results {
        if let Settled::Unsupported(reason) = settled {
            println!("reported-skipped: {id}: {reason}");
        }
    }
}

/// The case with the fields each runner reads altered: the expected
/// diagnostic (typing reads `expected.diagnostic`, boundary reads
/// `expectedDiagnostic`, state and invocation read `expected.diagnostic`
/// and `expected.value`, typing reads `expected.evaluation.value`).
fn altered(case: &Value) -> Value {
    let mut case = case.clone();
    if let Some(code) = case.get_mut("expectedDiagnostic") {
        *code = json!("altered_code");
    }
    if let Some(expected) = case.get_mut("expected").and_then(Value::as_object_mut) {
        if let Some(code) = expected.get_mut("diagnostic") {
            *code = json!("altered_code");
        }
        if let Some(value) = expected.get("value").and_then(Value::as_bool) {
            expected.insert("value".to_owned(), json!(!value));
        }
        if let Some(evaluation) = expected
            .get_mut("evaluation")
            .and_then(Value::as_object_mut)
        {
            if let Some(value) = evaluation.get("value").and_then(Value::as_bool) {
                evaluation.insert("value".to_owned(), json!(!value));
            }
        }
    }
    case
}

/// AC gap: QSL-3074. Provenance: QSL-2048. For every file that has a case
/// matching its fixture, each matching case, altered in the fields its runner
/// reads, is reported as a mismatch. The reference file has no matching case
/// and is covered by the next test.
#[trace("FR-350")]
#[test]
fn an_altered_expectation_is_a_mismatch_for_every_matched_case_of_each_file() {
    let outcome = outcome();
    let files = load().unwrap_or_else(|error| panic!("{error}"));
    for file in &files {
        let run = runner_for(file.name);
        let mut checked = 0;
        for case in &file.cases {
            let id = format!("{}/{}", file.name, id_of(case).expect("a case id"));
            if !outcome
                .results
                .iter()
                .any(|(name, settled)| *name == id && *settled == Settled::Matched)
            {
                continue;
            }
            let changed = altered(case);
            assert_ne!(&changed, case, "{id}: the alteration changed nothing");
            let result = run(&changed);
            assert!(
                matches!(result, Ok(Settled::Mismatch(_))),
                "{id}: an altered expectation was not reported as a mismatch: {result:?}"
            );
            checked += 1;
        }
        if file.name == "reference" {
            assert_eq!(
                checked, 0,
                "the reference file now has matching cases; cover them here"
            );
        } else {
            assert!(checked > 0, "{}: no matching case to alter", file.name);
        }
    }
}

/// AC gap: QSL-3074. Provenance: QSL-2048. The reference file executes no
/// case today. This is not an altered-expectation test: it asserts that every
/// reference case is reported unsupported and pinned by the registry, so one
/// that starts to execute, or to fail differently, fails the run.
#[trace("FR-350")]
#[test]
fn every_reference_case_is_unsupported_and_pinned_by_the_registry() {
    let outcome = outcome();
    let reference: Vec<(String, Settled)> = outcome
        .results
        .iter()
        .filter(|(id, _)| id.starts_with("reference/"))
        .cloned()
        .collect();
    assert_eq!(reference.len(), 10);
    assert!(reference
        .iter()
        .all(|(_, settled)| matches!(settled, Settled::Unsupported(_))));
    let tally = fspec_standard::settle(RUNNER, reference, fspec_standard::registry());
    let reference_only: Vec<&String> = tally
        .failures
        .iter()
        .filter(|text| !text.contains("which is no case of the fixtures"))
        .collect();
    assert!(reference_only.is_empty(), "{reference_only:?}");
    assert_eq!(tally.keyed.len(), 10);
}

/// AC gap: QSL-3074. Provenance: QSL-2048. A missing standard fails with a
/// message, and no run passes with zero cases.
#[trace("FR-350")]
#[test]
fn a_missing_standard_fails_clearly_and_never_runs_zero_cases() {
    let missing = std::path::Path::new("/nonexistent/fspec-dir-for-the-self-test");
    let error = fspec_standard::read_from(Some(missing.as_os_str()), "proposals/x")
        .expect_err("a missing FSPEC_DIR fails");
    assert!(
        error.contains("FSPEC_DIR=") && error.contains("is not a directory"),
        "{error}"
    );
    let empty = tempfile::tempdir().expect("a temporary directory");
    let error = fspec_standard::read_from(
        Some(empty.path().as_os_str()),
        &format!("{ROOT}typing-cases.json"),
    )
    .expect_err("a directory without the fixtures fails");
    assert!(error.contains("does not hold"), "{error}");
    let tally = fspec_standard::settle(RUNNER, Vec::new(), fspec_standard::registry());
    assert!(tally.matched.is_empty());
}
