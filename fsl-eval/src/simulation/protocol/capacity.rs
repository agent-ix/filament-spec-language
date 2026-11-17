// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-785: the capacity binding of a protocol subject. A queue's
//! `symbolic(<name>)` capacity takes its value only from the subject's
//! binding of that name; the package carries the name and no value.

use std::collections::BTreeMap;

use fsl_foundation::diagnostic::Code;
use fsl_semantics::check::{ChannelCapacity, CheckedProtocol};

use crate::model_subject::CapacityBinding;

/// Why a capacity binding was refused (FSpec FR-785 "Admission"), each
/// `invalid_runtime_input`.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CapacityRefusal {
    /// An entry whose name no queue of the clause uses as its symbolic
    /// capacity (`unknown-member`).
    #[error("invalid_runtime_input/unknown-member: capacity {name}")]
    UnknownName {
        /// The entry's name.
        name: String,
    },
    /// An entry whose value is not canonical decimal text of a natural
    /// number (`invalid-value`).
    #[error("invalid_runtime_input/invalid-value: capacity {name} = {value:?}")]
    InvalidValue {
        /// The entry's name.
        name: String,
        /// The value as supplied.
        value: String,
    },
    /// A name bound a second time (`duplicate-member`).
    #[error("invalid_runtime_input/duplicate-member: capacity {name}")]
    Duplicate {
        /// The repeated name.
        name: String,
    },
    /// A capacity name the clause uses with no entry, at a protocol system
    /// built without the requires-bound pre-check, such as a replayed
    /// counterexample whose subject lacks `capacities` (`missing-member`).
    #[error("invalid_runtime_input/missing-member: capacity {name}")]
    Missing {
        /// The unbound name.
        name: String,
    },
}

impl CapacityRefusal {
    /// The refusal's catalog code.
    pub fn code(&self) -> Code {
        Code::InvalidRuntimeInput
    }

    /// The refusal's catalog cause.
    pub fn cause(&self) -> &'static str {
        match self {
            Self::UnknownName { .. } => "unknown-member",
            Self::InvalidValue { .. } => "invalid-value",
            Self::Duplicate { .. } => "duplicate-member",
            Self::Missing { .. } => "missing-member",
        }
    }
}

/// A capacity name with no binding (FSpec FR-785 "Admission"): the name and
/// the qualified name of each channel whose queue uses it, in declaration
/// order.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct UnboundCapacity {
    /// The capacity name.
    pub name: String,
    /// The qualified name of each channel whose queue uses it.
    pub channels: Vec<String>,
}

/// The admitted capacity binding of one protocol clause: every entry names a
/// capacity of the clause, once.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Capacities {
    bound: BTreeMap<String, u64>,
    unbound: Vec<UnboundCapacity>,
}

impl Capacities {
    /// Admit `bindings` against the capacity names of `protocol`.
    ///
    /// # Errors
    ///
    /// [`CapacityRefusal::UnknownName`] for an entry no queue of the clause
    /// uses, [`CapacityRefusal::Duplicate`] for a name bound twice.
    pub fn admit(
        protocol: &CheckedProtocol,
        bindings: &[CapacityBinding],
    ) -> Result<Self, CapacityRefusal> {
        let mut names: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for channel in &protocol.channels {
            if let ChannelCapacity::Symbolic(name) = &channel.capacity {
                names
                    .entry(name.as_str())
                    .or_default()
                    .push(format!("{}::{}", protocol.name, channel.name));
            }
        }
        let mut bound = BTreeMap::new();
        for binding in bindings {
            if !names.contains_key(binding.name.as_str()) {
                return Err(CapacityRefusal::UnknownName {
                    name: binding.name.clone(),
                });
            }
            if bound.insert(binding.name.clone(), binding.value).is_some() {
                return Err(CapacityRefusal::Duplicate {
                    name: binding.name.clone(),
                });
            }
        }
        let unbound = names
            .into_iter()
            .filter(|(name, _)| !bound.contains_key(*name))
            .map(|(name, channels)| UnboundCapacity {
                name: name.to_owned(),
                channels,
            })
            .collect();
        Ok(Self { bound, unbound })
    }

    /// The bound value of `name`.
    pub fn value(&self, name: &str) -> Option<u64> {
        self.bound.get(name).copied()
    }

    /// Every capacity name with no binding, in name order.
    pub fn unbound(&self) -> &[UnboundCapacity] {
        &self.unbound
    }

    /// The capacity of `capacity`: the literal, or the bound value of the
    /// name.
    ///
    /// # Errors
    ///
    /// [`CapacityRefusal::Missing`] for a name with no binding.
    pub(crate) fn resolve(&self, capacity: &ChannelCapacity) -> Result<u64, CapacityRefusal> {
        match capacity {
            ChannelCapacity::Literal(value) => Ok(*value),
            ChannelCapacity::Symbolic(name) => self
                .value(name)
                .ok_or_else(|| CapacityRefusal::Missing { name: name.clone() }),
        }
    }
}

/// `text` read as canonical decimal text of a natural number: digits only,
/// no sign and no leading zero (FSpec FR-785 "The binding").
///
/// # Errors
///
/// [`CapacityRefusal::InvalidValue`] for any other text, or a value past
/// `u64`.
#[ix_trace_rs::string_edge]
pub fn capacity_value(name: &str, text: &str) -> Result<u64, CapacityRefusal> {
    let canonical = !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    canonical
        .then(|| text.parse::<u64>().ok())
        .flatten()
        .ok_or_else(|| CapacityRefusal::InvalidValue {
            name: name.to_owned(),
            value: text.to_owned(),
        })
}
