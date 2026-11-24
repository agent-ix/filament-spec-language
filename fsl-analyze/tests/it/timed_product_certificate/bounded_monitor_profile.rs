// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-186-AC-7: the admitted bounded timed Eventually form.

use super::*;

#[trace("FR-186-AC-7")]
#[test]
fn timed_eventually_with_closed_horizon_settles_at_ninety_nine_percent() {
    let claims = "\
        probabilistic Eventually using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ eventually[0 ms, 4 ms] holds(c.late) ] }\n";
    let delay_two_ms = "delay M::Call::send ~ discrete { 2 ms: 1 };\n";
    let world = retx(GUARD, true, delay_two_ms, claims);
    let record = settle(&world, "Eventually");
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("timed bounded claim has a terminal: {record:#?}"));
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        certification: fsl_replay::Certification::Certified,
    } = terminal.value()
    else {
        panic!("timed bounded claim settles with a checked proof: {record:#?}");
    };
    assert_eq!(value.value(), &ratio(99, 100));

    let document = document(&record);
    let terminal_timeout = document["values"]
        .as_array()
        .expect("certificate values")
        .iter()
        .any(|row| {
            let encoded = row["state"].as_str().expect("encoded ProductKey");
            let bytes = URL_SAFE_NO_PAD.decode(encoded).expect("base64 ProductKey");
            let key: Value = serde_json::from_slice(&bytes).expect("canonical ProductKey");
            let clocks = key[4][1][4].as_array().expect("digital clocks");
            let x = clocks
                .iter()
                .find(|clock| clock[0][1] == "x")
                .expect("clock x");
            x[1] == "2" && key[4][1][5] == json!(["deadline", "4", "5"])
        });
    assert!(
        terminal_timeout,
        "producer's t=5 rejecting state retains x=2 ms"
    );
    assert!(matches!(
        check(&world, "Eventually", &document),
        Ok(ExactProbabilityCertificateCheck::Accepted(_))
    ));
}
