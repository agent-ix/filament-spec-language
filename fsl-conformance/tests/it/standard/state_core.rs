// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2048: every case of the standard's state-core fixtures, compiled
//! through FSL's public parse, model selection and check.
//!
//! Typing judgments are rendered as a clause over the fixture's import alias
//! `M` at the case's anchor. Published `.native` sources are compiled as
//! published bytes, and the boundary cases that mutate a source apply the
//! fixture's mutation to those bytes. Every stage's catalog code is compared
//! with the fixture's expected diagnostic, syntax, typing and evaluation.
//! The fixtures declare their model binding unavailable: no domain package
//! for `example/config` or `example/rule-tests` is published, so model
//! selection runs with no supplied package and its result is what is
//! observed.

use filament_exact::Cancel;
use fsl_foundation::diagnostic::StageFailure;
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{self, DependencyInput, FrontEndFailure, ParseRequest, SpineLimits};
use fsl_semantics::model::intake::PackageInput;
use ix_trace_rs::trace;
use serde::Deserialize;
use serde_json::Value;

use super::{gap, Corpus, Difference, KnownGap};

const ROOT: &str = "proposals/state-core/fixtures";

/// The owner of every case whose expectation needs the model binding the
/// fixtures declare unavailable.
const UNBOUND: &str =
    "FSpec state-core: model binding unavailable, no domain package published for the fixture's model";
/// The owner of every case that mutates a review wire no FSL input reads.
const REVIEW_WIRE: &str =
    "FSpec state-core: agent-a review wire is not an FSL compiler input";

/// Cases that do not currently conform, asserted exactly.
const KNOWN_GAPS: &[KnownGap] = &[
    gap("typing/absent-parent-implication", UNBOUND),
];

#[derive(Debug, Deserialize)]
struct TypingCase {
    id: String,
    expression: String,
    anchor: String,
    expected: TypingExpected,
}

#[derive(Debug, Deserialize)]
struct TypingExpected {
    syntax: String,
    typing: String,
    diagnostic: Option<String>,
    evaluation: Evaluation,
}

#[derive(Debug, Deserialize)]
struct Evaluation {
    state: String,
    value: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct Execution {
    execution: String,
    value: Option<bool>,
    diagnostic: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExecutionCase {
    id: String,
    expected: Execution,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BoundaryCase {
    id: String,
    artifact: String,
    mutation: String,
    expected_diagnostic: String,
}

#[derive(Debug, Deserialize)]
struct ReferenceCase {
    id: String,
    expected: String,
}

#[derive(Debug, Deserialize)]
struct Cases<T> {
    cases: Vec<T>,
}

fn file(name: &str) -> Result<&'static [u8], String> {
    let path = format!("{ROOT}/{name}");
    filament_specification::published_file(&path)
        .ok_or_else(|| format!("the pinned FSpec package does not publish {path}"))
}

fn cases<T: serde::de::DeserializeOwned>(name: &str) -> Result<Vec<T>, String> {
    let cases: Cases<T> =
        serde_json::from_slice(file(name)?).map_err(|error| format!("{ROOT}/{name}: {error}"))?;
    if cases.cases.is_empty() {
        return Err(format!("{ROOT}/{name} publishes no cases"));
    }
    Ok(cases.cases)
}

/// Where the public front end stopped.
#[derive(Debug, PartialEq, Eq)]
enum Stopped {
    /// S1/S2 refused the bytes.
    Parse(String),
    /// I1 model selection refused.
    Select(String),
    /// S3 refused the checked package.
    Check(String),
}

impl std::fmt::Display for Stopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(code) => write!(f, "parse refused {code}"),
            Self::Select(code) => write!(f, "model selection refused {code}"),
            Self::Check(code) => write!(f, "check refused {code}"),
        }
    }
}

impl Stopped {
    fn code(&self) -> &str {
        match self {
            Self::Parse(code) | Self::Select(code) | Self::Check(code) => code,
        }
    }
}

fn failure_code(failure: &FrontEndFailure) -> String {
    match failure {
        StageFailure::Refused(refusal) => refusal.code().as_str().to_owned(),
        StageFailure::RefusedWithPayload(refusal, _) => refusal.code().as_str().to_owned(),
        StageFailure::Limit(_) | StageFailure::LimitWithPayload(..) => {
            "resource_exhausted".to_owned()
        }
        StageFailure::Cancelled(_) => "cancelled".to_owned(),
        StageFailure::Fault(fault) => format!("internal fault {fault:?}"),
    }
}

/// Compiles `bytes` through public parse, model selection with no supplied
/// domain package, and check. `Ok` means the unit checked.
fn compile(id: &str, bytes: &[u8]) -> Result<(), Stopped> {
    let identity = SourceIdentity::new("fspec-state-core", id);
    let cancel = Cancel::new();
    let limits = SpineLimits::default();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: id,
            bytes,
        },
        limits.source,
        &cancel,
    )
    .map_err(|failure| Stopped::Parse(failure_code(&failure)))?
    .into_value();
    let packages = PackageInput::default();
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(|failure| Stopped::Select(failure_code(&failure)))?
        .into_value();
    spine::check(
        &parsed,
        "state-core".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(|failure| Stopped::Check(failure_code(&failure)))?;
    Ok(())
}

/// The typing judgment's expression as a clause of the current native
/// edition over the fixture's import alias `M` and context type `Node`.
fn typing_source(case: &TypingCase, alias: &str, model: &str) -> Result<String, String> {
    let clause = match case.anchor.as_str() {
        "current" => format!("invariant Case using v on {alias}::Node at current"),
        "post" => format!("post Case using v on {alias}::Node::step"),
        other => return Err(format!("no clause adapter for anchor {other}")),
    };
    Ok(format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model {alias} = \"{model}\";\n\
         {clause} {{\n  {}\n}}\n",
        case.expression
    ))
}

fn typing_outcome(case: &TypingCase, alias: &str, model: &str) -> Result<(), Difference> {
    let expected = &case.expected;
    let wanted = format!(
        "syntax {}, typing {}, diagnostic {}, evaluation {}{}",
        expected.syntax,
        expected.typing,
        expected.diagnostic.as_deref().unwrap_or("none"),
        expected.evaluation.state,
        expected
            .evaluation
            .value
            .map_or(String::new(), |value| format!(" {value}"))
    );
    let source = typing_source(case, alias, model).map_err(|error| Difference::new(&wanted, error))?;
    match compile(&case.id, source.as_bytes()) {
        // The expression is not in the source language: the judgment is not
        // typed or evaluated, which is exactly what `unsupported` states.
        Err(Stopped::Parse(code))
            if expected.syntax == "unsupported"
                && expected.diagnostic.as_deref() == Some(code.as_str()) =>
        {
            Ok(())
        }
        // A typing refusal is only comparable once the model is bound.
        Err(stopped) => Err(Difference::new(wanted, stopped)),
        Ok(()) => Err(Difference::new(
            wanted,
            "checked; no snapshot input exists for the fixture's abstract hypotheses",
        )),
    }
}

fn execution_outcome(id: &str, source: &str, expected: &Execution) -> Result<(), Difference> {
    let wanted = format!(
        "execution {}{}{}",
        expected.execution,
        expected
            .value
            .map_or(String::new(), |value| format!(" {value}")),
        expected
            .diagnostic
            .as_deref()
            .map_or(String::new(), |code| format!(" {code}"))
    );
    let bytes = file(source).map_err(|error| Difference::new(&wanted, error))?;
    match compile(id, bytes) {
        Err(stopped) => Err(Difference::new(wanted, stopped)),
        Ok(()) => Err(Difference::new(
            wanted,
            "checked; the fixture's snapshots have no FSL observation adapter",
        )),
    }
}

/// The source bytes of a boundary case that mutates a published source, or
/// `None` when the case mutates a wire no FSL input reads.
fn boundary_source(case: &BoundaryCase) -> Result<Option<Vec<u8>>, String> {
    let original = || -> Result<String, String> {
        String::from_utf8(file("parent-order.native")?.to_vec()).map_err(|error| error.to_string())
    };
    let expression =
        "present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber";
    let mutated = match (case.artifact.as_str(), case.id.as_str()) {
        ("subject.json", "missing-model") => original()?,
        ("parent-order.native", "unsupported-collect") => {
            original()?.replacen(expression, "collect(self.children)", 1)
        }
        ("parent-order.native", "malformed-expression") => {
            let source = original()?;
            let at = source.rfind('}').ok_or("the source has a closing brace")?;
            format!("{}{}", &source[..at], &source[at + 1..])
        }
        ("parent-order.native", "undefined-option") => original()?.replacen(
            expression,
            "deref(value(self.parent)).versionNumber < self.versionNumber",
            1,
        ),
        ("parent-order.native", other) => {
            return Err(format!("no mutation adapter for {other}: {}", case.mutation))
        }
        _ => return Ok(None),
    };
    Ok(Some(mutated.into_bytes()))
}

fn boundary_outcome(case: &BoundaryCase) -> Result<(), Difference> {
    let expected = &case.expected_diagnostic;
    let Some(bytes) =
        boundary_source(case).map_err(|error| Difference::new(expected, error))?
    else {
        return Err(Difference::new(
            expected,
            format!("no FSL input reads {} ({})", case.artifact, case.mutation),
        ));
    };
    match compile(&case.id, &bytes) {
        Err(stopped) if stopped.code() == expected => Ok(()),
        Err(stopped) => Err(Difference::new(expected, stopped)),
        Ok(()) => Err(Difference::new(expected, "checked without a refusal")),
    }
}

fn reference_outcome(case: &ReferenceCase) -> Result<(), Difference> {
    Err(Difference::new(
        &case.expected,
        "no FSL input reads the ArtifactRef/SemanticRef review wire",
    ))
}

fn run_corpus() -> Result<Corpus, String> {
    let typing: Value =
        serde_json::from_slice(file("typing-cases.json")?).map_err(|error| error.to_string())?;
    let binding = &typing["modelBinding"];
    let alias = binding["importAlias"]
        .as_str()
        .ok_or("typing-cases.json names an import alias")?;
    let model = binding["types"]["Node"]["identity"]
        .as_str()
        .and_then(|identity| identity.strip_prefix("ix://"))
        .and_then(|identity| identity.split_once("/type/"))
        .map(|(model, _)| model)
        .ok_or("typing-cases.json names the Node type's model")?;
    let typing: Vec<TypingCase> = cases("typing-cases.json")?;
    let state: Vec<ExecutionCase> = cases("state-cases.json")?;
    let invocation: Vec<ExecutionCase> = cases("invocation-cases.json")?;
    let boundary: Vec<BoundaryCase> = cases("boundary-cases.json")?;
    let reference: Vec<ReferenceCase> = cases("reference-cases.json")?;
    let published = typing.len() + state.len() + invocation.len() + boundary.len() + reference.len();
    let mut corpus = Corpus::new("state-core", published);
    for case in &typing {
        corpus.record(format!("typing/{}", case.id), typing_outcome(case, alias, model));
    }
    for case in &state {
        corpus.record(
            format!("state/{}", case.id),
            execution_outcome(&case.id, "parent-order.native", &case.expected),
        );
    }
    for case in &invocation {
        corpus.record(
            format!("invocation/{}", case.id),
            execution_outcome(&case.id, "version-update.native", &case.expected),
        );
    }
    for case in &boundary {
        corpus.record(format!("boundary/{}", case.id), boundary_outcome(case));
    }
    for case in &reference {
        corpus.record(format!("reference/{}", case.id), reference_outcome(case));
    }
    Ok(corpus)
}

/// Provenance: QSL-2048. Every state-core case runs through the public
/// compile path; the failures are exactly the listed known gaps.
#[trace("FR-350-AC-20")]
#[test]
fn conformance_fspec_state_core_fixtures() {
    run_corpus()
        .unwrap_or_else(|error| panic!("{error}"))
        .assert_settled(KNOWN_GAPS);
}

/// Provenance: QSL-2048. The accounted cases are the published cases: the
/// five case files publish 51, 9, 7, 17 and 10 cases.
#[trace("FR-350-AC-21")]
#[test]
fn every_published_state_core_case_is_accounted() {
    let corpus = run_corpus().unwrap_or_else(|error| panic!("{error}"));
    let summary = corpus
        .settle(KNOWN_GAPS)
        .unwrap_or_else(|errors| panic!("{}", errors.join("\n")));
    assert_eq!(summary.cases, 51 + 9 + 7 + 17 + 10);
}

/// Provenance: QSL-2048. An altered expected code is reported as a mismatch.
#[trace("FR-350-AC-20")]
#[test]
fn an_altered_state_core_expectation_is_a_mismatch() {
    let mut case = cases::<BoundaryCase>("boundary-cases.json")
        .expect("boundary cases")
        .into_iter()
        .find(|case| case.id == "malformed-expression")
        .expect("malformed-expression");
    let observed = boundary_outcome(&case);
    case.expected_diagnostic = "altered_code".to_owned();
    let altered = boundary_outcome(&case).expect_err("an altered code fails");
    assert_eq!(altered.expected, "altered_code");
    assert_ne!(Err(altered), observed);
}

/// Provenance: QSL-2048. A missing published file fails before any case runs.
#[trace("FR-350-AC-20")]
#[test]
fn a_missing_state_core_file_is_an_error() {
    assert_eq!(
        cases::<ReferenceCase>("absent.json").err(),
        Some(format!(
            "the pinned FSpec package does not publish {ROOT}/absent.json"
        ))
    );
}
