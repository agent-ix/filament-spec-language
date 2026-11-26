// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 mutation registration for the explicit state-graph checker.

use super::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use fsl_certificate_mutation::{
    AllowedMutation, MutationId, MutationKind, Verdict, check_typed_with_adapter,
};
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

    let mut allowed = vec![
        AllowedMutation {
            id: MutationId {
                member: "$/ranking/1".into(),
                kind: MutationKind::RowDropped,
            },
            reason: "For a possible claim, ranks are required only for initial states and states on a d-1 successor chain; this row is a non-initial state no such chain uses, so the certificate remains valid.".into(),
        },
        AllowedMutation {
            id: MutationId {
                member: "$/ranking/2".into(),
                kind: MutationKind::RowDropped,
            },
            reason: "For a possible claim, the checker requires ranks for initial states and checks a d-1 successor for each ranked state; this non-initial row is not needed to satisfy those remaining links, so dropping it leaves the proof valid.".into(),
        },
    ];
    let ranks_by_state = ranks_of(&certificate);
    let successor_map = successors(&world, AB);
    for (row, (index, rank)) in certificate.ranks.iter().enumerate() {
        let state_index = usize::try_from(*index).expect("certificate index fits usize");
        let state = &certificate.closure[state_index];
        let has_same_rank_successor = successor_map.get(state).is_some_and(|next| {
            next.iter()
                .any(|successor| ranks_by_state.get(successor) == Some(rank))
        });
        let parent_links_remain = rank.checked_add(1).is_some_and(|new_rank| {
            ranks_by_state.iter().all(|(source, source_rank)| {
                if source == state || *source_rank != new_rank {
                    return true;
                }
                successor_map.get(source).is_some_and(|next| {
                    next.iter().any(|successor| {
                        successor != state && ranks_by_state.get(successor) == Some(rank)
                    })
                })
            })
        });
        if has_same_rank_successor && parent_links_remain {
            allowed.push(AllowedMutation {
                id: MutationId {
                    member: format!("$/ranking/{row}/rank"),
                    kind: MutationKind::RankPlus,
                },
                reason: "Increasing this row's rank preserves the required descent because it has a successor at the new d-1 rank; the unchanged rank-zero endpoint still satisfies the target predicate.".into(),
            });
        }
    }

    let mutations = check_typed_with_adapter(
        "R-32 explicit state graph",
        "config-reaches-two",
        &certificate,
        allowed,
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
    assert!(
        mutations > 0,
        "the typed certificate has generated mutations"
    );

    let initial = config(&world, 0, 0);
    let initial_index = certificate
        .closure
        .iter()
        .position(|key| key == &initial)
        .expect("initial state is in the closure");
    let mut without_initial_rank = certificate.clone();
    without_initial_rank
        .ranks
        .retain(|(index, _)| usize::try_from(*index).ok() != Some(initial_index));
    assert!(
        matches!(
            check(
                &world,
                AB,
                "ReachesTwo",
                over,
                &without_initial_rank,
                &ModelCheckLimits::default()
            ),
            TemporalCertificateCheck::Rejected(CertificateRejection {
                rule: CertificateRule::RankMissing,
                ..
            })
        ),
        "dropping the initial state rank must be rejected as RankMissing"
    );
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
            let rank = row
                .get("rank")?
                .as_i64()
                .and_then(|rank| u32::try_from(rank).ok())
                .unwrap_or(u32::MAX);
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
