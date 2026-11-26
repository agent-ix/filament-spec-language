// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-037's acceptance net for each shared wire type: the real writer's bytes
//! validate against FSpec's provider schema and read back through the same
//! type as an equal value that writes the same bytes. The written bytes are
//! hand-written from FSpec.

use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::StateKey;
use fsl_replay::shared_wire::{ProbabilisticBound, StateLocus, TransitionIdentity};
use ix_trace_rs::trace;
use serde_json::Value;

use crate::provider_schema_support::{assert_valid, provider_definition};

/// `bytes` as the schema's instance.
fn instance(bytes: &str) -> Value {
    serde_json::from_str(bytes).expect("JSON")
}

// FSpec FR-331: `[0,[]]` has the unpadded base64url `WzAsW11d`.
#[trace("FSpec-FR-331")]
#[test]
fn a_state_locus_conforms_and_round_trips() {
    let locus = StateLocus::from(StateKey::from_canonical_bytes(b"[0,[]]".to_vec()));
    let written = serde_json::to_string(&locus).expect("writes");
    assert_eq!(written, r#"{"key":"WzAsW11d"}"#);
    assert_valid(&provider_definition("State"), &instance(&written), "State");
    let read: StateLocus = serde_json::from_str(&written).expect("reads");
    assert_eq!(read, locus);
    assert_eq!(serde_json::to_string(&read).expect("writes"), written);
}

// FSpec FR-181: `{type, operation, receiver, arguments}`, a name identity
// written as the bare name.
#[trace("FSpec-FR-181", "FR-120-AC-40")]
#[test]
fn a_transition_identity_conforms_and_round_trips() {
    let transition = ModelTransition::new(
        r#""tick""#.to_owned(),
        None,
        vec![r#"{"type":"integer","value":"1"}"#.to_owned()],
        quire_canonical::Limits::new(u64::MAX),
    )
    .expect("a canonical identity");
    let identity = TransitionIdentity::borrowed(&transition);
    let written = serde_json::to_string(&identity).expect("writes");
    assert_eq!(
        written,
        r#"{"type":"transition","operation":"tick","receiver":null,"arguments":[{"type":"integer","value":"1"}]}"#
    );
    assert_valid(
        &provider_definition("SourceTransitionIdentity"),
        &instance(&written),
        "SourceTransitionIdentity",
    );
    let read: TransitionIdentity<'_> = serde_json::from_str(&written).expect("reads");
    assert_eq!(read, identity);
    assert_eq!(serde_json::to_string(&read).expect("writes"), written);
}

// FSpec FR-413's claim-bound table, each of its four forms.
#[trace("FSpec-FR-413")]
#[test]
fn each_listed_claim_bound_conforms_and_round_trips() {
    let schema = provider_definition("ProbabilisticBound");
    for bytes in [
        r#"{"type":"probability","comparison":"less-or-equal","threshold":{"numerator":"1","denominator":"2"}}"#,
        r#"{"type":"quantile","comparison":"greater-or-equal","threshold":{"numerator":"3","denominator":"1"},"level":{"numerator":"19","denominator":"20"}}"#,
        r#"{"type":"mean-of-fraction","comparison":"greater-or-equal","threshold":{"numerator":"1","denominator":"4"}}"#,
        r#"{"type":"long-run-fraction","comparison":"less-or-equal","threshold":{"numerator":"2","denominator":"3"}}"#,
    ] {
        let bound: ProbabilisticBound = serde_json::from_str(bytes).expect("reads");
        let written = serde_json::to_string(&bound).expect("writes");
        assert_valid(&schema, &instance(&written), "ProbabilisticBound");
        let read: ProbabilisticBound = serde_json::from_str(&written).expect("reads");
        assert_eq!(read, bound);
        // The member order is the serializer's; the members are FSpec's.
        assert_eq!(instance(&written), instance(bytes));
        assert_eq!(serde_json::to_string(&read).expect("writes"), written);
    }
}
