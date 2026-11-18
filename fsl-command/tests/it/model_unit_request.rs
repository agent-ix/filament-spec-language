// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-710-AC-6 to AC-9 (TC-2998): a model unit reaches every command that
//! takes a model, through a request's `models` member, `check`'s model
//! selections and a compile request that writes the compiled document.

use fsl_command::command::{self, CheckInputs};
use fsl_semantics::model::intake::package_input;
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// FR-710-AC-1's model unit.
const SHOP: &str = "language \"ix:native\" edition \"1-draft\";
domain \"example/shop\";
object Order identity (id) {
  id: Int[0, 999];
  total: Int[0, 100];
  operation pay(): Boolean modifies self.total;
}
population orders: Order;
";

/// FR-710-AC-4's model unit with an unresolved relationship target.
const GHOST: &str = "language \"ix:native\" edition \"1-draft\";
domain \"example/shop\";
object Order { relationship next: Ghost [0, 1]; }
";

/// A program selecting `identity` as `M`, with one invariant over `M::Order`.
fn program(identity: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile P = \"filament.model.complete/v1\";\n\
         model M = \"{identity}\";\n\
         invariant I using P on M::Order at current {{ self.total >= 0 }}\n"
    )
}

fn source(file: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:{file}"),
        "document": "ModelUnit", "formal_revision": 1})
}

/// Write `files` into `directory` and a `native-compile/1` request over
/// `program` and `models`, returning the request path.
fn compile_request(
    directory: &Path,
    files: &[(&str, &str)],
    program_file: &str,
    models: Value,
    extra: &[(&str, Value)],
) -> PathBuf {
    for (name, text) in files {
        std::fs::write(directory.join(name), text).unwrap();
    }
    let mut request = json!({"program": {"source": source(program_file)}});
    if !models.is_null() {
        request["models"] = models;
    }
    for (member, value) in extra {
        request[*member] = value.clone();
    }
    let path = directory.join("compile.json");
    let envelope = json!({"format": "native-compile/1", "request": request});
    std::fs::write(&path, serde_json::to_vec(&envelope).unwrap()).unwrap();
    path
}

fn refusal(error: &command::RunError) -> Value {
    error.value().expect("refusal envelope")
}

fn check_document(outcome: &command::CheckOutcome) -> Value {
    serde_json::from_slice(&outcome.document().to_bytes().unwrap()).unwrap()
}

#[test]
#[trace("TC-2998", "FR-710-AC-6")]
#[ignore = "QSL-1980: package input compiles a model unit and admits it under its domain identity; not implemented yet"]
fn a_package_input_compiles_a_model_unit_under_its_domain_identity() {
    let packages = package_input([SHOP.as_bytes()]).expect("a model unit is a package input");
    assert!(
        packages.contains_key("example/shop"),
        "{:?}",
        packages.keys()
    );
    let document: Value =
        serde_json::from_slice(&packages["example/shop"]).expect("the compiled document");
    assert_eq!(document["contractVersion"], "2.0.0", "{document}");
    let conflict = package_input([SHOP.as_bytes(), packages["example/shop"].as_slice()])
        .expect_err("a model unit and its document collide");
    assert!(
        format!("{conflict:?}").contains("ConflictingPackage"),
        "{conflict:?}"
    );
}

#[test]
#[trace("TC-2998", "FR-710-AC-7")]
#[ignore = "QSL-1980: a `native-model-unit/1` models entry compiles and admits the model unit; not implemented yet"]
fn a_native_model_unit_models_entry_compiles_like_its_document() {
    let directory = tempfile::tempdir().unwrap();
    let unit_entry = json!([{"format": "native-model-unit/1", "source": source("shop.fsl")}]);
    let path = compile_request(
        directory.path(),
        &[
            ("shop.fsl", SHOP),
            ("program.fsl", &program("example/shop")),
        ],
        "program.fsl",
        unit_entry,
        &[],
    );
    let from_unit = command::compile_outcome(&path).expect("the model unit route compiles");
    let document =
        package_input([SHOP.as_bytes()]).expect("the compiled document")["example/shop"].clone();
    std::fs::write(directory.path().join("shop.semantic-ir.json"), &document).unwrap();
    let document_entry =
        json!([{"format": "semantic-ir/2.0.0", "source": source("shop.semantic-ir.json")}]);
    let path = compile_request(directory.path(), &[], "program.fsl", document_entry, &[]);
    let from_document = command::compile_outcome(&path).expect("the document route compiles");
    assert_eq!(from_unit.package_bytes(), from_document.package_bytes());

    let ghost_entry = json!([{"format": "native-model-unit/1", "source": source("ghost.fsl")}]);
    let path = compile_request(
        directory.path(),
        &[("ghost.fsl", GHOST)],
        "program.fsl",
        ghost_entry,
        &[],
    );
    let ghost = refusal(
        &command::compile_outcome(&path)
            .err()
            .expect("Ghost refuses"),
    );
    assert_eq!(ghost["code"], "missing_declaration", "{ghost}");
    assert!(ghost.to_string().contains("ghost.fsl"), "{ghost}");

    let program_entry = json!([{"format": "native-model-unit/1", "source": source("program.fsl")}]);
    let path = compile_request(directory.path(), &[], "program.fsl", program_entry, &[]);
    let not_a_unit = refusal(&command::compile_outcome(&path).err().expect("no domain"));
    assert_eq!(not_a_unit["code"], "invalid-request", "{not_a_unit}");
    assert!(
        not_a_unit.to_string().contains("native-model-unit/1"),
        "{not_a_unit}"
    );
}

#[test]
#[trace("TC-2998", "FR-710-AC-8")]
#[ignore = "QSL-1980: check takes a model unit as a model selection and as its checked source; not implemented yet"]
fn check_takes_a_model_unit_as_a_selection_and_as_its_source() {
    let directory = tempfile::tempdir().unwrap();
    let shop = directory.path().join("shop.fsl");
    std::fs::write(&shop, SHOP).unwrap();
    let rule = directory.path().join("rule.fsl");
    std::fs::write(&rule, program("example/shop")).unwrap();
    let inputs = CheckInputs {
        models: vec![("example/shop".to_owned(), shop.clone())],
        ..CheckInputs::default()
    };
    let checked = check_document(&command::check_outcome(&rule, &inputs));
    assert_eq!(checked["category"], "success", "{checked}");
    assert_eq!(checked["last_stage"], "S4", "{checked}");

    let alone = check_document(&command::check_outcome(&shop, &CheckInputs::default()));
    assert_eq!(alone["category"], "success", "{alone}");
    assert_eq!(alone["last_stage"], "I1", "{alone}");

    let ghost = directory.path().join("ghost.fsl");
    std::fs::write(&ghost, GHOST).unwrap();
    let refused = check_document(&command::check_outcome(&ghost, &CheckInputs::default()));
    assert!(
        refused.to_string().contains("missing_declaration"),
        "{refused}"
    );
}

#[test]
#[trace("TC-2998", "FR-710-AC-9")]
#[ignore = "QSL-1980: a compile request over a model unit writes the compiled document; not implemented yet"]
fn a_compile_request_over_a_model_unit_writes_its_document() {
    let directory = tempfile::tempdir().unwrap();
    let output = json!({"file": "shop.semantic-ir.json"});
    let path = compile_request(
        directory.path(),
        &[("shop.fsl", SHOP)],
        "shop.fsl",
        Value::Null,
        &[("output", output)],
    );
    let outcome = command::compile_outcome(&path).expect("the model unit compiles");
    let expected =
        package_input([SHOP.as_bytes()]).expect("the compiled document")["example/shop"].clone();
    assert_eq!(outcome.package_bytes(), expected.as_slice());
    assert_eq!(
        std::fs::read(directory.path().join("shop.semantic-ir.json")).unwrap(),
        expected
    );

    for (member, value) in [
        ("models", json!([])),
        (
            "libraries",
            json!([{"identity": "test/geometry", "source": source("shop.fsl")}]),
        ),
        ("clocks", json!({})),
    ] {
        let path = compile_request(
            directory.path(),
            &[],
            "shop.fsl",
            Value::Null,
            &[(member, value)],
        );
        let refused = refusal(&command::compile_outcome(&path).err().expect("refuses"));
        assert_eq!(refused["code"], "invalid-request", "{member}: {refused}");
        assert!(refused.to_string().contains(member), "{refused}");
    }
}
