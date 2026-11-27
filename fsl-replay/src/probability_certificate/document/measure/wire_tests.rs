// SPDX-License-Identifier: AGPL-3.0-or-later
//! The finite-horizon measure document has one declared wire type: these tests pin
//! its bytes, its round trip and what its strict reader refuses.

use super::*;
use ix_trace_rs::trace;

fn limits() -> ProbabilityDocumentLimits {
    ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 8_000_000,
        states: 100,
        edges: 100,
        scalar_bits: 128,
        accounting: crate::AccountingLimits::default().0,
    }
}

fn key(tag: u8) -> ProbabilityProductKey {
    ProbabilityProductKey::from_canonical_bytes(
        format!("[\"filament.probability-product-key/v1\",{tag},null,null,0,[0,0,0],[],0]")
            .into_bytes(),
    )
}

fn rational(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

fn exact(optimization: ProbabilityObjective) -> MeasureProbabilityDocument {
    MeasureProbabilityDocument {
        identity: ObligationIdentity::new(
            filament_exact::NodeKey::new(
                "partial",
                vec![filament_exact::PathSegment::Text("lower".into())],
                0,
            )
            .unwrap(),
            Some(crate::ModelIdentity::new("m".to_owned())),
            3,
        ),
        payoff: MeasurePayoffKind::Event,
        values: vec![
            ExactProbabilityValue {
                state: key(1),
                value: rational(1, 2),
            },
            ExactProbabilityValue {
                state: key(2),
                value: rational(0, 1),
            },
        ],
        ranking: vec![(key(1), 1), (key(2), 0)],
        objective: Objective {
            kind: "steps",
            window: 4,
            reward: None,
            comparison: Some(MeasureComparison::Le),
            threshold: Some(rational(2, 1)),
            level: None,
            optimization,
            fairness: Vec::new(),
        },
        conditional: None,
    }
}

fn conditional(conditional: Conditional) -> MeasureProbabilityDocument {
    let mut document = exact(ProbabilityObjective::Workload);
    document.values.clear();
    document.ranking.clear();
    document.conditional = Some(Box::new(conditional));
    document
}

fn write(document: &MeasureProbabilityDocument) -> String {
    let bytes = write_measure_probability_certificate(document, limits(), &Cancel::new()).unwrap();
    String::from_utf8(bytes).unwrap()
}

fn read(text: &str) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    read_measure_probability_certificate(text.as_bytes(), limits(), &Cancel::new())
}

const GOLDEN_EXACT: &str = r#"{"identity":{"model":"m","node_id":{"ordinal":0,"package":"partial","path":["lower"]},"ordinal":3},"kind":"exact","objective":{"fairness":[],"measure":{"comparison":"le","kind":"steps","level":null,"reward":null,"threshold":{"denominator":"1","numerator":"2"},"unit":null,"window":4},"optimization":"minimum","payoff":"event","profile":"filament.probability-objective/finite-horizon-measure/v1"},"ranking":[{"rank":1,"state":"WyJmaWxhbWVudC5wcm9iYWJpbGl0eS1wcm9kdWN0LWtleS92MSIsMSxudWxsLG51bGwsMCxbMCwwLDBdLFtdLDBd"},{"rank":0,"state":"WyJmaWxhbWVudC5wcm9iYWJpbGl0eS1wcm9kdWN0LWtleS92MSIsMixudWxsLG51bGwsMCxbMCwwLDBdLFtdLDBd"}],"values":[{"state":"WyJmaWxhbWVudC5wcm9iYWJpbGl0eS1wcm9kdWN0LWtleS92MSIsMSxudWxsLG51bGwsMCxbMCwwLDBdLFtdLDBd","value":{"denominator":"2","numerator":"1"}},{"state":"WyJmaWxhbWVudC5wcm9iYWJpbGl0eS1wcm9kdWN0LWtleS92MSIsMixudWxsLG51bGwsMCxbMCwwLDBdLFtdLDBd","value":{"denominator":"1","numerator":"0"}}]}"#;
const GOLDEN_VACUOUS: &str = r#"{"conditional":{"mode":"vacuous"},"identity":{"model":"m","node_id":{"ordinal":0,"package":"partial","path":["lower"]},"ordinal":3},"kind":"conditional","objective":{"fairness":[],"measure":{"comparison":"le","kind":"steps","level":null,"reward":null,"threshold":{"denominator":"1","numerator":"2"},"unit":null,"window":4},"optimization":"workload","payoff":"event","profile":"filament.probability-objective/finite-horizon-measure/v1"},"ranking":[],"values":[]}"#;
const GOLDEN_RATIO: &str = r#"{"conditional":{"denominator":{"identity":{"model":"m","node_id":{"ordinal":0,"package":"partial","path":["lower"]},"ordinal":3},"kind":"exact","objective":{"fairness":[],"measure":{"comparison":"le","kind":"steps","level":null,"reward":null,"threshold":{"denominator":"1","numerator":"2"},"unit":null,"window":4},"optimization":"workload","payoff":"event","profile":"filament.probability-objective/finite-horizon-measure/v1"},"ranking":[{"rank":1,"state":"WyJmaWxhbWVudC5wcm9iYWJpbGl0eS1wcm9kdWN0LWtleS92MSIsMSxudWxsLG51bGwsMCxbMCwwLDBdLFtdLDBd"},{"rank":0,"state":"WyJmaWxhbWVudC5wcm9iYWJpbGl0eS1wcm9kdWN0LWtleS92MSIsMixudWxsLG51bGwsMCxbMCwwLDBdLFtdLDBd"}],"values":[{"state":"WyJmaWxhbWVudC5wcm9iYWJpbGl0eS1wcm9kdWN0LWtleS92MSIsMSxudWxsLG51bGwsMCxbMCwwLDBdLFtdLDBd","value":{"denominator":"2","numerator":"1"}},{"state":"WyJmaWxhbWVudC5wcm9iYWJpbGl0eS1wcm9kdWN0LWtleS92MSIsMixudWxsLG51bGwsMCxbMCwwLDBdLFtdLDBd","value":{"denominator":"1","numerator":"0"}}]},"mode":"ratio","numerator":{"identity":{"model":"m","node_id":{"ordinal":0,"package":"partial","path":["lower"]},"ordinal":3},"kind":"exact","objective":{"fairness":[],"measure":{"comparison":"le","kind":"steps","level":null,"reward":null,"threshold":{"denominator":"1","numerator":"2"},"unit":null,"window":4},"optimization":"workload","payoff":"event","profile":"filament.probability-objective/finite-horizon-measure/v1"},"ranking":[{"rank":1,"state":"WyJmaWxhbWVudC5wcm9iYWJpbGl0eS1wcm9kdWN0LWtleS92MSIsMSxudWxsLG51bGwsMCxbMCwwLDBdLFtdLDBd"},{"rank":0,"state":"WyJmaWxhbWVudC5wcm9iYWJpbGl0eS1wcm9kdWN0LWtleS92MSIsMixudWxsLG51bGwsMCxbMCwwLDBdLFtdLDBd"}],"values":[{"state":"WyJmaWxhbWVudC5wcm9iYWJpbGl0eS1wcm9kdWN0LWtleS92MSIsMSxudWxsLG51bGwsMCxbMCwwLDBdLFtdLDBd","value":{"denominator":"2","numerator":"1"}},{"state":"WyJmaWxhbWVudC5wcm9iYWJpbGl0eS1wcm9kdWN0LWtleS92MSIsMixudWxsLG51bGwsMCxbMCwwLDBdLFtdLDBd","value":{"denominator":"1","numerator":"0"}}]}},"identity":{"model":"m","node_id":{"ordinal":0,"package":"partial","path":["lower"]},"ordinal":3},"kind":"conditional","objective":{"fairness":[],"measure":{"comparison":"le","kind":"steps","level":null,"reward":null,"threshold":{"denominator":"1","numerator":"2"},"unit":null,"window":4},"optimization":"workload","payoff":"event","profile":"filament.probability-objective/finite-horizon-measure/v1"},"ranking":[],"values":[]}"#;

/// FR-201-AC-9: writing then strictly reading preserves the document byte for byte.
#[trace("FR-201-AC-9")]
#[test]
fn measure_document_bytes_are_unchanged_and_round_trip() {
    let ratio = conditional(Conditional::Ratio(
        Box::new(exact(ProbabilityObjective::Workload)),
        Box::new(exact(ProbabilityObjective::Workload)),
    ));
    for (document, golden) in [
        (exact(ProbabilityObjective::Minimum), GOLDEN_EXACT),
        (conditional(Conditional::Vacuous), GOLDEN_VACUOUS),
        (ratio, GOLDEN_RATIO),
    ] {
        let text = write(&document);
        assert_eq!(text, golden);
        let decoded = read(&text).unwrap();
        assert_eq!(write(&decoded), text);
    }
}

/// FR-201-AC-9: the strict reader refuses extra members and wrong shapes.
#[trace("FR-201-AC-9")]
#[test]
fn measure_document_reader_refuses_unknown_members_and_wrong_shapes() {
    let good = write(&exact(ProbabilityObjective::Minimum));
    assert!(read(&good).is_ok());
    let cases = [
        good.replacen("{\"identity\"", "{\"extra\":null,\"identity\"", 1),
        good.replacen("\"window\":4", "\"window\":4,\"extra\":1", 1),
        good.replacen("\"ordinal\":3", "\"ordinal\":3,\"extra\":1", 1),
        good.replacen("\"numerator\":\"2\"", "\"numerator\":\"2\",\"extra\":1", 1),
        good.replacen("\"window\":4", "\"window\":\"4\"", 1),
        good.replacen("\"values\":[", "\"values\":{\"x\":[", 1),
        good.replacen("\"kind\":\"exact\"", "\"kind\":7", 1),
    ];
    for case in cases {
        assert_ne!(case, good);
        assert!(read(&case).is_err(), "{case}");
    }
}
