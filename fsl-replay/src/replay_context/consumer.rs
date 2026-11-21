// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit consumer accounting, separate from immutable producer context.

use std::collections::BTreeMap;

use filament_exact::ScalarLimits;
use fsl_foundation::{AccountingSetting, SettingName};
use serde::{Deserialize, Serialize};

use super::ContextError;
use crate::{AccountingLimits, ReplayLimits};

/// Limits explicitly selected by the independent replay consumer.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectiveLimits {
    /// Consumer ceiling on retained/reconstructed replay inputs.
    pub replay_input_bytes: String,
    /// Complete consumer accounting-counter bounds.
    pub accounting_limits: BTreeMap<String, String>,
    /// Consumer accounting-setting overrides, and no other stage settings.
    pub stage_limits: BTreeMap<String, String>,
}

/// Validated consumer limits; construction never reads proving limits.
#[derive(Clone, Debug)]
pub struct ConsumerLimits {
    replay: ReplayLimits,
    accounting: ScalarLimits,
}

/// Read canonical unsigned decimal text without accepting another spelling.
pub fn unsigned(text: &str, pointer: &str) -> Result<u64, ContextError> {
    let value: u64 = text
        .parse()
        .map_err(|_| ContextError::invalid(pointer, "expected unsigned decimal string"))?;
    if value.to_string() != text {
        return Err(ContextError::invalid(
            pointer,
            "noncanonical unsigned decimal",
        ));
    }
    Ok(value)
}

/// Read a complete accounting object in the original native mapping.
pub fn accounting_from_wire(
    values: &BTreeMap<String, String>,
    pointer: &str,
) -> Result<ScalarLimits, ContextError> {
    if values.len() != AccountingSetting::ALL.len() {
        return Err(ContextError::invalid(
            pointer,
            "accounting object must contain every native counter exactly once",
        ));
    }
    let mut limits = AccountingLimits::default();
    for setting in AccountingSetting::ALL {
        let name = setting.counter();
        let value = values.get(name).ok_or_else(|| {
            ContextError::invalid(format!("{pointer}/{name}"), "missing accounting counter")
        })?;
        limits.set_bound(setting, unsigned(value, &format!("{pointer}/{name}"))?);
    }
    Ok(limits.0)
}

impl EffectiveLimits {
    /// Admit consumer-owned bounds independently of the producer request.
    pub fn admit(&self) -> Result<ConsumerLimits, ContextError> {
        let bytes = unsigned(
            &self.replay_input_bytes,
            "/effective_limits/replay_input_bytes",
        )?;
        let mut accounting = AccountingLimits(accounting_from_wire(
            &self.accounting_limits,
            "/effective_limits/accounting_limits",
        )?);
        for (name, value) in &self.stage_limits {
            let Some(SettingName::Accounting(setting)) = SettingName::from_name(name) else {
                return Err(ContextError::invalid(
                    format!("/effective_limits/stage_limits/{name}"),
                    "consumer stage entry is not an accounting setting",
                ));
            };
            accounting.set_bound(
                setting,
                unsigned(value, &format!("/effective_limits/stage_limits/{name}"))?,
            );
        }
        Ok(ConsumerLimits {
            replay: ReplayLimits::default().with_input_bytes(bytes),
            accounting: accounting.0,
        })
    }
}

impl ConsumerLimits {
    /// The explicit native replay read bound.
    pub fn replay(&self) -> ReplayLimits {
        self.replay
    }

    /// Consumer effective accounting after its own stage entries.
    pub fn accounting(&self) -> ScalarLimits {
        self.accounting
    }
}
