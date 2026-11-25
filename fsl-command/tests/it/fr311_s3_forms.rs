// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-311-AC-3: probabilistic declarations build at S2, then their semantic
//! defects are refused at S3 through the command's compile path.

use std::path::Path;

use fsl_command::command;
use fsl_forms::{DeclarationForm, build_unit};
use fsl_foundation::SourceIdentity;
use fsl_replay::{Operation, spine::CompileRefusal};
use fsl_semantics::check::{CheckCause, ProbabilisticCause, WeightOwner};
use ix_trace_rs::trace;
use serde_json::{Value, json};

const MODEL: &[u8] = include_bytes!("../../../fsl-conformance/cases/service-package.json");
const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.model.complete/v1\";\n\
    model Service = \"test/service\";\n\
    dimension Time;\n\
    unit ms : Time = rational(1, 1);\n";
const ZERO_RANDOM: &str = "random Server::attempt(d) ~ { 1 ms: 0 };";
const INCOMPLETE_WORKLOAD: &str = "workload Steady on Service { \
    weight Server::request = 1; weight Server::attempt = 1; }";

fn source(declaration: &str) -> String {
    format!("{HEADER}{declaration}\n")
}

fn request(directory: &Path, source: &str) -> std::path::PathBuf {
    std::fs::write(directory.join("program.native"), source).expect("program writes");
    std::fs::write(directory.join("service.json"), MODEL).expect("model writes");
    let path = directory.join("compile.json");
    let request = json!({"format": "native-compile/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source": {
            "file": "service.json", "authority": "test", "identity": "test/service",
            "document": "Service", "formal_revision": 1}}],
        "program": {"source": {"file": "program.native", "authority": "test",
            "identity": "fr-311-ac-3", "document": "ServiceUnit", "formal_revision": 1}}
    }});
    std::fs::write(
        &path,
        serde_json::to_vec(&request).expect("request serializes"),
    )
    .expect("request writes");
    path
}

fn assert_s2_builds(text: &str, declaration: &str, is_random: bool) {
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "fr-311-ac-3"),
        "program.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the declaration");
    assert!(
        parsed.is_admissible(),
        "S1 admits the declaration: {:?}",
        parsed.diagnostics()
    );
    let unit = build_unit(&parsed).expect("S2 builds the declaration");
    let forms = unit.forms();
    assert_eq!(
        forms.len(),
        3,
        "the unit, its time unit, and declaration build"
    );
    assert!(matches!(forms[0].form(), DeclarationForm::Dimension(_)));
    assert!(matches!(forms[1].form(), DeclarationForm::Unit(_)));
    let built = &forms[2];
    let span = built.span();
    assert_eq!(&text[span.start..span.end], declaration);
    match (is_random, built.form()) {
        (true, DeclarationForm::Random(form)) => {
            assert_eq!(form.parameter.name, "d");
            assert_eq!(form.support.len(), 1);
            assert_eq!(
                form.support[0].weight.kind,
                fsl_forms::ExactValueKind::Natural
            );
            assert_eq!(form.support[0].weight.first.to_string(), "0");
            assert_eq!(form.support[0].weight.second.to_string(), "1");
        }
        (false, DeclarationForm::Workload(form)) => {
            assert_eq!(form.name.name, "Steady");
            assert_eq!(form.weights.len(), 2);
            assert_eq!(form.weights[0].operation.name, "Server::request");
            assert_eq!(form.weights[1].operation.name, "Server::attempt");
        }
        (_, other) => panic!("the declaration builds as its authored form: {other:?}"),
    }
}

fn assert_s3_refuses(path: &Path, random: bool) {
    let error = command::compile_outcome(path)
        .err()
        .expect("S3 refuses the invalid probabilistic declaration");
    let command::RunCause::Spine(failure) = &error.cause else {
        panic!("a compiler-stage refusal: {:?}", error.cause);
    };
    assert_eq!(failure.refusal.stage().as_str(), "check");
    let refusals = match &failure.refusal {
        CompileRefusal::Check { refusals, .. }
        | CompileRefusal::CheckDiagnostics { refusals, .. } => refusals,
        other => panic!("the refusal comes from S3: {other:?}"),
    };
    if random {
        assert_eq!(
            refusals.len(),
            2,
            "the exact random declaration refusals: {refusals:?}"
        );
        assert!(matches!(
            refusals[0].cause.as_ref(),
            CheckCause::Probabilistic(cause)
                if matches!(cause.as_ref(), ProbabilisticCause::InvalidWeight {
                    owner: WeightOwner::Parameter { operation, parameter }, ..
                } if operation == "Server::attempt" && parameter == "d")
        ));
        assert!(matches!(
            refusals[1].cause.as_ref(),
            CheckCause::Probabilistic(cause)
                if matches!(cause.as_ref(), ProbabilisticCause::ValueOutsideType {
                    operation, parameter, ..
                } if operation == "Server::attempt" && parameter == "d")
        ));
    } else {
        assert_eq!(
            refusals.len(),
            1,
            "the exact workload refusal: {refusals:?}"
        );
    }
    let CheckCause::Probabilistic(cause) = refusals[0].cause.as_ref() else {
        panic!("a probabilistic check refusal: {:?}", refusals[0].cause);
    };
    match (random, cause.as_ref()) {
        (
            true,
            ProbabilisticCause::InvalidWeight {
                owner:
                    WeightOwner::Parameter {
                        operation,
                        parameter,
                    },
                ..
            },
        ) => assert_eq!(
            (operation.as_str(), parameter.as_str()),
            ("Server::attempt", "d")
        ),
        (
            false,
            ProbabilisticCause::IncompleteWorkload {
                workload,
                operation,
                ..
            },
        ) => assert_eq!(
            (workload.as_str(), operation.as_str()),
            ("Steady", "Server::reset")
        ),
        (true, ProbabilisticCause::InvalidWeight { .. }) => {}
        (_, other) => panic!("the refusal names the exact defect: {other:?}"),
    }
    let document: Value = serde_json::from_slice(
        &error
            .outcome_document(Operation::Package)
            .to_bytes()
            .expect("S3 refusal serializes"),
    )
    .expect("the outcome is JSON");
    assert_eq!(
        document["last_stage"], "S3",
        "the command reports S3: {document}"
    );
    assert_eq!(
        document["category"], "refusal",
        "the command refuses: {document}"
    );
    assert_eq!(
        document["diagnostics"]
            .as_array()
            .expect("diagnostics are an array")
            .iter()
            .map(|diagnostic| diagnostic["cause"].as_str().expect("cause is text"))
            .collect::<Vec<_>>(),
        if random {
            vec!["invalid-weight", "value-outside-type"]
        } else {
            vec!["incomplete-workload"]
        },
        "literal S3 causes: {document}"
    );
}

/// Both declarations build as their literal S2 forms, then the public compile
/// command rejects each at S3 with its exact probabilistic cause and subject.
#[trace("FR-311-AC-3")]
#[test]
fn random_zero_weight_and_missing_reset_build_at_s2_then_refuse_at_s3() {
    for (declaration, is_random) in [(ZERO_RANDOM, true), (INCOMPLETE_WORKLOAD, false)] {
        let text = source(declaration);
        assert_s2_builds(&text, declaration, is_random);
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = request(directory.path(), &text);
        assert_s3_refuses(&path, is_random);
    }
}
