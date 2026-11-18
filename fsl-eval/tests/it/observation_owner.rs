// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-861-AC-4 (TC-2442): `fsl-eval` takes the partial-value, completeness,
//! activation and repair authority rules from `filament-observation` and
//! holds no second implementation of them.

use fsl_eval::completeness::{revise_complete_through, CompleteThroughRefusal};
use fsl_eval::temporal::{CoveredList, ObservedPosition, ProgressCompleteness};
use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

fn sources(directory: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("a readable source directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

#[test]
#[trace("TC-2442", "FR-861-AC-4")]
#[ignore = "QSL-2145: fsl-eval still carries its own partial, completeness, activation and repair rules"]
fn fsl_eval_calls_the_observation_owner_for_its_four_authority_rule_sets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("the manifest");
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .expect("a [dependencies] table");
    assert!(
        dependencies.contains("filament-observation"),
        "fsl-eval names filament-observation in [dependencies]"
    );
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    let text: String = files
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("a source file"))
        .collect();
    let grouped: Vec<&str> = text
        .lines()
        .filter(|line| {
            line.trim_start()
                .starts_with("use filament_observation::authority::{")
        })
        .collect();
    for module in ["partial", "completeness", "activation", "repair"] {
        let named = text.contains(&format!("filament_observation::authority::{module}"))
            || grouped.iter().any(|line| {
                line.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|word| word == module)
            });
        assert!(named, "fsl-eval calls the owner's `{module}` module");
    }
}

/// Every shipped source file of `fsl-eval`, concatenated.
fn eval_source() -> String {
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    files
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("a source file"))
        .collect()
}

/// FR-944's quarter-tick endpoint of `value`, open or closed, at the lower
/// or upper end.
fn quarter(value: i128, open: bool, lower: bool) -> i128 {
    let scaled = value.checked_mul(4).expect("an i128 endpoint");
    match (open, lower) {
        (false, _) => scaled,
        (true, true) => scaled + 1,
        (true, false) => scaled - 1,
    }
}

/// Assert that `fsl-eval` maps its covers onto the owner's quarter-tick
/// intervals rather than comparing them itself.
fn assert_quarter_tick_owner_mapping(text: &str) {
    assert!(
        text.contains("EventTimeInterval"),
        "fsl-eval builds owner EventTimeIntervals for its covers"
    );
    assert!(
        text.contains("\"/4\""),
        "the owner interval's unit identity carries the `/4` suffix"
    );
}

#[test]
#[trace("TC-2624", "FR-944-AC-5")]
fn a_doubly_open_difference_stays_indeterminate_in_quarter_ticks() {
    use fsl_eval::temporal::StampInterval;
    let request = StampInterval {
        earliest: 0,
        latest: 1,
        earliest_open: true,
        latest_open: false,
    };
    let ack = StampInterval {
        earliest: 3,
        latest: 4,
        earliest_open: false,
        latest_open: true,
    };
    let scaled = |interval: StampInterval| {
        (
            quarter(interval.earliest.into(), interval.earliest_open, true),
            quarter(interval.latest.into(), interval.latest_open, false),
        )
    };
    assert_eq!(scaled(request), (1, 4));
    assert_eq!(scaled(ack), (12, 15));
    // relation_to(ack, request shifted by 4b) admits `after` and `before`.
    let ((request_lo, request_hi), (ack_lo, ack_hi)) = (scaled(request), scaled(ack));
    let bound = 4 * 3;
    assert!(ack_hi > request_lo + bound && ack_lo < request_hi + bound);
    // The half-tick mapping ([1, 2], [6, 7], shift 6) admits no `after`.
    let half = |value: i128, open: bool, lower: bool| match (open, lower) {
        (false, _) => 2 * value,
        (true, true) => 2 * value + 1,
        (true, false) => 2 * value - 1,
    };
    assert!(
        half(4, true, false) <= half(0, true, true) + 2 * 3,
        "half ticks read True"
    );
    assert_quarter_tick_owner_mapping(&eval_source());
}

#[test]
#[trace("TC-2624", "FR-944-AC-6")]
fn one_sided_eligibility_reads_the_scaled_endpoints() {
    // (lo, hi) of the scaled difference against an anchor of [0, 0].
    let at_least = |(lo, hi): (i128, i128), b: i128| {
        if lo >= 4 * b {
            Some(true)
        } else if hi < 4 * b {
            Some(false)
        } else {
            None
        }
    };
    let at_most = |(lo, hi): (i128, i128), b: i128| {
        if hi <= 4 * b {
            Some(true)
        } else if lo > 4 * b {
            Some(false)
        } else {
            None
        }
    };
    assert_eq!(at_least((8, 11), 2), Some(true));
    assert_eq!(at_least((4, 7), 2), Some(false));
    assert_eq!(at_least((4, 15), 2), None);
    assert_eq!(at_most((4, 7), 2), Some(true));
    assert_eq!(at_most((12, 15), 2), Some(false));
    assert_eq!(at_most((8, 11), 2), None);
    assert_eq!(at_least((5, 8), 1), Some(true));
    assert_eq!(at_least((0, 3), 1), Some(false));
    assert_eq!(at_most((0, 3), 0), None);
    assert_eq!(at_most((12 - 4, 15 - 1), 3), None);
    assert_eq!(at_least((12 - 4, 15 - 1), 2), Some(true));
    assert_quarter_tick_owner_mapping(&eval_source());
}

#[test]
#[trace("TC-2624", "FR-944-AC-7")]
fn quarter_tick_scaling_is_exact_at_the_domain_edge() {
    use fsl_eval::temporal::{ClockCover, Containment};
    let max = u64::MAX;
    let wide = i128::from(max);
    let counter = |k: u64| {
        let k = i128::from(k);
        (
            quarter(k - wide, false, true),
            quarter(k + 1 + wide, false, false) - 1,
        )
    };
    assert_eq!(counter(max), (0, 147_573_952_589_676_412_923));
    assert_eq!(
        counter(0),
        (-73_786_976_294_838_206_460, 73_786_976_294_838_206_463)
    );
    assert_eq!(
        quarter(i128::from(i64::MAX), false, true),
        36_893_488_147_419_103_228
    );
    assert_eq!(
        quarter(i128::from(i64::MIN), false, true),
        -36_893_488_147_419_103_232
    );
    // FR-944's owner reading of `within_bound` and `definitely_before` on
    // the quarter-tick intervals, compared with `ClockCover`'s answers.
    let within = |bound: i64, request: (i128, i128), ack: (i128, i128)| {
        let shift = quarter(i128::from(bound), false, true);
        let (lo, hi) = (request.0 + shift, request.1 + shift);
        let after = ack.1 > lo;
        let before_or_equal = ack.0 <= hi;
        match (after, before_or_equal) {
            (false, _) => Containment::True,
            (true, false) => Containment::False,
            (true, true) => Containment::Indeterminate,
        }
    };
    let cover = ClockCover::Counter { uncertainty: max };
    for bound in [i64::MIN, -1, 0, 1, i64::MAX] {
        for (request, ack) in [(0, max), (max, 0), (0, 0), (max, max)] {
            assert_eq!(
                cover.within_bound(bound, request, ack),
                within(bound, counter(request), counter(ack)),
                "bound {bound}, request {request}, ack {ack}"
            );
        }
    }
    for (a, b) in [(0, max), (max, 0), (0, 0)] {
        assert_eq!(
            cover.definitely_before(a, b),
            counter(a).1 < counter(b).0,
            "{a} before {b}"
        );
    }
    assert_quarter_tick_owner_mapping(&eval_source());
}

fn at(identity: &str, stamp: u64) -> ObservedPosition {
    ObservedPosition {
        identity: identity.to_owned(),
        stamp,
        fingerprint: None,
    }
}

/// FR-944's "Owner requirement: completeness revision" cases, run on the
/// one local function FR-861 names. This pins the rule text case by case; it
/// does not scan the source for a second comparison, so it carries no AC tag.
#[test]
#[trace("TC-2442")]
fn the_complete_through_rule_accepts_and_refuses_as_fr_944_states() {
    let held = [at("a", 0), at("b", 50), at("c", 100)];
    let covered: CoveredList = held.iter().cloned().collect();
    let retained = ProgressCompleteness::Complete { through: 100 };
    let revise = |asserted, positions: &[ObservedPosition]| {
        revise_complete_through(retained, asserted, &covered, positions)
    };
    for through in [100, 140] {
        assert_eq!(
            revise(ProgressCompleteness::Complete { through }, &held),
            Ok(()),
            "Complete through {through} after Complete through 100"
        );
    }
    assert_eq!(
        revise(ProgressCompleteness::Complete { through: 60 }, &held),
        Err(CompleteThroughRefusal::Retreat),
        "Complete through 60"
    );
    assert_eq!(
        revise(ProgressCompleteness::Incomplete, &held),
        Err(CompleteThroughRefusal::Retreat),
        "Incomplete"
    );
    let same = ProgressCompleteness::Complete { through: 100 };
    for (stamp, expected) in [(80, true), (100, true), (101, false)] {
        let uncovered = at("d", stamp);
        assert_eq!(
            revise(same, std::slice::from_ref(&uncovered)),
            if expected {
                Err(CompleteThroughRefusal::UncoveredPosition(Box::new(
                    uncovered,
                )))
            } else {
                Ok(())
            },
            "an uncovered position stamped {stamp} against a boundary of 100"
        );
    }
    assert_eq!(
        revise_complete_through(
            ProgressCompleteness::Incomplete,
            ProgressCompleteness::Incomplete,
            &covered,
            &held
        ),
        Ok(()),
        "a retained Incomplete refuses nothing"
    );
}
