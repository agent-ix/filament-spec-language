// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-196's product key carries the `on each` instance and its activation
//! phase, so an instance before and after activation, and two instances at
//! one model state, are distinct states.

use fsl_replay::{ProductAction, ProductActionRefusal};
use ix_trace_rs::trace;

const TAG: &str = "filament.probability-product-key/v1";
const INSTANCE_M: &str = r#"[{"t":"Link::Msg"},"m","0"]"#;
const INSTANCE_N: &str = r#"[{"t":"Link::Msg"},"n","0"]"#;

fn post_state(instance: &str, phase: &str) -> String {
    format!(r#"["post-state",["{TAG}",null,{instance},{phase},["m"],null,null,null]]"#)
}

#[trace("TC-3060", "FR-196-AC-15")]
#[test]
fn an_on_each_key_carries_its_instance_and_phase() {
    // FR-196-AC-15's Link start has m active and n waiting at one model key.
    let waiting = post_state(INSTANCE_M, r#""waiting""#);
    let active = post_state(INSTANCE_M, r#""active""#);
    let other = post_state(INSTANCE_N, r#""waiting""#);
    let origin = post_state("null", "null");
    let mut written = Vec::new();
    for text in [&waiting, &active, &other, &origin] {
        let parsed = ProductAction::parse(text.as_bytes()).expect("an eight-member key parses");
        let bytes = parsed.to_bytes().expect("it writes");
        assert_eq!(bytes, text.as_bytes(), "the key round-trips byte for byte");
        written.push(bytes);
    }
    assert_ne!(
        written[0], written[1],
        "waiting and active are distinct states"
    );
    assert_ne!(
        written[0], written[2],
        "m and n waiting at one model state have distinct keys"
    );
    assert_ne!(
        written[1], written[2],
        "the AC initial keys m active and n waiting are distinct"
    );
}

#[trace("TC-3060", "FR-196-AC-15")]
#[test]
fn a_six_member_key_is_refused() {
    let six = format!(r#"["post-state",["{TAG}",null,["m"],null,null,null]]"#);
    assert!(matches!(
        ProductAction::parse(six.as_bytes()),
        Err(ProductActionRefusal::Shape)
    ));
}

#[trace("TC-3060", "FR-196-AC-15")]
#[test]
fn an_instance_without_its_phase_or_a_phase_without_its_instance_is_refused() {
    for (instance, phase) in [(INSTANCE_M, "null"), ("null", r#""active""#)] {
        let half = post_state(instance, phase);
        assert!(matches!(
            ProductAction::parse(half.as_bytes()),
            Err(ProductActionRefusal::Shape)
        ));
    }
}
