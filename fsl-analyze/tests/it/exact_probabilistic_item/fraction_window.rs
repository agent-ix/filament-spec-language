// SPDX-License-Identifier: AGPL-3.0-or-later
//! The exact mean of a fraction over a long window on a two-state model stays
//! inside the default work allowance: the independent product check expands
//! each model state once, however many (remaining, count) product states
//! stand over it.
use super::*;
// The window-10 work boundary pins the current accounting of the independent
// product check; it is not stated by a requirement.

const WINDOW: u64 = 10;

fn mean(window: u64) -> prob_world::Model {
    prob_world::health(
        "self.healthy = not fault",
        "true: 1, false: 9999",
        false,
        &format!(
            "probabilistic Up using ev on M over (n: M::Node) under every scheduler on origin \
             {{ mean of (fraction holds(n.healthy) over [0, {window}]) >= 0.9 }}\n"
        ),
    )
}

fn settle_under(world: &World, work_units: u64) -> fsl_analyze::analyze::AnalyzeRecord {
    let engines = Engines::fsl().unwrap();
    let mut request = world.request("Up", ModelCheckLimits::default());
    request.subject.simulation.meter.work_units = work_units;
    let item = exact_probabilistic_item(RequestIndex::new(0), request);
    analyze_items(&engines, vec![item], &Cancel::new()).remove(0)
}

#[trace("TC-3152", "FR-1043-AC-5")]
#[test]
fn a_ten_step_fraction_mean_on_two_states_proves_within_the_default_work_allowance() {
    let world = World::new(&mean(WINDOW));
    let record = settle_under(&world, 1_000_000);
    assert!(
        matches!(
            record.terminal().expect("a terminal record").value(),
            TerminalValue::Proved { .. }
        ),
        "{record:?}"
    );
    // One start position up, then ten steps that each stay up with 9999/10000.
    assert_eq!(
        record.exact_probabilistic.as_ref().unwrap().value,
        ratio(10_999, 11_000)
    );
}

#[trace("TC-638", "FR-203-AC-13")]
#[test]
fn the_ten_step_fraction_mean_needs_exactly_the_measured_work_units() {
    let world = World::new(&mean(WINDOW));
    let needed = 294_699;
    assert!(matches!(
        settle_under(&world, needed).terminal().unwrap().value(),
        TerminalValue::Proved { .. }
    ));
    let short = settle_under(&world, needed - 1);
    assert_eq!(
        short.terminal().unwrap().value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::EvaluationMeter {
                counter: filament_exact::LimitKind::WorkUnits
            },
            value: needed - 1,
            required: None,
        })
    );
}

fn build_document(
    world: &World,
    edges: usize,
    work_units: u64,
) -> Result<(), fsl_replay::ProbabilityDocumentError> {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("Up")).unwrap();
    let mut accounting = fsl_replay::AccountingLimits::default().0;
    accounting.work_units = work_units;
    fsl_replay::measure_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        &fsl_replay::ExactProbabilityCertificate { values: Vec::new() },
        fsl_replay::MeasurePayoffKind::Fraction,
        fsl_replay::ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 8_000_000,
            states: 128,
            edges,
            scalar_bits: 128,
            accounting,
        },
        &Cancel::new(),
    )
    .map(|_| ())
}

/// A successor's key is encoded only once its edge is admitted, so a transition
/// ceiling that the second edge exceeds is reported before that edge's key work.
#[trace("TC-3152", "FR-1043-AC-7")]
#[test]
fn the_edge_ceiling_is_reported_before_the_next_successors_key_is_encoded() {
    let world = World::new(&mean(0));
    let stopped = build_document(&world, 1, 31_116);
    assert!(
        matches!(
            stopped,
            Err(fsl_replay::ProbabilityDocumentError::Limit {
                limit: fsl_replay::ProbabilityDocumentLimit::Edges,
                bound: 1,
                actual: 2,
            })
        ),
        "{stopped:?}"
    );
    assert!(matches!(
        build_document(&world, 1, 31_115),
        Err(fsl_replay::ProbabilityDocumentError::KeyEncoding(_))
    ));
}

/// FR-1042-AC-28: a mean ceiling settled by bounds takes the `upper` document
/// as the claim side. The exact mean is 10999/11000, below 0.99999.
#[trace("TC-3212", "FR-1042-AC-28")]
#[test]
fn a_mean_ceiling_certified_by_bounds_takes_the_upper_document_as_the_claim() {
    let model = prob_world::health(
        "self.healthy = not fault",
        "true: 1, false: 9999",
        false,
        &format!(
            "probabilistic Up using ev on M over (n: M::Node) under every scheduler on origin \
             {{ mean of (fraction holds(n.healthy) over [0, {WINDOW}]) <= 0.99999 }}\n"
        ),
    );
    let world = World::new(&model);
    let engines = Engines::fsl().unwrap();
    let mut request = world.request("Up", ModelCheckLimits::default());
    request.limits.max_rational_bits = 1;
    let item = exact_probabilistic_item(RequestIndex::new(0), request);
    let record = analyze_items(&engines, vec![item], &Cancel::new()).remove(0);
    let TerminalValue::Proved {
        basis: ProofBasis::ValueBounds { lower, upper, .. },
        ..
    } = record.terminal().expect("a terminal record").value()
    else {
        panic!("{record:?}");
    };
    let exact = ratio(10_999, 11_000);
    assert!(lower.value() <= &exact, "{lower:?}");
    let fsl_replay::ExactBound::Finite(upper) = upper else {
        panic!("{upper:?}");
    };
    assert!(upper.value() >= &exact, "{upper:?}");
    assert!(upper.value() <= &ratio(99_999, 100_000), "{upper:?}");
}
