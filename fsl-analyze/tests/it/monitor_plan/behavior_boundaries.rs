// SPDX-License-Identifier: AGPL-3.0-or-later
//! Plan premises and capacities from source-checked timed subjects.

use super::*;

fn derived(
    package: &CheckedPackage,
    target: &MonitorTarget,
) -> Result<fsl_replay::TimedMonitorPlan, MonitorPlanRefusal> {
    produce(
        package,
        fsl_semantics::library::LibraryName::new("boundary").expect("a library name"),
        package.graph().temporal_clauses()[0].clause(),
        target,
        &TimingObligationSettings::default(),
    )
    .map(|(plan, _)| plan)
}

#[trace("FR-251")]
#[test]
fn exact_fractional_bounds_convert_to_each_targets_tick_unit() {
    let (package, _) = checked_clause_in(
        "once[rational(1, 3) ms, rational(5, 2) ms] holds(c.versionNumber = 2)",
        "",
        true,
    );
    for (unit, lower, upper, rate, uncertainty) in [
        ("ms", ratio(1, 3), ratio(5, 2), ratio(2, 3), ratio(1, 4)),
        (
            "us",
            ratio(1000, 3),
            ratio(2500, 1),
            ratio(1, 1500),
            ratio(250, 1),
        ),
    ] {
        let target = MonitorTarget {
            unit: unit.to_owned(),
            max_event_rate: Some(rate.clone()),
            uncertainty: uncertainty.clone(),
            ..base_target()
        };
        let plan = derived(&package, &target).expect("source-bound fractional plan derives");
        let [(operator, interval)] = plan.intervals() else {
            panic!("exactly the written once interval");
        };
        assert_eq!(interval.lower(), &lower);
        assert_eq!(interval.upper(), Some(&upper));
        assert!(interval.lower_closed() && interval.upper_closed());
        assert_eq!(plan.rate_limit(), &rate);
        assert_eq!(plan.uncertainty(), &uncertainty);
        // ms: ceil((2/3)*(5/2 + 1 + 1/2))+1 = 4.
        // us: ceil((1/1500)*(2500 + 1 + 500))+1 = 4.
        assert_eq!(plan.buffers(), &[(*operator, 4)], "target {unit}");
        assert_eq!(plan.program().atoms().len(), 1);
    }
}

#[trace("FR-251")]
#[test]
fn buffer_capacity_rounds_up_after_both_tick_cover_and_uncertainty() {
    let (package, _) = checked_clause_in("once[0 ms, 2 ms] holds(c.versionNumber = 2)", "", true);
    for (uncertainty, expected) in [(ratio(0, 1), 3), (ratio(1, 2), 3), (ratio(3, 4), 4)] {
        let target = MonitorTarget {
            max_event_rate: Some(ratio(1, 2)),
            uncertainty,
            ..base_target()
        };
        let plan = derived(&package, &target).expect("positive fractional premises derive");
        assert_eq!(plan.buffers().len(), 1);
        // ceil(1/2 * (2 + 1 + 2*u)) + 1 at u = 0, 1/2, 3/4.
        assert_eq!(plan.buffers()[0].1, expected);
    }
}

#[trace("FR-251-AC-3")]
#[test]
fn counter_gap_admission_distinguishes_the_last_tick_from_a_full_wrap() {
    let (package, _) = checked_clause_in("once[0 ms, 1 ms] holds(c.versionNumber = 2)", "", true);
    for (width_bits, last_gap) in [(1, 1), (16, 65_535), (64, u64::MAX)] {
        let target = MonitorTarget {
            width_bits,
            max_reading_gap: last_gap,
            ..base_target()
        };
        let plan =
            derived(&package, &target).expect("every representable sub-wrap gap is admitted");
        assert_eq!(plan.binding().width_bits(), width_bits);
        assert_eq!(plan.max_reading_gap(), last_gap);
        if let Some(full_wrap) = last_gap.checked_add(1) {
            assert_eq!(
                derived(
                    &package,
                    &MonitorTarget {
                        max_reading_gap: full_wrap,
                        ..target
                    }
                ),
                Err(MonitorPlanRefusal::GapExceedsWidth {
                    max_reading_gap: full_wrap,
                    width_bits
                })
            );
        }
    }
}

#[trace("FR-251")]
#[test]
fn invalid_rate_uncertainty_and_unit_never_produce_a_plan() {
    let (package, _) = checked_clause_in("once[0 ms, 1 ms] holds(c.versionNumber = 2)", "", true);
    for target in [
        MonitorTarget {
            max_event_rate: Some(ratio(0, 1)),
            ..base_target()
        },
        MonitorTarget {
            max_event_rate: Some(ratio(-1, 2)),
            ..base_target()
        },
        MonitorTarget {
            uncertainty: ratio(-1, 2),
            ..base_target()
        },
    ] {
        assert_eq!(
            derived(&package, &target),
            Err(MonitorPlanRefusal::InvalidBufferInputs)
        );
    }
    assert_eq!(
        derived(
            &package,
            &MonitorTarget {
                unit: "absent_tick".to_owned(),
                ..base_target()
            }
        ),
        Err(MonitorPlanRefusal::InvalidTimeUnit {
            unit: "absent_tick".to_owned()
        })
    );
}
