// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hybrid declaration shape and member ordering, independent of solver support.
use super::time_declaration::{accepts, refuses_at};
use fsl_cst::Production;
use ix_trace_rs::trace;

/// Provenance: QSL-2076; hybrid-decl, hybrid-initial, hybrid-mode and equation.
#[test]
#[trace("FSpec-FR-339-AC-1")]
fn hybrid_declaration_refuses_near_misses() {
    accepts("hybrid Vessel using p { initial { mode Rising when { volume = 0 }; } mode Rising invariant { volume <= 12 } flow { volume' = 2; } transition to Waiting when { volume = 12 } reset { volume = 12 }; mode Waiting invariant { true } flow { volume' = 0; } }", Production::HybridDeclaration);
    for (bad, token, width, expected) in [
        ("hybrid Vessel using p { mode Rising invariant { true } flow { } }", "} }", 1, "expected identifier"),
        ("hybrid Vessel using p { mode Rising invariant { true } flow { volume' 2; } }", "2", 1, "expected `=`"),
        ("hybrid Vessel using p { initial { mode Rising when { }; } mode Rising invariant { true } flow { volume' = 2; } }", "};", 1, "expected an expression"),
        ("hybrid Vessel using p { initial { mode Rising when { true }; } initial { mode Rising when { true }; } mode Rising invariant { true } flow { volume' = 2; } }", "initial", 7, "expected `mode`"),
        ("hybrid Vessel using p { mode Rising flow { volume' = 2; } invariant { true } }", "flow", 4, "expected `invariant`"),
    ] {
        refuses_at(bad, token, width, expected);
    }
}
