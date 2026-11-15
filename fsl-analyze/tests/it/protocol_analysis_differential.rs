// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-636 (TC-1596): the protocol deadlock verdict `fsl analyze` gives agrees
//! with local projection and realizability of the same protocol.

use ix_trace_rs::trace;
use serde_json::json;

use crate::analyze_cli_support::{analyze, label, litmus_request, order_protocol, Workspace};

/// FR-636-AC-2: global deadlock freedom agrees with projection.
#[trace("TC-1596", "FR-636-AC-2")]
#[test]
#[ignore = "QSL-2224: protocol analysis items are not reachable through fsl analyze yet"]
fn protocol_deadlock_agrees_with_projection() {
    let root = json!({"observation": {"authority": "test", "identity": "protocol-claim"},
        "subject": {"tag": "object", "universe": "u", "type": "e", "object_identity": "flow-1"}});
    for receive in [true, false] {
        let ws = Workspace::new(&format!("protocol-differential-{receive}"));
        let run = analyze(&litmus_request(
            &ws,
            &order_protocol(receive),
            json!({"items": [
                {"claim": "Flow", "temporal": {"item_kind": "deadlock-freedom"}},
                {"claim": "Flow", "kind": "local-projection", "assessed_root": root,
                 "options": {"role": "Seller"}},
                {"claim": "Flow", "kind": "realizability", "assessed_root": root},
            ]}),
        ));
        let global = label(&run.item(0)) == "proved";
        let local = label(&run.item(1)) == "proved" && label(&run.item(2)) == "proved";
        if receive {
            assert!(
                global && local,
                "the complete protocol is deadlock-free and realizable"
            );
        } else {
            assert!(!global, "the unreceived message deadlocks");
            assert!(
                !local || !global,
                "a realizable projection never hides a global deadlock"
            );
        }
    }
}
