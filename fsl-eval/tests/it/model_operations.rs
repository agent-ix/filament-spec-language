// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-3040 step 3: FR-106-AC-71 admits present, absent and omitted optional
//! operation parameters against the TC-2054 `test/refunds` fixture.

use std::collections::BTreeMap;

use filament_eval::FamilyOutcome;
use filament_exact::{Meter, Outcome, ScalarLimits, Value};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input};
use fsl_semantics::model::observation::{
    AdmissionFailure, AdmittedObservations, ClauseFacts, ClauseSelection, ClauseSelectionInput,
    DocumentRef, ObservationLimits, OperationFacts, Provisions, admit_observations,
};
use ix_trace_rs::trace;
use serde_json::{Value as JsonValue, json};

#[path = "../support/tc2054_refunds.rs"]
mod fixture;

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

const DECLARATIONS: &str = "\
pre NoteBy using v on R::Order::note { present(secondApprover) }\n\
pre NoteVal using v on R::Order::note {\n\
  present(secondApprover) implies deref(value(secondApprover)).badge > 0\n\
}\n\
pre NoteSame using v on R::Order::note { secondApprover = secondApprover }\n";

fn package() -> CheckedPackage {
    let fixture_bytes = serde_json::to_vec(&fixture::document()).expect("fixture serializes");
    let packages = package_input([fixture_bytes.as_slice()]).expect("fixture package is read");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model R = {package:?};\n\
         {declarations}",
        package = fixture::PACKAGE,
        declarations = DECLARATIONS,
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "tc-2054-refunds"),
        "tc-2054-refunds.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source reads");
    assert!(
        parsed.is_admissible(),
        "{:?}\n{source}",
        parsed.diagnostics()
    );
    let unit = fsl_forms::build_unit(&parsed).expect("source unit builds");
    let selected = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("TC-2054 package is admitted");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("declarations assemble");
    let checked = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("TC-2054 unit checks: {refusals:?}"));
    CheckedPackage::link(checked)
}

fn document_ref(identity: &str) -> DocumentRef {
    DocumentRef {
        authority: "test".to_owned(),
        identity: identity.to_owned(),
    }
}

fn identity_json(reference: &DocumentRef) -> JsonValue {
    json!({"authority": reference.authority, "identity": reference.identity})
}

fn reference(population: &str, key: &str) -> JsonValue {
    json!({"reference": {"population": fixture::id(population), "key": key}})
}

fn admit_invocation(
    package: &CheckedPackage,
    parameters: JsonValue,
) -> Result<AdmittedObservations, AdmissionFailure> {
    let fixture_bytes = serde_json::to_vec(&fixture::document()).expect("fixture serializes");
    let packages = package_input([fixture_bytes.as_slice()]).expect("fixture package is read");
    let model = json!({"identity": fixture::PACKAGE});
    let snapshot = |name: &str, observation: &str| {
        let identity = document_ref(name);
        let bytes = serde_json::to_vec(&json!({
            "format": "filament.state.snapshot/v1",
            "identity": identity_json(&identity),
            "observation": observation,
            "model": model,
            "populations": [
                {
                    "population": fixture::id("staff"), "complete": true,
                    "objects": [
                        {"key": "s1", "type": fixture::id("Staff"), "fields": {"badge": {"integer": "1"}}},
                        {"key": "s2", "type": fixture::id("Staff"), "fields": {"badge": {"integer": "2"}}},
                    ],
                },
                {
                    "population": fixture::id("orders"), "complete": true,
                    "objects": [{"key": "o1", "type": fixture::id("Order"), "fields": {"total": {"integer": "100"}}}],
                },
                {"population": fixture::id("desks"), "complete": true, "objects": []},
            ],
        }))
        .expect("snapshot serializes");
        (identity, bytes)
    };
    let (pre, pre_bytes) = snapshot("pre", "pre");
    let (post, post_bytes) = snapshot("post", "post");
    let invocation_identity = document_ref("call");
    let document_reference =
        |reference: &DocumentRef| json!({"identity": identity_json(reference)});
    let invocation_bytes = serde_json::to_vec(&json!({
        "format": "filament.state.invocation/v1",
        "identity": identity_json(&invocation_identity),
        "model": model,
        "context": fixture::id("Order"),
        "operation": "note",
        "self": {"population": fixture::id("orders"), "key": "o1"},
        "pre": document_reference(&pre),
        "post": document_reference(&post),
        "parameters": parameters,
        "result": {"boolean": true},
        "created": [],
        "deleted": [],
    }))
    .expect("invocation serializes");
    let invocation = invocation_identity.clone();
    let snapshots = BTreeMap::from([(pre, pre_bytes), (post, post_bytes)]);
    let invocations = BTreeMap::from([(invocation.clone(), invocation_bytes)]);
    let checked = package
        .graph()
        .state_clause("NoteBy")
        .expect("the NoteBy clause");
    let facts = ClauseFacts {
        identity: checked.identity(),
        kind: checked.kind(),
        context: checked.context(),
        operation: checked.operation().map(|operation| OperationFacts {
            declaring: operation.declaring.clone(),
            declaration: operation.declaration.clone(),
        }),
        operations: Default::default(),
        named_populations: checked.named_reads(),
    };
    admit_observations(
        package.graph().model_selections(),
        package.graph().scope().types(),
        &facts,
        &packages,
        ModelNormalizationLimits::default(),
        &Provisions {
            snapshots: &snapshots,
            invocations: &invocations,
        },
        &ClauseSelection {
            name: "NoteBy".to_owned(),
            input: ClauseSelectionInput::Invocation { invocation },
        },
        ObservationLimits::default(),
    )
}

fn holds_note_by(package: &CheckedPackage, observations: &AdmittedObservations) -> bool {
    let evaluated = package
        .evaluate_clause(
            &QualifiedName::unqualified("NoteBy").expect("clause name"),
            observations,
            &mut Meter::new(UNLIMITED),
        )
        .expect("admitted observations evaluate");
    match evaluated.evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(value))) => value,
        other => panic!("NoteBy completes with a Boolean, not {other:?}"),
    }
}

fn refusal(
    result: Result<AdmittedObservations, AdmissionFailure>,
) -> (String, String, Option<String>, Option<String>) {
    match result {
        Err(AdmissionFailure::Refused(record)) => (
            record.catalog.code().to_owned(),
            record.catalog.cause().to_owned(),
            record.fields.get("field").cloned(),
            record.fields.get("path").cloned(),
        ),
        other => panic!("a refusal, not {other:?}"),
    }
}

/// FR-106-AC-71 / TC-3040 step 3: invocation admission interprets optional
/// operation parameters consistently with the checked `NoteBy` clause.
#[trace("FR-106-AC-71", "TC-3040", "FSpec-TC-2054")]
#[test]
fn a_run_passes_present_or_absent_for_an_optional_parameter() {
    let package = package();
    let first = reference("staff", "s1");
    let absent = json!({"absent": {}});
    let present = json!({"present": reference("staff", "s2")});

    let absent_observations = admit_invocation(
        &package,
        json!({"firstApprover": first, "secondApprover": absent.clone()}),
    )
    .expect("explicit absent optional parameter is admitted");
    assert!(!holds_note_by(&package, &absent_observations));

    let present_observations = admit_invocation(
        &package,
        json!({"firstApprover": reference("staff", "s1"), "secondApprover": present}),
    )
    .expect("present optional parameter is admitted");
    assert!(holds_note_by(&package, &present_observations));

    let omitted_observations =
        admit_invocation(&package, json!({"firstApprover": reference("staff", "s1")}))
            .expect("omitted trailing optional parameter is admitted as absent");
    assert!(!holds_note_by(&package, &omitted_observations));

    let wrong_kind = refusal(admit_invocation(
        &package,
        json!({"firstApprover": reference("staff", "s1"), "secondApprover": reference("staff", "s2")}),
    ));
    assert_eq!(wrong_kind.0, "invalid_runtime_input");
    assert_eq!(wrong_kind.1, "wrong-value-kind");
    assert_eq!(wrong_kind.2.as_deref(), Some("secondApprover"));
    assert_eq!(wrong_kind.3.as_deref(), Some("/parameters/secondApprover"));

    let missing_required = refusal(admit_invocation(
        &package,
        json!({"secondApprover": absent}),
    ));
    assert_eq!(missing_required.0, "invalid_runtime_input");
    assert_eq!(missing_required.1, "missing-member");
    assert_eq!(missing_required.2.as_deref(), Some("firstApprover"));
    assert_eq!(missing_required.3.as_deref(), Some("/parameters"));
}
