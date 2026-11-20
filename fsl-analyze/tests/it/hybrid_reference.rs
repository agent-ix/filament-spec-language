// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1019 (TC-2869, TC-2997): the reference hybrid provider
//! `fsl-hybrid-reference`, reached through `native-analyze/1` documents and
//! the command adapter's analyze entry with `Engine::hybrid_reference()`
//! passed among its `providers` beside `Engines::fsl()` (FR-635), as the
//! driver does by default. Each assertion reads
//! FR-621's outcome item: `record` (`result`, `basis` or `cause`,
//! `certification`, `backend`), `counterexample`, `replay`, `certificate`
//! and the echoed `request`.

use std::path::{Path, PathBuf};

use fsl_analyze::analyze::{Engine, Engines};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze_with_providers, Workspace};
use crate::hybrid_requests::{
    braking, braking_pair, claim, find, first_item, hybrid_fixture, interval, rational,
};

const REFERENCE: &str = "fsl-hybrid-reference";

/// The reference provider as the command entries take it: `vec![Engine::hybrid_reference()]`
/// (FR-1019).
fn reference_providers() -> Vec<Engine> {
    vec![Engine::hybrid_reference().expect("the reference descriptor registers")]
}

/// [`reference_providers`] with `fixture` registered beside them.
fn with_reference(fixture: Engine) -> Vec<Engine> {
    let mut providers = reference_providers();
    providers.push(fixture);
    providers
}

/// The first outcome item of `unit`'s `items` with no provider configured.
fn reference(name: &str, unit: &str, items: Value) -> Value {
    first_item(name, unit, items, json!({}), reference_providers()).1
}

fn one(claim: &str) -> Value {
    json!([{"claim": claim}])
}

/// FR-1019-AC-1: `SlowEnough` is proved by a certified outward-rounded
/// flowpipe covering horizon 10 with one jump.
#[test]
#[trace("TC-2869", "FR-1019-AC-1")]
fn the_reference_provider_proves_slow_enough() {
    let item = reference("ref-1", &braking_pair(), one("SlowEnough"));
    let record = &item["record"];
    assert_eq!(record["result"], "proved", "{item}");
    assert_eq!(record["basis"]["type"], "bounded-solver", "{item}");
    assert_eq!(
        record["basis"]["method"], "flowpipe-outward-rounded",
        "{item}"
    );
    assert_eq!(record["basis"]["jumps"], 1, "{item}");
    assert_eq!(record["basis"]["exhaustive"], json!(["time"]), "{item}");
    assert!(
        record["basis"]["horizon"].to_string().contains("10"),
        "{item}"
    );
    assert_eq!(record["certification"], "certified", "{item}");
    assert_eq!(record["backend"], REFERENCE, "{item}");
}

/// FR-1019-AC-2: `TooSlow` is refuted by the start-only trace at the root
/// box's lower corner, which replays exactly, the same on every run.
#[test]
#[trace("TC-2869", "FR-1019-AC-2")]
fn the_reference_provider_refutes_too_slow_deterministically() {
    let first = reference("ref-2a", &braking_pair(), one("TooSlow"));
    assert_eq!(first["record"]["result"], "refuted", "{first}");
    assert_eq!(first["replay"]["reproduced"], true, "{first}");
    let trace = find(&first["counterexample"], "trace").expect("a hybrid trace");
    assert_eq!(trace["jumps"], json!([]), "{first}");
    assert_eq!(trace["start"]["mode"], "Moving", "{first}");
    assert_eq!(
        trace["start"]["enclosure"]["v"]["lower"],
        rational("0"),
        "{first}"
    );
    assert_eq!(
        trace["start"]["enclosure"]["v"]["upper"],
        rational("0"),
        "{first}"
    );
    assert_eq!(
        trace["start"]["enclosure"]["a"]["lower"],
        rational("1"),
        "{first}"
    );
    assert_eq!(
        trace["start"]["enclosure"]["a"]["upper"],
        rational("1"),
        "{first}"
    );
    let second = reference("ref-2b", &braking_pair(), one("TooSlow"));
    assert_eq!(first["counterexample"], second["counterexample"]);
}

/// FR-1019-AC-3: a non-polynomial flow is outside the class and settles
/// `unsupported` with a located `expression-form` cause.
#[test]
#[trace("TC-2869", "FR-1019-AC-3")]
fn a_non_polynomial_flow_is_a_located_unsupported() {
    let unit = braking_pair().replace(
        "flow { v' = -a; a' = 0; }",
        "flow { v' = -a / (1 + v); a' = 0; }",
    );
    let item = reference("ref-3", &unit, one("SlowEnough"));
    assert_eq!(item["record"]["result"], "unsupported", "{item}");
    assert_eq!(item["record"]["backend"], REFERENCE, "{item}");
    let text = item.to_string();
    for member in [
        "unsupported_construct",
        "expression-form",
        "Moving",
        "\"v\"",
    ] {
        assert!(text.contains(member), "{member}: {item}");
    }
}

/// FR-1019-AC-4: `Engines::fsl()` does not hold the reference provider, and
/// a named fixture hybrid provider beside it routes to the fixture.
#[test]
#[trace("TC-2869", "FR-1019-AC-4")]
fn the_provider_path_stays_beside_the_reference_provider() {
    let engines = Engines::fsl().expect("the layer-A engines register");
    assert!(
        engines
            .registrations()
            .all(|descriptor| descriptor.candidate().id().as_str() != REFERENCE),
        "Engines::fsl() leaves the reference provider out"
    );
    let (_, alone) = first_item(
        "ref-4-alone",
        &braking_pair(),
        one("SlowEnough"),
        json!({}),
        Vec::new(),
    );
    let text = alone.to_string();
    assert_eq!(alone["record"]["result"], "unsupported", "{alone}");
    for member in ["temporal-satisfaction", "hybrid-dynamics"] {
        assert!(text.contains(member), "{member}: {alone}");
    }
    let entry = json!({"request_index": 0, "result": "proved", "certification": "uncertified",
        "basis": {"type": "bounded-solver", "method": "delta-complete",
            "horizon": {"numerator": "10", "denominator": "1"}, "jumps": 1,
            "exhaustive": ["time"]}});
    let (_, item) = first_item(
        "ref-4",
        &braking_pair(),
        json!([{"claim": "SlowEnough", "backend": "hybrid-fixture"}]),
        json!({}),
        with_reference(hybrid_fixture("hybrid-fixture", &entry)),
    );
    assert_eq!(item["record"]["result"], "proved", "{item}");
    assert_eq!(item["record"]["backend"], "hybrid-fixture", "{item}");
}

/// FR-1019-AC-5: the reference provider is the one hybrid candidate and
/// never a discrete one; beside a second hybrid provider, naming it routes
/// and naming neither settles `ambiguous-backend` naming both.
#[test]
#[trace("TC-2869", "FR-1019-AC-5")]
fn the_reference_provider_negotiates_by_dynamics() {
    let item = reference("ref-5a", &braking_pair(), one("SlowEnough"));
    assert_eq!(
        item["request"]["candidates"]["backends"],
        json!([REFERENCE]),
        "{item}"
    );
    let ws = Workspace::new("ref-5b");
    let discrete = analyze_with_providers(
        &crate::analyze_cli_support::counter_request(
            &ws,
            &crate::analyze_cli_support::bounded(),
            json!([{"claim": "Bounded"}]),
        ),
        reference_providers(),
    )
    .item(0);
    let backends = discrete["request"]["candidates"]["backends"]
        .as_array()
        .expect("a candidate set");
    assert_eq!(backends.len(), 1, "{discrete}");
    assert_ne!(backends[0], REFERENCE, "{discrete}");
    let entry = json!({"request_index": 0, "result": "proved", "certification": "uncertified",
        "basis": {"type": "bounded-solver", "method": "delta-complete",
            "horizon": {"numerator": "10", "denominator": "1"}, "jumps": 1,
            "exhaustive": ["time"]}});
    let (_, named) = first_item(
        "ref-5c",
        &braking_pair(),
        json!([{"claim": "SlowEnough", "backend": REFERENCE}]),
        json!({}),
        with_reference(hybrid_fixture("hybrid-fixture", &entry)),
    );
    assert_eq!(named["record"]["backend"], REFERENCE, "{named}");
    let (_, neither) = first_item(
        "ref-5d",
        &braking_pair(),
        one("SlowEnough"),
        json!({}),
        with_reference(hybrid_fixture("hybrid-fixture", &entry)),
    );
    let text = neither.to_string();
    for member in ["ambiguous-backend", REFERENCE, "hybrid-fixture"] {
        assert!(text.contains(member), "{member}: {neither}");
    }
    assert!(neither["record"].get("result").is_none(), "{neither}");
}

/// FR-1019-AC-6, descriptor clause: `Engines::fsl_with_reference_hybrid()`
/// holds every engine of `Engines::fsl()` and the reference provider, whose
/// descriptor publishes `dynamics` `["hybrid"]`, `bounds` and `options` with
/// their defaults. The step-budget clause is the ignored test below.
#[test]
#[trace("TC-2869", "FR-1019-AC-6")]
fn the_reference_provider_publishes_its_descriptor() {
    let plain = Engines::fsl().expect("the layer-A engines register");
    let full = Engines::fsl_with_reference_hybrid().expect("the engines register");
    let held: Vec<String> = full
        .registrations()
        .map(|descriptor| descriptor.candidate().id().as_str().to_owned())
        .collect();
    for descriptor in plain.registrations() {
        assert!(
            held.contains(&descriptor.candidate().id().as_str().to_owned()),
            "{held:?}"
        );
    }
    assert_eq!(held.len(), plain.registrations().count() + 1, "{held:?}");
    let reference = full
        .registrations()
        .find(|descriptor| descriptor.candidate().id().as_str() == REFERENCE)
        .expect("the reference provider is registered");
    assert_eq!(
        reference
            .dynamics()
            .map(|kinds| kinds.iter().map(|kind| kind.to_wire()).collect::<Vec<_>>()),
        Some(vec!["hybrid"])
    );
    let bounds = reference.solver_bounds().expect("published bounds");
    assert_eq!(bounds.max_solver_steps, Some(1_000_000));
    assert_eq!(bounds.max_solver_time, Some(60));
    let defaults: Vec<(&str, &str)> = reference
        .option_defaults()
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    assert_eq!(
        defaults,
        [
            ("precision_bits", "64"),
            ("refine_depth", "32"),
            ("step", "1/16")
        ]
    );
}

/// FR-1019-AC-6: a step budget of 1 stops the solver `limit-reached` naming
/// `hybrid.max_solver_steps`.
#[test]
#[trace("TC-2869", "FR-1019-AC-6")]
fn the_reference_provider_stops_at_its_step_budget() {
    let item = reference(
        "ref-6b",
        &braking_pair(),
        json!([{"claim": "SlowEnough", "hybrid": {"limits": {"max_solver_steps": 1}}}]),
    );
    assert_eq!(item["record"]["result"], "incomplete", "{item}");
    let text = item["record"].to_string();
    for member in [
        "limit-reached",
        "max_solver_steps",
        "hybrid.max_solver_steps",
    ] {
        assert!(text.contains(member), "{member}: {item}");
    }
}

/// FR-1019-AC-7: coarse options still prove and change the request.
#[test]
#[trace("TC-2869", "FR-1019-AC-7")]
fn coarse_options_still_prove_and_change_the_request() {
    let default = reference("ref-7a", &braking_pair(), one("SlowEnough"));
    let coarse = reference(
        "ref-7b",
        &braking_pair(),
        json!([{"claim": "SlowEnough", "options": {"precision_bits": "8", "step": "1/4"}}]),
    );
    assert_eq!(coarse["record"]["result"], "proved", "{coarse}");
    assert_eq!(coarse["record"]["certification"], "certified", "{coarse}");
    assert_ne!(coarse["request"], default["request"]);
}

/// FR-1019-AC-7: an out-of-domain or unknown option settles
/// `invalid-request` naming it, before any solver step.
#[test]
#[trace("TC-2869", "FR-1019-AC-7")]
#[trace("FSpec-TC-1891", "FSpec-FR-779-AC-2")]
fn an_out_of_domain_or_unknown_option_settles_invalid_request() {
    // A published option with a value outside its domain: the provider's
    // `check_options` refuses it before any solver step.
    let bad_value = reference(
        "ref-7c",
        &braking_pair(),
        json!([{"claim": "SlowEnough", "backend": "fsl-hybrid-reference",
            "options": {"step": "0"}}]),
    );
    assert!(bad_value["record"].get("result").is_none(), "{bad_value}");
    assert_eq!(
        bad_value["record"]["settled_by"], "disposition",
        "{bad_value}"
    );
    assert_eq!(bad_value["category"], "refusal", "{bad_value}");
    let text = bad_value.to_string();
    for member in [
        "invalid_capability",
        "invalid-option-value",
        "fsl-hybrid-reference",
        "\"option\":\"step\"",
        "\"value\":\"0\"",
    ] {
        assert!(text.contains(member), "{member}: {bad_value}");
    }
    // The refusal comes before the solver reads the model: over a model the
    // class check would settle `unsupported`, the bad value settles first.
    let outside = braking_pair().replace(
        "flow { v' = -a; a' = 0; }",
        "flow { v' = -a / (1 + v); a' = 0; }",
    );
    let before_solve = reference(
        "ref-7e",
        &outside,
        json!([{"claim": "SlowEnough", "backend": "fsl-hybrid-reference",
            "options": {"step": "0"}}]),
    );
    let text = before_solve.to_string();
    assert!(text.contains("invalid-option-value"), "{before_solve}");
    assert!(!text.contains("expression-form"), "{before_solve}");
    // A name the descriptor does not publish: FR-967's `unknown-option`.
    let unknown = reference(
        "ref-7d",
        &braking_pair(),
        json!([{"claim": "SlowEnough", "backend": "fsl-hybrid-reference",
            "options": {"colour": "blue"}}]),
    );
    assert!(unknown["record"].get("result").is_none(), "{unknown}");
    assert_eq!(unknown["record"]["settled_by"], "disposition", "{unknown}");
    assert_eq!(unknown["category"], "refusal", "{unknown}");
    let text = unknown.to_string();
    for member in ["unknown-option", "\"option\":\"colour\""] {
        assert!(text.contains(member), "{member}: {unknown}");
    }
    assert!(!text.contains("invalid-option-value"), "{unknown}");
}

/// FR-1019-AC-8: the equality guard `v = 0` with the boundary invariant
/// `v >= 0` gives one `Stopped` jump step at the exact jump times, and no
/// `Moving` box crosses below `v = 0`.
#[test]
#[trace("TC-2869", "FR-1019-AC-8")]
fn the_flowpipe_stops_at_the_guard_and_jumps_once() {
    let item = reference("ref-8", &braking_pair(), one("SlowEnough"));
    let steps = item["certificate"]["form"]["steps"]
        .as_array()
        .expect("a flowpipe certificate");
    let stopped: Vec<&Value> = steps
        .iter()
        .filter(|step| step["mode"] == "Stopped")
        .collect();
    assert_eq!(stopped.len(), 1, "{item}");
    assert_eq!(stopped[0]["box"]["v"], interval("0", "0"), "{item}");
    assert_eq!(stopped[0]["box"]["a"], interval("1", "2"), "{item}");
    assert_eq!(stopped[0]["time"], interval("0", "10"), "{item}");
    for step in steps.iter().filter(|step| step["mode"] == "Moving") {
        for member in ["box", "apriori"] {
            let numerator = step[member]["v"]["lower"]["numerator"]
                .as_str()
                .expect("a rational");
            assert!(!numerator.starts_with('-'), "{step}");
        }
    }
}

/// FR-1019-AC-9: a claim with no horizon is proved by a certified inductive
/// enclosure of the two modes' hulls.
#[test]
#[trace("TC-2869", "FR-1019-AC-9")]
fn a_claim_with_no_horizon_is_proved_inductively() {
    let unit = braking(&claim("Bounded", "always holds(h.v < 30)"));
    let item = reference("ref-9", &unit, one("Bounded"));
    assert_eq!(item["record"]["result"], "proved", "{item}");
    assert_eq!(
        item["record"]["basis"]["type"], "inductive-solver",
        "{item}"
    );
    assert_eq!(
        item["record"]["basis"]["method"], "inductive-enclosure",
        "{item}"
    );
    assert_eq!(item["record"]["certification"], "certified", "{item}");
    let boxes = &item["certificate"]["form"]["boxes"];
    assert_eq!(boxes["Moving"]["v"], interval("0", "20"), "{item}");
    assert_eq!(boxes["Stopped"]["v"], interval("0", "0"), "{item}");
}

/// FR-1019-AC-10: a violation inside a dwell no jump closes is reached by
/// the trace's `end`.
#[test]
#[trace("TC-2869", "FR-1019-AC-10")]
fn a_violation_in_a_final_dwell_is_cut_by_end() {
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid Coast using p {\n\
           initial { mode Down when { v = 8 }; }\n\
           mode Down invariant { v >= 0 } flow { v' = -1; }\n\
         }\n\
         temporal Above using p over (h: Coast) clock \"model-time\" \
         on origin { always [0, 10] holds(h.v > 5) }\n";
    let item = reference("ref-10", unit, one("Above"));
    assert_eq!(item["record"]["result"], "refuted", "{item}");
    let trace = find(&item["counterexample"], "trace").expect("a hybrid trace");
    assert_eq!(trace["jumps"], json!([]), "{item}");
    assert_eq!(
        trace["start"]["enclosure"]["v"]["lower"],
        rational("8"),
        "{item}"
    );
    assert_eq!(trace["end"]["time"], interval("3", "3"), "{item}");
    assert_eq!(trace["end"]["enclosure"]["v"], interval("5", "5"), "{item}");
}

/// FR-1019-AC-11: a cyclic polynomial flow refutes through a
/// `validated-witness` the core accepts.
#[test]
#[trace("TC-2869", "FR-1019-AC-11")]
fn a_cyclic_polynomial_flow_refutes_through_a_validated_witness() {
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid Grow using p {\n\
           initial { mode Up when { v = 1 }; }\n\
           mode Up invariant { v >= 1 and v <= 100 } flow { v' = v; }\n\
         }\n\
         temporal Below using p over (h: Grow) clock \"model-time\" \
         on origin { always [0, 5] holds(h.v < 30) }\n";
    let item = reference("ref-11", unit, one("Below"));
    assert_eq!(item["record"]["result"], "refuted", "{item}");
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
    assert_eq!(
        item["certificate"]["form"]["type"], "validated-witness",
        "{item}"
    );
    assert!(item["counterexample"].is_object(), "{item}");
}

/// The `[dependencies]` names of the manifest at `path`.
fn dependencies(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
    let mut inside = false;
    let mut names = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[dependencies]";
        } else if inside {
            if let Some((name, _)) = line.split_once('=') {
                names.push(name.trim().to_owned());
            }
        }
    }
    names
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

/// FR-1019-AC-12: `fsl-hybrid-reference` is ADR-011 layer H: its FSL and
/// ecosystem dependencies are H's cell, `fsl-analyze` depends on it, and no
/// core crate does.
#[test]
#[trace("TC-2869", "FR-1019-AC-12")]
fn the_reference_crate_sits_in_layer_h() {
    let root = workspace_root();
    let allowed = [
        "fsl-replay",
        "fsl-package",
        "fsl-semantics",
        "fsl-foundation",
        "filament-checked",
        "filament-semantic-value",
        "filament-exact",
    ];
    for name in dependencies(&root.join("fsl-hybrid-reference/Cargo.toml")) {
        if name.starts_with("fsl-") || name.starts_with("filament-") {
            assert!(allowed.contains(&name.as_str()), "layer H names {name}");
        }
    }
    assert!(dependencies(&root.join("fsl-analyze/Cargo.toml"))
        .contains(&"fsl-hybrid-reference".to_owned()));
    for core in [
        "fsl-foundation",
        "fsl-cst",
        "fsl-source",
        "fsl-forms",
        "fsl-semantics",
        "fsl-package",
        "fsl-eval",
        "fsl-route",
        "fsl-replay",
    ] {
        assert!(
            !dependencies(&root.join(format!("{core}/Cargo.toml")))
                .contains(&"fsl-hybrid-reference".to_owned()),
            "{core} depends on the reference provider"
        );
    }
}

/// FR-1019-AC-13: the `examples/hybrid-braking/` documents reach their
/// verdicts with no provider configured.
#[test]
#[trace("TC-2997", "FR-1019-AC-13")]
fn the_hybrid_braking_examples_reach_verdicts() {
    let examples = workspace_root().join("examples/hybrid-braking");
    let slow = analyze_with_providers(
        &examples.join("slow-enough.analyze.json"),
        reference_providers(),
    )
    .item(0);
    assert_eq!(slow["record"]["result"], "proved", "{slow}");
    assert_eq!(slow["record"]["certification"], "certified", "{slow}");
    assert_eq!(slow["record"]["backend"], REFERENCE, "{slow}");
    let fast = analyze_with_providers(
        &examples.join("too-slow.analyze.json"),
        reference_providers(),
    )
    .item(0);
    assert_eq!(fast["record"]["result"], "refuted", "{fast}");
    assert_eq!(fast["record"]["backend"], REFERENCE, "{fast}");
    assert_eq!(fast["replay"]["reproduced"], true, "{fast}");
}

const PRELUDE: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.temporal.infinite-trace/v1\";\n";

fn temporal(model: &str, name: &str, activation: &str, body: &str) -> String {
    format!(
        "{PRELUDE}{model}\ntemporal {name} using p over (h: {}) clock \"model-time\" \
         {activation} {{ {body} }}\n",
        model
            .strip_prefix("hybrid ")
            .and_then(|rest| rest.split_whitespace().next())
            .expect("a hybrid declaration"),
    )
}

fn never_proved(name: &str, unit: &str, claim: &str) -> Value {
    let item = reference(name, unit, one(claim));
    assert_ne!(item["record"]["result"], "proved", "{item}");
    item
}

/// Soundness (FSpec FR-652-AC-20): a zero-length closing step never proves
/// the false claim over `Z`, whose behaviour has `x = 2t`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-20")]
fn a_zero_length_closing_step_never_proves_a_false_claim() {
    let unit = temporal(
        "hybrid Z using p { initial { mode M when { x = 0 }; } \
         mode M invariant { x >= 0 and x <= 100 } flow { x' = 2; } }",
        "False",
        "on origin",
        "always [0, 10] holds(h.x <= 0)",
    );
    let item = never_proved("ref-z", &unit, "False");
    assert_eq!(item["record"]["result"], "refuted", "{item}");
}

/// Soundness (FSpec FR-652-AC-21): an atom reading the `on each` subject has
/// no sound reading over one box, so the false claim is never proved.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-21")]
fn an_atom_reading_the_activation_subject_is_never_proved() {
    let unit = temporal(
        "hybrid Sw using p { initial { mode A when { x = 0 }; } \
         mode A invariant { x = 0 } flow { x' = 0; } \
         transition to B when { x = 0 } reset { x = 10 }; \
         mode B invariant { x = 10 } flow { x' = 0; } }",
        "Same",
        "on each (q: Sw)",
        "always holds(h.x = q.x)",
    );
    never_proved("ref-sw", &unit, "Same");
}

/// Soundness (FSpec FR-652-AC-22): a jump after a closing dwell's first
/// step is still followed, so the false claim over `Late` is never proved.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-22")]
fn a_jump_after_the_first_step_of_a_closing_dwell_is_followed() {
    let unit = temporal(
        "hybrid Late using p { initial { mode A when { x = 1 and y = 0 }; } \
         mode A invariant { x = 1 and y = 0 } flow { x' = 0; y' = 0; } \
         transition to B when { y = 0 } reset { x = 0 }; \
         mode B invariant { x >= 0 and x <= 100 and y = 0 } flow { x' = 1; y' = 0; } }",
        "Stays",
        "on origin",
        "always [5, 10] holds(h.x >= 1)",
    );
    never_proved("ref-late", &unit, "Stays");
}

/// Soundness: a claim false only after the last step a step budget allows is
/// never proved; the run is `incomplete`, and with the budget lifted the
/// violation at time 15 is found.
#[test]
#[trace("TC-2869", "FR-1019-AC-6")]
fn a_claim_false_beyond_the_last_step_is_never_proved() {
    let unit = temporal(
        "hybrid Ramp using p { initial { mode R when { x = 0 and w = 0 }; } \
         mode R invariant { x >= 0 and x <= 100 and w = 0 } flow { x' = 1; w' = 0; } }",
        "Far",
        "on origin",
        "always [0, 20] holds(h.x <= 15)",
    );
    let (_, cut) = first_item(
        "ref-far-cut",
        &unit,
        json!([{"claim": "Far", "hybrid": {"limits": {"max_solver_steps": 3}}}]),
        json!({}),
        reference_providers(),
    );
    assert_eq!(cut["record"]["result"], "incomplete", "{cut}");
    let full = never_proved("ref-far", &unit, "Far");
    assert_eq!(full["record"]["result"], "refuted", "{full}");
}

const FORK: &str = "hybrid Fork using p { initial { mode A when { x = 0 }; } \
     mode A invariant { x >= 0 and x <= 10 } flow { x' = 1; } \
     transition to B when { x >= 5 } reset { x = 100 }; \
     transition to C when { x >= 7 } reset { x = 0 }; \
     mode B invariant { x >= 0 and x <= 200 } flow { x' = 0; } \
     mode C invariant { x >= 0 and x <= 1 } flow { x' = 0; } }";

const DROP: &str = "hybrid Drop using p { initial { mode A when { x = 1 }; } \
     mode A invariant { x >= 0 and x <= 1 } flow { x' = -1; } \
     transition to B when { x = 0 } reset { x = 0 }; \
     mode B invariant { x = 0 } flow { x' = 0; } }";

fn by_steps(unit: &str) -> String {
    unit.replace("model-time", "model-steps")
}

/// FR-1019-AC-16: over `Drop` under `clock "model-steps"` the claim is proved
/// with `jumps` 1 and `exhaustive` holding `jumps`, the certificate holding the
/// depth-1 `B` dwell's jump step.
#[test]
#[trace("TC-2869", "FR-1019-AC-16")]
fn a_model_steps_claim_over_drop_is_proved_with_its_jump_dwell() {
    let unit = by_steps(&temporal(DROP, "Held", "on origin", "always [0, 1] holds(h.x <= 1)"));
    let item = reference("ref-drop", &unit, one("Held"));
    let record = &item["record"];
    assert_eq!(record["result"], "proved", "{item}");
    assert_eq!(record["basis"]["type"], "bounded-solver", "{item}");
    assert_eq!(record["basis"]["jumps"], 1, "{item}");
    assert!(
        record["basis"]["exhaustive"]
            .as_array()
            .is_some_and(|dimensions| dimensions.contains(&json!("jumps"))),
        "{item}"
    );
    assert_eq!(record["certification"], "certified", "{item}");
    let steps = item["certificate"]["form"]["steps"]
        .as_array()
        .expect("a flowpipe certificate");
    let jump = steps
        .iter()
        .find(|step| step["mode"] == "B")
        .expect("the depth-1 dwell");
    assert_eq!(jump["from"], json!({"dwell": 0, "transition": 0}), "{item}");
}

/// FR-1019-AC-16: over `Fork`, `always [0, 10] holds(h.x >= 0)` is proved,
/// certified, with one jump step for each of `A`'s two transitions.
#[test]
#[trace("TC-2869", "FR-1019-AC-16")]
fn a_fork_is_proved_with_a_jump_step_for_each_transition() {
    let unit = temporal(FORK, "NonNegative", "on origin", "always [0, 10] holds(h.x >= 0)");
    let item = reference("ref-fork", &unit, one("NonNegative"));
    assert_eq!(item["record"]["result"], "proved", "{item}");
    assert_eq!(item["record"]["certification"], "certified", "{item}");
    let steps = item["certificate"]["form"]["steps"]
        .as_array()
        .expect("a flowpipe certificate");
    let from: Vec<&Value> = steps.iter().filter_map(|step| step.get("from")).collect();
    assert_eq!(
        from,
        [
            &json!({"dwell": 0, "transition": 0}),
            &json!({"dwell": 0, "transition": 1})
        ],
        "{item}"
    );
}

/// FR-1019 "Refutations" step 3: a trace crosses a jump, so the claim `x < 50`
/// over `Fork` is refuted through the jump at time 5 to `x = 100`.
#[test]
#[trace("TC-2869", "FR-1019-AC-2")]
fn a_candidate_trace_crosses_a_jump() {
    let unit = temporal(FORK, "Low", "on origin", "always [0, 10] holds(h.x < 50)");
    let item = reference("ref-fork-low", &unit, one("Low"));
    assert_eq!(item["record"]["result"], "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
    let trace = find(&item["counterexample"], "trace").expect("a hybrid trace");
    assert_eq!(trace["jumps"].as_array().map(Vec::len), Some(1), "{item}");
}

/// FR-1019 "Claims with no horizon": an `on each` claim with a horizon is
/// proved inductively.
#[test]
#[trace("TC-2869", "FR-1019-AC-9")]
fn an_on_each_claim_is_proved_by_the_inductive_enclosure() {
    let unit = temporal(
        "hybrid Sw using p { initial { mode A when { x = 0 }; } \
         mode A invariant { x = 0 } flow { x' = 0; } \
         transition to B when { x = 0 } reset { x = 0 }; \
         mode B invariant { x = 0 } flow { x' = 0; } }",
        "Zero",
        "on each (q: Sw)",
        "always [0, 5] holds(h.x = 0)",
    );
    let item = reference("ref-each", &unit, one("Zero"));
    assert_eq!(item["record"]["result"], "proved", "{item}");
    assert_eq!(item["record"]["basis"]["type"], "inductive-solver", "{item}");
}

/// FR-1019 "Refutations": the refutation root is one initial entry's box, so
/// the entries `v = 0` and `v = 10` never try `v = 5`.
#[test]
#[trace("TC-2869", "FR-1019-AC-2")]
fn a_refutation_tries_only_points_of_its_own_entry() {
    let unit = temporal(
        "hybrid Two using p { initial { mode M when { v = 0 and w = 0 }; mode M when { v = 10 and w = 0 }; } \
         mode M invariant { v >= 0 and v <= 100 and w = 0 } flow { v' = 1; w' = 0; } }",
        "Small",
        "on origin",
        "always [0, 1] holds(h.v < 5)",
    );
    let item = reference("ref-two", &unit, one("Small"));
    assert_eq!(item["record"]["result"], "refuted", "{item}");
    let trace = find(&item["counterexample"], "trace").expect("a hybrid trace");
    let lower = &trace["start"]["enclosure"]["v"]["lower"];
    assert!(*lower == rational("0") || *lower == rational("10"), "{item}");
    assert_eq!(*lower, trace["start"]["enclosure"]["v"]["upper"], "{item}");
}
