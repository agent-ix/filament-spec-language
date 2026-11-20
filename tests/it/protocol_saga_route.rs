// SPDX-License-Identifier: AGPL-3.0-or-later
//! The parts-saga protocols compiled from source and carried through
//! deadlock checking, projection and realizability (register row T-073).
//!
//! The sources are vendored from `filament-examples/examples/parts-saga`
//! into `tests/corpus/protocol/`, where `source_corpus::corpus_protocol`
//! also runs them; their requests carry the identity prefix
//! `example:parts-saga/`. Each one belongs to exactly one row of [`SAGAS`]:
//!
//! | Row | Protocols |
//! | -- | -- |
//! | bounded static: deadlock, projection per role, realizability | [`Row::BoundedStatic`] |
//! | refused by compile before analysis | [`Row::Refused`] |
//! | outside bounded static: typed projection refusal (G-111) | [`Row::Unsupported`] |
//! | not a protocol (the `jobs` state rule runs) | [`Row::NotAProtocol`] |

use filament_exact::Cancel;
use fsl_command::command;
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Where one vendored saga program falls.
enum Row {
    /// Compiles; every local projection and realizability is proved, and
    /// deadlock freedom has the stated result.
    BoundedStatic { deadlock: &'static str },
    /// Compile refuses it with this exit code, before any analysis.
    Refused { exit_code: u8 },
    /// Compiles; projecting `role` under bounded static gives the typed
    /// `Unsupported` outcome with this slug (G-111 owns the extension).
    Unsupported {
        role: &'static str,
        slug: &'static str,
    },
    /// A state-rule program in the same example, not a protocol.
    NotAProtocol,
}

const SAGAS: &[(&str, Row)] = &[
    (
        "declaration-minimal",
        Row::BoundedStatic { deadlock: "proved" },
    ),
    (
        "declaration-terminal-scheduling",
        Row::BoundedStatic { deadlock: "proved" },
    ),
    (
        "channels-loss-head",
        Row::BoundedStatic { deadlock: "proved" },
    ),
    (
        "channels-fifo-by-part",
        Row::BoundedStatic { deadlock: "proved" },
    ),
    ("steps-all-kinds", Row::BoundedStatic { deadlock: "proved" }),
    (
        "control-parallel-all",
        Row::BoundedStatic { deadlock: "proved" },
    ),
    (
        "declaration-on-each",
        Row::Unsupported {
            role: "Planning",
            slug: "activation-on-each",
        },
    ),
    (
        "roles-replicated",
        Row::Unsupported {
            role: "Worker",
            slug: "dynamic-role-population",
        },
    ),
    (
        "control-parallel-any-cancel",
        Row::Unsupported {
            role: "Machining",
            slug: "unsupported-join",
        },
    ),
    (
        "control-repeat-progress",
        Row::Unsupported {
            role: "Inspection",
            slug: "repeat",
        },
    ),
    (
        "protocol-compiles",
        Row::Unsupported {
            role: "Inspection",
            slug: "repeat",
        },
    ),
    (
        "inspection-compiles",
        Row::Unsupported {
            role: "Inspection",
            slug: "repeat",
        },
    ),
    (
        "staged-saga-deadlock-free",
        Row::Unsupported {
            role: "Inspection",
            slug: "repeat",
        },
    ),
    (
        "staged-saga-compensation",
        Row::Unsupported {
            role: "Inspection",
            slug: "repeat",
        },
    ),
    (
        "timeouts-await",
        Row::Unsupported {
            role: "Inspection",
            slug: "unsupported-control",
        },
    ),
    ("protocol-wrong-role", Row::Refused { exit_code: 20 }),
    ("protocol-overlapping-verdict", Row::Refused { exit_code: 20 }),
    ("declaration-no-finish", Row::Refused { exit_code: 20 }),
    ("channels-object-payload", Row::Refused { exit_code: 20 }),
    ("steps-effect-of-send", Row::Refused { exit_code: 20 }),
    ("steps-receive-other-channel", Row::Refused { exit_code: 20 }),
    ("steps-unknown-contract", Row::Refused { exit_code: 20 }),
    ("control-join-omits-branch", Row::Refused { exit_code: 20 }),
    (
        "control-join-any-needs-outstanding",
        Row::Refused { exit_code: 20 },
    ),
    ("control-repeat-no-progress", Row::Refused { exit_code: 21 }),
    ("compensation-zero-attempts", Row::Refused { exit_code: 20 }),
    (
        "timeouts-await-without-profile",
        Row::Refused { exit_code: 20 },
    ),
    ("jobs-rework-tracked", Row::NotAProtocol),
    ("jobs-rework-untracked", Row::NotAProtocol),
    ("jobs-third-rework", Row::NotAProtocol),
];

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/protocol")
}

/// The vendored request of `stem`, its file members made absolute so it
/// can be written anywhere.
fn request(stem: &str) -> Value {
    let path = corpus().join(format!("{stem}.request.json"));
    let mut request: Value = serde_json::from_slice(&std::fs::read(&path).unwrap())
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let absolute = |value: &mut Value| {
        let file = value.as_str().expect("a file member").to_owned();
        *value = json!(corpus().join(file));
    };
    absolute(&mut request["request"]["program"]["source"]["file"]);
    for model in request["request"]["models"].as_array_mut().unwrap() {
        absolute(&mut model["source"]["file"]);
    }
    request
}

/// The compile request of `stem`: an `analyze` request keeps only what
/// compile reads.
fn compile_request(stem: &str) -> Value {
    let mut request = request(stem);
    request["format"] = json!("native-compile/1");
    let members = request["request"].as_object_mut().unwrap();
    for member in ["items", "documents", "clause"] {
        members.remove(member);
    }
    request
}

fn write(directory: &Path, request: &Value) -> PathBuf {
    let path = directory.join("request.json");
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

fn source(stem: &str) -> String {
    std::fs::read_to_string(corpus().join(format!("{stem}.native"))).unwrap()
}

/// The protocol name and the role names a vendored source declares.
fn protocol_and_roles(stem: &str) -> (String, Vec<String>) {
    let text = source(stem);
    let mut protocol = None;
    let mut roles = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("protocol ") {
            protocol = rest.split_whitespace().next().map(str::to_owned);
        } else if let Some(rest) = line.strip_prefix("role ") {
            roles.extend(rest.split_whitespace().next().map(str::to_owned));
        }
    }
    (
        protocol.unwrap_or_else(|| panic!("{stem}: no protocol declaration")),
        roles,
    )
}

/// The vendored saga programs, found by their request identity.
fn vendored_sagas() -> BTreeSet<String> {
    let mut stems = BTreeSet::new();
    for entry in std::fs::read_dir(corpus()).unwrap() {
        let path = entry.unwrap().path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(stem) = name.strip_suffix(".request.json") else {
            continue;
        };
        let request: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        if request["request"]["program"]["source"]["identity"]
            .as_str()
            .is_some_and(|identity| identity.starts_with("example:parts-saga/"))
        {
            stems.insert(stem.to_owned());
        }
    }
    stems
}

#[test]
#[trace("FR-636-AC-2", "FR-636-AC-5", "FR-570-AC-1")]
fn bounded_static_sagas_check_project_and_realize() {
    let directory = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    for (stem, row) in SAGAS {
        let Row::BoundedStatic { deadlock } = row else {
            continue;
        };
        let (protocol, roles) = protocol_and_roles(stem);
        let mut request = compile_request(stem);
        request["format"] = json!("native-analyze/1");
        request["request"].as_object_mut().unwrap().remove("clocks");
        request["request"]["documents"] =
            json!([{"file": corpus().join("data/saga-start.json")}]);
        let binding = json!({"type":"model-binding","selection":"example/parts"});
        request["request"]["subject"] = json!({"protocol":{"protocol":protocol,
            "over":{"reference":{"population":"ix://example/parts/jobs","key":"j1"}},
            "roles": roles.iter().map(|role| (role.clone(), binding.clone()))
                .collect::<serde_json::Map<_, _>>()}});
        let mut items = roles
            .iter()
            .map(|role| {
                json!({"claim":protocol,"kind":"local-projection","options":{"role":role},
                    "assessed_root":crate::protocol_source_routes::root_wire(),
                    "temporal":{"profile":"bounded-static","max_depth":64}})
            })
            .collect::<Vec<_>>();
        items.push(json!({"claim":protocol,"kind":"realizability",
            "assessed_root":crate::protocol_source_routes::root_wire(),
            "premises":{"role_knowledge":[],"environment":[]},
            "temporal":{"profile":"bounded-static","max_depth":64,"max_history":16}}));
        items.push(json!({"claim":protocol,"kind":"global-conformance",
            "temporal":{"item_kind":"deadlock-freedom"}}));
        request["request"]["items"] = json!(items);
        let output = match command::analyze(
            &write(directory.path(), &request),
            &crate::negotiation::Fr290,
            &Cancel::new(),
        ) {
            Ok(output) => output,
            Err(error) => {
                failures.push(format!("{stem}: analyze refused: {error}"));
                continue;
            }
        };
        let records = output.value.as_value()["items"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if records.len() != roles.len() + 2 {
            failures.push(format!(
                "{stem}: expected one projection per role {roles:?} plus realizability and deadlock: {}",
                output.value
            ));
            continue;
        }
        for (index, item) in records.iter().enumerate() {
            let expected = if index == roles.len() + 1 {
                *deadlock
            } else {
                "proved"
            };
            if item["record"]["result"] != expected {
                failures.push(format!("{stem}: item {index} expected {expected}: {item}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn handoff(directory: &Path, stem: &str) -> fsl_command::checked_handoff::CompiledHandoff {
    let mut request = compile_request(stem);
    request["request"]["handoff"] = json!({"directory":"handoff","package_name":"saga-route",
        "native_join":{"work_units":1_u64<<28,"storage":1_u64<<24}});
    command::compile_outcome(&write(directory, &request))
        .unwrap_or_else(|error| panic!("{stem}: compile and emit the handoff: {error}"));
    fsl_command::checked_handoff::compile(&directory.join("handoff"))
        .unwrap_or_else(|error| panic!("{stem}: read and compile the handoff: {error}"))
}

#[test]
#[trace("FR-575-AC-1")]
fn unsupported_subsets_return_typed_refusals() {
    use fsl_replay::protocol_analysis::AnalysisOutcome;
    let mut failures = Vec::new();
    for (stem, row) in SAGAS {
        let Row::Unsupported { role, slug } = row else {
            continue;
        };
        let directory = tempfile::tempdir().unwrap();
        let package = handoff(directory.path(), stem);
        let result = crate::protocol_source_routes::project(&package, role);
        match result.outcome {
            AnalysisOutcome::Unsupported(cause) if cause.slug.as_str() == *slug => {
                if !package
                    .admitted()
                    .resolved_graph()
                    .nodes
                    .iter()
                    .any(|node| node.node_id == cause.node)
                {
                    failures.push(format!("{stem}: the refusal names no source construct"));
                }
            }
            other => failures.push(format!("{stem}: expected unsupported {slug}: {other:?}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[trace("FR-218-AC-5", "FR-218-AC-11")]
fn every_saga_protocol_is_classified() {
    let classified: BTreeSet<String> = SAGAS.iter().map(|(stem, _)| (*stem).to_owned()).collect();
    assert_eq!(
        classified.len(),
        SAGAS.len(),
        "a saga program is listed twice"
    );
    assert_eq!(
        vendored_sagas(),
        classified,
        "every vendored saga program is in exactly one row"
    );
    let directory = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    for (stem, row) in SAGAS {
        if matches!(row, Row::NotAProtocol) {
            assert!(
                !source(stem).contains("\nprotocol "),
                "{stem} declares a protocol"
            );
            continue;
        }
        let compiled = command::compile_outcome(&write(directory.path(), &compile_request(stem)));
        match (row, compiled) {
            (Row::Refused { exit_code }, Err(error)) => {
                if error.category().exit_code() != *exit_code {
                    failures.push(format!("{stem}: refused with the wrong exit code: {error}"));
                }
            }
            (Row::Refused { .. }, Ok(_)) => {
                failures.push(format!("{stem}: compiles but is listed as refused"));
            }
            (_, Err(error)) => {
                failures.push(format!("{stem}: listed as compiling but refused: {error}"));
            }
            (_, Ok(_)) => {}
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
