// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-331's spelling of a written certificate rejection, shared by split
//! integration test binaries.

use fsl_eval::simulation::StateKey;
use fsl_foundation::BackendId;
use fsl_replay::{
    Operation, OutcomeDocument, OutcomeItem, OutcomeStage, RequestIndex, TerminalRecord,
    TerminalValue,
};
use serde_json::json;

/// The `cause` the outcome writer writes for an item settled `value`, under
/// FR-331's member name `type` for the writer's `kind`.
pub(crate) fn written_cause(value: TerminalValue) -> serde_json::Value {
    let TerminalValue::Inconclusive(cause) = value else {
        panic!("a rejected certificate settles inconclusive: {value:?}");
    };
    let item = OutcomeItem::from_terminal(&TerminalRecord::inconclusive(
        RequestIndex::new(0),
        BackendId::new("en-1"),
        cause,
    ));
    let bytes = OutcomeDocument::settled(Operation::Analyze, Some(OutcomeStage::S6c), vec![item])
        .to_bytes()
        .expect("the outcome encodes");
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("the outcome is JSON");
    let mut cause = document["items"][0]["cause"].clone();
    let kind = cause
        .as_object_mut()
        .and_then(|members| members.remove("kind"))
        .expect("the cause names its kind");
    cause["type"] = kind;
    cause
}

/// The State locus FSpec FR-331 spells for the state key `key`.
pub(crate) fn state_locus(key: &StateKey) -> serde_json::Value {
    use base64::Engine as _;
    json!({"key": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.as_bytes())})
}
