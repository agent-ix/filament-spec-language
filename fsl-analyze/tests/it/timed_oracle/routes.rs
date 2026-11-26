// SPDX-License-Identifier: AGPL-3.0-or-later
//! Each analysis route run on a corpus model, read as one [`Verdict`].

use filament_exact::{Cancel, Rational};
use fsl_analyze::analyze::{
    analyze_items, exact_probabilistic_item, DispositionKind, Engines, Settlement,
};
use fsl_analyze::exact_probabilistic::{ExactProbLimits, ProductRequest};
use fsl_analyze::statistical::{check_statistical, StatisticalRequest};
use fsl_eval::model_subject::ModelSubject;
use fsl_foundation::selection::DefinitionRef;
use fsl_foundation::RequestIndex;
use fsl_replay::statistical::{
    Decision, StatisticalLimits, StatisticalMethod, StatisticalOutcome, TimedSampling,
};
use fsl_replay::{InconclusiveCause, ProofBasis, TerminalValue};
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, request_over, Workspace};
use crate::timed_world::World;

/// What one route said of one claim.
#[derive(Clone, Debug)]
pub(super) enum Verdict {
    /// Proved, with the exact value where the route states one; a sampled
    /// route's accepted test reads here with its estimate.
    Proved(Option<Rational>),
    /// Refuted; a sampled route's rejected test reads here.
    Refuted(Option<Rational>),
    /// A typed answer of no verdict, named: `unsupported:...`,
    /// `incomplete:...`, `declined:...`, `undecided:...` or
    /// `requires-bound`. It agrees with a claim only where a row names it.
    Declined(String),
    /// Inconclusive with the named FSpec cause: no admitted behaviour or no
    /// instance.
    Vacuous(&'static str),
    /// A rejected certificate, a replay disagreement, a failed or stopped
    /// run, an invalid request or no record at all. No row accepts it.
    Broken(#[expect(dead_code, reason = "read through Debug in failures")] String),
}

/// The name of the variant a `Debug` rendering starts with.
fn variant(value: &impl std::fmt::Debug) -> String {
    format!("{value:?}")
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect()
}

/// One route's verdict on one claim.
#[derive(Clone, Debug)]
pub(super) struct Answer {
    pub(super) route: &'static str,
    pub(super) verdict: Verdict,
}

fn inconclusive(cause: &str) -> Option<&'static str> {
    ["no-instance", "no-admitted-behaviour"]
        .into_iter()
        .find(|vacuous| *vacuous == cause)
}

/// EN-5, the exact probabilistic route (digital clocks or regions, as the
/// engine selects), with its certificate check.
pub(super) fn exact(world: &World, claim: &str) -> Answer {
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let request = ProductRequest {
        subject: ModelSubject {
            simulation: world.request(),
            proof_bounds: Vec::new(),
        },
        claim: world
            .package
            .graph()
            .probabilistic_model()
            .claim(claim)
            .unwrap_or_else(|| panic!("the claim {claim} checks")),
        limits: ExactProbLimits::default(),
    };
    let item = exact_probabilistic_item(RequestIndex::new(0), request);
    let record = analyze_items(&engines, vec![item], &Cancel::new(), fsl_replay::CertificationPolicy::AdmitAny).remove(0);
    let verdict = match &record.settlement {
        Settlement::Disposed(disposed) => match disposed.kind {
            DispositionKind::RequiresBound => Verdict::Declined("requires-bound".to_owned()),
            DispositionKind::Unsupported => disposed.diagnostic.as_ref().map_or_else(
                || Verdict::Broken(format!("{disposed:?}")),
                |diagnostic| Verdict::Declined(format!("unsupported:{}", diagnostic.code.cause())),
            ),
            DispositionKind::InvalidRequest => Verdict::Broken(format!("{disposed:?}")),
        },
        Settlement::Terminal(terminal) => match terminal.value() {
            TerminalValue::Proved {
                basis: ProofBasis::ExactValue { value, .. },
                ..
            } => Verdict::Proved(Some(value.value().clone())),
            TerminalValue::Proved { .. } => Verdict::Proved(None),
            TerminalValue::Refuted => Verdict::Refuted(None),
            TerminalValue::Unsupported(cause) => {
                Verdict::Declined(format!("unsupported:{}", variant(cause)))
            }
            TerminalValue::Incomplete(cause) => {
                Verdict::Declined(format!("incomplete:{}", variant(cause)))
            }
            value @ TerminalValue::Declined { .. } => {
                Verdict::Declined(format!("declined:{}", variant(value)))
            }
            TerminalValue::Inconclusive(InconclusiveCause::NoInstance) => {
                Verdict::Vacuous("no-instance")
            }
            TerminalValue::Inconclusive(InconclusiveCause::NoAdmittedBehaviour) => {
                Verdict::Vacuous("no-admitted-behaviour")
            }
            value => Verdict::Broken(format!("{value:?}")),
        },
    };
    Answer {
        route: "en-5",
        verdict,
    }
}

/// EN-4, the sampled route, under the Okamoto method.
pub(super) fn sampled(world: &World, claim: &str) -> Answer {
    let outcome = check_statistical(
        StatisticalRequest {
            workers: 1,
            subject: ModelSubject {
                simulation: world.request(),
                proof_bounds: Vec::new(),
            },
            claim: world
                .package
                .graph()
                .probabilistic_model()
                .claim(claim)
                .unwrap_or_else(|| panic!("the claim {claim} checks")),
            method: StatisticalMethod::Okamoto,
            seed: 7,
            sampler: DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1")
                .expect("a valid DefinitionRef"),
            limits: StatisticalLimits::default(),
            max_witnesses: 1,
            symmetry: Vec::new(),
            timed_sampling: TimedSampling::default(),
        },
        &Cancel::new(),
    );
    let verdict = match outcome {
        Ok(StatisticalOutcome::Completed(verdict)) => match verdict.decision {
            Decision::Accepted => Verdict::Proved(verdict.estimate.clone()),
            Decision::Rejected => Verdict::Refuted(verdict.estimate.clone()),
            Decision::Undecided(cause) => {
                Verdict::Declined(format!("undecided:{}", variant(&cause)))
            }
        },
        Ok(StatisticalOutcome::Unsupported { cause, .. }) => {
            Verdict::Declined(format!("unsupported:{}", variant(&cause)))
        }
        // No row expects a stopped run or a refused request.
        other => Verdict::Broken(format!("{other:?}").chars().take(300).collect()),
    };
    Answer {
        route: "en-4",
        verdict,
    }
}

fn item_verdict(item: &Value) -> Verdict {
    let record = &item["record"];
    let text = |value: &Value| value.as_str().unwrap_or_default().to_owned();
    let broken = || Verdict::Broken(record.to_string().chars().take(300).collect());
    match record["result"].as_str() {
        Some("proved") => Verdict::Proved(None),
        Some("refuted") => Verdict::Refuted(None),
        Some(result @ ("unsupported" | "declined" | "incomplete")) => {
            let diagnostic = &record["diagnostic"];
            // FSpec FR-272's `not_digitisable` member of an EN-1 decline.
            let member = [
                &diagnostic["not_digitisable"],
                &diagnostic["detail"]["not_digitisable"],
            ]
            .into_iter()
            .find(|member| member.is_object());
            let rule = member.and_then(|m| m["rule"].as_str()).unwrap_or("none");
            let span = member.map_or_else(String::new, |m| {
                format!("{}-{}", m["span"]["start"], m["span"]["end"])
            });
            Verdict::Declined(format!(
                "{result}:{}/{}:{rule}@{span}",
                text(&diagnostic["code"]),
                text(&diagnostic["cause"]),
            ))
        }
        Some("inconclusive") => inconclusive(record["cause"]["kind"].as_str().unwrap_or_default())
            .map_or_else(broken, Verdict::Vacuous),
        _ => broken(),
    }
}

/// EN-6 (zones, the default) and EN-1 (`digital-clocks`) on `claim`, through
/// the command, over the snapshots `snapshots`.
pub(super) fn command_over(
    name: &str,
    world: &World,
    claim: &str,
    snapshots: &[Vec<u8>],
) -> [Answer; 2] {
    let workspace = Workspace::new(&format!("timed-oracle-{name}"));
    let unit = String::from_utf8(world.unit.clone()).expect("a UTF-8 unit");
    let items = json!({"items": [
        {"claim": claim},
        {"claim": claim, "options": {"method": "digital-clocks"}},
    ]});
    let request = request_over(&workspace, &unit, &world.document, snapshots, items);
    let run = analyze(&request);
    [("en-6", 0), ("en-1", 1)].map(|(route, at)| Answer {
        route,
        verdict: item_verdict(&run.item(at)),
    })
}

/// [`command_over`] the world's own snapshot.
pub(super) fn command(name: &str, world: &World, claim: &str) -> [Answer; 2] {
    let snapshots: Vec<Vec<u8>> = world.snapshots.values().cloned().collect();
    command_over(name, world, claim, &snapshots)
}
