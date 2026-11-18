// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-862 (TC-2443), FSpec FR-715 (FSpec TC-1750): `command::run` reads a
//! `native-replay/1` request file in FSpec FR-336's replay request encoding.
//! A replay reads its clock configurations from the proving package, so a
//! `clocks` member is refused at stage `request`, naming it.

use crate::command_harness::{compile_file, run_file, Output};
use fsl_foundation::SourceIdentity;
use fsl_replay::{
    call_site, replay, DependencyEntryWire, DependencyInput, QualifiedName, ReplayLimits,
    ReplayPackage, ReplayRequestWire, ReplaySource, RequestTargetWire, StateEnvironment,
    SuppliedLibrary,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn write(directory: &Path, name: &str, request: &Value) -> PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

fn refusal(output: &Output) -> Value {
    assert!(output.stdout.is_empty(), "expected no result document");
    serde_json::from_slice(&output.stderr).expect("a refusal envelope")
}

/// A replay request object in FSpec FR-336's mapping, with `extra` merged
/// into it. Its members are well formed in shape; the refusals below are
/// decided before the package is admitted.
fn replay_request(extra: Value) -> Value {
    let mut request = json!({
        "profile_selections": [],
        "package": {},
        "dependencies": [],
        "replay_target": {"members": {"function": "entry"}},
        "source": {"kind": "input", "assignments": []},
        "obligation_identity": {"node_id": {}, "model": null, "ordinal": "0"},
        "backend": "fsl",
        "state_environment": [],
        "accounting_limits": {},
        "stage_limits": {},
        "supplied_limits": null,
        "object_environment": null,
        "declared_domains": [],
        "byte_provision": []
    });
    if let (Some(members), Some(added)) = (request.as_object_mut(), extra.as_object()) {
        for (name, value) in added {
            members.insert(name.clone(), value.clone());
        }
    }
    json!({"format": "native-replay/1", "request": request})
}

/// The request-stage refusal naming `member`, never the unknown-format
/// refusal a reader without the replay format gives.
fn assert_names(output: &Output, member: &str) {
    assert_eq!(output.status.code(), Some(20));
    let envelope = refusal(output);
    assert_eq!(envelope["stage"], "request", "{envelope}");
    assert_eq!(envelope["code"], "invalid-request", "{envelope}");
    assert!(
        envelope.to_string().contains(&format!("\"{member}\""))
            || envelope.to_string().contains(&format!("/{member}")),
        "the refusal names {member}: {envelope}"
    );
    assert!(
        !envelope.to_string().contains("native-replay/1 is not"),
        "a replay request is not an unknown format: {envelope}"
    );
}

/// FR-862-AC-1, FSpec FR-715-AC-1 (negative): a `clocks` member, an unknown
/// member and an empty request each refuse at stage `request`.
#[test]
#[trace("TC-2443", "FR-862-AC-1", "FSpec-TC-1750", "FSpec-FR-715-AC-1")]
fn a_replay_request_file_refuses_a_clocks_member() {
    let directory = tempfile::tempdir().unwrap();
    let clocks = replay_request(json!({"clocks": {"plant": {"kind": "timed", "unit": "ms"}}}));
    assert_names(
        &run_file(&write(directory.path(), "clocks.json", &clocks)),
        "clocks",
    );
    let extra = replay_request(json!({"extra": true}));
    assert_names(
        &run_file(&write(directory.path(), "extra.json", &extra)),
        "extra",
    );
    let empty = json!({"format": "native-replay/1", "request": {}});
    assert_names(
        &run_file(&write(directory.path(), "empty.json", &empty)),
        "profile_selections",
    );
}

const AUTHORITY: &str = "agent-ix";
const PROGRAM_ID: &str = "test:clocks-program";
const LIBRARY_ID: &str = "test:clocks-plant";
const LIBRARY: &str = "test/plant";
const PROGRAM: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     import \"test/plant\" as P;\n\
     function entry using v(): Boolean pure { P::flag() }\n";
/// A program over the same library whose function settles without calling it.
const OWN_VALUE: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     import \"test/plant\" as P;\n\
     function entry using v(): Boolean pure { true }\n";
const LIBRARY_HEAD: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
     record Reading { level: Int[0, 9]; }\n\
     function flag using v(): Boolean pure { true }\n";
/// FR-640-AC-10's library clause, on a clock only the library uses.
const LIBRARY_CLAUSE: &str = "temporal Late using w over (r: Reading) clock \"lib-ms\" \
     on origin { eventually[0,5] holds(r.level > 0) }\n";

fn source(file: &str, identity: &str) -> Value {
    json!({"file": file, "authority": AUTHORITY, "identity": identity,
        "document": "Clocks", "formal_revision": 1})
}

/// One replay of `entry` over the package `command::compile` writes for
/// `program` and `library`, in both forms: the `native-replay/1` request
/// file and the in-process request the same members construct.
struct Fixture {
    directory: tempfile::TempDir,
    package: Value,
    program: &'static str,
    library: String,
}

impl Fixture {
    fn compile(program: &'static str, library: &str, clocks: Option<Value>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("unit.native"), program).unwrap();
        std::fs::write(directory.path().join("plant.native"), library).unwrap();
        let mut request = json!({
            "models": [],
            "program": {"source": source("unit.native", PROGRAM_ID)},
            "libraries": [{"identity": LIBRARY, "source": source("plant.native", LIBRARY_ID)}]});
        if let Some(clocks) = clocks {
            request["clocks"] = clocks;
        }
        let path = write(
            directory.path(),
            "compile.json",
            &json!({"format": "native-compile/1", "request": request}),
        );
        let output = compile_file(&path);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Self {
            package: serde_json::from_slice(&output.stdout).unwrap(),
            directory,
            program,
            library: library.to_owned(),
        }
    }

    /// The function call site of `entry`. `call_site` emits with no clock
    /// selections, so the site is taken over the library without its clause:
    /// the function's key does not depend on the library's clauses.
    fn site(&self) -> fsl_replay::CallSite<fsl_replay::FunctionSite> {
        let dependencies = DependencyInput::new([SuppliedLibrary {
            identity: LIBRARY.to_owned(),
            source: SourceIdentity::new(AUTHORITY, LIBRARY_ID),
            path: "plant.native".to_owned(),
            bytes: LIBRARY_HEAD.as_bytes().to_vec(),
        }])
        .unwrap();
        call_site(
            SourceIdentity::new(AUTHORITY, PROGRAM_ID),
            self.package["name"].as_str().unwrap().to_owned(),
            "unit.native",
            self.program.as_bytes(),
            std::iter::empty::<&[u8]>(),
            &dependencies,
            &entry(),
        )
        .expect("the program's call site resolves")
    }

    /// The `native-replay/1` request over `package`, in FSpec FR-336's
    /// mapping, with the sources in `byte_provision` files.
    fn request_file(&self, package: &Value) -> PathBuf {
        let site = self.site();
        let graph = fsl_replay::WireGraph::of_package_value(package).unwrap();
        let occurrence = &site.site.declaration;
        let obligation = site.site.obligation_identity();
        let request = json!({
            "profile_selections": [],
            "package": package,
            "dependencies": [{"identity": LIBRARY,
                "sources": [{"authority": AUTHORITY, "identity": LIBRARY_ID}]}],
            "replay_target": {
                "members": {"function": "entry"},
                "arguments": false,
                "occurrence_key": {
                    "node_id": graph.wire_value(occurrence.node()).unwrap(),
                    "role": occurrence.origin().role().as_str(),
                    "ordinal": occurrence.origin().ordinal().to_string()},
                "over": null},
            "source": {"kind": "input", "assignments": []},
            "obligation_identity": {
                "node_id": graph.wire_value(obligation.node()).unwrap(),
                "model": null,
                "ordinal": obligation.ordinal()},
            "backend": "fr-862-ac-2",
            "state_environment": [],
            "accounting_limits": {},
            "stage_limits": {},
            "supplied_limits": null,
            "object_environment": null,
            "declared_domains": [],
            "byte_provision": [
                {"authority": AUTHORITY, "identity": PROGRAM_ID, "bytes_path": "unit.native"},
                {"authority": AUTHORITY, "identity": LIBRARY_ID, "bytes_path": "plant.native"}]
        });
        write(
            self.directory.path(),
            "replay.json",
            &json!({"format": "native-replay/1", "request": request}),
        )
    }

    /// The same replay constructed in process (FR-640-AC-11's request).
    fn wire(&self, package: &Value) -> ReplayRequestWire {
        let site = self.site();
        ReplayRequestWire {
            package: canonical(package),
            profile_selections: Vec::new(),
            dependencies: vec![DependencyEntryWire {
                identity: LIBRARY.to_owned(),
                sources: vec![(AUTHORITY.to_owned(), LIBRARY_ID.to_owned())],
                refinement_internals: None,
            }],
            replay_target: RequestTargetWire::function(entry(), site.site.declaration.clone()),
            source: ReplaySource::Input(Vec::new()),
            obligation_identity: site.site.obligation_identity().clone(),
            backend: "fr-862-ac-2".to_owned(),
            state_environment: StateEnvironment::default(),
            accounting_limits: fsl_replay::AccountingLimits::default().0,
            stage_limits: BTreeMap::new(),
            supplied_limits: None,
            object_environment: None,
            declared_domains: Vec::new(),
            byte_provision: vec![
                (
                    AUTHORITY.to_owned(),
                    PROGRAM_ID.to_owned(),
                    self.program.as_bytes().to_vec(),
                ),
                (
                    AUTHORITY.to_owned(),
                    LIBRARY_ID.to_owned(),
                    self.library.as_bytes().to_vec(),
                ),
            ],
            documents: Vec::new(),
        }
    }

    /// Run the request file through `command::run`, check its result is the
    /// in-process replay's, and return that result row.
    fn assert_replays_as_in_process(&self) -> Value {
        let output = run_file(&self.request_file(&self.package));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let document: Value = serde_json::from_slice(&output.stdout).unwrap();
        let expected = replay(self.wire(&self.package), ReplayLimits::default())
            .expect("the in-process replay settles");
        let expected: Value = serde_json::from_str(&expected.to_function_json().unwrap()).unwrap();
        assert_eq!(document, json!({"results": [expected]}));
        document["results"][0].clone()
    }
}

fn entry() -> QualifiedName {
    QualifiedName::new(vec![filament_exact::Identifier::new("entry").unwrap()]).unwrap()
}

fn canonical(package: &Value) -> ReplayPackage {
    let encoded = serde_json::to_vec(package).unwrap();
    let document = quire_canonical::read(&encoded, ReplayLimits::default().input_bytes).unwrap();
    let bytes = quire_canonical::to_vec(
        &document.root(),
        quire_canonical::Limits::new(ReplayLimits::default().input_bytes),
    )
    .unwrap();
    ReplayPackage::read(&bytes, ReplayLimits::default().input_bytes).unwrap()
}

/// FR-862 (TC-2443): a complete FR-336 request read through `command::run`
/// replays to the value the in-process replay of the same members gives,
/// for a library whose clauses need no clock configuration.
#[test]
#[trace("TC-2443")]
fn a_replay_request_file_replays_as_the_in_process_request_does() {
    let row = Fixture::compile(OWN_VALUE, LIBRARY_HEAD, None).assert_replays_as_in_process();
    assert_eq!(row["arm"], "input", "{row}");
    assert!(row["settlement"].is_string(), "{row}");
}

/// FR-862-AC-2: FR-640-AC-11's replay of `entry`, read from a request file,
/// settles as the in-process replay does, and with the package's `lib-ms`
/// dependency clock selection removed refuses `missing-clock-selection`.
#[test]
#[trace("TC-2443", "FR-862-AC-2")]
#[ignore = "QSL-2274: FR-640-AC-11's program does not compile yet and replay does not read library clock selections from the proving package"]
fn a_replay_request_file_replays_a_library_clock_from_the_package() {
    let fixture = Fixture::compile(
        PROGRAM,
        &format!("{LIBRARY_HEAD}{LIBRARY_CLAUSE}"),
        Some(json!({"lib-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}})),
    );
    let row = fixture.assert_replays_as_in_process();
    assert_eq!(row["settlement"], "reproduced-without-witness", "{row}");
    assert_eq!(row["value"], true, "{row}");
    let mut missing = fixture.package.clone();
    missing["lock"]
        .as_object_mut()
        .unwrap()
        .remove("dependency_clock_selections");
    let output = run_file(&fixture.request_file(&missing));
    assert_ne!(output.status.code(), Some(0));
    let envelope = refusal(&output).to_string();
    for named in ["missing-clock-selection", "lib-ms", LIBRARY] {
        assert!(envelope.contains(named), "names {named}: {envelope}");
    }
}

/// FR-286-AC-21: a replay's outcome document carries the `replayed` result,
/// each entry the `results` document's row.
#[test]
#[trace("TC-2963", "FR-286-AC-21")]
#[ignore = "QSL-2292: the replay outcome document carries no result member yet"]
fn a_replay_outcome_carries_and_renders_its_results() {
    let fixture = Fixture::compile(OWN_VALUE, LIBRARY_HEAD, None);
    let path = fixture.request_file(&fixture.package);
    let result = fsl_command::command::run(&path, &crate::negotiation::Fr290).expect("replays");
    let rows = result.value.as_value()["results"].clone();
    let outcome = result.outcome.expect("an outcome document");
    let document: Value = serde_json::from_slice(&outcome.to_bytes().expect("serializes")).unwrap();
    assert_eq!(document["result"]["kind"], "replayed", "{document}");
    assert_eq!(document["result"]["results"], rows, "{document}");
}
