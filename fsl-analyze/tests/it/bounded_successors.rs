// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-851 (TC-2411): a modified integer field whose postcondition bounds it
//! is enumerated only inside those bounds, so the rover mode-machine
//! liveness item finishes at default limits. The rover domain package and
//! its start snapshot are the filament-examples rover fixture.

use std::time::{Duration, Instant};

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, Run, Workspace};

const MODEL: &str = include_str!("../fixtures/rover/rover.semantic-ir.json");
const START: &str = include_str!("../fixtures/rover/start-driving.json");

/// The `loseLidar` postcondition's speed bounds.
const LOSE_LIDAR_BOUNDS: &str = "self.speed <= 300 and self.speed <= pre(self.speed)";

/// The rover rules with `bounds` as `loseLidar`'s speed conjunct, and the
/// liveness clause `StopsAfterLidarLoss`.
fn rules(bounds: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile inf = \"filament.temporal.infinite-trace/v1\";\n\
         model Rover = \"example/rover\";\n\
         pre LidarLossWhileDriving using v on Rover::Rover::loseLidar {{ self.mode = Rover::Mode::Driving and self.lidarOk }}\n\
         post LidarLossDegrades using v on Rover::Rover::loseLidar {{ self.mode = Rover::Mode::Degraded and not self.lidarOk and ({bounds}) }}\n\
         pre BrakeWhenDegraded using v on Rover::Rover::brake {{ self.mode = Rover::Mode::Degraded }}\n\
         post BrakeEntersSafeStop using v on Rover::Rover::brake {{ self.mode = Rover::Mode::SafeStop and self.speed <= 100 and self.speed <= pre(self.speed) }}\n\
         pre HaltInSafeStop using v on Rover::Rover::halt {{ self.mode = Rover::Mode::SafeStop and not self.stopped }}\n\
         post HaltStops using v on Rover::Rover::halt {{ self.speed = 0 and self.stopped }}\n\
         pre FailsafeAfterHalt using v on Rover::Rover::enterFailsafe {{ self.mode = Rover::Mode::SafeStop and self.stopped }}\n\
         post FailsafeEntered using v on Rover::Rover::enterFailsafe {{ self.mode = Rover::Mode::Failsafe }}\n\
         pre SampleUntilFailsafe using v on Rover::Rover::sample {{ self.mode != Rover::Mode::Failsafe }}\n\
         post SampleRecordsVote using v on Rover::Rover::sample {{ self.voteWrong = ((faultA and faultB) or (faultA and faultC) or (faultB and faultC)) }}\n\
         terminal Parked using v on Rover when {{ self.mode = Rover::Mode::Failsafe }};\n\
         temporal StopsAfterLidarLoss using inf over (r: Rover::Rover) clock \"model-steps\" on origin {{\n\
           fair weak Rover::Rover::brake;\n\
           fair weak Rover::Rover::halt;\n\
           always (holds(not r.lidarOk) implies eventually holds(r.stopped))\n\
         }}\n"
    )
}

/// Analyze `StopsAfterLidarLoss` over the rover fixture with `bounds` and
/// the item's `temporal.limits`.
fn run(name: &str, bounds: &str, limits: Option<Value>) -> Run {
    let ws = Workspace::new(name);
    ws.write("rover.fsl", rules(bounds).as_bytes());
    ws.write("model.json", MODEL.as_bytes());
    ws.write("start.json", START.as_bytes());
    let mut item = json!({"claim": "StopsAfterLidarLoss"});
    if let Some(limits) = limits {
        item["temporal"] = json!({"limits": limits});
    }
    let request = json!({"format": "native-analyze/1", "request": {
        "program": {"source": {"file": "rover.fsl", "authority": "example",
            "identity": "example:rover/mode-machine", "document": "modemachine",
            "formal_revision": 1}},
        "models": [{"format": "semantic-ir/2.0.0", "source": {"file": "model.json",
            "authority": "example", "identity": "example:rover/model", "document": "model",
            "formal_revision": 1}}],
        "documents": [{"file": "start.json"}],
        "items": [item]}});
    analyze(&ws.write("analyze.json", request.to_string().as_bytes()))
}

/// FR-851-AC-1, AC-2: with `max_candidates` 400 the item explores to
/// completion and reports the unlimited run's exploration, since no
/// expansion generates more than `loseLidar`'s 301 speeds plus `sample`'s
/// assignments; with 300 it stops on `loseLidar`'s 301 kept candidates.
#[trace("TC-2411", "FR-851-AC-1")]
#[trace("TC-2411", "FR-851-AC-2")]
#[test]
#[ignore = "QSL-2287: successor generation enumerates a bounded integer field over its whole range"]
fn bounded_fields_are_enumerated_inside_their_postcondition_interval() {
    let unlimited = run("bounded-unlimited", LOSE_LIDAR_BOUNDS, None);
    let narrowed = run(
        "bounded-400",
        LOSE_LIDAR_BOUNDS,
        Some(json!({"max_candidates": 400})),
    );
    assert_eq!(label(&narrowed.item(0)), "proved", "{}", narrowed.item(0));
    assert_eq!(
        narrowed.item(0)["exploration"],
        unlimited.item(0)["exploration"]
    );
    let short = run(
        "bounded-300",
        LOSE_LIDAR_BOUNDS,
        Some(json!({"max_candidates": 300})),
    );
    assert_eq!(label(&short.item(0)), "incomplete", "{}", short.item(0));
    assert_eq!(short.item(0)["record"]["cause"]["limit"], "max_candidates");
}

/// FR-851-AC-3 (negative): a bound not of the `self.f op e` form, and a
/// bound under `or`, narrow nothing, so 400 candidates do not cover the
/// 2001-value speed range.
#[trace("TC-2411", "FR-851-AC-3")]
#[test]
#[ignore = "QSL-2287: successor generation enumerates a bounded integer field over its whole range"]
fn an_ineligible_bound_is_enumerated_over_the_whole_range() {
    for (name, bounds) in [
        ("bounded-not-form", "300 - self.speed >= 0"),
        ("bounded-or", "self.speed <= 300 or self.stopped"),
    ] {
        let run = run(name, bounds, Some(json!({"max_candidates": 400})));
        assert_eq!(label(&run.item(0)), "incomplete", "{name}: {}", run.item(0));
        assert_eq!(
            run.item(0)["record"]["cause"]["limit"],
            "max_candidates",
            "{name}"
        );
    }
}

/// FR-851-AC-4: the rover liveness item proves at EN-1's published defaults
/// within 60 seconds.
#[trace("TC-2411", "FR-851-AC-4")]
#[test]
#[ignore = "QSL-2287: the rover mode-machine search does not finish within its budget"]
fn the_rover_mode_machine_item_proves_at_default_limits() {
    let started = Instant::now();
    let run = run("bounded-rover", LOSE_LIDAR_BOUNDS, None);
    assert!(started.elapsed() < Duration::from_secs(60));
    let item = run.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["record"]["basis"]["type"], "exhaustive", "{item}");
    assert_eq!(run.exit, 0);
}
