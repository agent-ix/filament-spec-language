// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fair reachability keeps exact evidence when the value itself fits its budget.

use filament_exact::{Cancel, Rational};
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use fsl_replay::{ModelCheckLimits, ProofBasis, TerminalValue};
use ix_trace_rs::trace;

use crate::prob_world::{self, World};

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(
        filament_exact::Integer::from(numerator),
        filament_exact::Integer::from(denominator),
    )
    .expect("a nonzero denominator")
}

/// FR-1041-AC-1: a fair coin's exactly representable minimum stays exact even
/// when the request's precision setting would permit the interval route.
#[trace("TC-3150", "FR-1041-AC-1")]
#[test]
fn fair_reachability_keeps_an_exact_value_with_a_small_rational_budget() {
    let source = prob_world::fair_bounds_w(
        "probabilistic FairMin using inf on M over (w: M::W) \
         under every scheduler fair strong W::go on origin \
         { probability >= rational(1,2) [ eventually holds(w.g) ] }",
    );
    let world = World::new(&source);
    let mut request = world.request("FairMin", ModelCheckLimits::default());
    request.limits.max_rational_bits = 2;
    request.limits.max_precision_bits = 16;
    let record = analyze_items(
        &Engines::fsl().expect("the linked engines are admitted"),
        vec![exact_probabilistic_item(
            fsl_foundation::RequestIndex::new(0),
            request,
        )],
        &Cancel::new(),
    )
    .remove(0);
    let terminal = record.terminal().expect("the fair claim settles");
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        ..
    } = terminal.value()
    else {
        panic!("the exact value 1/2 fits the two-bit budget: {terminal:?}");
    };
    assert_eq!(value.value(), &ratio(1, 2));
}
