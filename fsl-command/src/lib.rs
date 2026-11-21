// SPDX-License-Identifier: AGPL-3.0-or-later
//! Non-core request-file intake and orchestration (FR-027, FR-100, FR-287).
//!
//! Filesystem reads live here, above the qualified core. The driver consumes
//! prepared typed inputs and the core's outcome documents without depending
//! on the FSL root package.

#![forbid(unsafe_code)]

pub mod checked_handoff;
pub mod command;
pub mod emit_checked_v2;
pub mod formal_source;
pub mod replay_request;
pub mod sources;
pub mod unique_json;

pub use sources::Sources;

/// FR-286: typed refusal pairs produced by the command adapter itself.
pub const CATALOG_PAIRS: &[fsl_foundation::diagnostic::CatalogCode] = &[
    fsl_foundation::diagnostic::CatalogCode::new("ill_typed", "type-mismatch"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "wrong-role-mapping"),
];

/// Return the deduplicated union of each cause-owning crate's catalog pairs.
pub fn diagnostic_pairs() -> Vec<fsl_foundation::diagnostic::CatalogCode> {
    fn include(
        target: &mut Vec<fsl_foundation::diagnostic::CatalogCode>,
        source: &[fsl_foundation::diagnostic::CatalogCode],
    ) {
        for pair in source {
            if !target.contains(pair) {
                target.push(*pair);
            }
        }
    }

    let mut pairs = Vec::new();
    include(&mut pairs, fsl_foundation::CATALOG_PAIRS);
    include(&mut pairs, fsl_cst::CATALOG_PAIRS);
    include(&mut pairs, fsl_forms::CATALOG_PAIRS);
    include(&mut pairs, fsl_semantics::CATALOG_PAIRS);
    include(&mut pairs, fsl_route::CATALOG_PAIRS);
    include(&mut pairs, fsl_eval::CATALOG_PAIRS);
    include(&mut pairs, fsl_package::CATALOG_PAIRS);
    include(&mut pairs, fsl_replay::CATALOG_PAIRS);
    include(&mut pairs, fsl_replay::outcome::hint::SEMANTIC_VALUE_PAIRS);
    include(&mut pairs, fsl_analyze::CATALOG_PAIRS);
    include(&mut pairs, fsl_inspect::CATALOG_PAIRS);
    include(&mut pairs, CATALOG_PAIRS);
    #[cfg(feature = "quire-extraction")]
    include(&mut pairs, fsl_source::CATALOG_PAIRS);
    pairs
}

/// Build the renderer's source provision from the admitted sources in an outcome.
pub fn render_sources(sources: &Sources) -> fsl_inspect::RenderSources {
    fsl_inspect::RenderSources::from_sources(sources.as_slice().iter().cloned())
}

#[cfg(test)]
mod hint_catalog_tests {
    use fsl_foundation::diagnostic::Code;
    use fsl_replay::outcome::hint::{
        check_hint_catalog, hint_for, HintCatalogError, HintRow, HINTS,
    };
    use ix_trace_rs::trace;

    /// FR-286-AC-29/30: every row is known, valid, unique, and uses its exact key.
    #[trace("TC-3113", "FR-286-AC-29", "FR-286-AC-30")]
    #[test]
    fn command_hint_catalog_covers_typed_pairs_and_rejects_bad_rows() {
        let known = super::diagnostic_pairs();
        check_hint_catalog(HINTS, &known).expect("the published hint catalog is valid");
        assert_eq!(HINTS.len(), 23);
        for row in HINTS {
            assert_eq!(hint_for(row.code, row.cause), Some(row.text));
        }
        assert_eq!(
            hint_for(Code::OutputFailure, Some("output-failure")),
            None
        );
        assert_eq!(hint_for(Code::InvalidRequest, None), None);

        let unknown = [HintRow {
            code: Code::IllTyped,
            cause: Some("type-mistmatch"),
            text: "Correct the type mismatch.",
        }];
        assert!(matches!(
            check_hint_catalog(&unknown, &known),
            Err(HintCatalogError::UnknownPair { .. })
        ));
        let duplicate = [
            HintRow {
                code: Code::IllTyped,
                cause: Some("type-mismatch"),
                text: "Correct the type mismatch.",
            },
            HintRow {
                code: Code::IllTyped,
                cause: Some("type-mismatch"),
                text: "Resolve the type mismatch.",
            },
        ];
        assert!(matches!(
            check_hint_catalog(&duplicate, &known),
            Err(HintCatalogError::DuplicatePair { .. })
        ));
        let malformed = [HintRow {
            code: Code::IllTyped,
            cause: Some("type-mismatch"),
            text: "Fix this\nnow.",
        }];
        assert!(matches!(
            check_hint_catalog(&malformed, &known),
            Err(HintCatalogError::MalformedText { .. })
        ));
    }
}

