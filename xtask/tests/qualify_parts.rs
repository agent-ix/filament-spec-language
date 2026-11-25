// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1396: the `qualify` gate's verdict covers the corpus, the lifecycle
//! matrix and the parity of the linked backends (FR-567).

use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::TempDir;

use ix_trace_rs::trace;
use serde_json::Value;
use xtask::qualify::{self, QualifyInputs};

fn fspec_dir() -> TempDir {
    xtask::published_fspec::directory("proposals/")
}

/// Generated parity units for a test that asserts nothing about them.
#[cfg(feature = "test-support")]
const FEW_UNITS: u64 = 8;

/// The command's inputs over the published fixtures. With `test-support` the
/// generated parity units are [`FEW_UNITS`].
fn inputs<'a>() -> (TempDir, QualifyInputs<'a>) {
    let fspec = fspec_dir();
    let fixtures = fspec
        .path()
        .join("proposals/conformance-corpus-v1/fixtures");
    let inputs = QualifyInputs {
        fspec: fspec.path().to_path_buf(),
        operands: vec!["--corpus".into(), fixtures.into_os_string()],
        #[cfg(feature = "test-support")]
        matrix: None,
        #[cfg(feature = "test-support")]
        parity: None,
        #[cfg(feature = "test-support")]
        parity_count: Some(FEW_UNITS),
        #[cfg(not(feature = "test-support"))]
        _marker: std::marker::PhantomData,
    };
    (fspec, inputs)
}

#[cfg(feature = "test-support")]
fn decimal_parity_fixture() -> TempDir {
    use std::fs;

    let directory = tempfile::tempdir().expect("decimal parity fixture");
    fs::write(
        directory.path().join("unit.native"),
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction half using v(): Decimal[-100, 100; 0, 2; nearest-even] pure { 1.5 }\n",
    )
    .expect("decimal source");
    fs::write(
        directory.path().join("call.json"),
        r#"{"function":"half","arguments":[],"accounting":{}}"#,
    )
    .expect("decimal call");
    let corpus = serde_json::json!({
        "contract_version":"filament.conformance-corpus/v1",
        "vectors":[{
            "id":"rounding-run",
            "capability_id":"V1-TOOL-004",
            "class":"positive",
            "applicability":{"consumer_types":["runtime"]},
            "subject":{"consumer_type":"runtime","operation":"run"},
            "input":{"files":[
                {"kind":"source","path":"unit.native"},
                {"kind":"call","path":"call.json"}
            ]},
            "expected":{"disposition":"value","value":null}
        }],
        "results":[],
        "coverage":{
            "primary":[{"capability_id":"V1-TOOL-004","applicability":{
                "consumer_types":["runtime"],
                "semantic_profiles":["filament.value.complete/v1"]
            }}],
            "criteria_rollups":[],
            "consumer_rollups":[]
        }
    });
    write_corpus(directory.path(), &corpus);
    directory
}

/// Writes `document` as the canonical corpus document in `directory`.
#[cfg(feature = "test-support")]
fn write_corpus(directory: &Path, document: &Value) {
    std::fs::write(
        directory.join("complete-v1.json"),
        serde_json_canonicalizer::to_vec(document).expect("canonical corpus"),
    )
    .expect("corpus document");
}

fn report(outcome: &xtask::Outcome) -> Value {
    serde_json::from_str(&outcome.stdout).expect("the report is JSON")
}

fn part<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["parts"]
        .as_array()
        .and_then(|parts| parts.iter().find(|part| part["part"] == name))
        .unwrap_or_else(|| panic!("the report names the part {name}: {report}"))
}

#[cfg(feature = "test-support")]
#[derive(Clone, Copy)]
struct RoundingBackend;

#[cfg(feature = "test-support")]
struct RoundingPrepared<'p>(fsl_eval::execution::InterpreterPrepared<'p>);

#[cfg(feature = "test-support")]
impl fsl_eval::execution::ExecutionBackend for RoundingBackend {
    type Prepared<'p> = RoundingPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p fsl_package::CheckedPackage,
        entries: &[fsl_eval::execution::EntrySelection],
        limits: &fsl_eval::execution::PrepareLimits,
        cancel: &filament_exact::Cancel,
    ) -> Result<Self::Prepared<'p>, fsl_eval::execution::PrepareFailure> {
        fsl_eval::execution::Interpreter
            .prepare(package, entries, limits, cancel)
            .map(RoundingPrepared)
    }
}

#[cfg(feature = "test-support")]
impl fsl_eval::execution::PreparedEntries for RoundingPrepared<'_> {
    fn unsupported(
        &self,
    ) -> &[(
        fsl_eval::execution::EntrySelection,
        filament_eval::UnsupportedEntry,
    )] {
        fsl_eval::execution::PreparedEntries::unsupported(&self.0)
    }

    fn call(
        &self,
        entry: &fsl_eval::value::QualifiedName,
        arguments: &[filament_exact::Value],
        environment: &fsl_semantics::model::object_environment::ObjectEnvironment,
        limits: &fsl_eval::execution::CallLimits,
        run: &mut filament_semantic_value::supplied_stop::SuppliedRun,
        cancel: &filament_exact::Cancel,
    ) -> Result<filament_eval::Evaluation, filament_eval::CallFailure> {
        let mut result = fsl_eval::execution::PreparedEntries::call(
            &self.0,
            entry,
            arguments,
            environment,
            limits,
            run,
            cancel,
        )?;
        if let filament_eval::FamilyOutcome::Evaluated(filament_exact::Outcome::Completed(
            filament_exact::Value::Decimal(value),
        )) = &result.outcome
        {
            result.outcome =
                filament_eval::FamilyOutcome::Evaluated(filament_exact::Outcome::Completed(
                    filament_exact::Value::Decimal(round_to_integer(value)),
                ));
        }
        Ok(result)
    }

    fn evaluate(
        &self,
        clause: &fsl_eval::value::QualifiedName,
        input: &fsl_semantics::model::observation::AdmittedObservations,
        accounting: &filament_exact::ScalarLimits,
        cancel: &filament_exact::Cancel,
    ) -> Result<filament_eval::Evaluation, filament_eval::CallFailure> {
        fsl_eval::execution::PreparedEntries::evaluate(&self.0, clause, input, accounting, cancel)
    }
}

#[cfg(feature = "test-support")]
fn round_to_integer(value: &filament_exact::Decimal) -> filament_exact::Decimal {
    use filament_exact::{Integer, NonZeroInteger};
    let normalized = value.normalized();
    let divisor = Integer::from(10_i64.pow(normalized.scale()));
    let nonzero = NonZeroInteger::try_from(&divisor).expect("a decimal scale has a nonzero base");
    let (mut quotient, remainder) = normalized.coefficient().div_rem_truncating(nonzero);
    if remainder.abs().mul(&Integer::from(2_i64)) >= divisor {
        let one = Integer::from(1_i64);
        quotient = if normalized.coefficient().is_negative() {
            quotient.sub(&one)
        } else {
            quotient.add(&one)
        };
    }
    filament_exact::Decimal::new(quotient, 0)
}

/// FR-567-AC-1: all three parts pass with a linked backend that delegates to
/// the interpreter.
#[cfg(feature = "test-support")]
#[trace("TC-1396", "FR-567-AC-1")]
#[test]
fn a_parity_backed_run_names_the_three_parts_and_qualifies() {
    use filament_eval::ExecutionBackendChoice;
    use fsl_eval::execution::Interpreter;
    let (_fspec, mut inputs) = inputs();
    let interpreter = Interpreter;
    inputs.parity = Some(xtask::qualify::ParityTestOverride {
        reference: &interpreter,
        backends: vec![(ExecutionBackendChoice::Jit, &interpreter, None)],
    });
    let outcome = qualify::run(&inputs).expect("the gate runs");
    assert_eq!(outcome.code, 0, "{}", outcome.stdout);
    let report = report(&outcome);
    assert_eq!(report["verdict"], "qualified");
    assert_eq!(report["qualification"], "complete-v1");
    assert_eq!(part(&report, "corpus")["result"], "passed");
    assert_eq!(part(&report, "lifecycle-matrix")["result"], "passed");
    assert_eq!(part(&report, "parity")["result"], "passed");
}

/// FR-567-AC-2: with the matrix replaced by one whose `check`/`refusal` case
/// expects exit 21, the run is `not-qualified`, names
/// `lifecycle-matrix/check/refusal` and exits 1, although the corpus passes.
#[cfg(feature = "test-support")]
#[trace("TC-1396", "FR-567-AC-2")]
#[test]
fn a_failing_matrix_case_fails_the_gate_by_name() {
    use fsl_conformance::{lifecycle_matrix, MatrixCell, OutcomeKind, Verb};
    let cells: Vec<MatrixCell> = lifecycle_matrix()
        .iter()
        .map(|cell| match cell {
            MatrixCell::Case(case)
                if case.verb == Verb::Check && case.kind == OutcomeKind::Refusal =>
            {
                MatrixCell::Case(fsl_conformance::MatrixCase {
                    expected_exit: 21,
                    ..*case
                })
            }
            other => *other,
        })
        .collect();
    let (_fspec, mut gate) = inputs();
    gate.matrix = Some(cells);
    let interpreter = fsl_eval::execution::Interpreter;
    gate.parity = Some(xtask::qualify::ParityTestOverride {
        reference: &interpreter,
        backends: vec![(
            filament_eval::ExecutionBackendChoice::Jit,
            &interpreter,
            None,
        )],
    });
    let outcome = qualify::run(&gate).expect("the gate runs");
    assert_eq!(outcome.code, 1, "{}", outcome.stdout);
    let report = report(&outcome);
    assert_eq!(report["verdict"], "not-qualified");
    assert_eq!(part(&report, "corpus")["result"], "passed");
    assert_eq!(
        part(&report, "lifecycle-matrix")["failures"],
        serde_json::json!(["lifecycle-matrix/check/refusal"])
    );
}

/// FR-567-AC-3: the test-only substituted wrong backend names the parity
/// backend and the finding in the final qualification report.
#[cfg(feature = "test-support")]
#[trace("TC-1396", "FR-567-AC-3")]
#[test]
fn a_wrong_jit_backend_fails_qualification_with_finding_detail() {
    let (_fspec, mut inputs) = inputs();
    let interpreter = fsl_eval::execution::Interpreter;
    let backend = RoundingBackend;
    let corpus = decimal_parity_fixture();
    inputs.operands[1] = corpus.path().into();
    inputs.parity = Some(xtask::qualify::ParityTestOverride {
        reference: &interpreter,
        backends: vec![(filament_eval::ExecutionBackendChoice::Jit, &backend, None)],
    });
    let outcome = qualify::run(&inputs).expect("the gate runs");
    assert_eq!(outcome.code, 1, "{}", outcome.stdout);
    let report = report(&outcome);
    assert_eq!(report["verdict"], "not-qualified");
    let failures = part(&report, "parity")["failures"]
        .as_array()
        .expect("the parity finding is retained");
    assert!(
        failures.iter().any(|failure| {
            failure
                .as_str()
                .is_some_and(|text| text.contains("parity/jit") && text.contains("Difference"))
        }),
        "parity failures: {failures:#?}"
    );
}

/// FR-567-AC-4: no linked backend makes parity a passing `no-backend` part.
#[cfg(feature = "test-support")]
#[trace("TC-1396", "FR-567-AC-4")]
#[test]
fn no_linked_backend_is_a_passing_no_backend_part() {
    use fsl_eval::execution::Interpreter;
    let (_fspec, mut inputs) = inputs();
    let interpreter = Interpreter;
    inputs.parity = Some(xtask::qualify::ParityTestOverride {
        reference: &interpreter,
        backends: Vec::new(),
    });
    let outcome = qualify::run(&inputs).expect("the gate runs");
    assert_eq!(outcome.code, 0, "{}", outcome.stdout);
    let report = report(&outcome);
    assert_eq!(report["verdict"], "qualified");
    assert_eq!(part(&report, "parity")["result"], "no-backend");
}

/// FR-567-AC-4: a faulting substituted reference makes the parity part fail
/// with a typed `reference-fault` finding.
#[cfg(feature = "test-support")]
#[trace("TC-1396", "FR-567-AC-4")]
#[test]
fn reference_fault_fails_qualification_by_finding() {
    use fsl_eval::execution::Interpreter;
    let (_fspec, mut inputs) = inputs();
    let fault = fsl_conformance::FaultInjectingBackend;
    let interpreter = Interpreter;
    inputs.parity = Some(xtask::qualify::ParityTestOverride {
        reference: &fault,
        backends: vec![(
            filament_eval::ExecutionBackendChoice::Jit,
            &interpreter,
            None,
        )],
    });
    let outcome = qualify::run(&inputs).expect("the gate runs");
    assert_eq!(outcome.code, 1, "{}", outcome.stdout);
    let report = report(&outcome);
    assert_eq!(report["verdict"], "not-qualified");
    let failures = part(&report, "parity")["failures"]
        .as_array()
        .expect("parity findings are reported");
    assert!(
        failures.iter().any(|failure| {
            failure
                .as_str()
                .is_some_and(|text| text.contains("parity/jit") && text.contains("ReferenceFault"))
        }),
        "parity failures: {failures:#?}"
    );
}

/// An interpreter that counts the units it was asked to prepare.
#[cfg(feature = "test-support")]
#[derive(Default)]
struct CountingBackend {
    prepares: std::sync::atomic::AtomicU64,
}

#[cfg(feature = "test-support")]
impl fsl_eval::execution::ExecutionBackend for CountingBackend {
    type Prepared<'p> = fsl_eval::execution::InterpreterPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p fsl_package::CheckedPackage,
        entries: &[fsl_eval::execution::EntrySelection],
        limits: &fsl_eval::execution::PrepareLimits,
        cancel: &filament_exact::Cancel,
    ) -> Result<Self::Prepared<'p>, fsl_eval::execution::PrepareFailure> {
        self.prepares
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        fsl_eval::execution::Interpreter.prepare(package, entries, limits, cancel)
    }
}

/// FR-567-AC-7: a replacement generated-unit count sets how many units the
/// parity part generates, and the command line offers no such setting.
#[cfg(feature = "test-support")]
#[trace("TC-1396", "FR-567-AC-7")]
#[test]
fn a_replacement_unit_count_sets_the_generated_units_and_has_no_option() {
    let (_fspec, mut inputs) = inputs();
    let corpus = std::path::PathBuf::from(&inputs.operands[1]);
    let interpreter = fsl_eval::execution::Interpreter;
    let mut corpus_only = fsl_conformance::ParityInputs::new(&corpus);
    corpus_only.count = 0;
    let corpus_cases = fsl_conformance::run_parity(
        &[(
            filament_eval::ExecutionBackendChoice::Jit,
            &interpreter,
            None,
        )],
        &corpus_only,
        &fsl_conformance::ParityLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("the fixture corpus is readable")
    .units;

    let counting = CountingBackend::default();
    inputs.parity_count = Some(3);
    inputs.parity = Some(xtask::qualify::ParityTestOverride {
        reference: &interpreter,
        backends: vec![(filament_eval::ExecutionBackendChoice::Jit, &counting, None)],
    });
    let outcome = qualify::run(&inputs).expect("the gate runs");
    assert_eq!(outcome.code, 0, "{}", outcome.stdout);
    assert_eq!(
        counting.prepares.load(std::sync::atomic::Ordering::Relaxed),
        corpus_cases + 3
    );

    let refused = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["qualify", "--corpus"])
        .arg(&corpus)
        .args(["--count", "3"])
        .output()
        .expect("xtask starts");
    assert_eq!(refused.status.code(), Some(2));
    assert!(refused.stdout.is_empty(), "no verdict is given");
}

/// FR-567-AC-8: one vector that cannot be built fails the `parity` part and
/// refuses nothing; a corpus with two vectors of one id, or with a
/// parent-traversing input path, refuses before any part runs.
#[cfg(feature = "test-support")]
#[trace("TC-1396", "FR-567-AC-8")]
#[test]
fn an_unbuildable_vector_fails_parity_and_a_duplicate_id_refuses() {
    let (fspec, mut inputs) = inputs();
    let fixtures = fspec
        .path()
        .join("proposals/conformance-corpus-v1/fixtures");
    let corpus = TempDir::new().expect("corpus directory");
    std::fs::create_dir_all(corpus.path().join("inputs")).expect("input directory");
    std::fs::copy(
        fixtures.join("inputs/native.native"),
        corpus.path().join("inputs/native.native"),
    )
    .expect("fixture source");
    let mut document: Value = serde_json::from_slice(
        &std::fs::read(fixtures.join("complete-v1.json")).expect("fixture corpus"),
    )
    .expect("fixture corpus JSON");
    let mut absent = document["vectors"][0].clone();
    absent["id"] = serde_json::json!("parity-absent-source");
    absent["class"] = serde_json::json!("positive");
    absent
        .as_object_mut()
        .expect("a vector object")
        .remove("mutation_of");
    absent["subject"] = serde_json::json!({"consumer_type":"runtime","operation":"run"});
    absent["applicability"] = serde_json::json!({"consumer_types":["runtime"]});
    absent["input"] =
        serde_json::json!({"files":[{"kind":"source","path":"inputs/absent.native"}]});
    absent["expected"] = serde_json::json!({"disposition":"value","value":null});
    let mut traversing = absent.clone();
    traversing["id"] = serde_json::json!("parity-traversing-source");
    traversing["input"]["files"][0]["path"] = serde_json::json!("../outside.native");
    let vectors = document["vectors"].as_array_mut().expect("vectors");
    vectors.push(absent);
    write_corpus(corpus.path(), &document);

    inputs.operands[1] = corpus.path().into();
    let interpreter = fsl_eval::execution::Interpreter;
    inputs.parity = Some(xtask::qualify::ParityTestOverride {
        reference: &interpreter,
        backends: vec![(
            filament_eval::ExecutionBackendChoice::Jit,
            &interpreter,
            None,
        )],
    });
    let outcome = qualify::run(&inputs).expect("one unbuildable vector refuses nothing");
    assert_eq!(outcome.code, 1, "{}", outcome.stdout);
    let report = report(&outcome);
    assert_eq!(report["verdict"], "not-qualified");
    let failures = part(&report, "parity")["failures"]
        .as_array()
        .expect("parity findings are reported");
    assert_eq!(failures.len(), 1, "{failures:#?}");
    let failure = failures[0].as_str().expect("a failure name");
    assert!(
        failure.starts_with("parity/jit: UnbuildableCase(")
            && failure.contains("unit=parity-absent-source"),
        "{failure}"
    );

    let mut duplicated = document.clone();
    let vectors = duplicated["vectors"].as_array_mut().expect("vectors");
    let duplicate = vectors[0].clone();
    vectors.push(duplicate);
    let mut traversed = document;
    traversed["vectors"]
        .as_array_mut()
        .expect("vectors")
        .push(traversing);
    let counting = CountingBackend::default();
    inputs.parity = Some(xtask::qualify::ParityTestOverride {
        reference: &interpreter,
        backends: vec![(filament_eval::ExecutionBackendChoice::Jit, &counting, None)],
    });
    for refused in [duplicated, traversed] {
        write_corpus(corpus.path(), &refused);
        let refusal = match qualify::run(&inputs) {
            Err(refusal) => refusal,
            Ok(outcome) => panic!("the preflight refuses: {}", outcome.stdout),
        };
        assert!(matches!(
            refusal,
            xtask::error::Error::QualificationRefusal(_)
        ));
        assert_eq!(refusal.exit_code(), 2);
    }
    assert_eq!(
        counting.prepares.load(std::sync::atomic::Ordering::Relaxed),
        0
    );
}
fn digits_end(value: &str, from: usize) -> usize {
    value[from..]
        .find(|character: char| !character.is_ascii_digit())
        .map_or(value.len(), |offset| from + offset)
}

fn explicit_criteria(value: &str) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = value[cursor..].find("FR-") {
        let start = cursor + offset;
        let family_start = start + 3;
        let family_end = digits_end(value, family_start);
        if family_end + 4 <= value.len() && &value[family_end..family_end + 4] == "-AC-" {
            let criterion_start = family_end + 4;
            let criterion_end = digits_end(value, criterion_start);
            if criterion_end > criterion_start {
                found.push((
                    format!(
                        "FR-{}-AC-{}",
                        &value[family_start..family_end],
                        &value[criterion_start..criterion_end]
                    ),
                    criterion_end,
                ));
                cursor = criterion_end;
                continue;
            }
        }
        cursor = family_end.max(start + 3);
    }
    found
}

fn criteria_for_location(prefix: &str) -> Vec<String> {
    let segment = prefix
        .rfind(')')
        .map_or(prefix, |close| &prefix[close + 1..]);
    let explicit = explicit_criteria(segment);
    let family = explicit
        .last()
        .map(|(criterion, _)| criterion.split('-').nth(1).expect("family").to_owned())
        .or_else(|| {
            explicit_criteria(prefix)
                .last()
                .map(|(criterion, _)| criterion.split('-').nth(1).expect("family").to_owned())
        });
    let mut criteria = explicit
        .iter()
        .map(|(criterion, _)| criterion.clone())
        .collect::<Vec<_>>();
    if let Some(family) = family {
        let mut cursor = 0;
        while let Some(offset) = segment[cursor..].find("AC-") {
            let start = cursor + offset;
            if start >= 3 && &segment[start - 3..start] == "-AC" {
                cursor = start + 3;
                continue;
            }
            let number_start = start + 3;
            let number_end = digits_end(segment, number_start);
            if number_end > number_start {
                let criterion = format!("FR-{family}-AC-{}", &segment[number_start..number_end]);
                if !criteria.contains(&criterion) {
                    criteria.push(criterion);
                }
                cursor = number_end;
            } else {
                cursor = number_start;
            }
        }
    }
    criteria
}

fn trace_at(source: &str, line_number: usize) -> Option<String> {
    let lines: Vec<_> = source.lines().collect();
    let mut at = line_number
        .checked_sub(1)?
        .min(lines.len().saturating_sub(1));
    loop {
        if lines[at].contains("#[trace(") {
            let mut attribute = lines[at].to_owned();
            while !attribute.contains(")]") && at + 1 < lines.len() {
                at += 1;
                attribute.push_str(lines[at]);
            }
            return Some(attribute);
        }
        if at == 0 {
            return None;
        }
        at -= 1;
    }
}

/// FR-500-AC-2 (TC-1294): every tabled successor source location resolves to
/// a test whose nearest `#[trace]` attribute carries the stated criterion.
#[trace("TC-1294", "FR-500-AC-2")]
#[test]
fn every_staged_successor_location_names_its_traced_test() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let spec = root.join(
        "spec/functional/FR-500-map-each-fr-042-criterion-to-its-checked-package-successor.md",
    );
    let text = std::fs::read_to_string(&spec).expect("FR-500 staging table");
    let source_root = root.canonicalize().expect("workspace root");
    let mut locations = 0usize;

    for row in text
        .lines()
        .filter(|line| line.starts_with("| AC-") && line.contains(".rs:"))
    {
        let successor_cell = row.split('|').nth(2).expect("successor cell");
        let mut cursor = 0;
        while let Some(open_offset) = successor_cell[cursor..].find('(') {
            let open = cursor + open_offset;
            let Some(close_offset) = successor_cell[open + 1..].find(')') else {
                break;
            };
            let close = open + 1 + close_offset;
            let group = &successor_cell[open + 1..close];
            if group.contains(".rs:") {
                let criteria = criteria_for_location(&successor_cell[..open]);
                assert!(!criteria.is_empty(), "criteria before {group:?}");
                let mut inherited_directory = Path::new("").to_path_buf();
                for word in group.split_whitespace() {
                    let token = word.trim_matches(['`', '(', ')', ',', ';', ':']);
                    let Some((path, line_numbers)) = token.split_once(':') else {
                        continue;
                    };
                    if !path.ends_with(".rs") {
                        continue;
                    }
                    let relative = if let Some(path) = path.strip_prefix("QP/") {
                        inherited_directory = Path::new("fsl-package/src/emit/tests").to_path_buf();
                        inherited_directory.join(path)
                    } else if let Some(path) = path.strip_prefix("EV/") {
                        inherited_directory = if path == "protocol_slots.rs" {
                            Path::new("fsl-eval/tests/it").to_path_buf()
                        } else {
                            Path::new("fsl-eval/tests/it/protocol_slots").to_path_buf()
                        };
                        inherited_directory.join(path)
                    } else {
                        let path = Path::new(path);
                        if path.components().count() > 1 {
                            inherited_directory = path
                                .parent()
                                .expect("a referenced source path has a parent")
                                .to_path_buf();
                            path.to_path_buf()
                        } else {
                            inherited_directory.join(path)
                        }
                    };
                    let source_path = source_root.join(relative);
                    let source = std::fs::read_to_string(&source_path)
                        .unwrap_or_else(|error| panic!("{}: {error}", source_path.display()));
                    for line in line_numbers.split(',') {
                        let line = line.parse::<usize>().unwrap_or_else(|error| {
                            panic!("invalid source location {token:?}: {error}")
                        });
                        let attribute = trace_at(&source, line).unwrap_or_else(|| {
                            panic!(
                                "{}:{line} has a preceding trace attribute",
                                source_path.display()
                            )
                        });
                        assert!(
                            criteria
                                .iter()
                                .any(|criterion| attribute.contains(criterion)),
                            "{}:{line}: expected one of {criteria:?} in {attribute}",
                            source_path.display()
                        );
                        locations += 1;
                    }
                }
            }
            cursor = close + 1;
        }
    }
    assert!(
        locations > 0,
        "the table contains checked successor locations"
    );
}

/// FR-500-AC-3 (TC-1294): every deletion-stage row has a named owner and an
/// explicit condition for deletion.
#[trace("TC-1294", "FR-500-AC-3")]
#[test]
fn every_staging_row_names_its_owner_and_deletion_condition() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let spec = root.join(
        "spec/functional/FR-500-map-each-fr-042-criterion-to-its-checked-package-successor.md",
    );
    let text = std::fs::read_to_string(&spec).expect("FR-500 staging table");
    let table = text
        .split_once("## Deletion staging per legacy file")
        .map(|(_, rest)| rest)
        .expect("the deletion staging section")
        .split_once("## Acceptance Criteria")
        .map(|(table, _)| table)
        .expect("the acceptance criteria follow the staging table");
    let rows = table
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.contains('|').then_some(())?;
            let mut fields = line.split('|').map(str::trim).collect::<Vec<_>>();
            if fields.first() == Some(&"") {
                fields.remove(0);
            }
            if fields.last() == Some(&"") {
                fields.pop();
            }
            let separator = fields == ["---", "---", "---"];
            let header = fields == ["Legacy file or group", "Deleted when", "Owner"];
            (!header && !separator).then_some(fields)
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 7, "the table's seven legacy groups are covered");
    for fields in rows {
        assert_eq!(fields.len(), 3, "each staging row has three cells");
        assert!(!fields[0].is_empty(), "the legacy file is named");
        assert!(!fields[1].is_empty(), "the deletion condition is named");
        assert!(!fields[2].is_empty(), "the owner is named");
    }
}

/// The default run, with no substitution: the linked JIT backend is compared
/// with the interpreter. Over the passing fixture corpus with every matrix case
/// passing, the report names the three parts as passed,
/// `complete-v1: qualified`, exit 0. Carries no AC tag: FR-567-AC-1 is stated
/// over a delegating test backend.
#[trace("TC-1396")]
#[test]
fn a_passing_run_names_the_three_parts_and_qualifies() {
    #[allow(unused_mut)]
    let (_fspec, mut inputs) = inputs();
    // The default run keeps the command's generated-unit count.
    #[cfg(feature = "test-support")]
    {
        inputs.parity_count = None;
    }
    let outcome = qualify::run(&inputs).expect("the gate runs");
    assert_eq!(outcome.code, 0, "{}", outcome.stdout);
    let report = report(&outcome);
    assert_eq!(report["verdict"], "qualified");
    assert_eq!(report["qualification"], "complete-v1");
    assert_eq!(part(&report, "corpus")["result"], "passed");
    assert_eq!(part(&report, "lifecycle-matrix")["result"], "passed");
    assert_eq!(part(&report, "parity")["result"], "passed");
}

/// FR-567-AC-5's source-tree half (so this test carries no AC tag): the
/// report's FSL version is the root package's `CARGO_PKG_VERSION`, not xtask's,
/// and it is the version the root manifest declares. The criterion's
/// scratch-copy half is asserted by the filtered subprocess test below.
#[trace("TC-1396")]
#[test]
fn the_report_names_the_root_package_version() {
    let (_fspec, inputs) = inputs();
    let outcome = qualify::run(&inputs).expect("the gate runs");
    let version = report(&outcome)["fsl_version"].clone();
    assert_eq!(version, filament_spec_language::VERSION);
    assert_ne!(
        version,
        env!("CARGO_PKG_VERSION"),
        "xtask's own version is not the FSL version"
    );
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml"))
            .expect("the root manifest");
    assert!(
        manifest.contains(&format!(
            "version = \"{}\"",
            filament_spec_language::VERSION
        )),
        "the root manifest declares the reported version"
    );
}

/// FR-567-AC-5: a scratch source tree rebuilt with a different root package
/// version reports that build's version, not the version compiled into xtask.
#[trace("TC-1396", "FR-567-AC-5")]
#[test]
fn a_scratch_build_reports_its_changed_root_package_version() {
    const CHILD: &str = "QSL_2095_FR567_VERSION_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let (_fspec, inputs) = inputs();
        let outcome = qualify::run(&inputs).expect("the scratch gate runs");
        assert_eq!(outcome.code, 0, "{}", outcome.stdout);
        assert_eq!(report(&outcome)["fsl_version"], "9.9.9");
        return;
    }

    let scratch = TempDir::new().expect("scratch checkout");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace member");
    let mut archive = Command::new("git")
        .current_dir(repository)
        .args(["archive", "--format=tar", "HEAD"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("git creates a source archive");
    let unpack = Command::new("tar")
        .args(["-xf", "-", "-C"])
        .arg(scratch.path())
        .stdin(archive.stdout.take().expect("archive stdout"))
        .status()
        .expect("tar unpacks the source archive");
    let archived = archive.wait_with_output().expect("git archive completes");
    assert!(archived.status.success(), "git archive succeeds");
    assert!(unpack.success(), "source archive extraction succeeds");

    let initialized = Command::new("git")
        .current_dir(scratch.path())
        .args(["init", "--quiet"])
        .status()
        .expect("scratch tree initializes for patch application");
    assert!(initialized.success(), "scratch repository initializes");

    // Apply this checkout's uncommitted edits to tracked and staged files on top
    // of its committed tree, so the child exercises the code under test. A clean
    // checkout has an empty diff, which `git apply` rejects.
    let changes = Command::new("git")
        .current_dir(repository)
        .args(["diff", "--binary", "HEAD"])
        .output()
        .expect("git reads the working diff");
    assert!(changes.status.success(), "git diff succeeds");
    if !changes.stdout.is_empty() {
        let applied = Command::new("git")
            .current_dir(scratch.path())
            .arg("apply")
            .stdin(Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .expect("git apply stdin")
                    .write_all(&changes.stdout)?;
                child.wait()
            })
            .expect("working changes apply to the scratch tree");
        assert!(
            applied.success(),
            "the scratch tree includes working changes"
        );
    }

    let manifest_path = scratch.path().join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path).expect("root manifest");
    let (before_package, package_and_after) = manifest
        .split_once("[package]")
        .expect("root package section");
    let (package, after_package) = package_and_after
        .split_once("\n[features]")
        .expect("features follow the root package section");
    let changed_package = package.replace("version = \"0.3.0\"", "version = \"9.9.9\"");
    assert_ne!(
        changed_package, package,
        "the root version is replaced once"
    );
    std::fs::write(
        &manifest_path,
        format!("{before_package}[package]{changed_package}\n[features]{after_package}"),
    )
    .expect("scratch manifest is written");

    let lock_path = scratch.path().join("Cargo.lock");
    let lock = std::fs::read_to_string(&lock_path).expect("workspace lockfile");
    let changed_lock = lock.replace(
        "name = \"filament-spec-language\"\nversion = \"0.3.0\"",
        "name = \"filament-spec-language\"\nversion = \"9.9.9\"",
    );
    std::fs::write(&lock_path, changed_lock).expect("scratch lockfile is written");

    // The scratch build has this build's features, so its run is as small.
    let features: &[&str] = if cfg!(feature = "test-support") {
        &["--features", "test-support"]
    } else {
        &[]
    };
    let output = Command::new("cargo")
        .current_dir(scratch.path())
        .args(["test", "--locked", "-j", "8", "-p", "xtask"])
        .args(features)
        .args([
            "--test",
            "qualify_parts",
            "a_scratch_build_reports_its_changed_root_package_version",
            "--",
            "--exact",
        ])
        .env(CHILD, "1")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_TARGET_DIR", scratch.path().join("target"))
        .output()
        .expect("filtered scratch-version test runs");
    assert!(
        output.status.success(),
        "scratch-version test succeeds:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
