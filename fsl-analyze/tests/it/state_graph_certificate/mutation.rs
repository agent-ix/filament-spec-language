// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 mutation registration for the explicit state-graph checker.

use super::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use fsl_certificate_mutation::{Verdict, check_typed_with_adapter};
use fsl_eval::simulation::StateKey;
use fsl_replay::StateGraphCertificate;
use ix_trace_rs::trace;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Trace: FR-1051-AC-11, FR-1051-AC-13.
#[trace("FR-1051-AC-11", "FR-1051-AC-13")]
#[test]
fn explicit_state_graph_certificate_uses_a_typed_adapter() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let (over, certificate) = reaches_two(&world);
    let original_indices = certificate
        .closure
        .iter()
        .enumerate()
        .map(|(index, key)| {
            (
                URL_SAFE_NO_PAD.encode(key.as_bytes()),
                u64::try_from(index).ok(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let check = check_typed_with_adapter(
        "R-32 explicit state graph",
        "config-reaches-two",
        &certificate,
        Vec::new(),
        encode_certificate,
        |document| decode_certificate(document, &original_indices),
        |changed| {
            if matches!(
                check(
                    &world,
                    AB,
                    "ReachesTwo",
                    over.clone(),
                    changed,
                    &ModelCheckLimits::default(),
                ),
                TemporalCertificateCheck::Accepted
            ) {
                Verdict::Accepted
            } else {
                Verdict::Rejected
            }
        },
    )
    .expect("single mutations are checked through the explicit route");
    assert!(check > 0, "the typed certificate has generated mutations");
}

fn encode_certificate(certificate: &StateGraphCertificate) -> Option<Value> {
    let closure = certificate
        .closure
        .iter()
        .map(|key| json!({"state": URL_SAFE_NO_PAD.encode(key.as_bytes())}))
        .collect::<Vec<_>>();
    let ranks = certificate
        .ranks
        .iter()
        .map(|(index, rank)| {
            let index = usize::try_from(*index).ok()?;
            let key = certificate.closure.get(index)?;
            Some(json!({"state": URL_SAFE_NO_PAD.encode(key.as_bytes()), "rank": rank}))
        })
        .collect::<Option<Vec<_>>>()?;
    let counts = certificate
        .counts
        .iter()
        .map(|(index, count, order)| {
            let index = usize::try_from(*index).ok()?;
            let key = certificate.closure.get(index)?;
            Some(json!({"state": URL_SAFE_NO_PAD.encode(key.as_bytes()), "count": count, "order": order}))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(json!({"closure": closure, "ranking": ranks, "counts": counts}))
}

fn decode_certificate(
    document: &Value,
    original_indices: &BTreeMap<String, Option<u64>>,
) -> Option<StateGraphCertificate> {
    let state_key = |row: &Value| {
        let encoded = row.get("state")?.as_str()?;
        let bytes = URL_SAFE_NO_PAD.decode(encoded).ok()?;
        Some((encoded.to_owned(), StateKey::from_canonical_bytes(bytes)))
    };
    let closure = document
        .get("closure")?
        .as_array()?
        .iter()
        .map(|row| state_key(row).map(|(_, key)| key))
        .collect::<Option<Vec<_>>>()?;
    let ranks = document
        .get("ranking")?
        .as_array()?
        .iter()
        .map(|row| {
            let (encoded, _) = state_key(row)?;
            let index = original_indices.get(&encoded).copied()??;
            let rank = u32::try_from(row.get("rank")?.as_u64()?).ok()?;
            Some((index, rank))
        })
        .collect::<Option<Vec<_>>>()?;
    let counts = document
        .get("counts")?
        .as_array()?
        .iter()
        .map(|row| {
            let (encoded, _) = state_key(row)?;
            let index = original_indices.get(&encoded).copied()??;
            let count = u8::try_from(row.get("count")?.as_u64()?).ok()?;
            let order = row.get("order")?.as_u64()?;
            Some((index, count, order))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(StateGraphCertificate {
        closure,
        ranks,
        counts,
    })
}
