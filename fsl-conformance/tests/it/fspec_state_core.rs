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
//!   a case named `source/<file>` checks the verbatim source as published;
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
        AdmissionFailure::Fault(fault) => (false, format!("fault:{fault:?}")),
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
        ClauseDisposition::Evaluate(CallOutcome::Incomplete(incomplete)) => {
            match incomplete.limits_field() {
                "work_units" => Observed::Incomplete("resource_exhausted".to_owned()),
                other => Observed::Incomplete(other.to_owned()),
            }
        }
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

/// Boundary and typing fixtures state a diagnostic code and an absent value,
/// not a category: the codes are compared.
fn compare_code(id: &str, wanted: &Observed, actual: &Observed) -> Settled {
    match (code_only(wanted), code_only(actual)) {
        (Some(wanted_code), Some(actual_code)) if wanted_code == actual_code => Settled::Matched,
        _ => Settled::Mismatch(format!("{id}: expected {wanted:?}, actual {actual:?}")),
    }
}

fn compare(id: &str, wanted: &Observed, actual: &Observed) -> Settled {
    if wanted == actual {
        Settled::Matched
    } else {
        Settled::Mismatch(format!("{id}: expected {wanted:?}, actual {actual:?}"))
    }
}

/// The code of a refusal or incomplete result, whichever it is.
fn code_only(observed: &Observed) -> Option<&str> {
    match observed {
        Observed::Refused(code) | Observed::Incomplete(code) => Some(code),
        Observed::Completed(_) | Observed::Other(_) => None,
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
fn model<S: AsRef<str>>(modifies: &[S], may_delete: bool) -> String {
    let mut document: Value = serde_json::from_str(
        &include_str!("../../../examples/config-version/model.semantic-ir.json")
            .replace("example/config-version", PACKAGE),
    )
    .expect("the ConfigVersion example is JSON");
    let fields: Vec<Value> = modifies
        .iter()
        .map(|field| json!(format!("{CONFIG_VERSION}/{}", field.as_ref())))
        .collect();
    document["types"][1]["operations"][0]["frame"]["modifies"] = Value::Array(fields);
    if may_delete {
        document["types"][1]["operations"][0]["frame"]["deletes"] = json!([CONFIG_VERSION]);
    }
    document.to_string()
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
            let operands = case["operands"]
                .as_array()
                .ok_or("identity-equality needs operands")?;
            let argument = |parameter: &str, key: &Value| ClauseArgument {
                parameter: parameter.to_owned(),
                value: ClauseArgumentValue::Reference {
                    population: POPULATION.to_owned(),
                    key: key.as_str().unwrap_or_default().to_owned(),
                },
            };
            ClauseRunSelection::Function {
                name: "sameIdentity".to_owned(),
                arguments: vec![argument("a", &operands[0]), argument("b", &operands[1])],
                snapshot: identity.clone(),
                snapshot_path: None,
            }
        }
        Some(other) => return Ok(Settled::Unsupported(format!("{id}: primitive {other}"))),
    };
    let actual = run(Run {
        unit,
        model: Some(model(&["versionNumber"], false)),
        snapshots: BTreeMap::from([(identity, bytes)]),
        invocations: BTreeMap::new(),
        selection,
        work: WORK,
    })?;
    Ok(compare(
        id,
        &expected_observation(&case["expected"])?,
        &actual,
    ))
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
        model: Some(model(&changed, !deleted.is_empty())),
        snapshots,
        invocations: BTreeMap::from([(invocation.clone(), document.to_string().into_bytes())]),
        selection: ClauseRunSelection::Clause(ClauseSelection {
            name: clause.to_owned(),
            input: ClauseSelectionInput::Invocation { invocation },
        }),
        work: WORK,
    })?;
    Ok(compare(
        id,
        &expected_observation(&case["expected"])?,
        &actual,
    ))
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
        Err(fault) => Observed::Other(format!("fault {fault:?}")),
    }
}

fn run_boundary(case: &Value) -> Result<Settled, String> {
    let id = id_of(case)?;
    let wanted = Observed::Refused(
        case["expectedDiagnostic"]
            .as_str()
            .ok_or("a boundary case names its diagnostic")?
            .to_owned(),
    );
    let published = text("parent-order.native")?;
    let good = modern(&published);
    let model_json = model(&["versionNumber"], false);
    let actual = match id {
        "unknown-language-profile" => check_unit(
            &good.replace("filament.model.complete/v1", "filament.model.complete/v999"),
            Some(model_json),
        )?,
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
            let mutated = model_json.replace(
                &format!("\"typeRef\":\"{CONFIG_VERSION}\""),
                "\"typeRef\":\"ix://foreign/config/type/Hidden\"",
            );
            check_unit(&good, Some(mutated))?
        }
        "ambiguous-export" => {
            let mut document: Value =
                serde_json::from_str(&model_json).map_err(|error| error.to_string())?;
            let duplicate = document["types"][1].clone();
            document["types"]
                .as_array_mut()
                .ok_or("types")?
                .push(duplicate);
            check_unit(&good, Some(document.to_string()))?
        }
        "unbounded-integer" => {
            let mut document: Value =
                serde_json::from_str(&model_json).map_err(|error| error.to_string())?;
            document["types"][0]["constraints"]
                .as_array_mut()
                .ok_or("constraints")?
                .pop();
            check_unit(&good, Some(document.to_string()))?
        }
        "wrong-invocation" | "deleted-object" | "exhausted-evaluation" => return run_future_input(id, &wanted),
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
    Ok(compare_code(id, &wanted, &actual))
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
        "deleted-object" => (
            objects(true, json!({"type": FIXTURE_TYPE, "object": "gone"})),
            WORK,
        ),
        "exhausted-evaluation" => (objects(true, Value::Null), 0),
        _ => {
            return Ok(Settled::Unsupported(format!(
                "{id}: the snapshot wire carries no invocation member, so pre and post cannot name different invocations"
            )))
        }
    };
    let actual = run(Run {
        unit,
        model: Some(model(&["versionNumber"], false)),
        snapshots: BTreeMap::from([(identity.clone(), snapshot(&document, "current", &identity))]),
        invocations: BTreeMap::new(),
        selection: current_selection(clause, &identity, Some("a")),
        work,
    })?;
    Ok(compare_code(id, wanted, &actual))
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
    let compiled = check_unit(&unit, Some(model(&["versionNumber"], false)))?;
    let wanted_typing = expected["typing"].as_str().unwrap_or_default();
    if wanted_typing != "accepted-under-hypotheses" {
        let wanted = Observed::Refused(
            expected["diagnostic"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        );
        return Ok(compare_code(id, &wanted, &compiled));
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
        model: Some(model(&["versionNumber"], false)),
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
    Ok(compare(id, &wanted, &actual))
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
            let settled = run(case).unwrap_or_else(|error| Settled::Mismatch(error));
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
                .and_then(|source| check_unit(&source, Some(model(&["versionNumber"], false))))
                .map_or_else(Settled::Mismatch, |observed| match observed {
                    Observed::Completed(true) => Settled::Matched,
                    other => Settled::Mismatch(format!(
                        "source/{file}: the published source should parse and check, actual {other:?}"
                    )),
                });
            (format!("source/{file}"), settled)
        })
        .collect()
}

/// Provenance: QSL-2048. Every state-core case runs through the compiler.
#[trace("FR-350-AC-20", "FR-350-AC-21")]
#[test]
fn conformance_fspec_state_core_source_cases() {
    let files = load().unwrap_or_else(|error| panic!("{error}"));
    let listed: usize = files.iter().map(|file| file.cases.len()).sum();
    let mut results = run_all(&files);
    assert_eq!(results.len(), listed, "a case was not executed or reported");
    results.extend(run_sources());
    let tally = fspec_standard::settle(RUNNER, results, fspec_standard::registry());
    for id in &tally.matched {
        println!("passed: {id}");
    }
    if let Err(error) = fspec_standard::report(RUNNER, &tally) {
        panic!("{error}");
    }
    println!(
        "conformance: {} state-core source cases matched",
        tally.matched.len()
    );
}

/// Provenance: QSL-2048. Executed plus reported-skipped cases equal the
/// cases the fixtures hold, so a case cannot disappear silently.
#[trace("FR-350-AC-20")]
#[test]
fn executed_and_reported_cases_equal_the_cases_in_the_fixtures() {
    let files = load().unwrap_or_else(|error| panic!("{error}"));
    let counts: Vec<(&str, usize)> = files.iter().map(|f| (f.name, f.cases.len())).collect();
    assert_eq!(
        counts,
        [
            ("typing", 51),
            ("state", 9),
            ("boundary", 17),
            ("reference", 10),
            ("invocation", 7)
        ],
        "the published fixtures changed shape; the count is the standard's"
    );
    let results = run_all(&files);
    let skipped = results
        .iter()
        .filter(|(_, settled)| matches!(settled, Settled::Unsupported(_)))
        .count();
    let executed = results.len() - skipped;
    let listed: usize = counts.iter().map(|(_, count)| count).sum();
    assert_eq!(executed + skipped, listed);
    for (id, settled) in &results {
        if let Settled::Unsupported(reason) = settled {
            println!("reported-skipped: {id}: {reason}");
        }
    }
}

/// Provenance: QSL-2048. An altered expected code is a mismatch in each file.
#[trace("FR-350-AC-20")]
#[test]
fn an_altered_expected_code_is_a_mismatch_in_each_file() {
    let files = load().unwrap_or_else(|error| panic!("{error}"));
    for file in &files {
        let run = runner_for(file.name);
        let altered = |case: &Value| -> Value {
            let mut case = case.clone();
            for key in ["expected", "expectedDiagnostic"] {
                match &mut case[key] {
                    Value::String(code) => *code = "altered_code".to_owned(),
                    Value::Object(expected) => {
                        expected.insert("diagnostic".into(), json!("altered_code"));
                        expected.insert("execution".into(), json!("refused"));
                        expected.remove("value");
                    }
                    _ => {}
                }
            }
            case
        };
        // The first case the runner executes. A file whose cases are all
        // reported unsupported has none: its altered case must still not
        // match, and it is reported unsupported, not matched.
        let executed = file
            .cases
            .iter()
            .find(|case| matches!(run(case), Ok(Settled::Matched) | Ok(Settled::Mismatch(_))))
            .unwrap_or(&file.cases[0]);
        let result = run(&altered(executed)).expect("the altered case runs");
        let reported = matches!(result, Settled::Mismatch(_))
            || (file.name == "reference" && matches!(result, Settled::Unsupported(_)));
        assert!(
            reported,
            "{}: an altered expected code was not reported: {result:?}",
            file.name
        );
    }
}

/// Provenance: QSL-2048. A missing standard fails with a message, and no
/// run passes with zero cases.
#[trace("FR-350-AC-20")]
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
