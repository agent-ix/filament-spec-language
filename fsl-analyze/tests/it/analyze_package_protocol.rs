// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940-AC-6: protocol litmus and memory-model items settle equally from a
//! source-checked package and its admitted handoff bytes.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, label, litmus_request, package_selection, Run, Workspace,
};
use crate::litmus::{sb_run, sb_unit};

fn subject() -> Value {
    let binding = json!({"type": "model-binding", "selection": "test/litmus"});
    json!({"protocol": {
        "protocol": "SB",
        "over": {"reference": {"population": "ix://test/litmus/cores", "key": "core"}},
        "roles": {"w": binding, "c": binding},
    }})
}

fn request_extra(items: Value) -> Value {
    json!({
        "subject": subject(),
        "memory": [{"parallel": "SB::Both", "model": "tso"}],
        "items": items,
    })
}

fn request_without_memory(items: Value) -> Value {
    json!({
        "subject": subject(),
        "items": items,
    })
}

fn package_request(
    ws: &Workspace,
    request_path: &std::path::Path,
) -> (std::path::PathBuf, Vec<u8>) {
    let document: Value =
        serde_json::from_slice(&std::fs::read(request_path).expect("the source request exists"))
            .expect("the source request is JSON");
    let (members, bytes) = package_selection(ws, &document["request"]);
    let package_document = json!({"format": "native-analyze/1", "request": members});
    (
        ws.write(
            "analyze-package.json",
            package_document.to_string().as_bytes(),
        ),
        bytes,
    )
}

fn package_run(ws: &Workspace, request_path: &std::path::Path) -> Run {
    analyze(&package_request(ws, request_path).0)
}

fn terminal_request(
    ws: &Workspace,
    captures: &str,
    terminal: &str,
    blocked: bool,
) -> std::path::PathBuf {
    let run = if blocked {
        format!(
            "sequence Main {{ {} }}",
            crate::fill::attempt("Blocked", "setTwo", "TwoPre, TwoPost", "attempted", "true")
        )
    } else {
        "sequence Main {}".to_owned()
    };
    let body = format!(
        "{}protocol Flow using pp over (k: M::Cell) on origin {{
            {captures}
            role w on M::Cell;
            {terminal}
            run {run}
            finish Done as (d: Boolean) {{ d }};
        }}",
        crate::fill::contracts("false")
    );
    let unit = crate::fill::unit_text_with(crate::fill::PROTOCOL_PROFILE, &body);
    let mut documents = crate::fill::Documents::new();
    documents.start("s0", &[("c", 0)]);
    let snapshots: Vec<Vec<u8>> = documents.snapshots.values().cloned().collect();
    // Select the terminal fixture's domain closure; the shared Fill corpus also
    // contains unrelated variant and compensation record declarations.
    let mut model: Value =
        serde_json::from_slice(&crate::fill::document()).expect("the Fill model is JSON");
    model["types"]
        .as_array_mut()
        .expect("model types")
        .retain(|node| {
            matches!(
                node["identity"].as_str(),
                Some("ix://test/fill/Level" | "ix://test/fill/Cell" | "ix://test/fill/Idle")
            )
        });
    model["constructs"]
        .as_array_mut()
        .expect("model constructs")
        .retain(|construct| {
            matches!(
                construct["kind"]["name"].as_str(),
                Some("value_type" | "object_type" | "population")
            )
        });
    let model = serde_json::to_vec(&model).expect("the terminal model serializes");
    crate::analyze_cli_support::request_over(
        ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Flow", "kind": "temporal-satisfaction",
        "temporal": {"item_kind": "deadlock-freedom", "subject": {"protocol": {
            "protocol": "Flow",
            "over": {"reference": {"population": "ix://test/fill/cells", "key": "c"}},
            "roles": {"w": {"type": "model-binding", "selection": "test/fill"}}
        }}}}]}),
    )
}

/// Terminal declarations classify stopped live instances identically on both routes.
#[trace("TC-2958", "FR-940-AC-6", "FR-850-AC-11", "FR-211-AC-2")]
#[test]
fn package_terminal_classifies_blocked_live_instances() {
    for (name, terminal, expected) in [
        ("absent", "", "refuted"),
        ("any", "terminal any;", "proved"),
        ("true", "terminal when { k.v = 0 };", "proved"),
        ("false", "terminal when { k.v = 1 };", "refuted"),
    ] {
        let ws = Workspace::new(&format!("package-terminal-{name}"));
        let path = terminal_request(&ws, "", terminal, true);
        let source = analyze(&path);
        assert!(
            source.item(0).get("record").is_some(),
            "source terminal {name}: {}",
            source.document()
        );
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl"))
            .expect("remove source before package analysis");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
        if expected == "refuted" {
            assert!(
                source.item(0)["counterexample"]["blocked"]
                    .as_array()
                    .is_some_and(|blocked| !blocked.is_empty()),
                "{}",
                source.document()
            );
            assert_eq!(package.item(0)["replay"]["reproduced"], true);
        }
    }
}

/// A finished instance is intended even when its terminal predicate is false.
#[trace("TC-2958", "FR-940-AC-6", "FR-211-AC-2")]
#[test]
fn package_terminal_preserves_finished_instances() {
    for (name, terminal) in [
        ("any", "terminal any; scheduling adversarial;"),
        ("false", "terminal when { k.v = 1 };"),
    ] {
        let ws = Workspace::new(&format!("package-terminal-finished-{name}"));
        let path = terminal_request(&ws, "", terminal, false);
        let source = analyze(&path);
        assert!(
            source.item(0).get("record").is_some(),
            "source finished {name}: {}",
            source.document()
        );
        assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
        let package = package_run(&ws, &path);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
    }
}

/// Origin captures initialize in order from the admitted observation and feed terminal predicates.
#[trace("TC-2958", "FR-940-AC-6", "FR-211-AC-2", "FR-383-AC-5")]
#[test]
fn package_terminal_reads_origin_captures_in_authored_order() {
    for (name, predicate, expected) in [("true", "saved", "proved"), ("false", "later", "refuted")]
    {
        let ws = Workspace::new(&format!("package-terminal-captures-{name}"));
        let members = format!("terminal when {{ {predicate} }};");
        let path = terminal_request(
            &ws,
            "capture saved: Boolean = k.v = 0; capture later: Boolean = not saved;",
            &members,
            true,
        );
        let source = analyze(&path);
        assert!(
            source.item(0).get("record").is_some(),
            "source capture {name}: {}",
            source.document()
        );
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl"))
            .expect("remove source before package analysis");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
        if expected == "refuted" {
            assert_eq!(package.item(0)["replay"]["reproduced"], true);
        }
    }
}

/// Refusing replay retains the same typed cause for a terminal deadlock on both routes.
#[trace("TC-2958", "FR-940-AC-22", "FR-211-AC-2")]
#[test]
fn package_terminal_preserves_replay_refusal_code_and_cause() {
    let ws = Workspace::new("package-terminal-replay-bound");
    let path = terminal_request(
        &ws,
        "capture saved: Boolean = k.v = 1;",
        "terminal when { saved };",
        true,
    );
    let (package_path, bytes) = package_request(&ws, &path);
    let below = u64::try_from(bytes.len() / 2).expect("the package length");
    let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, below);
    assert!(
        source.item(0).get("record").is_some(),
        "source replay refusal: {}",
        source.document()
    );
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    let cause = &source.item(0)["record"]["cause"];
    assert_eq!(cause["kind"], "replay-refused");
    assert_eq!(cause["code"], "stage_limit_exceeded");
    assert_eq!(cause["cause"], "input-bytes-exceeded");
    let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// Invalid deadlock subjects refuse before exploration with identical typed records.
#[trace("TC-2958", "FR-940-AC-6", "FR-850-AC-11")]
#[test]
fn package_terminal_invalid_subject_records_match_source() {
    for (name, member) in [
        ("missing", "temporal.subject.protocol"),
        ("mismatched", "temporal.subject.protocol"),
        ("capacity", "capacities"),
    ] {
        let ws = Workspace::new(&format!("package-terminal-invalid-{name}"));
        let path = terminal_request(&ws, "", "terminal any;", false);
        let mut document: Value =
            serde_json::from_slice(&std::fs::read(&path).expect("source request"))
                .expect("request JSON");
        let subject = &mut document["request"]["items"][0]["temporal"]["subject"];
        match name {
            "missing" => {
                subject
                    .as_object_mut()
                    .expect("subject object")
                    .remove("protocol");
            }
            "mismatched" => subject["protocol"]["protocol"] = json!("Other"),
            "capacity" => subject["protocol"]["capacities"] = json!({"unused": "-1"}),
            _ => unreachable!("the listed malformed cases"),
        }
        std::fs::write(&path, document.to_string()).expect("write malformed selection");
        let source = analyze(&path);
        let item = source.item(0);
        assert_eq!(item["category"], "refusal", "{}", source.document());
        assert_eq!(
            item["record"]["diagnostic"]["code"], "invalid_runtime_input",
            "{item}"
        );
        assert_eq!(
            item["record"]["diagnostic"]["cause"], "invalid-value",
            "{item}"
        );
        assert_eq!(
            item["record"]["diagnostic"]["detail"]["member"], member,
            "{item}"
        );
        assert!(
            item.get("exploration").is_none(),
            "invalid selection explored states: {item}"
        );
        assert_eq!(source.exit, 20, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl")).expect("remove source before package refusal");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), item, "{}", package.document());
        assert_eq!(package.exit, source.exit);
    }
}

/// FR-940-AC-6: the store-buffering litmus claim under TSO has equal records,
/// counterexamples and package-byte replay on both analyze routes.
#[trace("TC-2958", "FR-940-AC-6", "TC-3050", "FR-850-AC-11")]
#[test]
fn the_tso_litmus_claim_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let ws = Workspace::new("package-litmus-tso");
    let source_request =
        litmus_request(&ws, &body, request_extra(json!([{"claim": "NoBothZero"}])));
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(source.item(0)["replay"]["reproduced"], true);

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "refuted", "{}", package.document());
    assert_eq!(package.item(0)["record"], source.item(0)["record"]);
    assert_eq!(
        package.item(0)["counterexample"],
        source.item(0)["counterexample"]
    );
    assert_eq!(package.item(0)["replay"], source.item(0)["replay"]);
    assert_eq!(package.item(0)["replay"]["reproduced"], true);
}

/// Release/acquire uses the same package protocol, effective memory model
/// and bytes-only counterexample replay as the source route.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn the_ra_litmus_claim_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let ws = Workspace::new("package-litmus-ra");
    let mut extra = request_extra(json!([{"claim": "NoBothZero"}]));
    extra["memory"][0]["model"] = json!("ra");
    let source_request = litmus_request(&ws, &body, extra);
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(source.item(0)["replay"]["reproduced"], true);

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "refuted", "{}", package.document());
    assert_eq!(package.item(0), source.item(0));
    assert_eq!(package.exit, source.exit);
}

/// FR-940-AC-6: the protocol's race-freedom result reads the same admitted
/// protocol and memory-model facts on the package route.
#[trace("TC-2958", "FR-940-AC-6", "TC-3050", "FR-850-AC-11")]
#[test]
fn the_tso_race_freedom_item_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "", ""));
    let ws = Workspace::new("package-race-tso");
    let source_request = litmus_request(
        &ws,
        &body,
        request_extra(json!([{
            "claim": "SB::Both",
            "temporal": {"item_kind": "race-freedom"},
        }])),
    );
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(source.item(0)["replay"]["reproduced"], true);

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "refuted", "{}", package.document());
    assert_eq!(package.item(0)["record"], source.item(0)["record"]);
    assert_eq!(
        package.item(0)["counterexample"],
        source.item(0)["counterexample"]
    );
    assert_eq!(package.item(0)["replay"], source.item(0)["replay"]);
    assert_eq!(package.item(0)["replay"]["reproduced"], true);
}

/// FR-850-AC-11: the same SB protocol with no request memory member uses its
/// source `sc` default on both analyze routes, under which the litmus claim
/// holds.
#[trace("TC-3050", "FR-850-AC-11")]
#[test]
fn the_default_sc_litmus_claim_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "", ""));
    let ws = Workspace::new("package-litmus-default-sc");
    let source_request = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "NoBothZero"}])),
    );
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "proved", "{}", package.document());
    assert_eq!(package.item(0)["record"], source.item(0)["record"]);
}

/// A later attempt retains the earlier attempt's result binder while its
/// constraint reads it, on both protocol routes.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_later_package_attempt_can_read_an_earlier_result_binder() {
    let run = "sequence Main { \
        attempt A by c on M::Core::look contracts [LooksY, Looked] as (a: Boolean) { true }; \
        attempt B by c on M::Core::copy contracts [CopyR] as (b: Boolean) { not a }; \
        }";
    let body = sb_unit(run).replace(
        crate::litmus::NO_BOTH_ZERO,
        "temporal BinderLifetime using t0 over (k: M::Core) clock \
         \"model-steps\" on origin { always holds(true) }\n",
    );
    let ws = Workspace::new("package-protocol-binder-lifetime");
    let source_request = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "BinderLifetime"}])),
    );
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    assert_eq!(source.exit, 0, "{}", source.document());

    let package = package_run(&ws, &source_request);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

fn true_claim(run: &str) -> String {
    sb_unit(run).replace(
        crate::litmus::NO_BOTH_ZERO,
        "temporal BinderLifetime using t0 over (k: M::Core) clock \
         \"model-steps\" on origin { always holds(true) }\n",
    )
}

/// Finish retains a result binder and observes the current model state.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn package_finish_reads_the_previous_result_and_current_model() {
    let body = true_claim(
        "sequence Main { \
        attempt A by c on M::Core::look contracts [LooksY, Looked] as (a: Boolean) { true }; \
        }",
    )
    .replace("{ d };", "{ d and not a and k.r1 = 0 };");
    let ws = Workspace::new("package-protocol-finish-binders");
    let path = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "BinderLifetime"}])),
    );
    let source = analyze(&path);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    let package = package_run(&ws, &path);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// An attempt's object reference is admitted against its current observation.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_package_attempt_constraint_reads_the_current_model() {
    let body = true_claim(
        "sequence Main { \
        attempt A by c on M::Core::look contracts [LooksY, Looked] as (a: Boolean) { true }; \
        attempt B by c on M::Core::copy contracts [CopyR] as (b: Boolean) { k.r1 = 0 }; \
        }",
    );
    let ws = Workspace::new("package-protocol-attempt-observation");
    let path = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "BinderLifetime"}])),
    );
    let source = analyze(&path);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    let package = package_run(&ws, &path);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// The protocol replay shares the request's package-byte ceiling on both routes.
#[trace("TC-2958", "FR-940-AC-22")]
#[test]
fn package_protocol_replay_obeys_the_request_input_bytes_bound() {
    let ws = Workspace::new("package-protocol-replay-input-bytes");
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let path = litmus_request(&ws, &body, request_extra(json!([{"claim": "NoBothZero"}])));
    let (package_path, bytes) = package_request(&ws, &path);
    let below = u64::try_from(bytes.len() / 2).expect("the package length");
    let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, below);
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    assert_eq!(source.item(0)["record"]["cause"]["kind"], "replay-refused");
    assert_eq!(
        source.item(0)["record"]["cause"]["code"],
        "stage_limit_exceeded"
    );
    assert_eq!(
        source.item(0)["record"]["cause"]["cause"],
        "input-bytes-exceeded"
    );
    assert_eq!(
        source.item(0)["record"]["cause"]["setting"],
        "replay.input_bytes"
    );
    let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// Every memory entry is admitted before items run, including unused protocols.
#[trace("TC-2959", "FR-1033-AC-4", "FR-850-AC-11")]
#[test]
fn bad_package_memory_entries_refuse_the_whole_request() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    for (index, memory) in [
        json!([{"parallel": "Nope::Both", "model": "tso"}]),
        json!([{"parallel": "SB::Both::left::L::Sx", "model": "tso"}]),
        json!([{"parallel": "SB::Both", "model": "tso"}, {"parallel": "SB::Both", "model": "ra"}]),
    ]
    .into_iter()
    .enumerate()
    {
        let ws = Workspace::new(&format!("package-invalid-memory-{index}"));
        let path = litmus_request(
            &ws,
            &body,
            json!({"subject": subject(), "memory": memory, "items": [{"claim": "NoBothZero"}]}),
        );
        let source = analyze(&path);
        assert_eq!(source.exit, 20, "{}", source.document());
        assert_eq!(source.document()["items"], json!([]));
        let package = package_run(&ws, &path);
        assert_eq!(package.exit, 20, "{}", package.document());
        assert_eq!(package.document()["items"], json!([]));
        assert_eq!(
            package.document()["diagnostics"],
            source.document()["diagnostics"]
        );
        assert_eq!(
            package.document()["category"],
            source.document()["category"]
        );
    }
}

/// A valid memory entry for another protocol leaves this subject's identity alone.
#[trace("TC-2959", "FR-1033", "FR-850-AC-11")]
#[test]
fn another_protocols_memory_does_not_change_the_package_subject() {
    let run = sb_run("", "release", "acquire");
    let body = sb_unit(&run) + &crate::litmus::protocol("Other", &run);
    let ws = Workspace::new("package-other-protocol-memory");
    let path = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "NoBothZero"}])),
    );
    let baseline = analyze(&path);
    assert_eq!(
        label(&baseline.item(0)),
        "proved",
        "{}",
        baseline.document()
    );
    let path = litmus_request(
        &ws,
        &body,
        json!({"subject": subject(), "memory": [{"parallel": "Other::Both", "model": "tso"}],
            "items": [{"claim": "NoBothZero"}]}),
    );
    let source = analyze(&path);
    assert_eq!(source.item(0), baseline.item(0), "{}", source.document());
    let package = package_run(&ws, &path);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}
