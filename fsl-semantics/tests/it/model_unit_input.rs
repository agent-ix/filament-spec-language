// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-710-AC-5 (TC-1943): a package input supplies a model unit in place of
//! a domain package document, and intake admits it as it admits the compiled
//! document.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{
    admit, admit_unit, compile_model_unit, package_input, package_input_from_sources,
};
use ix_trace_rs::trace;

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

/// A unit selecting `identity` as `M`, with one invariant over `M::Order`.
fn unit(identity: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile P = \"filament.model.complete/v1\";\n\
         model M = \"{identity}\";\n\
         invariant I using P on M::Order at current {{ self.total >= 0 }}\n"
    )
}

/// The unit's selections admitted against an input holding the model unit.
fn admit(identity: &str) -> Result<PackageDeclarations, String> {
    let text = unit(identity);
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "model-unit-input"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let packages = package_input([SHOP.as_bytes()]).expect("a package input admits a model unit");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .map_err(|refusal| format!("{refusal:?}"))?;
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        models,
        Vec::new(),
    )
    .map_err(|refusal| format!("{refusal:?}"))
}

#[trace("FR-710-AC-5", "TC-1943")]
#[test]
fn a_supplied_model_unit_is_admitted_as_its_compiled_document() {
    let mut declarations = admit("example/shop").expect("the model unit is admitted");
    declarations
        .link_dispatch(ModelNormalizationLimits::default())
        .expect("dispatch links");
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .expect("the invariant over M::Order checks");
}

#[trace("FR-710-AC-5", "TC-1943")]
#[test]
fn a_model_unit_for_another_identity_is_refused_wrong_model_selection() {
    let refusal = admit("example/other").expect_err("the identities differ");
    assert!(refusal.contains("WrongModelSelection"), "{refusal}");
}

#[trace("FR-710-AC-6", "TC-2998")]
#[test]
fn compile_model_unit_returns_its_identity_and_stable_document_bytes() {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:shop-model"),
        "shop.fsl",
        SHOP.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let compiled = compile_model_unit(&parsed, ModelNormalizationLimits::default())
        .expect("the model unit compiles");
    let repeated = compile_model_unit(&parsed, ModelNormalizationLimits::default())
        .expect("the same model unit compiles again");
    assert_eq!(compiled.identity(), "example/shop");
    assert_eq!(compiled.bytes(), repeated.bytes());
    let document: serde_json::Value =
        serde_json::from_slice(compiled.bytes()).expect("compiled document is JSON");
    assert_eq!(document["contractVersion"], "2.0.0");
    assert_eq!(document["package"]["identity"], "example/shop");
    assert_eq!(
        document["types"][0]["fields"][0]["origin"]["source"]["sourceIdentity"],
        "ix://agent-ix/test:shop-model"
    );
}

#[trace("FR-710-AC-6", "TC-2998")]
#[test]
fn package_input_from_sources_keeps_source_labels_and_compiled_document_conflicts() {
    let source = fsl_cst::diagnostic::read_source(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:shop-model"),
        "shop.fsl",
        SHOP.as_bytes(),
        fsl_cst::Limits::default().source_bytes,
    )
    .expect("source metadata admitted");
    let packages = package_input_from_sources([&source]).expect("source compiles to input");
    let document: serde_json::Value =
        serde_json::from_slice(&packages["example/shop"]).expect("compiled document is JSON");
    assert_eq!(
        document["types"][0]["fields"][0]["origin"]["source"]["sourceIdentity"],
        "ix://agent-ix/test:shop-model"
    );
    let compiled = package_input([SHOP.as_bytes()]).expect("anonymous model unit compiles");
    let conflict = package_input([SHOP.as_bytes(), compiled["example/shop"].as_slice()])
        .expect_err("a model unit and its compiled document share one identity");
    assert!(
        format!("{conflict:?}").contains("ConflictingPackage"),
        "{conflict:?}"
    );
}

#[trace("FR-710-AC-2", "TC-1942")]
#[test]
fn a_model_unit_compiles_its_relationship_and_population() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/graph";
object Node { relationship next: Node [0, 1]; }
population nodes: Node;
"#;
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "graph-model"),
        "graph.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let compiled = compile_model_unit(&parsed, ModelNormalizationLimits::default())
        .expect("the relationship model compiles");
    let document: serde_json::Value =
        serde_json::from_slice(compiled.bytes()).expect("compiled document is JSON");
    assert_eq!(
        document["types"][0]["relationships"][0]["identity"],
        "ix://example/graph/relationship/Node-next-Node"
    );
    assert_eq!(
        document["types"][0]["relationships"][0]["targetEnd"]["multiplicity"]["lower"],
        0
    );
    assert_eq!(document["populations"][0]["extent"], "closed");
    let packages = package_input([source.as_bytes()]).expect("model unit becomes package input");
    admit(
        "example/graph",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the compiled relationship document");
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn a_model_unit_with_an_unresolved_relationship_target_refuses_at_the_name() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/graph";
object Node { relationship next: Ghost [0, 1]; }
"#;
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "ghost-model"),
        "ghost.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let [diagnostic] = compile_model_unit(&parsed, ModelNormalizationLimits::default())
        .expect_err("an undeclared relationship target is refused");
    assert_eq!(diagnostic.code, fsl_cst::CompleteCode::MissingDeclaration);
    assert_eq!(diagnostic.cause.as_str(), "missing-name");
    let region = diagnostic.region.as_ref().expect("located refusal");
    let span = parsed
        .source()
        .render(region)
        .expect("source region renders");
    assert_eq!(span.start.byte, source.find("Ghost").expect("target name"));
    assert_eq!(span.end.byte, span.start.byte + "Ghost".len());
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn duplicate_object_declarations_refuse_at_the_second_name() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/graph";
object Node { }
object Node { }
"#;
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "duplicate-model"),
        "duplicate.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let [diagnostic] = compile_model_unit(&parsed, ModelNormalizationLimits::default())
        .expect_err("duplicate object identities are refused");
    assert_eq!(diagnostic.code, fsl_cst::CompleteCode::AmbiguousDeclaration);
    assert_eq!(diagnostic.cause.as_str(), "ambiguous-name");
    let span = parsed
        .source()
        .render(diagnostic.region.as_ref().expect("located refusal"))
        .expect("source region renders");
    let second_name = source.rfind("Node").expect("second declaration name");
    assert_eq!(span.start.byte, second_name);
    assert_eq!(span.end.byte, second_name + "Node".len());
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn an_operation_on_a_record_refuses_as_an_unsupported_construct() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/records";
record Row { value: Integer; operation update(); }
"#;
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "record-operation-model"),
        "records.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let [diagnostic] = compile_model_unit(&parsed, ModelNormalizationLimits::default())
        .expect_err("FSpec does not admit operations on record value types");
    assert_eq!(diagnostic.code, fsl_cst::CompleteCode::UnsupportedConstruct);
    assert_eq!(diagnostic.cause.as_str(), "declaration-form");
    let span = parsed
        .source()
        .render(diagnostic.region.as_ref().expect("located refusal"))
        .expect("source region renders");
    let operation = source.find("operation").expect("operation token");
    assert_eq!(span.start.byte, operation);
    assert_eq!(span.end.byte, operation + "operation".len());
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_a_bounded_native_type_alias() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/scalars";
type Small = Int[0, 3];
object Item { value: Small; }
population items: Item;
"#;
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "scalar-model"),
        "scalar.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let compiled = compile_model_unit(&parsed, ModelNormalizationLimits::default())
        .expect("the scalar alias model compiles");
    let document: serde_json::Value =
        serde_json::from_slice(compiled.bytes()).expect("compiled document is JSON");
    assert_eq!(document["types"][0]["scalar"], "integer");
    assert_eq!(
        document["types"][0]["constraints"][0]["operands"]["value"],
        "0"
    );
    assert_eq!(
        document["types"][0]["constraints"][1]["operands"]["value"],
        "3"
    );
    let packages =
        package_input([source.as_bytes()]).expect("compiled alias becomes package input");
    admit(
        "example/scalars",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the compiled scalar alias");
}
