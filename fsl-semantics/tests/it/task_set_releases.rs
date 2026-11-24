// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-249 (FSpec FR-419): S3 checks the `release S.t on M::o;`
//! declarations of a unit against its task sets and the operations of its
//! models, and records each on the checked task sets. The lowering itself
//! (FR-249-AC-1 and AC-2) is not under test here. Run over TC-458's
//! `ConfigVersion` fixture package with a second operation, `command`
//! (`crate::model_operations`).

use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::checking::CheckingLimits;
use fsl_foundation::diagnostic::Code;
use fsl_semantics::check::{CheckCause, CheckRefusal, CheckedGraph, TemporalCause, TimedConstruct};
use ix_trace_rs::trace;
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_profiles, ambiguous_operation_document,
    attempt_update_modifies_version_number, config_unit_with_profiles,
    config_version_document_with_operations, empty_frame, operation,
};

const PROFILES: &str = "profile t = \"filament.temporal.complete/v1\";\n";

const SET: &str = "dimension Time;\n
     unit s : Time = rational(1, 1);\n
     unit ms : Time = rational(1, 1000) * s;\n
     taskset Ctl {\n
     task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n
     task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n
     }\n";

fn document() -> Vec<u8> {
    config_version_document_with_operations(vec![
        attempt_update_modifies_version_number(),
        operation("command", json!([]), None, empty_frame()),
    ])
}

fn unit_text(declarations: &str) -> String {
    format!("{SET}{declarations}\n")
}

fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    admit_and_assemble_with_profiles(&document(), PROFILES, &unit_text(declarations))
        .expect("S2 keeps the release declarations")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
}

fn checked(declarations: &str) -> CheckedGraph {
    check(declarations).unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"))
}

/// The code, tag and span text of each refusal of `declarations`.
fn refused(declarations: &str) -> Vec<(Code, &'static str, String)> {
    let text = config_unit_with_profiles(&document(), PROFILES, &unit_text(declarations)).0;
    check(declarations)
        .expect_err("S3 refuses")
        .iter()
        .map(|refusal| {
            let CheckCause::Temporal(cause) = refusal.cause.as_ref() else {
                panic!("a temporal refusal: {refusal:?}");
            };
            let span = cause.span();
            (
                cause.code(),
                cause.tag(),
                text[span.start..span.end].to_owned(),
            )
        })
        .collect()
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn a_release_is_recorded_on_its_operation_and_a_second_task_is_allowed() {
    let graph = checked(
        "release Ctl.a on ConfigVersion::attemptUpdate;\n
         release Ctl.b on ConfigVersion::attemptUpdate;",
    );
    let sets = graph.task_sets();
    let [first, second] = sets.releases() else {
        panic!("two releases: {:?}", sets.releases());
    };
    assert_eq!((first.task_set(), first.task()), ("Ctl", "a"));
    assert_eq!((second.task_set(), second.task()), ("Ctl", "b"));
    assert_eq!(first.operation().name(), "attemptUpdate");
    let released: Vec<_> = sets
        .released_by(first.operation().key())
        .map(|release| release.task())
        .collect();
    assert_eq!(released, ["a", "b"]);
}

#[trace("TC-704", "FR-249-AC-13")]
#[test]
fn a_mode_switch_is_checked_in_source_order_and_requires_a_release_to_back_a_claim() {
    let graph = checked(
        "mode Ctl hi on ConfigVersion::attemptUpdate;\n\
         mode Ctl hi on ConfigVersion::command;",
    );
    let [first, second] = graph.task_sets().mode_switches() else {
        panic!("two mode switches: {:?}", graph.task_sets().mode_switches());
    };
    assert_eq!(first.task_set(), "Ctl");
    assert_eq!(first.operation().name(), "attemptUpdate");
    assert_eq!(second.operation().name(), "command");

    assert_eq!(
        refused("mode Nope hi on ConfigVersion::attemptUpdate;"),
        [(Code::MissingDeclaration, "missing-name", "Nope".to_owned())]
    );
    assert_eq!(
        refused("mode Ctl hi on ConfigVersion::nope;"),
        [(
            Code::MissingDeclaration,
            "missing-name",
            "ConfigVersion::nope".to_owned()
        )]
    );
    assert_eq!(
        refused(
            "mode Ctl hi on ConfigVersion::attemptUpdate;\n\
             mode Ctl hi on ConfigVersion::attemptUpdate;"
        ),
        [(
            Code::ConflictingDeclaration,
            "duplicate-declaration",
            "mode Ctl hi on ConfigVersion::attemptUpdate;".to_owned()
        )]
    );

    let unbacked = checked(
        "mode Ctl hi on ConfigVersion::attemptUpdate;\n\
         schedulable CtlFp on Ctl under fixed_priority;",
    );
    let claim = unbacked
        .task_sets()
        .claims()
        .iter()
        .find(|claim| claim.task_set == "Ctl")
        .and_then(|claim| claim.occurrence())
        .expect("a keyed schedulability claim");
    assert!(!unbacked.requirements()[&claim]
        .requirements()
        .task_product());
}

#[trace("TC-704", "FR-249-AC-7")]
#[test]
fn only_a_task_set_with_a_checked_release_requires_a_task_product() {
    let released = checked(
        "schedulable CtlFp on Ctl under fixed_priority;\n\
         release Ctl.a on ConfigVersion::attemptUpdate;",
    );
    let claim = released
        .task_sets()
        .claims()
        .iter()
        .find(|claim| claim.task_set == "Ctl")
        .and_then(|claim| claim.occurrence())
        .expect("a keyed schedulability claim");
    assert!(released.requirements()[&claim]
        .requirements()
        .task_product());

    let unreleased = checked("schedulable CtlFp on Ctl under fixed_priority;");
    let claim = unreleased
        .task_sets()
        .claims()
        .iter()
        .find(|claim| claim.task_set == "Ctl")
        .and_then(|claim| claim.occurrence())
        .expect("a keyed schedulability claim");
    assert!(!unreleased.requirements()[&claim]
        .requirements()
        .task_product());
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn an_operation_that_releases_nothing_records_no_release() {
    let graph = checked("release Ctl.a on ConfigVersion::attemptUpdate;");
    let [release] = graph.task_sets().releases() else {
        panic!("one release");
    };
    let other = check("release Ctl.a on ConfigVersion::command;")
        .expect("command releases a")
        .task_sets()
        .releases()
        .first()
        .map(|release| release.operation().key().clone())
        .expect("a release");
    assert_ne!(&other, release.operation().key());
    assert_eq!(graph.task_sets().released_by(&other).count(), 0);
    // A unit with a task set and no release records none.
    let bare = check("").expect("a unit with a task set and no release checks");
    assert!(bare.task_sets().releases().is_empty());
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn a_second_release_of_one_task_on_one_operation_refuses_at_the_second() {
    let reported = refused(
        "release Ctl.a on ConfigVersion::attemptUpdate;\n
         release Ctl.a on ConfigVersion::attemptUpdate;",
    );
    assert_eq!(
        reported,
        [(
            Code::ConflictingDeclaration,
            "duplicate-declaration",
            "release Ctl.a on ConfigVersion::attemptUpdate;".to_owned()
        )]
    );
    // The same task on another operation is not a duplicate.
    checked(
        "release Ctl.a on ConfigVersion::attemptUpdate;\n
         release Ctl.a on ConfigVersion::command;",
    );
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn an_unresolved_task_set_task_or_operation_refuses_at_the_name() {
    for (declaration, at) in [
        ("release Nope.a on ConfigVersion::attemptUpdate;", "Nope"),
        ("release Ctl.z on ConfigVersion::attemptUpdate;", "z"),
        (
            "release Ctl.a on ConfigVersion::nope;",
            "ConfigVersion::nope",
        ),
    ] {
        assert_eq!(
            refused(declaration),
            [(Code::MissingDeclaration, "missing-name", at.to_owned())],
            "{declaration}"
        );
    }
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn a_task_of_another_task_set_is_not_a_task_of_the_named_one() {
    let other = "taskset Other {\n
         task z periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n
         }\n";
    let declarations = format!("{other}release Ctl.z on ConfigVersion::attemptUpdate;");
    assert_eq!(
        refused(&declarations),
        [(Code::MissingDeclaration, "missing-name", "z".to_owned())]
    );
    checked(&format!(
        "{other}release Other.z on ConfigVersion::attemptUpdate;"
    ));
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn every_unresolved_name_of_a_release_is_reported() {
    let reported = refused("release Nope.a on ConfigVersion::nope;");
    assert_eq!(
        reported,
        [
            (Code::MissingDeclaration, "missing-name", "Nope".to_owned()),
            (
                Code::MissingDeclaration,
                "missing-name",
                "ConfigVersion::nope".to_owned()
            ),
        ]
    );
}

#[trace("TC-704", "FR-230-AC-7")]
#[test]
fn a_release_is_refused_unless_the_timed_facet_is_selected() {
    let text = format!("{SET}release Ctl.a on ConfigVersion::attemptUpdate;\n");
    let refusals = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"filament.value.complete/v1\";\n",
        &text,
    )
    .expect("S2 keeps the forms")
    .check(
        CheckMode::Linked,
        CheckingLimits::default(),
        &crate::reward_evaluator::ZeroRewardEvaluator,
    )
    .expect_err("the facet gate refuses");
    let found = refusals.iter().any(|refusal| {
        matches!(
            refusal.cause.as_ref(),
            CheckCause::Temporal(cause) if matches!(
                &**cause,
                TemporalCause::TimedFacetRequired { construct: TimedConstruct::Release, .. }
            ) && cause.code() == Code::UnsupportedConstruct && cause.tag() == "declaration-form"
        )
    });
    assert!(found, "{refusals:?}");
}

#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn releases_check_over_a_time_dense_model() {
    let graph = checked(
        "time Config dense unit ms;\n
         release Ctl.a on ConfigVersion::attemptUpdate;\n
         release Ctl.b on ConfigVersion::command;",
    );
    assert_eq!(graph.task_sets().releases().len(), 2);
}

/// `Both` inherits `dup` from two unrelated types, so `Both::dup` names no one
/// operation: `ambiguous_declaration`/`ambiguous-name` at the operation name.
#[trace("TC-704", "FR-249-AC-6")]
#[test]
fn an_operation_inherited_from_two_unrelated_types_is_ambiguous() {
    let declarations = "release Ctl.a on Config::Both::dup;";
    let text = unit_text(declarations);
    let refusals =
        admit_and_assemble_with_profiles(&ambiguous_operation_document(), PROFILES, &text)
            .expect("S2 keeps the release")
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &crate::reward_evaluator::ZeroRewardEvaluator,
            )
            .expect_err("the operation is ambiguous");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::Temporal(cause) = refusal.cause.as_ref() else {
        panic!("a temporal refusal: {refusal:?}");
    };
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::AmbiguousDeclaration, "ambiguous-name")
    );
}

/// FR-249-AC-10: a release of a task whose period is zero or negative is
/// refused at check by the task set's own `invalid-task-set`, at the period,
/// and the release adds no `missing-name`.
#[trace("TC-3008", "FR-249-AC-10")]
#[test]
fn a_release_of_a_non_positive_period_task_refuses_at_the_period() {
    for period in ["0 ms", "-4 ms"] {
        let reported = refused(&format!(
            "taskset Bad {{ task p periodic {period} deadline 4 ms wcet 1 ms priority 1 lo; }}\n
             release Bad.p on ConfigVersion::attemptUpdate;"
        ));
        let [(code, tag, at)] = &reported[..] else {
            panic!("one refusal for {period}: {reported:?}");
        };
        assert_eq!((*code, *tag), (Code::InvalidTimedModel, "invalid-task-set"));
        assert!(
            at.ends_with(period.trim_start_matches('-')),
            "{period}: at {at:?}"
        );
    }
}

#[trace("TC-704", "FR-249-AC-13")]
#[test]
fn mode_switches_are_recorded_in_source_order_beside_the_releases() {
    let graph = checked(
        "release Ctl.a on ConfigVersion::attemptUpdate;\n
         mode Ctl hi on ConfigVersion::command;\n
         mode Ctl hi on ConfigVersion::attemptUpdate;",
    );
    let sets = graph.task_sets();
    assert_eq!(sets.releases().len(), 1);
    let [first, second] = sets.mode_switches() else {
        panic!("two mode switches: {:?}", sets.mode_switches());
    };
    assert_eq!(
        (first.task_set(), first.operation().name()),
        ("Ctl", "command")
    );
    assert_eq!(
        (second.task_set(), second.operation().name()),
        ("Ctl", "attemptUpdate")
    );
    assert!(first.span().start < second.span().start);
    // A unit with no `mode` declaration records none.
    assert!(checked("").task_sets().mode_switches().is_empty());
}

#[trace("TC-704", "FR-249-AC-13")]
#[test]
fn unresolved_and_repeated_mode_switches_refuse_at_the_name_or_the_second() {
    for (declaration, at) in [
        ("mode Nope hi on ConfigVersion::command;", "Nope"),
        ("mode Ctl hi on ConfigVersion::nope;", "ConfigVersion::nope"),
    ] {
        assert_eq!(
            refused(declaration),
            [(Code::MissingDeclaration, "missing-name", at.to_owned())],
            "{declaration}"
        );
    }
    assert_eq!(
        refused("mode Nope hi on ConfigVersion::nope;"),
        [
            (Code::MissingDeclaration, "missing-name", "Nope".to_owned()),
            (
                Code::MissingDeclaration,
                "missing-name",
                "ConfigVersion::nope".to_owned()
            ),
        ]
    );
    assert_eq!(
        refused(
            "mode Ctl hi on ConfigVersion::command;\n
             mode Ctl hi on ConfigVersion::command;"
        ),
        [(
            Code::ConflictingDeclaration,
            "duplicate-declaration",
            "mode Ctl hi on ConfigVersion::command;".to_owned()
        )]
    );
}

#[trace("TC-704", "FR-249-AC-13")]
#[test]
fn a_mode_switch_on_an_ambiguous_operation_refuses_ambiguous_name() {
    let text = unit_text("mode Ctl hi on Config::Both::dup;");
    let refusals =
        admit_and_assemble_with_profiles(&ambiguous_operation_document(), PROFILES, &text)
            .expect("S2 keeps the mode switch")
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &crate::reward_evaluator::ZeroRewardEvaluator,
            )
            .expect_err("the operation is ambiguous");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::Temporal(cause) = refusal.cause.as_ref() else {
        panic!("a temporal refusal: {refusal:?}");
    };
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::AmbiguousDeclaration, "ambiguous-name")
    );
}

#[trace("TC-704", "FR-230-AC-7")]
#[test]
fn a_mode_switch_is_refused_unless_the_timed_facet_is_selected() {
    let text = format!("{SET}mode Ctl hi on ConfigVersion::command;\n");
    let refusals = admit_and_assemble_with_profiles(
        &document(),
        "profile t = \"filament.value.complete/v1\";\n",
        &text,
    )
    .expect("S2 keeps the forms")
    .check(
        CheckMode::Linked,
        CheckingLimits::default(),
        &crate::reward_evaluator::ZeroRewardEvaluator,
    )
    .expect_err("the facet gate refuses");
    let found = refusals.iter().any(|refusal| {
        matches!(
            refusal.cause.as_ref(),
            CheckCause::Temporal(cause) if matches!(
                &**cause,
                TemporalCause::TimedFacetRequired { construct: TimedConstruct::ModeSwitch, .. }
            )
        )
    });
    assert!(found, "{refusals:?}");
}
