// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2850: the exact engine's limit, cancel and fault exits settle as their
//! own outcome, never as `unsupported`/`not-yet-implemented`.
use super::*;
use crate::long_run_certificate::availability_with_claims;
use fsl_analyze::analyze::terminal_records;

const NOT_YET: &str = "unsupported_construct/not-yet-implemented";

fn long_run(name: &str, predicate: &str, bound: &str) -> String {
    format!(
        "probabilistic {name} using inf on M over (s: M::Server) under Steady on origin \
         {{ long_run_fraction holds({predicate}) {bound} }}\n"
    )
}

fn diagnostic_of(record: fsl_analyze::analyze::AnalyzeRecord) -> (TerminalValue, Option<String>) {
    let terminal = terminal_records(vec![record]).remove(0);
    (
        terminal.value().clone(),
        terminal.diagnostic().map(|code| code.to_string()),
    )
}

/// A long-run claim whose product reads an undefined value (a deleted
/// object's field) is routed to the undefined-path producer as every other
/// exact form is; FSL FR-202 admits the form, EN-5 does not build its path, so
/// it settles as that unbuilt form and not as an engine fault.
#[trace("TC-763", "FR-281-AC-11")]
#[test]
fn a_long_run_claim_that_reads_an_undefined_value_is_unbuilt_not_a_fault() {
    let claim = "probabilistic Dropped using inf on M over (c: M::Coin) under Even on origin \
                 { long_run_fraction holds(c.v = 0) >= 0.5 }";
    let world = World::new(&prob_world::coin(claim));
    let record = settle(&world, "Dropped", ModelCheckLimits::default());
    let (value, diagnostic) = diagnostic_of(record);
    assert_ne!(value, TerminalValue::Failed, "{diagnostic:?}");
    assert_eq!(diagnostic.as_deref(), Some(NOT_YET), "{value:?}");
}

/// Expected-reward claims enter the budgeted product builder too. The coin's
/// delete draw reaches an undefined read before the reward's target, and EN-5
/// has no expected-reward undefined-path producer.
#[trace("TC-763", "FR-281-AC-11")]
#[test]
fn an_expected_reward_claim_that_reads_an_undefined_value_is_unbuilt() {
    let claim =
        "probabilistic RewardUndefined using inf on M over (c: M::Coin) under Even on origin \
                { expected accumulate cost until holds(c.v = 1) <= 1 }";
    let model = prob_world::coin(&format!("reward cost on Coin::drop = 1;\n{claim}"));
    let world = World::new(&model);
    let product = fsl_analyze::exact_probabilistic::build_product(
        world.request("RewardUndefined", ModelCheckLimits::default()),
        &Cancel::new(),
    )
    .expect("the product builder accepts the expected-reward claim");
    assert!(
        matches!(
            &product,
            fsl_analyze::exact_probabilistic::ProbBuild::Undefined(_)
        ),
        "the coin's delete draw must reach the undefined predicate path: {product:?}"
    );
    let record = settle(&world, "RewardUndefined", ModelCheckLimits::default());
    let (value, diagnostic) = diagnostic_of(record);
    assert_ne!(value, TerminalValue::Failed, "{diagnostic:?}");
    assert_eq!(diagnostic.as_deref(), Some(NOT_YET), "{value:?}");
}

/// A cancel at any poll of a long-run run settles incomplete or with its
/// verdict, never as an unbuilt form or a fault.
#[trace("TC-763", "FR-281-AC-11")]
#[test]
fn cancelling_a_long_run_claim_at_any_poll_is_never_unbuilt() {
    let world = availability_with_claims(
        long_run("Lower", "s.phase = 0", ">= rational(1800,1801)"),
        "",
    );
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let run = |cancel: &Cancel| {
        let item = exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("Lower", ModelCheckLimits::default()),
        );
        diagnostic_of(analyze_items(&engines, vec![item], cancel).remove(0))
    };
    let live = Cancel::new();
    let _counting = live.count_charges();
    let (control, _) = run(&live);
    assert!(
        matches!(control, TerminalValue::Proved { .. }),
        "{control:?}"
    );
    let total = live.charges();
    let stride = (total / 40).max(1);
    let mut stopped = 0_u32;
    for at in (1..=total).step_by(stride as usize) {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let polls = Arc::new(AtomicU32::new(0));
        let seen = Arc::clone(&polls);
        let cancel = Cancel::observing(move || {
            if seen.fetch_add(1, Ordering::Relaxed) + 1 == at {
                weak.upgrade()
                    .expect("the run owns the slot")
                    .get()
                    .expect("the slot is initialized")
                    .cancel(CancelCause::Requested);
            }
        });
        slot.set(cancel.clone()).expect("initialized once");
        let (value, diagnostic) = run(&cancel);
        assert_ne!(diagnostic.as_deref(), Some(NOT_YET), "poll {at}: {value:?}");
        assert!(
            matches!(
                value,
                TerminalValue::Proved { .. } | TerminalValue::Incomplete(_)
            ),
            "poll {at} of {total}: {value:?} {diagnostic:?}"
        );
        stopped += u32::from(matches!(value, TerminalValue::Incomplete(_)));
    }
    assert!(stopped > 0, "some poll must land inside the run");
}
