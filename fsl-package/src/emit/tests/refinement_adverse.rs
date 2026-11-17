// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1181 slice 6: the adverse corpus of the refinement emission. Each
//! refusal the V2 refinement stack introduced is driven from source and
//! asserted by code, catalog cause and locus.
//!
//! Covered elsewhere, with code, cause and locus: the memory-row refusals
//! (`refinement/protocol/tests.rs`, FR-432-AC-3/22), the catalog's
//! trigger-binder internal fault (`refinement/internal_list.rs`), the
//! FR-375-AC-21 reader joins (`admission_corpus.rs`).

use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::location::{
    ExpressionLocation as Location, ExpressionOwner as Origin,
};
use fsl_foundation::Code;
use fsl_semantics::check::{CheckCause, CheckRefusal, ProtocolConstruct};

use super::admission_corpus::{protocol_rows_running, rows_refinement, MAPS, PROTOCOL_ROWS};
use super::protocol_reference::not_emitted;
use super::*;

/// The refusals of `unit` over `documents`, which must not check.
fn refusals(unit: &str, documents: &[Vec<u8>]) -> Vec<CheckRefusal> {
    let packages =
        fsl_semantics::model::intake::package_input(documents.iter().map(Vec::as_slice)).unwrap();
    assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckMode::Linked, CheckingLimits::default())
        .map(|_| ())
        .expect_err("the unit refuses")
}

/// The source text a span names.
fn spelled(unit: &str, start: usize, end: usize) -> &str {
    &unit[start..end]
}

/// A concrete protocol clause that holds a construct with no node form
/// leaves the refinement's class catalog unkeyed: the clause refuses
/// `unsupported_construct`/`not-yet-implemented` at its declaration naming
/// the construct, and the refinement refuses with the same cause, located
/// at its first step row naming the clause. Each construct of QSL-1186 is a case;
/// a predicate join is written (FR-466), so it is none.
#[trace("FR-375-AC-23", "FR-092-AC-18", "TC-310", "TC-1021")]
#[test]
fn a_refinement_over_an_unwritten_clause_refuses_naming_the_construct() {
    let event =
        |name: &str, binder: &str| format!("event {name} by w as ({binder}: Boolean) {{ true }};");
    let cases: [(ProtocolConstruct, String, Vec<&str>); 3] = [
        (
            ProtocolConstruct::RepeatDecisionReadsBody,
            "sequence Main { repeat Again by w visible (e.ok) while { true } \
             sequence Body { event E by w as (e: Ack) { true }; } }"
                .to_owned(),
            vec!["step C::Impl::E -> stutter;"],
        ),
        (
            ProtocolConstruct::PostTestRepeat,
            "sequence Main { repeat Again by w visible (e.ok) while { not e.ok } \
             sequence Body { event E by w as (e: Ack) { true }; } }"
                .to_owned(),
            vec!["step C::Impl::E -> stutter;"],
        ),
        (
            ProtocolConstruct::RepeatStepsThroughRepeat,
            format!(
                "sequence Main {{ repeat Loop by w visible () while {{ true }} \
                 repeat Inner by w visible () while {{ true }} sequence Step {{ {} }} }}",
                event("T", "t")
            ),
            vec!["step C::Impl::T -> stutter;"],
        ),
    ];
    for (construct, run, extra) in cases {
        let mut rows: Vec<&str> = PROTOCOL_ROWS.to_vec();
        rows.extend(extra);
        let (unit, documents) = protocol_rows_running(&run, &rows);
        let unit = unit.replacen(
            "protocol Impl",
            "record Ack { ok: Boolean; }\n\
             protocol Impl",
            1,
        );
        let found = refusals(&unit, &documents);
        let [clause, refinement] = &found[..] else {
            panic!("{construct:?}: two refusals: {found:#?}");
        };
        let expected = not_emitted(construct);
        assert_eq!(clause.cause, expected, "{construct:?}");
        assert_eq!(
            clause.location,
            Location::root(Origin::TypeDeclaration {
                name: "protocol Impl".to_owned()
            }),
            "{construct:?}"
        );
        for refusal in [clause, refinement] {
            assert_eq!(
                (refusal.cause.code(), refusal.cause.cause()),
                (Code::UnsupportedConstruct, Some("not-yet-implemented")),
                "{construct:?}"
            );
        }
        let CheckCause::Refinement(refused) = &refinement.cause else {
            panic!("{construct:?}: {refinement:?}");
        };
        assert_eq!(refused.refinement, "R", "{construct:?}");
        assert!(
            matches!(&refused.cause, fsl_semantics::check::RefinementCause::Check(cause) if *cause == expected),
            "{construct:?}: {refused:?}"
        );
        // Located at the first step row that names the clause's classes.
        let row = spelled(&unit, refused.span.start, refused.span.end);
        assert_eq!(row, PROTOCOL_ROWS[0], "{construct:?}");
    }
}
