// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-640-AC-11: a replay recompiles a library whose clause uses a clock of
//! its own under the configuration the proving package records in its lock
//! `dependency_clock_selections`, and refuses when that record is missing,
//! holds an unused entry or holds an entry of the wrong kind.
use crate::command_harness::compile_file;
use fsl_foundation::SourceIdentity;
use fsl_replay::{
    call_site, replay, DependencyEntryWire, DependencyInput, QualifiedName, ReplayLimits,
    ReplayPackage, ReplayRefusal, ReplayRequestWire, ReplayResult, ReplaySource,
    RequestTargetWire, StateEnvironment, SuppliedLibrary,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

const AUTHORITY: &str = "agent-ix";
const PROGRAM_ID: &str = "test:clocks-program";
const LIBRARY_ID: &str = "test:clocks-plant";
const LIBRARY: &str = "test/plant";
const PROGRAM: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     import \"test/plant\" as P;\n\
     function entry using v(): Boolean pure { P::flag() }\n";
const LIBRARY_HEAD: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
     record Reading { level: Int[0, 9]; }\n\
     function flag using v(): Boolean pure { true }\n";
const LIBRARY_CLAUSE: &str = "temporal Late using w over (r: Reading) clock \"lib-ms\" \
     on origin { eventually[0,5] holds(r.level > 0) }\n";

fn source(file: &str, identity: &str) -> Value {
    json!({"file": file, "authority": AUTHORITY, "identity": identity,
        "document": "Clocks", "formal_revision": 1})
}

fn library_text() -> String {
    format!("{LIBRARY_HEAD}{LIBRARY_CLAUSE}")
}

/// The proving package `command::compile` writes for the program and the
/// library whose clause alone uses `lib-ms`.
fn proving_package(directory: &Path) -> Vec<u8> {
    std::fs::write(directory.join("unit.native"), PROGRAM).unwrap();
    std::fs::write(directory.join("plant.native"), library_text()).unwrap();
    let request = json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": source("unit.native", PROGRAM_ID)},
        "libraries": [{"identity": LIBRARY, "source": source("plant.native", LIBRARY_ID)}],
        "clocks": {"lib-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}}}});
    let path = directory.join("compile.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let output = compile_file(&path);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
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

/// A `function` replay of `entry` over `package`, with `test/plant` as the
/// one `dependencies` entry and no clock configuration in the request.
fn request(package: &Value) -> ReplayRequestWire {
    let name = package["name"].as_str().expect("a package name").to_owned();
    let selected = QualifiedName::new(vec![filament_exact::Identifier::new("entry").unwrap()])
        .unwrap();
    // `call_site` emits with no clock selections, so the site is taken from
    // the same program over the library without its clause: the function's
    // key does not depend on the library's clauses.
    let dependencies = DependencyInput::new([SuppliedLibrary {
        identity: LIBRARY.to_owned(),
        source: SourceIdentity::new(AUTHORITY, LIBRARY_ID),
        path: "plant.native".to_owned(),
        bytes: LIBRARY_HEAD.as_bytes().to_vec(),
    }])
    .unwrap();
    let located = call_site(
        SourceIdentity::new(AUTHORITY, PROGRAM_ID),
        name,
        "unit.native",
        PROGRAM.as_bytes(),
        std::iter::empty::<&[u8]>(),
        &dependencies,
        &selected,
    )
    .expect("the program's call site resolves");
    ReplayRequestWire {
        package: canonical(package),
        profile_selections: Vec::new(),
        dependencies: vec![DependencyEntryWire {
            identity: LIBRARY.to_owned(),
            sources: vec![(AUTHORITY.to_owned(), LIBRARY_ID.to_owned())],
            refinement_internals: None,
        }],
        replay_target: RequestTargetWire::function(selected, located.site.declaration.clone()),
        source: ReplaySource::Input(Vec::new()),
        obligation_identity: located.site.obligation_identity().clone(),
        backend: "fr-640-ac-11".to_owned(),
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
                PROGRAM.as_bytes().to_vec(),
            ),
            (
                AUTHORITY.to_owned(),
                LIBRARY_ID.to_owned(),
                library_text().into_bytes(),
            ),
        ],
        documents: Vec::new(),
    }
}

fn refused(package: &Value, cause: &str, pointer: &str) {
    let refusal = replay(request(package), ReplayLimits::default())
        .expect_err("the recorded clocks do not serve the library");
    assert!(
        matches!(refusal, ReplayRefusal::Recompile { .. }),
        "{refusal:?}"
    );
    assert!(format!("{refusal:?}").contains(cause), "{refusal:?}");
    assert_eq!(refusal.pointer().as_deref(), Some(pointer), "{refusal:?}");
}

#[trace("TC-2213", "FR-640-AC-11", "FSpec-TC-1662", "FSpec-FR-323-AC-38")]
#[test]
#[ignore = "QSL-2274: replay does not read library clock selections from the proving package yet"]
fn replay_recovers_a_library_only_clock_from_the_proving_package() {
    let directory = tempfile::tempdir().unwrap();
    let package: Value = serde_json::from_slice(&proving_package(directory.path())).unwrap();
    assert_eq!(
        package["lock"]["dependency_clock_selections"],
        json!([{"clock_name": "lib-ms",
            "clock": {"kind": "timestamped_event", "timestamp_unit": "ms"}}])
    );
    let result = replay(request(&package), ReplayLimits::default())
        .expect("the library recompiles under the recorded clock");
    let ReplayResult::Input(input) = result else {
        panic!("an input replay");
    };
    assert_eq!(input.value(), Some(fsl_replay::EvaluatedValue::Boolean(true)));

    let mut missing = package.clone();
    missing["lock"]
        .as_object_mut()
        .unwrap()
        .remove("dependency_clock_selections");
    refused(
        &missing,
        "missing-clock-selection",
        "/package/lock/dependency_clock_selections",
    );

    let mut spare = package.clone();
    spare["lock"]["dependency_clock_selections"]
        .as_array_mut()
        .unwrap()
        .push(json!({"clock_name": "spare",
            "clock": {"kind": "timestamped_event", "timestamp_unit": "ms"}}));
    refused(
        &spare,
        "unused-clock-selection",
        "/package/lock/dependency_clock_selections/1",
    );

    let mut wrong = package;
    wrong["lock"]["dependency_clock_selections"][0]["clock"] = json!({"kind": "fixed_sample",
        "epoch": {"kind": "integer", "decimal": "0"},
        "period": {"kind": "integer", "decimal": "1"}, "unit": "ms"});
    refused(
        &wrong,
        "clock-kind-mismatch",
        "/package/lock/dependency_clock_selections/0/clock/kind",
    );
}
