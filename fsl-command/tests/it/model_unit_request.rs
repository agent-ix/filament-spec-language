// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-710-AC-6 to AC-9 (TC-2998): a model unit reaches every command that
//! takes a model, through a request's `models` member, `check`'s model
//! selections and a compile request that writes the compiled document.

use fsl_command::command::{self, CheckInputs};
use fsl_foundation::SourceIdentity;
use fsl_replay::Operation;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{compile_model_unit, package_input};
use ix_trace_rs::trace;
use serde_json::{Value, json};
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
         invariant I using P on M::Order at current {{ self.total >= 0 }}\n\
         function one using P(): Int[1, 1] pure {{ 1 }}\n"
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

fn run_request(
    directory: &Path,
    files: &[(&str, &str)],
    program_file: &str,
    models: Value,
) -> PathBuf {
    for (name, text) in files {
        std::fs::write(directory.join(name), text).unwrap();
    }
    let request = json!({"format": "native-run/1", "request": {
        "models": models,
        "program": {"source": source(program_file)},
        "call": {"function": "one", "arguments": [], "accounting": {"work_units": 1000}}
    }});
    let path = directory.join("run.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
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
        unit_entry.clone(),
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

    let unit_run = run_request(
        directory.path(),
        &[
            ("shop.fsl", SHOP),
            ("program.fsl", &program("example/shop")),
        ],
        "program.fsl",
        unit_entry,
    );
    let from_unit = command::run(&unit_run, &crate::negotiation::Fr290)
        .expect("the native model-unit run completes");
    let document_run = run_request(directory.path(), &[], "program.fsl", document_entry.clone());
    let from_document = command::run(&document_run, &crate::negotiation::Fr290)
        .expect("the semantic document run completes");
    assert_eq!(
        from_unit.value.as_value()["outcome"],
        from_document.value.as_value()["outcome"]
    );

    let ghost_entry = json!([{"format": "native-model-unit/1", "source": source("ghost.fsl")}]);
    let path = compile_request(
        directory.path(),
        &[("ghost.fsl", GHOST)],
        "program.fsl",
        ghost_entry,
        &[],
    );
    let ghost_error = command::compile_outcome(&path).expect_err("Ghost refuses");
    let ghost = refusal(&ghost_error);
    let ghost_outcome: Value = serde_json::from_slice(
        &ghost_error
            .outcome_document(Operation::Compile)
            .to_bytes()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(ghost["code"], "missing_declaration", "{ghost}");
    assert_eq!(ghost_outcome["diagnostics"][0]["cause"], "missing-name");
    assert_eq!(ghost_outcome["last_stage"], "I1");
    assert_eq!(
        ghost_outcome["diagnostics"][0]["locus"]["file"],
        "ghost.fsl"
    );

    let program_entry = json!([{"format": "native-model-unit/1", "source": source("program.fsl")}]);
    let path = compile_request(directory.path(), &[], "program.fsl", program_entry, &[]);
    let not_a_unit = refusal(&command::compile_outcome(&path).err().expect("no domain"));
    assert_eq!(not_a_unit["code"], "invalid-request", "{not_a_unit}");
    assert!(not_a_unit.to_string().contains("models[0]"), "{not_a_unit}");
    assert!(
        not_a_unit.to_string().contains("native-model-unit/1"),
        "{not_a_unit}"
    );

    let wrong_format = json!([{"format": "semantic-ir/2.0.0", "source": source("shop.fsl")}]);
    let path = compile_request(
        directory.path(),
        &[("shop.fsl", SHOP)],
        "program.fsl",
        wrong_format,
        &[],
    );
    let malformed_error =
        command::compile_outcome(&path).expect_err("a model unit is not a semantic-ir document");
    let malformed = refusal(&malformed_error);
    assert_eq!(malformed["code"], "invalid_model_binding", "{malformed}");
    let malformed_outcome: Value = serde_json::from_slice(
        &malformed_error
            .outcome_document(Operation::Compile)
            .to_bytes()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        malformed_outcome["diagnostics"][0]["cause"],
        "malformed-declaration"
    );
}

#[test]
#[trace("TC-2998", "FR-710-AC-8")]
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
    let document = package_input([SHOP.as_bytes()]).expect("the compiled document");
    let document_path = directory.path().join("shop.semantic-ir.json");
    std::fs::write(&document_path, &document["example/shop"]).unwrap();
    let document_inputs = CheckInputs {
        models: vec![("example/shop".to_owned(), document_path)],
        ..CheckInputs::default()
    };
    let checked_document = check_document(&command::check_outcome(&rule, &document_inputs));
    assert_eq!(checked["category"], checked_document["category"]);
    assert_eq!(checked["diagnostics"], checked_document["diagnostics"]);
    assert_eq!(checked["last_stage"], checked_document["last_stage"]);

    let other_rule = directory.path().join("other-rule.fsl");
    std::fs::write(&other_rule, program("example/other")).unwrap();
    let other_inputs = CheckInputs {
        models: vec![("example/other".to_owned(), shop.clone())],
        ..CheckInputs::default()
    };
    let wrong_selection = check_document(&command::check_outcome(&other_rule, &other_inputs));
    assert_eq!(wrong_selection["category"], "failure", "{wrong_selection}");
    assert_eq!(
        wrong_selection["diagnostics"][0]["code"], "invalid_model_binding",
        "{wrong_selection}"
    );
    assert_eq!(
        wrong_selection["diagnostics"][0]["cause"], "wrong-model-selection",
        "{wrong_selection}"
    );

    let alone = check_document(&command::check_outcome(&shop, &CheckInputs::default()));
    assert_eq!(alone["category"], "success", "{alone}");
    assert_eq!(alone["last_stage"], "I1", "{alone}");

    let ghost = directory.path().join("ghost.fsl");
    std::fs::write(&ghost, GHOST).unwrap();
    let refused = check_document(&command::check_outcome(&ghost, &CheckInputs::default()));
    assert_eq!(
        refused["diagnostics"][0]["code"], "missing_declaration",
        "{refused}"
    );
    assert_eq!(
        refused["diagnostics"][0]["cause"], "missing-name",
        "{refused}"
    );
    assert_eq!(refused["last_stage"], "I1", "{refused}");
    assert_eq!(
        refused["diagnostics"][0]["locus"]["file"], "ghost.fsl",
        "{refused}"
    );
}

#[test]
#[trace("TC-2998", "FR-710-AC-9")]
fn a_compile_request_over_a_model_unit_writes_its_document() {
    let directory = tempfile::tempdir().unwrap();
    let output = json!({"package": "shop.semantic-ir.json"});
    let path = compile_request(
        directory.path(),
        &[("shop.fsl", SHOP)],
        "shop.fsl",
        Value::Null,
        &[("output", output)],
    );
    let outcome = command::compile_outcome(&path).expect("the model unit compiles");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("agent-ix", "test:shop.fsl"),
        "shop.fsl",
        SHOP.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the model unit parses");
    let expected = compile_model_unit(&parsed, ModelNormalizationLimits::default())
        .expect("the model unit compiles")
        .bytes()
        .to_vec();
    assert_eq!(outcome.package_bytes(), expected.as_slice());
    let outcome_document: Value =
        serde_json::from_slice(&outcome.document().to_bytes().unwrap()).unwrap();
    assert_eq!(outcome_document["last_stage"], "I1");
    assert_eq!(
        serde_json::from_slice::<Value>(outcome.package_bytes()).unwrap()["contractVersion"],
        "2.0.0"
    );
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
        let output_path = format!("refused-{member}.json");
        let path = compile_request(
            directory.path(),
            &[],
            "shop.fsl",
            Value::Null,
            &[(member, value), ("output", json!({"package": output_path}))],
        );
        let refused = refusal(&command::compile_outcome(&path).err().expect("refuses"));
        assert_eq!(refused["code"], "invalid-request", "{member}: {refused}");
        assert!(refused.to_string().contains(member), "{refused}");
        assert!(!directory.path().join(output_path).exists());
    }
}
