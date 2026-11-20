// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared located-admission assertions for the claim grammar cases.

use std::collections::BTreeSet;

use fsl_cst::{CompleteCause, CompleteCode, Limits, ParsedSource, Production};
use fsl_foundation::{SourceIdentity, Span};

pub(super) fn unit(fragment: &str) -> String {
    format!("language \"ix:native\" edition \"1-draft\";\n// λ keeps byte offsets distinct\nprofile v = \"filament.temporal.complete/v1\";\n{fragment}")
}

pub(super) fn parse(text: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity::new("claim-contracts", "claim-near-misses"),
        "claims.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("bounded authored source and valid metadata")
}

pub(super) fn admitted(text: &str) -> ParsedSource {
    let parsed = parse(text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    assert_eq!(parsed.cst().render(), text.as_bytes());
    parsed
}

pub(super) fn node_texts<'s>(
    text: &'s str,
    parsed: &ParsedSource,
    kind: Production,
) -> Vec<&'s str> {
    parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == kind)
        .map(|node| &text[node.span().start..node.span().end])
        .collect()
}

pub(super) fn refused_at(fragment: &str, anchor: &str, token: &str) -> ParsedSource {
    let text = unit(fragment);
    let parsed = parse(&text);
    assert!(
        !parsed.is_admissible(),
        "accepted malformed claim: {fragment}"
    );
    assert_eq!(parsed.cst().render(), text.as_bytes());
    let diagnostics = parsed.diagnostics();
    assert_eq!(diagnostics.len(), 1, "{fragment}: {diagnostics:?}");
    let diagnostic = &diagnostics[0];
    let at = if token.is_empty() {
        text.len()
    } else {
        text.find(anchor).expect("authored anchor")
    };
    assert_eq!(
        (diagnostic.code, diagnostic.cause),
        (
            CompleteCode::InvalidSyntax,
            if token.is_empty() {
                CompleteCause::UnexpectedEnd
            } else {
                CompleteCause::UnexpectedToken
            }
        ),
        "{fragment}: {diagnostic:?}"
    );
    assert_eq!(
        diagnostic.byte_span(),
        Some(Span {
            start: at,
            end: at + token.len()
        }),
        "{fragment}: {diagnostic:?}"
    );
    assert_eq!(
        diagnostic.source,
        SourceIdentity::new("claim-contracts", "claim-near-misses")
    );
    assert_eq!(diagnostic.path, "claims.native");
    parsed
}

pub(super) fn refused(fragment: &str, anchor: &str, token: &str, expected: &str) {
    let parsed = refused_at(fragment, anchor, token);
    let diagnostic = &parsed.diagnostics()[0];
    if let Some(expected) = expected.strip_prefix("expected ") {
        fn alternatives(text: &str) -> BTreeSet<&str> {
            text.split(" or ").collect()
        }
        let actual = diagnostic
            .message
            .strip_prefix("expected ")
            .unwrap_or_else(|| panic!("{fragment}: {:?}", diagnostic.message));
        assert_eq!(alternatives(actual), alternatives(expected), "{fragment}");
    } else {
        assert_eq!(diagnostic.message, expected, "{fragment}");
    }
}
