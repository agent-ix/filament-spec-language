// SPDX-License-Identifier: AGPL-3.0-or-later
//! Probe: the counts, outcomes and one-shot measurements the
//! criterion benchmarks do not produce -- which inputs a layer admits or
//! refuses, input sizes in the unit each layer counts, CST identity bytes
//! hashed, and wall time plus peak RSS for inputs too slow to sample
//! repeatedly.
//!
//! ```text
//! fsl-bench-probe parse                     # nesting 0..=64, volume sizes
//! fsl-bench-probe parse-volume <N>          # one parse of N functions, limits raised
//! fsl-bench-probe cst                       # hashed bytes per source byte
//! fsl-bench-probe check <chain|independent|self-recursive|text-cluster|deep-wide|deep-wide-chain> <N>   # one check, one process
//! fsl-bench-probe eval <N>                  # one f0 call on an N-chain
//! fsl-bench-probe model <types> <depth> <members> # record counts, outcomes
//! fsl-bench-probe mc <axis> <sizes...>      # one model-check, reach, key or sample run
//! fsl-bench-probe mc-marginal <axis> <small...> -- <large...>  # marginal bytes per state
//! fsl-bench-probe package <emit|read|replay/request> <1|10|100> # isolated timed-package probe
//! ```
//!
//! `check` measures one input per process because peak RSS is process-wide
//! and monotone (`fsl_bench::rss`); run it once per size.

use std::num::NonZeroUsize;
use std::process::{Command, ExitCode};
use std::time::Instant;

use fsl_bench::check::{self, call_head, completed_with_five, linked_chain};
use fsl_bench::mc::{self, AxisError, Work};
use fsl_bench::model::{self, ModelShape};
use fsl_bench::package;
use fsl_bench::parse::{self, ParseOutcome};
use fsl_bench::rss::peak_rss_kib;
use fsl_eval::simulation::model::ModelSystem;
use fsl_semantics::model::domain_package::DomainPackageRecord;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::population::AdmissionOutcome;
use ix_trace_rs::string_edge;

/// The parser volume sizes the probe reports (functions per source).
const VOLUME_SIZES: [usize; 6] = [100, 500, 1_000, 2_000, 3_000, 4_000];

fn usage() -> ExitCode {
    eprintln!(
        "usage: fsl-bench-probe parse | cst | check <chain|independent|self-recursive|text-cluster|deep-wide|deep-wide-chain> <N> | eval <N> | model <types> <depth> <members> | mc <axis> <sizes...> | mc-marginal <axis> <sizes...> -- <sizes...> | package <emit|read|replay/request> <1|10|100>"
    );
    ExitCode::from(2)
}

fn size(argument: Option<&String>) -> Option<usize> {
    argument?.replace('_', "").parse().ok()
}

fn rss() -> String {
    peak_rss_kib().map_or_else(|| "unavailable".to_owned(), |kib| kib.to_string())
}

fn describe(outcome: &ParseOutcome) -> String {
    match outcome {
        ParseOutcome::Admitted { tokens, nodes } => {
            format!("admitted tokens={tokens} nodes={nodes}")
        }
        ParseOutcome::Recovered { first_code } => {
            format!("recovered first_code={}", first_code.unwrap_or("none"))
        }
        ParseOutcome::Refused {
            code,
            cause,
            message,
        } => format!("refused code={code} cause={cause} message={message:?}"),
    }
}

fn probe_parse() {
    let mut max_admitted = None;
    for depth in 0..=64 {
        let text = parse::nested_source(depth);
        let outcome = ParseOutcome::of(&parse::parse(&text));
        println!(
            "parse.depth depth={depth} bytes={} {}",
            text.len(),
            describe(&outcome)
        );
        if outcome.is_admitted() {
            max_admitted = Some(depth);
        }
    }
    println!(
        "parse.depth.max_admitted {}",
        max_admitted.map_or_else(|| "none".to_owned(), |depth| depth.to_string())
    );
    for functions in VOLUME_SIZES {
        let text = parse::volume_source(functions);
        let outcome = ParseOutcome::of(&parse::parse(&text));
        println!(
            "parse.volume functions={functions} bytes={} {}",
            text.len(),
            describe(&outcome)
        );
    }
}

fn probe_cst() {
    let mut inputs: Vec<(String, String)> = VOLUME_SIZES
        .iter()
        .map(|&functions| {
            (
                format!("volume functions={functions}"),
                parse::volume_source(functions),
            )
        })
        .collect();
    inputs.extend(
        (0..=4).map(|depth| (format!("nested depth={depth}"), parse::nested_source(depth))),
    );
    for (label, text) in inputs {
        match parse::parse(&text) {
            Ok(parsed) if parsed.is_admissible() => {
                println!(
                    "cst.nodes {label} source_bytes={} nodes={}",
                    text.len(),
                    parsed.cst().nodes().len(),
                );
            }
            other => println!("cst.nodes {label} {}", describe(&ParseOutcome::of(&other))),
        }
    }
}

/// `#[string_edge]`: `shape` is a command-line word, matched here to pick
/// the generator; it selects no family semantics.
#[string_edge]
fn probe_check(shape: &str, functions: usize) -> ExitCode {
    let declarations = match shape {
        "chain" => check::call_chain(functions),
        "independent" => check::independent(functions),
        "self-recursive" => fsl_bench::recursion::self_recursive(functions),
        "text-cluster" => fsl_bench::text_cluster::text_cluster(functions),
        // `functions` is the field-name length here.
        "deep-wide" => fsl_bench::text_cluster::deep_wide(
            fsl_bench::text_cluster::DEEP_WIDE_CHAIN,
            fsl_bench::text_cluster::DEEP_WIDE_LEVELS,
            functions,
        ),
        // `functions` is the chain length here, with one-byte names.
        "deep-wide-chain" => fsl_bench::text_cluster::deep_wide(
            functions,
            fsl_bench::text_cluster::DEEP_WIDE_LEVELS,
            1,
        ),
        _ => return usage(),
    };
    let start = Instant::now();
    let result = check::check(declarations);
    let elapsed = start.elapsed();
    let verdict = match &result {
        Ok(_) => "checked".to_owned(),
        Err(refusals) => format!(
            "refused refusals={} first={:?}",
            refusals.len(),
            refusals.first().map(|refusal| &refusal.cause)
        ),
    };
    println!(
        "check shape={shape} functions={functions} wall_ms={:.1} peak_rss_kib={} {verdict}",
        elapsed.as_secs_f64() * 1e3,
        rss()
    );
    ExitCode::SUCCESS
}

fn probe_eval(functions: usize) -> ExitCode {
    let package = linked_chain(functions);
    let objects = ObjectEnvironment::default();
    let start = Instant::now();
    let evaluation = call_head(&package, &objects);
    let elapsed = start.elapsed();
    println!(
        "eval frames={functions} wall_us={:.1} completed_with_five={} peak_rss_kib={}",
        elapsed.as_secs_f64() * 1e6,
        completed_with_five(&evaluation),
        rss()
    );
    ExitCode::SUCCESS
}

/// One isolated package or replay request, with setup excluded from elapsed
/// time and process peak RSS retained for the size.
#[string_edge]
fn probe_package(axis: &str, count: usize) -> ExitCode {
    if axis == "replay/request" {
        return probe_isolated_replay(count);
    }
    probe_package_fixture(axis, count)
}

fn probe_package_fixture(axis: &str, count: usize) -> ExitCode {
    let fixture = match package::fixture(count) {
        Ok(fixture) => fixture,
        Err(error) => {
            eprintln!("package fixture refused: {error}");
            return ExitCode::FAILURE;
        }
    };
    let read_identity =
        fsl_semantics::library::LibraryName::new("package").expect("benchmark library identity");
    let read_limits = fsl_package::V2ReadLimits::bounded(1 << 24);
    let witness_supplier = fsl_package::SourceWitnessSupplier::new(&fixture.emission);
    let started = Instant::now();
    let parse_calls: Option<u64> = None;
    let succeeded = match axis {
        "emit" => fsl_package::emit_package(
            fixture.checked.package(),
            |_| Some(fixture.region.clone()),
            fsl_package::EmitLimits::new(1 << 32),
        )
        .is_ok(),
        "read" => fsl_package::read_evaluation_emission(
            fixture.checked.package(),
            &fixture.emission,
            read_identity,
            &fixture.packages,
            read_limits,
            fsl_package::EmitLimits::new(1 << 32),
            witness_supplier,
        )
        .is_ok(),
        _ => {
            eprintln!("unknown package axis: {axis}");
            return ExitCode::from(2);
        }
    };
    let elapsed_ms = started.elapsed().as_secs_f64() * 1e3;
    let peak_rss = rss();
    let parse_calls =
        parse_calls.map_or_else(|| "unavailable".to_owned(), |calls| calls.to_string());
    println!(
        "axis={axis} M={count} wall_ms={elapsed_ms:.6} peak_rss_kib={peak_rss} parse_calls_per_source_document={parse_calls} source_documents=1 succeeded={succeeded}"
    );
    if succeeded {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn probe_isolated_replay(count: usize) -> ExitCode {
    let artifact_dir = std::env::temp_dir().join(format!(
        "fsl-bench-replay-probe-{}-{count}",
        std::process::id()
    ));
    if let Err(error) = std::fs::create_dir_all(&artifact_dir) {
        eprintln!("create replay probe artifact directory: {error}");
        return ExitCode::FAILURE;
    }
    let artifact_path = artifact_dir.join("request.json");
    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(error) => {
            eprintln!("locate replay probe executable: {error}");
            let _ = std::fs::remove_dir_all(&artifact_dir);
            return ExitCode::FAILURE;
        }
    };
    let preparation = Command::new(&executable)
        .arg("package-fixture-worker")
        .arg(count.to_string())
        .arg(&artifact_path)
        .output();
    match preparation {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            eprintln!(
                "replay fixture preparation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let _ = std::fs::remove_dir_all(&artifact_dir);
            return ExitCode::FAILURE;
        }
        Err(error) => {
            eprintln!("start replay fixture preparation: {error}");
            let _ = std::fs::remove_dir_all(&artifact_dir);
            return ExitCode::FAILURE;
        }
    }
    let replay = Command::new(&executable)
        .arg("package-replay-worker")
        .arg(count.to_string())
        .arg(&artifact_path)
        .output();
    let replay = match replay {
        Ok(output) => output,
        Err(error) => {
            eprintln!("start isolated replay worker: {error}");
            let _ = std::fs::remove_dir_all(&artifact_dir);
            return ExitCode::FAILURE;
        }
    };
    let report = String::from_utf8_lossy(&replay.stdout);
    print!("{report}");
    if !replay.status.success() {
        eprint!("{}", String::from_utf8_lossy(&replay.stderr));
    }
    let row = report
        .lines()
        .find(|line| line.starts_with("axis=replay/request "));
    let write_result = row.map_or_else(
        || Err("isolated replay worker did not emit a report row".to_owned()),
        |row| write_replay_probe_report(count, row),
    );
    let _ = std::fs::remove_dir_all(&artifact_dir);
    if let Err(error) = write_result {
        eprintln!("write replay probe report: {error}");
        return ExitCode::FAILURE;
    }
    if replay.status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn prepare_replay_probe_artifact(count: usize, path: &std::path::Path) -> ExitCode {
    let fixture = match package::fixture(count) {
        Ok(fixture) => fixture,
        Err(error) => {
            eprintln!("package fixture refused: {error}");
            return ExitCode::FAILURE;
        }
    };
    let unit = package::source(count);
    let request = match package::replay_request(&fixture, &unit) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("replay request refused: {error}");
            return ExitCode::FAILURE;
        }
    };
    match package::write_replay_probe_artifact(path, &request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("prepare replay probe artifact: {error}");
            ExitCode::FAILURE
        }
    }
}

fn replay_probe_worker(count: usize, path: &std::path::Path) -> ExitCode {
    let request = match package::read_replay_probe_artifact(path) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("replay probe request reconstruction failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let limits = fsl_replay::ReplayLimits::default().with_input_bytes(1 << 30);
    let started = Instant::now();
    #[cfg(feature = "counters")]
    let (result, parse_calls) = fsl_bench::counters::replay_with_parse_calls(request, limits);
    #[cfg(not(feature = "counters"))]
    let (result, parse_calls) = (fsl_replay::replay(request, limits), 0_u64);
    let elapsed_ms = started.elapsed().as_secs_f64() * 1e3;
    let peak_rss = rss();
    let succeeded = result.is_ok();
    println!(
        "axis=replay/request M={count} wall_ms={elapsed_ms:.6} peak_rss_kib={peak_rss} parse_calls_per_source_document={parse_calls} source_documents=1 succeeded={succeeded}"
    );
    if peak_rss == "unavailable" {
        eprintln!("replay/request requires a numeric peak RSS measurement");
        return ExitCode::FAILURE;
    }
    if succeeded {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn write_replay_probe_report(count: usize, row: &str) -> Result<(), String> {
    let path = if let Some(path) = std::env::var_os("QSL_PACKAGE_PROBE_REPORT") {
        std::path::PathBuf::from(path)
    } else {
        std::env::var_os("CARGO_TARGET_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("target"))
            .join("package-probe-report")
            .join(format!("replay-{count}"))
    };
    let parent = path
        .parent()
        .ok_or_else(|| "replay probe report path has no parent directory".to_owned())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("create replay probe report directory: {error}"))?;
    std::fs::write(&path, row.as_bytes())
        .map_err(|error| format!("write {}: {error}", path.display()))
}

fn probe_model(shape: ModelShape, members: usize) -> ExitCode {
    let document = model::document(shape);
    let bytes = document.len();
    let offer = model::offer(document);
    let domain_package = match model::intake(&offer) {
        Ok(domain_package) => std::sync::Arc::new(domain_package),
        Err(failure) => {
            println!("model intake refused: {failure:?}");
            return ExitCode::FAILURE;
        }
    };
    let count = |pick: fn(&DomainPackageRecord) -> bool| {
        domain_package
            .records
            .iter()
            .filter(|record| pick(record))
            .count()
    };
    println!(
        "model.intake types={} depth={} document_bytes={bytes} records={} object_types={} fields={} populations={}",
        shape.types,
        shape.depth,
        domain_package.records.len(),
        count(|record| matches!(record, DomainPackageRecord::ObjectType(_))),
        count(|record| matches!(record, DomainPackageRecord::FieldMember(_))),
        count(|record| matches!(record, DomainPackageRecord::Population(_))),
    );
    let view = model::view(&domain_package);
    println!(
        "model.normalize declarations={} type_identities={}",
        view.declarations().len(),
        view.type_identities().len()
    );
    let universe = view.object_universe_of(&model::key(&model::chain_type(0)));
    println!("model.universe ok={}", universe.is_some());
    let population = model::population_document(shape, members);
    match model::admit_population(&view, &population) {
        AdmissionOutcome::Admitted(binding) => {
            println!("model.admit members={}", binding.members().len());
        }
        other => println!("model.admit not admitted: {other:?}"),
    }
    ExitCode::SUCCESS
}

/// One parse of `functions` declarations with every S1 limit raised to fit
/// it: wall time and outcome.
fn probe_parse_volume(functions: usize) -> ExitCode {
    let text = parse::volume_source(functions);
    let limits = fsl_cst::Limits::default()
        .with_source_bytes(usize::MAX)
        .with_tokens(usize::MAX)
        .with_nodes(usize::MAX);
    let started = Instant::now();
    let parsed = parse::parse_with(&text, limits);
    let s1 = started.elapsed().as_millis();
    let outcome = ParseOutcome::of(&parsed);
    let started = Instant::now();
    let built = parsed.is_ok_and(|parsed| fsl_forms::build_unit(&parsed).is_ok());
    println!(
        "functions={functions} bytes={} {} s1_ms={s1} s2_built={built} s2_ms={}",
        text.len(),
        describe(&outcome),
        started.elapsed().as_millis()
    );
    ExitCode::SUCCESS
}

/// Build the world of `axis` at `sizes` and run it once; the run alone is
/// timed. The `Work` and the seconds the run took.
/// `#[string_edge]`: this CLI boundary selects a benchmark axis from its word.
#[string_edge]
fn run_axis(axis: &str, sizes: &[usize]) -> Result<(Work, f64), AxisError> {
    let at = |index: usize| {
        sizes
            .get(index)
            .copied()
            .ok_or_else(|| AxisError(format!("{axis}: missing size {index}")))
    };
    let timed = |run: &mut dyn FnMut() -> Result<Work, AxisError>| {
        let started = Instant::now();
        run().map(|work| (work, started.elapsed().as_secs_f64()))
    };
    let workers = |limit: usize| NonZeroUsize::new(limit).unwrap_or(NonZeroUsize::MIN);
    match axis {
        "state_graph" | "state_graph_symmetric" => {
            let symmetric = axis == "state_graph_symmetric";
            let world = mc::ring_state_graph(at(0)?, at(1)?, symmetric)?;
            timed(&mut || mc::run_state_graph(&world, symmetric))
        }
        "safety" | "deadlock" => {
            let world = mc::ring_safety(at(0)?, at(1)?)?;
            timed(&mut || mc::run_safety(&world, axis == "deadlock").map(|(work, _)| work))
        }
        "liveness" => {
            let world = mc::ring_liveness(at(0)?, at(1)?)?;
            timed(&mut || mc::run_liveness(&world).map(|(work, _)| work))
        }
        "frame_candidates" => {
            let world = mc::wide(at(0)?, at(1)?)?;
            timed(&mut || mc::run_frame_candidates(&world))
        }
        "observation" => {
            let world = mc::observation(at(0)?)?;
            timed(&mut || mc::run_observation(&world))
        }
        "reach_1" | "reach_2" | "reach_4" | "reach_all" => {
            let world = mc::protocol(at(0)?)?;
            let limit = match axis {
                "reach_1" => workers(1),
                "reach_2" => workers(2),
                "reach_4" => workers(4),
                _ => std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN),
            };
            timed(&mut || mc::run_reach(&world, limit))
        }
        "encode" => {
            let world = mc::keys(at(0)?, at(1)?)?;
            let system =
                ModelSystem::new(world.simulation()).map_err(|e| AxisError(format!("{e:?}")))?;
            timed(&mut || mc::run_encode(&system))
        }
        "sample" => {
            let epsilon = u32::try_from(at(0)?).map_err(|e| AxisError(e.to_string()))?;
            let world = mc::service(epsilon)?;
            timed(&mut || mc::run_sample(&world))
        }
        other => Err(AxisError(format!("unknown axis {other}"))),
    }
}

/// The sizes after the axis name, up to a `--`, and those after it.
/// `#[string_edge]`: this CLI boundary parses size words and the separator.
#[string_edge]
fn size_lists(arguments: &[String]) -> Result<(Vec<usize>, Vec<usize>), String> {
    let mut halves = arguments.splitn(2, |word| word == "--");
    let parse = |words: &[String]| -> Result<Vec<usize>, String> {
        words
            .iter()
            .map(|word| size(Some(word)).ok_or_else(|| format!("not a size: {word:?}")))
            .collect()
    };
    let first = halves.next().map_or(Ok(Vec::new()), parse)?;
    let second = halves.next().map_or(Ok(Vec::new()), parse)?;
    Ok((first, second))
}

/// `mc <axis> <sizes...>`: one run, one process: states/s, edges/s or
/// samples/s, and peak RSS.
fn probe_axis(axis: &str, sizes: &[usize]) -> ExitCode {
    match run_axis(axis, sizes) {
        Ok((work, seconds)) => {
            println!(
                "axis={axis} sizes={sizes:?} {} secs={seconds:.6} peak_rss_kib={}",
                rates(&work, seconds),
                rss()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// `mc-marginal <axis> <small sizes...> -- <large sizes...>`: the small run
/// then the large one in one process, so the peak is monotone, and the
/// bytes the large run added per state it added.
fn probe_marginal(axis: &str, arguments: &[String]) -> ExitCode {
    let (small, large) = match size_lists(arguments) {
        Ok(lists) => lists,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let (Ok((first, _)), Some(before)) = (run_axis(axis, &small), peak_rss_kib()) else {
        eprintln!("marginal: the small run or its peak RSS is unavailable");
        return ExitCode::FAILURE;
    };
    let (Ok((second, _)), Some(after)) = (run_axis(axis, &large), peak_rss_kib()) else {
        eprintln!("marginal: the large run or its peak RSS is unavailable");
        return ExitCode::FAILURE;
    };
    if second.states <= first.states {
        eprintln!(
            "marginal: the large run reaches {} states, not more than the small run's {}",
            second.states, first.states
        );
        return ExitCode::FAILURE;
    }
    let added = second.states.saturating_sub(first.states);
    let bytes = after.saturating_sub(before).saturating_mul(1024);
    let per_state = bytes
        .checked_div(added)
        .map_or_else(|| "undefined".to_owned(), |per| per.to_string());
    println!("axis={axis} small={small:?} large={large:?} states_added={added} rss_added_bytes={bytes} marginal_bytes_per_state={per_state}");
    ExitCode::SUCCESS
}

fn rates(work: &Work, seconds: f64) -> String {
    let per = |count: u64| {
        if seconds > 0.0 {
            count as f64 / seconds
        } else {
            0.0
        }
    };
    format!(
        "states={} edges={} samples={} key_bytes={} states_per_s={:.1} edges_per_s={:.1} samples_per_s={:.1}",
        work.states, work.edges, work.samples, work.key_bytes, per(work.states), per(work.edges), per(work.samples)
    )
}

/// `#[string_edge]`: dispatches the probe's command-line subcommand words.
#[string_edge]
fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("parse") => {
            probe_parse();
            ExitCode::SUCCESS
        }
        Some("parse-volume") => match size(arguments.get(1)) {
            Some(functions) => probe_parse_volume(functions),
            None => usage(),
        },
        Some("cst") => {
            probe_cst();
            ExitCode::SUCCESS
        }
        Some("check") => match (arguments.get(1), size(arguments.get(2))) {
            (Some(shape), Some(functions)) => probe_check(shape, functions),
            _ => usage(),
        },
        Some("eval") => match size(arguments.get(1)) {
            Some(functions) => probe_eval(functions),
            None => usage(),
        },
        Some("package") => match (arguments.get(1), size(arguments.get(2))) {
            (Some(axis), Some(count)) if package::MODEL_COUNTS.contains(&count) => {
                probe_package(axis, count)
            }
            _ => usage(),
        },
        Some("package-fixture-worker") => match (size(arguments.get(1)), arguments.get(2)) {
            (Some(count), Some(path)) if package::MODEL_COUNTS.contains(&count) => {
                prepare_replay_probe_artifact(count, std::path::Path::new(path))
            }
            _ => usage(),
        },
        Some("package-replay-worker") => match (size(arguments.get(1)), arguments.get(2)) {
            (Some(count), Some(path)) if package::MODEL_COUNTS.contains(&count) => {
                replay_probe_worker(count, std::path::Path::new(path))
            }
            _ => usage(),
        },
        Some("model") => match (
            size(arguments.get(1)),
            size(arguments.get(2)),
            size(arguments.get(3)),
        ) {
            (Some(types), Some(depth), Some(members)) => {
                probe_model(ModelShape { types, depth }, members)
            }
            _ => usage(),
        },
        Some("mc") => match arguments.get(1) {
            Some(axis) => match size_lists(&arguments[2..]) {
                Ok((sizes, rest)) if rest.is_empty() => probe_axis(axis, &sizes),
                Ok(_) => usage(),
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            },
            None => usage(),
        },
        Some("mc-marginal") => match arguments.get(1) {
            Some(axis) => probe_marginal(axis, &arguments[2..]),
            None => usage(),
        },
        _ => usage(),
    }
}
