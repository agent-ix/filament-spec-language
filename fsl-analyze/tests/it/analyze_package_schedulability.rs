// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-850 "Equality with the source route", FR-940 (QSL-2785): a
//! `schedulability` item of a `native-analyze/1` request that selects a v2
//! checked package settles over the package's own task-set rows exactly as
//! the item of the request over the source settles, with the source deleted,
//! and never settles `unsupported` for a fact the source route reads.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, label, package_selection, request_over, Workspace,
};

/// ADR-026 section 12's task sets: `Full` fits an EDF test exactly at
/// utilization 1, `Over` is utilized at 11/10, `Ctl` is a fixed-priority set.
const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile t = \"filament.temporal.complete/v1\";\n\
    dimension Time;\nunit ms : Time = rational(1, 1);\n\
    taskset Full {\n\
    task a periodic 10 ms deadline 10 ms wcet 5 ms priority 1 lo;\n\
    task b periodic 20 ms deadline 20 ms wcet 10 ms priority 2 lo;\n}\n\
    taskset Over {\n\
    task a periodic 10 ms deadline 10 ms wcet 6 ms priority 1 lo;\n\
    task b periodic 20 ms deadline 20 ms wcet 10 ms priority 2 lo;\n}\n\
    taskset Ctl {\n\
    task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n\
    task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n\
    task c periodic 12 ms deadline 12 ms wcet 5 ms priority 3 lo;\n}\n\
    taskset Mc {\n\
    task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n\
    task b periodic 6 ms deadline 6 ms wcet 1 ms priority 2 hi wcet_hi 3 ms;\n}\n\
    schedulable FullEdf on Full under edf;\n\
    schedulable CtlFp on Ctl under fixed_priority;\n\
    schedulable OverEdf on Over under edf;\n\
    schedulable McAmc on Mc under amc;\n\
    response AResponse5 on Full.a <= 5 ms under fixed_priority;\n\
    response AResponse4 on Full.a <= 4 ms under fixed_priority;\n";

/// The source route's document and exit, and the package route's over the
/// package compiled from the same request with the source deleted.
fn both(name: &str, items: &Value) -> (Value, i32, Value, i32) {
    let ws = Workspace::new(name);
    let members = json!({"models": [], "items": items});
    let path = request_over(
        &ws,
        SOURCE,
        &crate::counter::document(false),
        &[],
        members,
    );
    let source = analyze(&path);
    let request: Value = serde_json::from_slice(&std::fs::read(&path).expect("the request"))
        .expect("request JSON");
    let (selected, _) = package_selection(&ws, &request["request"]);
    std::fs::remove_file(ws.path("program.fsl")).expect("remove the source");
    let document = json!({"format": "native-analyze/1", "request": selected});
    let package = analyze(&ws.write("analyze-package.json", document.to_string().as_bytes()));
    (
        source.document(),
        source.exit,
        package.document(),
        package.exit,
    )
}

/// The package route's whole document and exit equal the source route's.
fn assert_parity(name: &str, claim: &str) -> Value {
    let (source, source_exit, package, package_exit) = both(name, &json!([{"claim": claim}]));
    assert_eq!(package, source, "{package}");
    assert_eq!(package_exit, source_exit, "{package}");
    package["items"][0].clone()
}

/// FR-247-AC-1: an EDF task set that fits proves with a certified closed form.
#[trace("TC-1593", "FR-940")]
#[test]
fn a_package_edf_task_set_that_fits_equals_the_source() {
    let item = assert_parity("package-sched-edf-fits", "FullEdf");
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["record"]["basis"]["analysis"], "edf", "{item}");
    assert_eq!(item["record"]["certification"], "certified", "{item}");
}

/// FR-247-AC-2: an over-utilized EDF task set refutes with the exact
/// utilization 11/10.
#[trace("TC-1593", "FR-940")]
#[test]
fn a_package_edf_overload_refutes_as_the_source_does() {
    let item = assert_parity("package-sched-edf-over", "OverEdf");
    assert_eq!(label(&item), "refuted", "{item}");
    let miss = &item["counterexample"]["counterexample"];
    assert_eq!(miss["type"], "schedulability-miss", "{miss}");
    assert_eq!(miss["miss"]["utilization"]["numerator"], "11", "{miss}");
    assert_eq!(miss["miss"]["utilization"]["denominator"], "10", "{miss}");
}

/// FR-633-AC-2: a fixed-priority response that meets its bound proves.
#[trace("TC-1593", "FR-940")]
#[test]
fn a_package_fixed_priority_response_that_meets_its_bound_equals_the_source() {
    let item = assert_parity("package-sched-fp-meets", "AResponse5");
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["record"]["basis"]["analysis"], "fixed-priority", "{item}");
}

/// FR-633-AC-2: a response of 5 against a bound of 4 refutes with the task,
/// job, iterates, response and bound of the source route's miss.
#[trace("TC-1593", "FR-940")]
#[test]
fn a_package_fixed_priority_response_miss_refutes_as_the_source_does() {
    let item = assert_parity("package-sched-fp-miss", "AResponse4");
    assert_eq!(label(&item), "refuted", "{item}");
    let miss = &item["counterexample"]["counterexample"]["miss"];
    assert_eq!(miss["task"], "a", "{miss}");
    assert_eq!(miss["response"]["numerator"], "5", "{miss}");
    assert_eq!(miss["bound"]["numerator"], "4", "{miss}");
    assert!(miss["job"].is_string() && miss["iterates"].is_array(), "{miss}");
}

/// FR-633-AC-1: a fixed-priority schedulable claim and an AMC claim settle
/// beside the others, in the request's order.
#[trace("TC-1593", "FR-940")]
#[test]
fn package_schedulability_items_keep_the_requests_order() {
    let items = json!([{"claim": "OverEdf"}, {"claim": "CtlFp"}, {"claim": "McAmc"},
        {"claim": "AResponse4"}]);
    let (source, source_exit, package, package_exit) = both("package-sched-mixed", &items);
    assert_eq!(package, source, "{package}");
    assert_eq!(package_exit, source_exit);
}

/// FR-246-AC-4: a named backend outside the claim's requirement candidates
/// stays excluded on the package route, as it is on the source route.
#[trace("TC-2961", "FR-246-AC-4")]
#[trace("TC-3009", "FR-246-AC-5", "FR-075-AC-23")]
#[test]
fn a_package_schedulability_claim_rejects_a_backend_outside_its_requirements() {
    let items = json!([{"claim": "FullEdf", "backend": "en-6"}]);
    let (source, source_exit, package, package_exit) =
        both("package-sched-requirement-candidates", &items);

    assert_eq!(package, source, "{package}");
    assert_eq!(package_exit, source_exit);
    let item = &package["items"][0];
    assert_eq!(label(item), "unsupported", "{item}");
    assert_eq!(item["request"]["candidates"]["backends"], json!([]), "{item}");
}

/// FR-246-AC-5: only a schedulable claim in a released task set is
/// release-backed in the package rows; a response claim remains unbacked.
#[trace("TC-3009", "FR-246-AC-5")]
#[test]
fn package_schedulability_rows_preserve_release_backing_by_claim_kind() {
    let declarations = "taskset Pair {\n\
        task a periodic 4 ms deadline 3 ms wcet 1 ms priority 1 lo;\n\
        }\n\
        schedulable PairFp on Pair under fixed_priority;\n\
        response PairAResponse on Pair.a <= 3 ms under fixed_priority;\n\
        release Pair.a on M::Call::fire_a;\n";
    let model = crate::zone_search::rpc_document_with(&[("fire_a", &["phase"])]);
    let unit = crate::analyze_family_cli::rpc_unit(4, declarations);
    let ws = Workspace::new("package-sched-release-backed-marker");
    let path = request_over(
        &ws,
        &unit,
        &model,
        &[crate::analyze_family_cli::rpc_snapshot()],
        json!({"items": [{"claim": "PairFp"}, {"claim": "PairAResponse"}]}),
    );
    let source = analyze(&path);
    let request: Value = serde_json::from_slice(&std::fs::read(&path).expect("request"))
        .expect("request JSON");
    let (selected, bytes) = package_selection(&ws, &request["request"]);
    let package: Value = serde_json::from_slice(&bytes).expect("checked package JSON");
    let rows = package["schedulability_claims"]
        .as_array()
        .expect("schedulability claim rows");
    let names = package["claim_names"].as_array().expect("claim names");
    let mut markers = rows
        .iter()
        .map(|row| {
            let name = names
                .iter()
                .find(|name| {
                    name["declaration"] == row["declaration"]
                        && name["occurrence"]["ordinal"] == row["occurrence"]
                })
                .expect("the package names the schedulability claim");
            (
                name["name"].as_str().expect("claim name"),
                row["release_backed"].as_bool().expect("release-backed flag"),
            )
        })
        .collect::<Vec<_>>();
    markers.sort_unstable_by_key(|(name, _)| *name);
    assert_eq!(markers, [("PairAResponse", false), ("PairFp", true)]);

    std::fs::remove_file(ws.path("program.fsl")).expect("remove source before package route");
    let document = json!({"format": "native-analyze/1", "request": selected});
    let packaged = analyze(&ws.write("analyze-package.json", document.to_string().as_bytes()));
    assert_eq!(packaged.document(), source.document(), "{}", packaged.document());
    assert_eq!(packaged.exit, source.exit);
    for item in packaged.document()["items"].as_array().expect("items") {
        assert_eq!(item["request"]["candidates"]["backends"], json!(["en-7"]));
    }
}

/// FR-850-AC-3 (negative): package bytes missing the claim rows or nodes
/// refuse the request `invalid_package`, with no internal failure.
#[trace("TC-1593", "FR-850-AC-3")]
#[test]
fn malformed_package_rows_refuse_a_schedulability_item_typed() {
    let ws = Workspace::new("package-sched-malformed");
    let members = json!({"models": [], "items": [{"claim": "FullEdf"}]});
    let path = request_over(&ws, SOURCE, &crate::counter::document(false), &[], members);
    let request: Value = serde_json::from_slice(&std::fs::read(&path).expect("the request"))
        .expect("request JSON");
    let (selected, package) = package_selection(&ws, &request["request"]);
    let good: Value = serde_json::from_slice(&package).expect("package JSON");
    let mut no_rows = good.clone();
    no_rows["schedulability_claims"] = json!([{"declaration": "x"}]);
    let mut no_nodes = good;
    no_nodes["nodes"] = json!([]);
    for (name, bytes) in [
        ("empty", b"{}".to_vec()),
        ("bad-claim-row", no_rows.to_string().into_bytes()),
        ("no-nodes", no_nodes.to_string().into_bytes()),
    ] {
        ws.write("handoff/checked-package.json", &bytes);
        let document = json!({"format": "native-analyze/1", "request": selected});
        let run = analyze(&ws.write("analyze-bad.json", document.to_string().as_bytes()));
        let document = run.document();
        assert_eq!(
            document["diagnostics"][0]["code"], "invalid_package",
            "{name}: {document}"
        );
        assert_eq!(document["items"], json!([]), "{name}");
        assert_eq!(run.exit, 20, "{name}");
    }
}

/// The package orders a task set's rows by task name, not by declaration:
/// a set declared out of name order, with jitter and blocking, settles with
/// the source route's whole document for every closed-form analysis.
#[trace("TC-1593", "FR-940")]
#[test]
fn a_package_task_set_declared_out_of_name_order_equals_the_source() {
    let source = "language \"ix:native\" edition \"1-draft\";\n\
        profile t = \"filament.temporal.complete/v1\";\n\
        dimension Time;\nunit ms : Time = rational(1, 1);\n\
        taskset Rev {\n\
        task z periodic 12 ms deadline 12 ms wcet 5 ms priority 1 jitter 1 ms blocking 1 ms lo;\n\
        task m sporadic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n\
        task a periodic 4 ms deadline 4 ms wcet 1 ms priority 3 lo;\n}\n\
        taskset Plain {\n\
        task z periodic 12 ms deadline 12 ms wcet 5 ms priority 1 lo;\n\
        task m periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n\
        task a periodic 4 ms deadline 4 ms wcet 1 ms priority 3 lo;\n}\n\
        schedulable RevFp on Rev under fixed_priority;\n\
        response RevZ on Rev.z <= 3 ms under fixed_priority;\n\
        schedulable PlainEdf on Plain under edf;\n";
    let ws = Workspace::new("package-sched-order");
    let members = json!({"models": [], "items": [{"claim": "RevFp"}, {"claim": "RevZ"},
        {"claim": "PlainEdf"}]});
    let path = request_over(&ws, source, &crate::counter::document(false), &[], members);
    let sourced = analyze(&path);
    let request: Value = serde_json::from_slice(&std::fs::read(&path).expect("the request"))
        .expect("request JSON");
    let (selected, _) = package_selection(&ws, &request["request"]);
    std::fs::remove_file(ws.path("program.fsl")).expect("remove the source");
    let document = json!({"format": "native-analyze/1", "request": selected});
    let package = analyze(&ws.write("analyze-package.json", document.to_string().as_bytes()));
    assert_eq!(package.document(), sourced.document(), "{}", package.document());
    assert_eq!(package.exit, sourced.exit);
}

/// FR-255-AC-24 (TC-721), FR-247: `schedule.max_demand_points` passed by the
/// caller bounds the QPA walk of a schedulability item on the source route
/// and the package route alike, and the item settles `incomplete` naming the
/// limit, the caller's value and the setting.
#[trace("TC-721", "FR-255-AC-24")]
#[test]
fn caller_schedule_max_demand_points_bounds_both_routes() {
    use fsl_foundation::Setting;

    let period = 1_048_580_u64;
    let long_period = period * period + 1;
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"filament.temporal.complete/v1\";\n\
         dimension Time;\nunit ms : Time = rational(1, 1);\n\
         taskset Many {{\n\
         task a periodic {period} ms deadline {period} ms wcet {} ms priority 1 lo;\n\
         task b periodic {long_period} ms deadline {long_period} ms wcet {period} ms priority 2 lo;\n\
         }}\n\
         schedulable ManyEdf on Many under edf;\n",
        period - 1
    );
    let ws = Workspace::new("caller-schedule-demand-points");
    let path = request_over(
        &ws,
        &unit,
        &crate::counter::document(false),
        &[],
        json!({"models": [], "items": [{"claim": "ManyEdf"}]}),
    );
    let request: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("the request")).expect("request JSON");
    let (selected, _) = package_selection(&ws, &request["request"]);
    let package = ws.write(
        "analyze-package.json",
        json!({"format": "native-analyze/1", "request": selected})
            .to_string()
            .as_bytes(),
    );
    let mut runs = vec![(
        "source",
        crate::analyze_cli_support::analyze_under(
            &path,
            &[(Setting::ScheduleMaxDemandPoints, 3)],
        ),
    )];
    std::fs::remove_file(ws.path("program.fsl")).expect("remove the source");
    runs.push((
        "package",
        crate::analyze_cli_support::analyze_under(
            &package,
            &[(Setting::ScheduleMaxDemandPoints, 3)],
        ),
    ));
    for (route, run) in runs {
        let item = run.item(0);
        assert_eq!(run.exit, 22, "{route}: {}", run.document());
        assert_eq!(label(&item), "incomplete", "{route}: {item}");
        let cause = &item["record"]["cause"];
        assert_eq!(cause["limit"], "max_demand_points", "{route}: {item}");
        assert_eq!(cause["value"], 3, "{route}: {item}");
        assert_eq!(cause["setting"], "schedule.max_demand_points", "{route}: {item}");
    }
    for operand in ["schedule.max_demand_points=0", "schedule.max_demand_points=1.5"] {
        let refusal = fsl_foundation::setting::parse_operands([operand]).unwrap_err();
        assert_eq!(refusal.cause, fsl_foundation::UsageCause::NotAnInteger, "{operand}");
    }
    let mut replay = fsl_command::command::CommandLimits::replay();
    assert!(replay
        .set(
            fsl_foundation::SettingName::Stage(Setting::ScheduleMaxDemandPoints),
            3
        )
        .is_err());
}
