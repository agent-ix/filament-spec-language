// SPDX-License-Identifier: AGPL-3.0-or-later
//! The counterexample body's lossless key view. The stored FR-352 key keeps
//! its existing numeric representation outside this body.

use std::collections::BTreeSet;

use filament_contract_model::CheckedNodeId;
use serde::Serialize;

use crate::protocol_analysis::{
    AnalyzerIdentity, Bounds, CapacityPremise, ChannelPremise, ChoicePremise, DeliveryPremise,
    OrderingPremise, OverflowPremise, ProtocolAnalysisKey,
};

#[derive(Serialize)]
#[serde(transparent)]
pub(super) struct KeyWire<'a>(KeyMembers<'a>);

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum KeyMembers<'a> {
    Projection {
        definition_key: &'a str,
        role: &'a str,
        profile: &'a str,
        channels: Vec<ChannelWire<'a>>,
        choices: &'a [ChoicePremise],
        bounds: BoundsWire,
    },
    Realizability {
        definition_key: &'a str,
        strategy_profile: &'a str,
        channels: Vec<ChannelWire<'a>>,
        choices: &'a [ChoicePremise],
        role_knowledge: &'a Option<BTreeSet<String>>,
        environment: &'a Option<BTreeSet<String>>,
        bounds: BoundsWire,
        analyzer: &'a AnalyzerIdentity,
    },
}

impl<'a> KeyWire<'a> {
    pub(super) fn of(key: &'a ProtocolAnalysisKey) -> Self {
        Self(match key {
            ProtocolAnalysisKey::Projection(key) => KeyMembers::Projection {
                definition_key: &key.definition_key,
                role: &key.role,
                profile: &key.profile,
                channels: key.channels.iter().map(ChannelWire::of).collect(),
                choices: &key.choices,
                bounds: BoundsWire::of(&key.bounds),
            },
            ProtocolAnalysisKey::Realizability(key) => KeyMembers::Realizability {
                definition_key: &key.definition_key,
                strategy_profile: &key.strategy_profile,
                channels: key.premises.channels.iter().map(ChannelWire::of).collect(),
                choices: &key.premises.choices,
                role_knowledge: &key.premises.role_knowledge,
                environment: &key.premises.environment,
                bounds: BoundsWire::of(&key.bounds),
                analyzer: &key.analyzer,
            },
        })
    }
}

#[derive(Serialize)]
struct BoundsWire {
    max_depth: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_history: Option<String>,
}

impl BoundsWire {
    fn of(bounds: &Bounds) -> Self {
        Self {
            max_depth: bounds.max_depth.to_string(),
            max_history: bounds.max_history.map(|value| value.to_string()),
        }
    }
}

#[derive(Serialize)]
struct ChannelWire<'a> {
    channel: &'a CheckedNodeId,
    sender: &'a CheckedNodeId,
    receiver: &'a CheckedNodeId,
    message_type: &'a CheckedNodeId,
    ordering: &'a OrderingPremise,
    delivery: DeliveryWire,
    capacity: CapacityWire<'a>,
    overflow: &'a OverflowPremise,
}

impl<'a> ChannelWire<'a> {
    fn of(channel: &'a ChannelPremise) -> Self {
        Self {
            channel: &channel.channel,
            sender: &channel.sender,
            receiver: &channel.receiver,
            message_type: &channel.message_type,
            ordering: &channel.ordering,
            delivery: match channel.delivery {
                DeliveryPremise::Unknown => DeliveryWire::Unknown,
                DeliveryPremise::AtMostOnce => DeliveryWire::AtMostOnce,
                DeliveryPremise::AtLeastOnce => DeliveryWire::AtLeastOnce,
                DeliveryPremise::ExactlyOncePremise => DeliveryWire::ExactlyOncePremise,
                DeliveryPremise::Between { lower, upper } => DeliveryWire::Between {
                    lower: lower.to_string(),
                    upper: upper.to_string(),
                },
            },
            capacity: match &channel.capacity {
                CapacityPremise::Literal { value } => CapacityWire::Literal {
                    value: value.to_string(),
                },
                CapacityPremise::Symbolic { name, value } => CapacityWire::Symbolic {
                    name,
                    value: value.map(|value| value.to_string()),
                },
            },
            overflow: &channel.overflow,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum DeliveryWire {
    Unknown,
    AtMostOnce,
    AtLeastOnce,
    ExactlyOncePremise,
    Between { lower: String, upper: String },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum CapacityWire<'a> {
    Literal {
        value: String,
    },
    Symbolic {
        name: &'a str,
        value: Option<String>,
    },
}
