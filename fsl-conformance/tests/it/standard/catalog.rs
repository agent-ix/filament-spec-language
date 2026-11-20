// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2051: every checked-operation catalog v1 identity and every FR-322
//! node form, produced by a checked-in FSL source whose emitted package the
//! strict reader admits.
//!
//! The sources are every `*-accepted.native` and `catalog-source-*.native`
//! file under `fsl-conformance/cases/`. Each is compiled through public
//! parse, model selection and check, and its emitted package is read back
//! with `fsl_package::read_evaluation_package`. An operation or node form is
//! produced when an admitted package contains it. The node forms are the
//! `(node_tag, semantic_form)` pairs of the standard's checked-package/v2
//! schema `*Node` definitions.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use filament_exact::Cancel;
use filament_verification_contracts::operation_catalog::CHECKED_OPERATION_CATALOG_V1;
use fsl_foundation::SourceIdentity;
use fsl_package::{EmitLimits, V2ReadLimits};
use fsl_replay::spine::{self, DependencyInput, ParseRequest, SpineLimits};
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::intake::PackageInput;
use ix_trace_rs::trace;
use serde_json::Value;

use super::{gap, Corpus, Difference, KnownGap};

const SCHEMA: &str = "proposals/checked-package-v2/schema.json";

const UNPRODUCED: &str = "QSL-2051: no checked-in source produces it yet";

/// Cases that do not currently conform, asserted exactly.
const KNOWN_GAPS: &[KnownGap] = &[
    gap("operation/filament.op.collection.index", "IR-1491"),
    gap("operation/filament.op.model.reaches_field", "QSL-1904"),
    gap("source/index-accepted.native", "IR-1491"),
    gap("form/model/object_type", UNPRODUCED),
];

/// One checked-in source.
struct Source {
    name: String,
    text: String,
}

fn cases_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("cases")
}

fn is_producer(name: &str) -> bool {
    name.ends_with("-accepted.native") || name.starts_with("catalog-source-")
}

fn sources(root: &Path) -> Result<Vec<Source>, String> {
    let mut found = Vec::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        let entries = std::fs::read_dir(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        for entry in entries {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.is_dir() {
                directories.push(path);
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if is_producer(name) {
                let text = std::fs::read_to_string(&path)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
                found.push(Source {
                    name: name.to_owned(),
                    text,
                });
            }
        }
    }
    found.sort_by(|left, right| left.name.cmp(&right.name));
    if found.is_empty() {
        return Err(format!("no producer sources under {}", root.display()));
    }
    Ok(found)
}

fn catalog_identities(catalog: &str) -> Result<BTreeSet<String>, String> {
    let catalog: Value = serde_json::from_str(catalog).map_err(|error| error.to_string())?;
    let rows = catalog["operations"]
        .as_array()
        .ok_or("the catalog has operation rows")?;
    let identities: BTreeSet<String> = rows
        .iter()
        .map(|row| {
            row["identity"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("operation row without identity: {row}"))
        })
        .collect::<Result<_, _>>()?;
    if identities.is_empty() {
        return Err("the published catalog is empty".to_owned());
    }
    Ok(identities)
}

/// The `(node_tag, semantic_form)` pairs the schema's `*Node` definitions
/// declare.
fn node_forms(schema: &[u8]) -> Result<BTreeSet<(String, String)>, String> {
    let schema: Value = serde_json::from_slice(schema).map_err(|error| error.to_string())?;
    let definitions = schema["$defs"]
        .as_object()
        .ok_or("the schema has $defs")?;
    let mut forms = BTreeSet::new();
    for (name, definition) in definitions {
        if !name.ends_with("Node") {
            continue;
        }
        for part in definition["allOf"].as_array().into_iter().flatten() {
            let properties = &part["properties"];
            let Some(tag) = properties["node_tag"]["const"].as_str() else {
                continue;
            };
            let form = &properties["semantic_form"];
            let listed: Vec<&Value> = match (form["enum"].as_array(), form.get("const")) {
                (Some(values), _) => values.iter().collect(),
                (None, Some(value)) => vec![value],
                (None, None) => return Err(format!("{name} declares no semantic_form")),
            };
            for value in listed {
                let value = value
                    .as_str()
                    .ok_or_else(|| format!("{name}: non-text semantic_form"))?;
                forms.insert((tag.to_owned(), value.to_owned()));
            }
        }
    }
    if forms.is_empty() {
        return Err("the schema declares no node forms".to_owned());
    }
    Ok(forms)
}

/// Compiles `source` and reads its emitted package back with the strict
/// reader, returning the admitted package JSON.
fn compile_and_read(source: &Source) -> Result<Value, String> {
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let identity = SourceIdentity::new("catalog-source", &source.name);
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: &source.name,
            bytes: source.text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(|error| format!("parse: {error:?}"))?
    .into_value();
    let inputs = PackageInput::default();
    let models = spine::select(&parsed, &inputs, limits.model, &cancel)
        .map_err(|error| format!("select: {error:?}"))?
        .into_value();
    let checked = spine::check(
        &parsed,
        "catalog-source".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(|error| format!("check: {error:?}"))?
    .into_value();
    let read = fsl_package::read_evaluation_package(
        checked.package(),
        LibraryName::new("catalog-source").map_err(|error| format!("{error:?}"))?,
        &inputs,
        V2ReadLimits::bounded(16_777_216),
        EmitLimits::new(16_777_216),
    )
    .map_err(|error| format!("strict read: {error:?}"))?;
    if !read.emission().omitted().is_empty() {
        return Err(format!(
            "emission omitted {} nodes",
            read.emission().omitted().len()
        ));
    }
    serde_json::from_slice(read.emission().package().bytes()).map_err(|error| error.to_string())
}

fn operation_identities(value: &Value, found: &mut BTreeSet<String>) {
    match value {
        Value::Object(fields) => {
            if let Some(identity) = fields.get("identity").and_then(Value::as_str) {
                if identity.starts_with("filament.op.") {
                    found.insert(identity.to_owned());
                }
            }
            for value in fields.values() {
                operation_identities(value, found);
            }
        }
        Value::Array(values) => {
            for value in values {
                operation_identities(value, found);
            }
        }
        _ => {}
    }
}

/// What an admitted package contains.
#[derive(Default)]
struct Produced {
    operations: BTreeSet<String>,
    forms: BTreeSet<(String, String)>,
}

impl Produced {
    fn add(&mut self, package: &Value) -> Result<(), String> {
        let nodes = package["semantic_graph"]["nodes"]
            .as_array()
            .ok_or("the admitted package has graph nodes")?;
        for node in nodes {
            if let (Some(tag), Some(form)) =
                (node["node_tag"].as_str(), node["semantic_form"].as_str())
            {
                self.forms.insert((tag.to_owned(), form.to_owned()));
            }
            if node["node_tag"] == "expression" && node["body"]["term"] == "application" {
                operation_identities(&node["body"], &mut self.operations);
            }
        }
        Ok(())
    }
}

fn run_corpus(catalog: &BTreeSet<String>, forms: &BTreeSet<(String, String)>) -> Corpus {
    let sources = sources(&cases_root()).unwrap_or_else(|error| panic!("{error}"));
    let mut corpus = Corpus::new("catalog", sources.len() + catalog.len() + forms.len());
    let mut produced = Produced::default();
    for source in &sources {
        let outcome = compile_and_read(source)
            .and_then(|package| produced.add(&package))
            .map_err(|observed| Difference::new("checked and strictly admitted", observed));
        corpus.record(format!("source/{}", source.name), outcome);
    }
    for identity in &produced.operations {
        if !catalog.contains(identity) {
            corpus.record(
                format!("unpublished/{identity}"),
                Err(Difference::new("a catalog v1 operation", "not in the catalog")),
            );
        }
    }
    for identity in catalog {
        let outcome = if produced.operations.contains(identity) {
            Ok(())
        } else {
            Err(Difference::new(
                "emitted by an admitted checked-in source",
                "no admitted source emits it",
            ))
        };
        corpus.record(format!("operation/{identity}"), outcome);
    }
    for (tag, form) in forms {
        let outcome = if produced.forms.contains(&(tag.clone(), form.clone())) {
            Ok(())
        } else {
            Err(Difference::new(
                "emitted by an admitted checked-in source",
                "no admitted source emits it",
            ))
        };
        corpus.record(format!("form/{tag}/{form}"), outcome);
    }
    corpus
}

fn standard_forms() -> BTreeSet<(String, String)> {
    node_forms(filament_specification::published_file(SCHEMA).expect("checked-package/v2 schema"))
        .unwrap_or_else(|error| panic!("{error}"))
}

/// Provenance: QSL-2051. Every catalog operation and node form is produced
/// by admitted checked-in source, or is an exactly listed known gap.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn conformance_catalog_operations_and_node_forms_from_source() {
    let catalog = catalog_identities(CHECKED_OPERATION_CATALOG_V1)
        .unwrap_or_else(|error| panic!("{error}"));
    run_corpus(&catalog, &standard_forms()).assert_settled(KNOWN_GAPS);
}

/// Provenance: QSL-2051. A fake catalog entry is reported by its identity.
#[test]
#[trace("FR-350")]
fn an_added_catalog_entry_is_reported_by_its_identity() {
    let mut catalog = catalog_identities(CHECKED_OPERATION_CATALOG_V1)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(catalog.insert("filament.op.conformance.nonexistent".to_owned()));
    let errors = run_corpus(&catalog, &standard_forms())
        .settle(KNOWN_GAPS)
        .expect_err("the added entry has no producer");
    assert_eq!(
        errors,
        vec![
            "catalog: operation/filament.op.conformance.nonexistent: expected emitted by an admitted checked-in source, observed no admitted source emits it (not a listed known gap)"
                .to_owned()
        ]
    );
}

/// Provenance: QSL-2051. A fake node form is reported by its tag and form.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn an_added_node_form_is_reported_by_its_name() {
    let catalog = catalog_identities(CHECKED_OPERATION_CATALOG_V1)
        .unwrap_or_else(|error| panic!("{error}"));
    let mut forms = standard_forms();
    assert!(forms.insert(("expression".to_owned(), "nonexistent".to_owned())));
    let errors = run_corpus(&catalog, &forms)
        .settle(KNOWN_GAPS)
        .expect_err("the added form has no producer");
    assert_eq!(
        errors,
        vec![
            "catalog: form/expression/nonexistent: expected emitted by an admitted checked-in source, observed no admitted source emits it (not a listed known gap)"
                .to_owned()
        ]
    );
}

/// Provenance: QSL-2051. An empty source tree or catalog cannot pass.
#[test]
#[trace("FR-350")]
fn an_empty_source_tree_or_catalog_is_an_error() {
    let empty = tempfile::tempdir().expect("temporary directory");
    assert_eq!(
        sources(empty.path()).err(),
        Some(format!("no producer sources under {}", empty.path().display()))
    );
    assert_eq!(
        catalog_identities(r#"{"operations":[]}"#).err(),
        Some("the published catalog is empty".to_owned())
    );
}
