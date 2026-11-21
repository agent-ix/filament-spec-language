// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-283: aggregate names come from actual caller regions or checked components.

use filament_exact::{Cancel, FieldValue, Value, ValueType};
use filament_semantic_value::catalog::Category;
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::monitor::{
    monitor, MonitorClause, MonitorLimits, MonitorOutcome, MonitorRefusal, MonitorRequest,
    MonitorTrace, MonitorVerdict,
};
use fsl_foundation::diagnostic::StageFailure;
use fsl_foundation::source::provenance::SourceRegion;
use fsl_foundation::{Source, SourceIdentity};
use fsl_package::{ClockConfiguration, TemporalClockSelection};
use fsl_replay::spine::{
    self, ClauseDisposition, DependencyInput, ParseRequest, SpineLimits,
    StateClauseExecutionPoints, TemporalVerdict, TraceInput,
};
use fsl_semantics::check::FormulaNodeId;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::observation::ObservationLimits;
use ix_trace_rs::trace;
use serde_json::json;

use crate::counter::{document, Documents, UNLIMITED};

const PATH: &str = "aggregate-rendering.native";
const EVENT: &str = "filament.temporal.event-position.false-extension/v1";
const TIMESTAMPED: &str = "filament.temporal.timestamped-event.finite-window/v1";

struct Compiled {
    unit: spine::CheckedUnit,
    sources: Vec<Source>,
}

fn identity() -> SourceIdentity {
    SourceIdentity::new("agent-ix", "aggregate-rendering")
}

fn unit_text(profile: &str, model: &str, body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile t = \"{profile}\";\n\
         model Config = \"{model}\";\n\
         record Sample {{ v: Int[0, 9]; w: Int[0, 9]; }}\n\
         temporal Window using t over (e: Sample) clock \"events\" on origin \
         {{ always[0,0] ({body}) }}\n"
    )
}

fn compile(documents: &Documents, text: &str, clock: ClockConfiguration) -> Compiled {
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let source = identity();
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: PATH,
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("the actual source parses")
    .into_value();
    let models = spine::select(&parsed, &documents.packages, limits.model, &cancel)
        .expect("the model selects")
        .into_value();
    let cst = fsl_cst::parse(source, PATH, text.as_bytes(), fsl_cst::Limits::default())
        .expect("the source parses for its declaration span");
    let forms = fsl_forms::build_unit(&cst).expect("the source forms");
    let declaration = forms
        .forms()
        .iter()
        .find(|form| matches!(form.form(), fsl_forms::DeclarationForm::Temporal(_)))
        .expect("the actual temporal declaration");
    let selections = [TemporalClockSelection::new(
        cst.source().reference().clone(),
        declaration.span(),
        clock,
    )];
    let unit = spine::check_with_execution_points_and_source_inputs(
        &parsed,
        "fixture".to_owned(),
        &StateClauseExecutionPoints::new(),
        spine::SourceInputs {
            temporal_selections: &selections,
            check_mode: Some(CheckMode::Kernel),
            ..spine::SourceInputs::default()
        },
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .expect("the actual source checks")
    .into_value();
    let mut sources = vec![unit.source().clone()];
    sources.extend(unit.libraries().iter().cloned());
    Compiled { unit, sources }
}

fn event(documents: &Documents, body: &str) -> Compiled {
    compile(
        documents,
        &unit_text(EVENT, "test/counter", body),
        ClockConfiguration::EventPosition {
            sequence_authority: "aggregate-samples".into(),
        },
    )
}

/// Inspect producer facts separately from assertions about the rendered name.
fn aggregate_candidates(
    compiled: &Compiled,
    expected_uses: usize,
) -> (Vec<usize>, Vec<SourceRegion>, SourceRegion) {
    let graph = compiled.unit.package().graph();
    let node = graph
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == "Window")
        .expect("Window is checked");
    let atoms = node
        .clause()
        .aggregates()
        .keys()
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(atoms.len(), expected_uses, "actual formula uses");
    let key = node
        .formula_node_key(FormulaNodeId::from_arena_index(atoms[0]))
        .expect("the atom has an actual checked key");
    for atom in &atoms {
        assert_eq!(
            node.formula_node_key(FormulaNodeId::from_arena_index(*atom)),
            Some(key),
            "all uses resolve to the same checked aggregate"
        );
    }
    let claim_region = graph
        .occurrence(node.identity(), node.claim())
        .and_then(|location| graph.region(location))
        .expect("the actual Claim occurrence identifies its source");
    let text = compiled.sources[0].text();
    let start = text
        .find("temporal Window")
        .expect("literal declaration start");
    let end = text.rfind('}').expect("literal final declaration brace") + 1;
    let declaration = SourceRegion::new(
        claim_region.source().clone(),
        u64::try_from(start).expect("the literal start fits a source region"),
        u64::try_from(end).expect("the literal end fits a source region"),
    )
    .expect("the authored whole declaration is a valid region");
    let candidates = graph
        .occurrences()
        .filter(|(identity, origin, _)| identity == key && origin.role().as_str() == "expression")
        .filter_map(|(_, _, location)| graph.region(location))
        .filter(|region| {
            region.source() == declaration.source()
                && region.start() >= declaration.start()
                && region.end() <= declaration.end()
        })
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(candidates.len(), expected_uses, "actual caller regions");
    (atoms, candidates, declaration)
}

fn monitor_samples(
    documents: &Documents,
    compiled: &Compiled,
    sources: &[Source],
) -> Result<MonitorOutcome, StageFailure<MonitorRefusal>> {
    let package = compiled.unit.package();
    let sample_type = package
        .graph()
        .scope()
        .types()
        .composites()
        .find(|declaration| declaration.name() == "Sample")
        .expect("the checked record type")
        .key();
    let samples = [filament_exact::from_admitted_slots(
        sample_type.clone(),
        vec![
            FieldValue::Present(Value::Integer(0.into())),
            FieldValue::Present(Value::Integer(1.into())),
        ]
        .into_boxed_slice(),
    )];
    monitor(
        package,
        &MonitorRequest {
            clauses: vec![MonitorClause {
                name: "Window".to_owned(),
                over: None,
            }],
            trace: MonitorTrace::Samples {
                subject: ValueType::Composite(sample_type),
                samples: &samples,
            },
            snapshots: &documents.snapshots,
            packages: &documents.packages,
            sources,
            temporal_selections: &[],
        },
        &MonitorLimits {
            observation: ObservationLimits::default(),
            normalization: ModelNormalizationLimits::default(),
            accounting: UNLIMITED,
            max_window_values: 1 << 20,
            retained_positions: fsl_analyze::monitor::DEFAULT_RETAINED_POSITIONS,
            retained_identities: fsl_analyze::monitor::DEFAULT_RETAINED_IDENTITIES,
        },
        &Cancel::new(),
    )
    .map(|completed| completed.into_value())
}

fn assert_shortage(
    outcome: &MonitorOutcome,
    expected: &str,
    expected_region: &SourceRegion,
    count: u64,
    min_count: u64,
) {
    let [record] = outcome.records.as_slice() else {
        panic!("one actual monitor record");
    };
    assert_eq!(record.atom(), Some(expected));
    assert_eq!(record.atom_locus(), Some(expected_region));
    assert_eq!(record.verdict, MonitorVerdict::InsufficientData);
    assert_eq!(record.category, Category::Undefined);
    let ClauseDisposition::Temporal(TemporalVerdict::Undefined {
        at,
        cause:
            spine::TemporalUndefinedCause::InsufficientData {
                count: actual_count,
                min_count: actual_min_count,
                ..
            },
    }) = &record.report().disposition
    else {
        panic!("actual shortage, got {:?}", record.report().disposition);
    };
    assert_eq!(
        (*at, *actual_count, *actual_min_count),
        (0, count, min_count)
    );
}

#[trace("TC-1295", "FR-283-AC-19")]
#[test]
fn unique_and_identical_shared_aggregates_keep_exact_caller_text_and_shortage() {
    let documents = Documents::new(document(false));
    let atom = "count over past[0,9] min_count 2 >= 1";
    for (body, uses) in [(atom.to_owned(), 1), (format!("({atom}) and ({atom})"), 2)] {
        let compiled = event(&documents, &body);
        let (_, candidates, declaration) = aggregate_candidates(&compiled, uses);
        let expected_region = if uses == 1 {
            &candidates[0]
        } else {
            &declaration
        };
        let outcome = monitor_samples(&documents, &compiled, &compiled.sources)
            .expect("the actual sample monitor completes");
        assert_shortage(&outcome, atom, expected_region, 1, 2);
    }
}

#[trace("TC-1295", "FR-283-AC-19")]
#[test]
fn shared_aggregate_names_use_checked_components_without_changing_the_result() {
    let documents = Documents::new(document(false));
    for atom in [
        "count over past[0, 9] min_count 2 >= 1",
        "sum of e.v over past[0, 9] min_count 2 <= 15",
        "min of e.v over past[0, 9] min_count 2 >= 0",
        "max of e.v over past[0, 9] min_count 2 <= 9",
        "fraction holds(e.v > 1) over past[0, 9] min_count 2 >= rational(1, 2)",
        "quantile rational(1, 2) of e.v where holds(e.w = 1) over past[0, 9] min_count 2 <= 9",
    ] {
        let other = atom.replace(' ', "  ");
        for body in [
            format!("({atom}) and ({other})"),
            format!("({other}) and ({atom})"),
        ] {
            let compiled = event(&documents, &body);
            let (atoms, _, declaration) = aggregate_candidates(&compiled, 2);
            for index in atoms {
                let rendered = spine::render_clause_atom(
                    compiled.unit.package(),
                    &compiled.sources,
                    "Window",
                    index,
                )
                .expect("the actual checked aggregate is rendered");
                assert_eq!(rendered.name, atom);
                assert_eq!(rendered.region, declaration);
            }
            let outcome = monitor_samples(&documents, &compiled, &compiled.sources)
                .expect("the actual sample monitor completes");
            assert_shortage(&outcome, atom, &declaration, 1, 2);
        }
    }
}

#[trace("TC-1295", "FR-283-AC-19")]
#[test]
fn an_omitted_min_count_and_reformatted_filter_have_one_canonical_name() {
    let documents = Documents::new(document(false));
    let first = "count where holds(e.v > 1) over past[0,9] >= 1";
    let second = "count where holds( e.v>1 ) over past[ 0 , 9 ] min_count 1 >= 1";
    let expected = "count where holds(e.v > 1) over past[0, 9] min_count 1 >= 1";
    for body in [
        format!("({first}) and ({second})"),
        format!("({second}) and ({first})"),
    ] {
        let compiled = event(&documents, &body);
        let (_, _, declaration) = aggregate_candidates(&compiled, 2);
        let outcome = monitor_samples(&documents, &compiled, &compiled.sources)
            .expect("the actual sample monitor completes");
        assert_shortage(&outcome, expected, &declaration, 0, 1);
    }
}

fn config_units_document() -> Vec<u8> {
    let mut domain: serde_json::Value =
        serde_json::from_slice(&crate::units::Domain::config_version().document)
            .expect("the real ConfigVersion fixture is JSON");
    let origin = domain["types"][0]["origin"].clone();
    let time = "ix://example/config-version/dimension/Time";
    let second = "ix://example/config-version/unit/s";
    domain["dimensions"] = json!([{
        "identity": time, "name": "Time", "base": true, "terms": [], "origin": origin,
    }]);
    domain["units"] = json!([
        {"identity": second, "name": "s", "dimension": time, "scale": "1", "offset": "0", "origin": origin},
        {"identity": "ix://example/config-version/unit/ms", "name": "ms", "dimension": time,
         "target": second, "scale": "1/1000", "offset": "0", "origin": origin},
    ]);
    serde_json::to_vec(&domain).expect("the real package plus its declared units remains JSON")
}

const DURATION_LABEL: &str = "count where holds(e.v > 1) over past[rational(0, 1) ix://example/config-version/unit/ms, rational(3000, 1) ix://example/config-version/unit/ms] min_count 1 >= 1";

fn duration_case() -> (Documents, Compiled) {
    let documents = Documents::new(config_units_document());
    let first = "count where holds(e.v > 1) over past[0 Config::s, 3 Config::s] >= 1";
    let second =
        "count where holds( e.v>1 ) over past[ 0 Config::ms , 3000 Config::ms ] min_count 1 >= 1";
    let compiled = compile(
        &documents,
        &unit_text(
            TIMESTAMPED,
            "example/config-version",
            &format!("({first}) and ({second})"),
        ),
        ClockConfiguration::TimestampedEvent {
            timestamp_unit: "ix://example/config-version/unit/ms".into(),
        },
    );
    (documents, compiled)
}

#[trace("TC-1295", "FR-283-AC-19")]
#[test]
fn equivalent_config_duration_bounds_name_the_checked_normalized_window() {
    let (_, compiled) = duration_case();
    let (atoms, _, declaration) = aggregate_candidates(&compiled, 2);
    for index in atoms {
        let rendered =
            spine::render_clause_atom(compiled.unit.package(), &compiled.sources, "Window", index)
                .expect("the actual checked duration aggregate is rendered");
        assert_eq!(rendered.name, DURATION_LABEL);
        assert_eq!(rendered.region, declaration);
    }
}

#[trace("TC-1295", "FR-283-AC-19")]
#[test]
fn canonical_aggregate_names_never_replace_missing_or_invalid_caller_sources() {
    let documents = Documents::new(document(false));
    let compiled = event(
        &documents,
        "(count where holds(e.v > 1) over past[0,9] >= 1) and \
         (count where holds( e.v>1 ) over past[ 0 , 9 ] min_count 1 >= 1)",
    );
    let (_, candidates, declaration) = aggregate_candidates(&compiled, 2);
    let valid = compiled.sources[0].text().as_bytes();
    let mut invalid_sources = vec![Vec::new()];
    invalid_sources.push(vec![Source::read_typed(
        SourceIdentity::new("agent-ix", "different-reference"),
        PATH,
        valid,
        1 << 20,
    )
    .expect("the wrong-reference caller source is valid UTF-8")]);
    for region in candidates.into_iter().chain(std::iter::once(declaration)) {
        let start = usize::try_from(region.start()).expect("fixture start fits usize");
        let end = usize::try_from(region.end()).expect("fixture end fits usize");
        invalid_sources.push(vec![Source::read_typed(
            identity(),
            PATH,
            &valid[..end - 1],
            1 << 20,
        )
        .expect("the shortened caller source is valid UTF-8")]);
        let mut split = valid.to_vec();
        split[start - 1..=start].copy_from_slice("é".as_bytes());
        invalid_sources.push(vec![Source::read_typed(identity(), PATH, &split, 1 << 20)
            .expect("the replacement is valid UTF-8 but the region starts inside it")]);
        let mut split_end = valid.to_vec();
        split_end[end - 1..=end].copy_from_slice("é".as_bytes());
        invalid_sources.push(vec![Source::read_typed(
            identity(),
            PATH,
            &split_end,
            1 << 20,
        )
        .expect("the replacement is valid UTF-8 but the region ends inside it")]);
    }
    for sources in invalid_sources {
        let Err(StageFailure::Refused(MonitorRefusal::SourceNotSupplied { clause, source })) =
            monitor_samples(&documents, &compiled, &sources)
        else {
            panic!("canonical printing must not hide an unavailable caller region");
        };
        assert_eq!(clause, "Window");
        assert_eq!(source, identity());
    }
}

/// The compiled-source entry accepts timestamped record samples even though
/// the higher-level monitor's `Samples` adapter only takes event positions.
#[trace("TC-1295", "FR-283-AC-19")]
#[test]
fn timestamped_duration_aggregate_retains_its_actual_shortage_and_locus() {
    let (documents, compiled) = duration_case();
    let (atoms, _, declaration) = aggregate_candidates(&compiled, 2);
    let trace = TraceInput::Timestamped {
        observations: vec![spine::TimestampedObservation {
            document: spine::sample_position(
                &json!({"v": {"integer": "0"}, "w": {"integer": "1"}}).to_string(),
            ),
            stamp: 0,
            order_key: None,
        }],
        progress: spine::ProgressAssertion {
            watermark: 0,
            completeness: spine::ProgressCompleteness::Complete { through: 0 },
            scope: spine::ProgressClosure::Closed,
            execution: spine::ProgressClosure::Closed,
        },
        window: Box::new(spine::TimestampedWindow {
            binding: Some(spine::TraceBinding {
                profile: TIMESTAMPED.to_owned(),
                clock_name: "events".to_owned(),
                parameters: ClockConfiguration::TimestampedEvent {
                    timestamp_unit: "ix://example/config-version/unit/ms".into(),
                },
            }),
            authoritative_origin: true,
            cover: None,
            missing_valuations: Vec::new(),
            late: Default::default(),
        }),
    };
    let report = spine::run_package_temporal(&spine::PackageTemporalRequest {
        package: compiled.unit.package(),
        packages: &documents.packages,
        model_limits: ModelNormalizationLimits::default(),
        snapshots: &documents.snapshots,
        observation_limits: ObservationLimits::default(),
        accounting: UNLIMITED,
        name: "Window",
        trace: &trace,
        over: None,
        ledger: None,
        sources: &compiled.sources,
        authentication: None,
        cancel: &Cancel::new(),
    });
    let ClauseDisposition::Temporal(TemporalVerdict::Undefined {
        at,
        cause:
            spine::TemporalUndefinedCause::InsufficientData {
                atom,
                count,
                min_count,
            },
    }) = &report.disposition
    else {
        panic!(
            "the real timestamped aggregate's shortage, got {:?}",
            report.disposition
        );
    };
    assert_eq!((*at, *count, *min_count), (0, 0, 1));
    assert!(
        atoms.contains(atom),
        "the result names an actual checked aggregate"
    );
    assert_eq!(report.category(), Category::Undefined);
    assert_eq!(report.basis.as_str(), "unavailable");
    let window = report
        .finite_window
        .as_ref()
        .expect("the actual finite-window report");
    assert_eq!(window.basis.as_str(), "unavailable");
    assert_eq!(
        window
            .atom_regions
            .get(&FormulaNodeId::from_arena_index(*atom)),
        Some(&declaration),
        "the result retains the shared aggregate's selected declaration locus",
    );
    let rendered =
        spine::render_clause_atom(compiled.unit.package(), &compiled.sources, "Window", *atom)
            .expect("the result's actual aggregate is rendered");
    assert_eq!(rendered.name, DURATION_LABEL);
    assert_eq!(rendered.region, declaration);
}
