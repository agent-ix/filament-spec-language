// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared timed fixture helpers used by analyzer integration test targets.

use serde_json::json;

pub(crate) fn rpc_unit(t: u32, claim: &str) -> String {
    rpc_unit_with(t, ">= 1", ">=", claim)
}

pub(crate) fn rpc_unit_with(t: u32, reply: &str, timeout: &str, claim: &str) -> String {
    use crate::timed_world::{unit_text, DENSE, PACKAGE};
    let body = format!(
        "{DENSE}pre SendIdle using v on M::Call::send {{ self.phase = 0 }}\n\
         post Send using v on M::Call::send {{ self.phase = 1 and self.inflight and not self.late and self.x = 0 ms }}\n\
         pre Reply using v on M::Call::reply {{ self.inflight and self.x {reply} ms }}\n\
         post Replied using v on M::Call::reply {{ not self.inflight and self.late = (pre(self.phase) = 3) and (pre(self.phase) = 1 implies self.phase = 2) and (pre(self.phase) != 1 implies self.phase = pre(self.phase)) }}\n\
         pre Timeout using v on M::Call::timeout {{ self.phase = 1 and self.x {timeout} {t} ms }}\n\
         post TimedOut using v on M::Call::timeout {{ self.phase = 3 }}\n\
         pre Reset using v on M::Call::reset {{ not self.inflight and (self.phase = 2 or self.phase = 3) }}\n\
         post ResetIdle using v on M::Call::reset {{ self.phase = 0 }}\n\
         time invariant ServerDeadline when {{ self.inflight }} {{ self.x <= 3 ms }}\n\
         time invariant ClientDeadline when {{ self.phase = 1 }} {{ self.x <= {t} ms }}\n\
         {claim}"
    );
    unit_text(PACKAGE, crate::zone_search::TIMED, &body)
}

pub(crate) fn rpc_snapshot() -> Vec<u8> {
    let package = crate::timed_world::PACKAGE;
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "s0"},
        "observation": "current", "model": {"identity": package},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": format!("ix://{package}/calls"), "complete": true,
            "objects": [{"key": "c", "type": format!("ix://{package}/Call"),
                "fields": {"phase": {"integer": "0"}, "inflight": {"boolean": false},
                    "late": {"boolean": false}, "retried": {"boolean": false}}}]}],
    })
    .to_string()
    .into_bytes()
}
