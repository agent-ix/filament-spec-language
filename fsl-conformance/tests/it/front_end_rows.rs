// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1388 and TC-1389: the `check` and `package` rows.

use filament_semantic_value::catalog::Category;
use fsl_conformance::{
    run_case, CaseInput, FrontEndInput, FrontEndLimit, MatrixCase, OutcomeKind, Verb,
};
use ix_trace_rs::trace;

use crate::support::{case_of, diagnostic, outcome_of, written};

/// A cancel before the call is incomplete at no stage, with the cancel's
/// cause as the diagnostic's.
fn assert_cancelled(verb: Verb, kind: OutcomeKind, cause: &str, operation: &str) {
    let outcome = outcome_of(verb, kind);
    assert_eq!(outcome.exit_code(), 22, "{verb:?}/{kind:?}");
    let document = written(&outcome);
    assert_eq!(document["operation"], operation);
    assert_eq!(document["category"], "incomplete");
    assert_eq!(document["last_stage"], serde_json::Value::Null);
    assert_eq!(
        diagnostic(&outcome),
        ("cancelled".to_owned(), Some(cause.to_owned()))
    );
}

/// FR-564-AC-1: each of `check`'s six applicable cells returns its kind.
#[trace("TC-1388", "FR-564-AC-1")]
#[test]
fn check_returns_each_applicable_kind() {
    let success = outcome_of(Verb::Check, OutcomeKind::Success);
    assert_eq!(success.exit_code(), 0);
    assert_eq!(written(&success)["category"], "success");

    let refusal = outcome_of(Verb::Check, OutcomeKind::Refusal);
    assert_eq!(refusal.exit_code(), 20);
    assert_eq!(diagnostic(&refusal).0, "ill_typed");

    let unsupported = outcome_of(Verb::Check, OutcomeKind::Unsupported);
    assert_eq!(unsupported.exit_code(), 21);
    assert_eq!(
        diagnostic(&unsupported),
        (
            "unsupported_construct".to_owned(),
            Some("declaration-form".to_owned())
        )
    );

    assert_cancelled(Verb::Check, OutcomeKind::Timeout, "deadline", "check");
    assert_cancelled(Verb::Check, OutcomeKind::Cancellation, "requested", "check");

    let incomplete = outcome_of(Verb::Check, OutcomeKind::Incomplete);
    assert_eq!(incomplete.exit_code(), 22);
    assert_eq!(written(&incomplete)["category"], "incomplete");
    assert_eq!(diagnostic(&incomplete).0, "stage_limit_exceeded");
    assert!(
        crate::support::document(&incomplete).diagnostics()[0]
            .message()
            .contains("s3.nodes"),
        "{:?}",
        crate::support::document(&incomplete).diagnostics()
    );
}

/// FR-564-AC-2: each of `package`'s five applicable cells returns its kind.
#[trace("TC-1389", "FR-564-AC-2")]
#[test]
fn package_returns_each_applicable_kind() {
    let success = outcome_of(Verb::Package, OutcomeKind::Success);
    assert_eq!(success.exit_code(), 0);
    assert_eq!(written(&success)["operation"], "package");

    let unsupported = outcome_of(Verb::Package, OutcomeKind::Unsupported);
    assert_eq!(unsupported.exit_code(), 21);
    assert_eq!(
        diagnostic(&unsupported),
        (
            "unsupported_construct".to_owned(),
            Some("not-yet-implemented".to_owned())
        )
    );
    assert_eq!(written(&unsupported)["last_stage"], "S4");
    assert!(crate::support::document(&unsupported).diagnostics()[0]
        .message()
        .contains("workload `Both`"));
    assert_eq!(
        crate::support::document(&unsupported).category(),
        Category::Unsupported
    );

    assert_cancelled(Verb::Package, OutcomeKind::Timeout, "deadline", "package");
    assert_cancelled(
        Verb::Package,
        OutcomeKind::Cancellation,
        "requested",
        "package",
    );

    let incomplete = outcome_of(Verb::Package, OutcomeKind::Incomplete);
    assert_eq!(incomplete.exit_code(), 22);
    assert_eq!(diagnostic(&incomplete).0, "stage_limit_exceeded");
    assert!(crate::support::document(&incomplete).diagnostics()[0]
        .message()
        .contains("emit.output_bytes"));
}

/// The incomplete cells sit exactly one below the counter their input
/// reaches: one above, the same operation succeeds.
#[trace("TC-1388", "FR-564-AC-1")]
#[trace("TC-1389", "FR-564-AC-2")]
#[test]
fn the_incomplete_cells_are_one_below_the_counter_their_input_reaches() {
    let at_counter = |verb, limit_at_counter: fn(u64) -> FrontEndLimit| {
        let case = case_of(verb, OutcomeKind::Incomplete);
        let (CaseInput::Check(input) | CaseInput::Package(input)) = case.input else {
            panic!("a front-end case");
        };
        let (FrontEndLimit::S3Nodes(below) | FrontEndLimit::OutputBytes(below)) = input.limit
        else {
            panic!("a limited case");
        };
        if verb == Verb::Package {
            use fsl_replay::spine::{
                check, package, parse, select, DependencyInput, PackageLimits, ParseRequest,
                SpineLimits,
            };
            let cancel = filament_exact::Cancel::new();
            let limits = SpineLimits::default();
            let source = fsl_foundation::SourceIdentity::new(
                case.parameters.source_authority,
                case.parameters.source_identity,
            );
            let parsed = parse(
                &ParseRequest {
                    source: &source,
                    path: input.unit.file_name,
                    bytes: input.unit.bytes,
                },
                limits.source,
                &cancel,
            )
            .expect("the package source parses")
            .into_value();
            let packages = fsl_semantics::model::intake::package_input(
                input.packages.iter().map(|input| input.bytes),
            )
            .expect("the package models admit");
            let models = select(&parsed, &packages, limits.model, &cancel)
                .expect("the package models select")
                .into_value();
            let checked = check(
                &parsed,
                case.parameters.package_name.to_owned(),
                &models,
                &DependencyInput::default(),
                limits,
                &cancel,
            )
            .expect("the package checks")
            .into_value();
            let emitted = package(&checked, PackageLimits::default(), &cancel)
                .expect("the package emits without a byte limit")
                .into_value();
            // Independence comes from the fixed literal boundary below and
            // the paired real `run_case` calls: `emit.output_bytes` limited
            // to `below` exits 22 with `stage_limit_exceeded`, while `below +
            // 1` exits 0. This read/re-encode observes the canonical size,
            // but uses the emitter's own bytes and is not an independent
            // check by itself.
            let document = quire_canonical::read(emitted.package().bytes(), u64::MAX)
                .expect("the emitted package reads back");
            let metered =
                quire_canonical::to_vec(&document, quire_canonical::Limits::new(u64::MAX))
                    .expect("the package re-encodes");
            let metered = u64::try_from(metered.len()).expect("the metered count fits u64");
            assert_eq!(
                below + 1,
                metered,
                "COMPILE_OUTPUT_BYTES_BELOW (fsl-conformance/src/cases.rs) is stale: the canonical \
                 encoder meters the COMPILE package at {metered} bytes; re-measure that single \
                 constant as {metered} - 1"
            );
        }
        let limited = run_case(&case).expect("the limited case runs");
        assert_eq!(
            limited.exit_code(),
            22,
            "{verb:?} must reach its stated limit"
        );
        assert_eq!(diagnostic(&limited).0, "stage_limit_exceeded");
        let wrap = |input| match case.input {
            CaseInput::Check(_) => CaseInput::Check(input),
            _ => CaseInput::Package(input),
        };
        let counter = MatrixCase {
            input: wrap(FrontEndInput {
                limit: limit_at_counter(below + 1),
                ..input
            }),
            ..case
        };
        let outcome = run_case(&counter).expect("the case runs");
        assert_eq!(
            outcome.exit_code(),
            0,
            "{verb:?}: {:?}",
            crate::support::document(&outcome).diagnostics()
        );
    };
    at_counter(Verb::Check, FrontEndLimit::S3Nodes);
    at_counter(Verb::Package, FrontEndLimit::OutputBytes);
}
