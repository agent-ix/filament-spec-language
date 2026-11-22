// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-850 "Equality with the source route", FR-940 (QSL-2765): a checked
//! package whose domain package declares a payload-free enumeration is
//! analyzed from the package bytes alone with the source route's items,
//! aggregate, last stage and exit code, and a package whose enumeration rows
//! the strict reader cannot read is refused `invalid_package`.

use fsl_semantics::model::intake::meaning;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, label, package_selection, request_over, Run, Workspace, HANDOFF,
};

const PACKAGE: &str = "test/machine";

fn ident(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

fn origin(identity: &str) -> Value {
    json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
        "inputIdentities": [identity]}})
}

fn wire_type(identity: &str, kind: &str, extra: Value) -> Value {
    let mut node = json!({
        "identity": identity, "displayName": identity,
        "kind": {"module": PACKAGE, "name": kind}, "roles": [],
        "origin": origin(identity), "constraints": [], "extensions": [],
        "unknownPolicy": "reject",
    });
    for (name, value) in extra.as_object().expect("extra members") {
        node[name] = value.clone();
    }
    node
}

fn operation(name: &str) -> Value {
    let identity = ident(&format!("Machine/{name}"));
    json!({
        "identity": identity, "name": name, "params": [], "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.fspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {"modifies": [ident("Machine/mode")], "creates": [], "deletes": []},
    })
}

/// The `test/machine` domain package: an enumeration `Mode` whose cases are
/// authored out of canonical (byte) order, a population `machines` of
/// `Machine` with the field `mode: Mode` and the operations `start`,
/// `finish` and `stop`.
fn document() -> Vec<u8> {
    let constructs: Vec<Value> = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
    ]
    .into_iter()
    .map(|(name, meaning)| {
        json!({"kind": {"module": PACKAGE, "name": name}, "moduleVersion": "1.0.0",
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    })
    .chain([json!({"kind": {"module": PACKAGE, "name": "variant_type"},
        "moduleVersion": "1.0.0",
        "construct": {"identity": "none", "shape": "enumeration",
            "members": {"variants": "required"}, "meaning": meaning::VARIANT_TYPE}})])
    .collect();
    let cases: Vec<Value> = ["Idle", "Busy", "Done"]
        .iter()
        .enumerate()
        .map(|(line, case)| {
            json!({"identity": ident(&format!("variant/Mode-{case}")), "name": case,
                "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.fspec",
                    "startLine": line + 1, "startColumn": 3}}})
        })
        .collect();
    let mode = ident("Machine/mode");
    let population = ident("machines");
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": ident("spec"), "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {"identity": PACKAGE, "version": "1.0.0",
            "mappingVersions": [], "profileVersions": []},
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": [
            wire_type(&ident("Mode"), "variant_type", json!({"variants": cases})),
            wire_type(&ident("Machine"), "object_type", json!({
                "supertypes": [],
                "fields": [{
                    "identity": mode, "name": "mode", "typeRef": ident("Mode"),
                    "presence": "required", "nullable": false, "defaultKind": "none",
                    "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                    "origin": origin(&mode),
                }],
                "operations": [operation("start"), operation("finish"), operation("stop")],
                "relationships": [],
            })),
        ],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [ident("Machine")], "extent": "closed", "origin": origin(&population),
        }],
    })
    .to_string()
    .into_bytes()
}

/// The machine `a` at the case `case` of `Mode`, as a `start` snapshot.
fn snapshot(case: &str) -> Vec<u8> {
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "start"},
        "observation": "current", "model": {"identity": PACKAGE},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": ident("machines"), "complete": true,
            "objects": [{"key": "a", "type": ident("Machine"),
                "fields": {"mode": {"variant": {"type": ident("Mode"), "case": case}}}}]}],
    })
    .to_string()
    .into_bytes()
}

/// The unit selecting the machine as `M`: `start`, `finish` and `stop` walk
/// `Idle`, `Busy`, `Done` and back, and `claims` follow.
fn unit(claims: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile t0 = \"filament.temporal.infinite-trace/v1\";\n\
         model M = \"{PACKAGE}\";\n\
         pre CanStart using v on M::Machine::start {{ self.mode = M::Mode::Idle }}\n\
         post Started using v on M::Machine::start {{ self.mode = M::Mode::Busy }}\n\
         pre CanFinish using v on M::Machine::finish {{ self.mode = M::Mode::Busy }}\n\
         post Finished using v on M::Machine::finish {{ self.mode = M::Mode::Done }}\n\
         pre CanStop using v on M::Machine::stop {{ self.mode = M::Mode::Done }}\n\
         post Stopped using v on M::Machine::stop {{ self.mode = M::Mode::Idle }}\n\
         {claims}"
    )
}

/// The state-graph claims and an invariant the model keeps.
const CLAIMS: &str = concat!(
    "possible ReachDone using v over (m: M::Machine) { m.mode = M::Mode::Done }\n",
    "always possible BackToIdle using v over (m: M::Machine) { m.mode = M::Mode::Idle }\n",
    "invariant KnownMode using v on M::Machine at current { self.mode = M::Mode::Idle or self.mode = M::Mode::Busy or self.mode = M::Mode::Done }\n",
);

fn request_of(path: &std::path::Path) -> Value {
    let document: Value =
        serde_json::from_slice(&std::fs::read(path).expect("the request")).expect("JSON");
    document["request"].clone()
}

fn analyze_members(ws: &Workspace, name: &str, members: &Value) -> Run {
    let document = json!({"format": "native-analyze/1", "request": members});
    analyze(&ws.write(name, document.to_string().as_bytes()))
}

/// The unit `claims` analyzed with `items` over a `case` snapshot, from
/// source and from its handoff directory: the workspace, the package
/// request members and both runs.
fn both_routes(
    name: &str,
    case: &str,
    claims: &str,
    items: &Value,
) -> (Workspace, Value, Run, Run) {
    let ws = Workspace::new(name);
    let path = request_over(
        &ws,
        &unit(claims),
        &document(),
        &[snapshot(case)],
        json!({"items": items}),
    );
    let source = analyze(&path);
    let (members, _) = package_selection(&ws, &request_of(&path));
    let package = analyze_members(&ws, "analyze-package.json", &members);
    (ws, members, source, package)
}

fn assert_equal_documents(source: &Run, package: &Run) {
    for member in ["items", "aggregate", "last_stage", "category", "registry"] {
        assert_eq!(
            package.document()[member],
            source.document()[member],
            "{member}: {}",
            package.document()
        );
    }
    assert_eq!(package.exit, source.exit);
}

/// FR-850-AC-2, FR-940-AC-11: the two state-graph claims and the invariant over an enumeration field settle from the package with the
/// source route's items, none `unsupported`.
#[trace("TC-2410", "FR-850-AC-2")]
#[trace("TC-3034", "FR-940-AC-11")]
#[test]
fn a_package_with_an_enumeration_settles_its_claims_as_the_source_route() {
    let names = ["ReachDone", "BackToIdle", "KnownMode"];
    let items = json!(names.map(|name| json!({"claim": name})));
    let (_ws, _, source, package) = both_routes("package-enum-claims", "Idle", CLAIMS, &items);
    assert_eq!(label(&source.item(2)), "proved", "{}", source.document());
    for (index, name) in names.iter().enumerate() {
        assert_eq!(package.item(index), source.item(index), "{name}");
        assert!(
            !package.item(index).to_string().contains("unsupported"),
            "{name}: {}",
            package.item(index)
        );
    }
    assert_equal_documents(&source, &package);
}

/// A snapshot naming a case the enumeration does not declare is refused by
/// both routes with the same typed document.
#[trace("TC-2410", "FR-850-AC-2")]
#[test]
fn an_unknown_enumeration_case_in_a_package_request_refuses_as_the_source_route() {
    let items = json!([{"claim": "KnownMode"}]);
    let (_ws, _, source, package) = both_routes("package-enum-unknown", "Nope", CLAIMS, &items);
    assert_ne!(source.exit, 0, "{}", source.document());
    assert_eq!(package.exit, source.exit, "{}", package.document());
    assert_eq!(package.document(), source.document());
    assert!(
        !package
            .document()
            .to_string()
            .contains("not-yet-implemented"),
        "{}",
        package.document()
    );
}

/// A package whose enumeration shape rows are malformed is refused by the
/// strict reader at the request, before any engine runs.
#[trace("TC-2410", "FR-850-AC-3")]
#[test]
fn malformed_enumeration_rows_refuse_invalid_package() {
    let items = json!([{"claim": "KnownMode"}]);
    let (ws, members, _, _) = both_routes("package-enum-malformed", "Idle", CLAIMS, &items);
    let path = ws.path(&format!("{HANDOFF}/identities.json"));
    let good: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("identities")).expect("JSON");
    assert_eq!(
        good["model_enum_shapes"].as_array().expect("rows").len(),
        1,
        "{good}"
    );
    let mut repeated = good.clone();
    let first = repeated["model_enum_shapes"][0]["cases"][0].clone();
    repeated["model_enum_shapes"][0]["cases"]
        .as_array_mut()
        .expect("cases")
        .push(first);
    let mut dropped = good.clone();
    dropped["model_enum_shapes"][0]["cases"]
        .as_array_mut()
        .expect("cases")
        .pop();
    let mut emptied = good;
    emptied["model_enum_shapes"] = json!([]);
    for (name, bad) in [
        ("repeated", repeated),
        ("dropped", dropped),
        ("emptied", emptied),
    ] {
        std::fs::write(&path, bad.to_string()).expect("write");
        let run = analyze_members(&ws, &format!("analyze-{name}.json"), &members);
        let diagnostic = run.document()["diagnostics"][0].clone();
        assert_eq!(diagnostic["code"], "invalid_package", "{name}: {diagnostic}");
        assert_eq!(run.document()["items"], json!([]), "{name}");
    }
}
