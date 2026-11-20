// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{identity, package};
use fsl_foundation::Code;
use fsl_foundation::source::provenance::RawSourceRef;
use fsl_replay::spine::DependencyInput;
use ix_trace_rs::trace;
use serde_json::json;

#[test]
#[trace("FR-110-AC-1")]
fn a_value_profile_resolves_without_inventing_a_clause_profile_selection() {
    let package = package("function three using v(): Integer pure { 3 }");
    assert_eq!(package["lock"]["profile_selections"], json!([]));
}

#[test]
#[trace("FR-110-AC-2", "FR-110-AC-3")]
fn unknown_and_wrong_role_header_selections_refuse_at_the_exact_identity_literal() {
    for (selection, cause) in [
        ("test:unknown-profile", "unsupported-selection"),
        ("ix:native", "wrong-selection-role"),
    ] {
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"{selection}\";\nfunction flag using v(): Boolean pure {{ true }}\n"
        );
        let refusal = crate::support::front_end::emitted(
            identity(),
            "profiles.native",
            text.as_bytes(),
            &Default::default(),
            &DependencyInput::default(),
        )
        .expect_err("a root profile selection must resolve in its role");
        assert_eq!(refusal.code(), Code::UnknownProfile, "{refusal:?}");
        assert_eq!(refusal.cause(), Some(cause), "{refusal:?}");
        let region = refusal.region().expect("selection literal locus");
        assert_eq!(
            region.source(),
            &RawSourceRef::new("agent-ix", "test:lowering-intake-boundaries").unwrap()
        );
        let start = usize::try_from(region.start()).unwrap();
        let end = usize::try_from(region.end()).unwrap();
        assert_eq!(&text[start..end], format!("\"{selection}\""));
    }
}
