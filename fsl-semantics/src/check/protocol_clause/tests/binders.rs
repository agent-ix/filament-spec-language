// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-113's binder no-shadowing rule.

use ix_trace_rs::trace;

use super::attempt_fixture::shared_binder_protocols;
use super::{checks, protocol_causes, recovery_flow, refusals};
use crate::check::CheckRefusal;
use crate::check::refusal::{ProtocolAnchorCause, ShadowedDeclaration};

/// The `Shadow` cause of `refusals`, expecting exactly one.
fn shadow_cause(refusals: &[CheckRefusal]) -> (&str, &ShadowedDeclaration) {
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    match protocol_causes(refusals)[0] {
        ProtocolAnchorCause::Shadow { name, shadowed, .. } => (name, shadowed),
        other => panic!("a Shadow cause, got {other:?}"),
    }
}

/// FR-113-AC-5 (TC-512), first case: a `capture` named `forward` inside
/// `Undo`, where `forward` is already the template's own bound
/// parameter (`as (forward: Boolean)`, renamed from the fixture's own
/// `failure`), refuses `ambiguous_declaration`/`ambiguous-name` at the
/// capture, naming the parameter it shadows.
#[trace("TC-512", "FR-113-AC-5")]
#[test]
fn a_capture_shadowing_its_templates_own_bound_parameter_refuses() {
    let source = recovery_flow("Main::Committed", "")
        .replacen("as (failure: Boolean)", "as (forward: Boolean)", 1)
        .replacen(
            "clock \"ticks\" {\nactivate first",
            "clock \"ticks\" {\ncapture forward: Boolean = true;\nactivate first",
            1,
        );
    assert!(
        source.contains("capture forward: Boolean = true;"),
        "fixture sanity: the capture must be inserted"
    );
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].cause.code().as_str(), "ambiguous_declaration");
    assert_eq!(found[0].cause.cause(), Some("ambiguous-name"));
    let (name, shadowed) = shadow_cause(&found);
    assert_eq!(name, "forward");
    assert!(
        matches!(shadowed, ShadowedDeclaration::Binder(_)),
        "{shadowed:?}"
    );
}

/// FR-113-AC-5 (TC-512), second case: an event record binder named
/// after the unit's own `v` profile alias (FR-113 "Refusals" reaches a
/// model or profile alias the same way; this fixture declares no
/// `model`, so its `v` profile alias stands in for a model alias)
/// refuses at the binder, naming the alias. A second, unrelated
/// protocol in the same unit whose own record binder is also `v`
/// checks (FR-113 "Refusals": "Two separate protocols may reuse a
/// binder name" -- this asserts the alias/native-declaration half is
/// package-wide while a binder's own shadow check stays local to its
/// protocol).
#[trace("TC-512", "FR-113-AC-5")]
#[test]
fn a_record_binder_shadowing_the_units_profile_alias_refuses() {
    let source = recovery_flow("Main::Committed", "").replacen(
        "as (applied: Boolean)",
        "as (v: Boolean)",
        1,
    );
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].cause.code().as_str(), "ambiguous_declaration");
    let (name, shadowed) = shadow_cause(&found);
    assert_eq!(name, "v");
    assert!(
        matches!(shadowed, ShadowedDeclaration::ProfileAlias),
        "{shadowed:?}"
    );

    // A second protocol, naming its own `effect`'s record binder `v`
    // too: this protocol's own anchors and declarations are entirely
    // separate from the first, but `v` is still package-wide, so it
    // still refuses -- the "two separate protocols" carve-out is about
    // reusing a binder name *between* protocols, not about escaping the
    // package-level alias/native-declaration check.
    let second = recovery_flow("Main::Committed", "")
        .replacen("protocol RecoveryFlow", "protocol RecoveryFlowTwo", 1)
        .replacen("as (applied: Boolean)", "as (v: Boolean)", 1);
    let combined = format!("{source}\n{second}");
    let (_, found) = refusals(&combined);
    let shadow_causes: Vec<&ProtocolAnchorCause> = protocol_causes(&found)
        .into_iter()
        .filter(|cause| matches!(cause, ProtocolAnchorCause::Shadow { .. }))
        .collect();
    assert_eq!(shadow_causes.len(), 2, "{shadow_causes:?}");
    for cause in shadow_causes {
        match cause {
            ProtocolAnchorCause::Shadow { name, shadowed, .. } => {
                assert_eq!(name, "v");
                assert!(matches!(shadowed, ShadowedDeclaration::ProfileAlias));
            }
            other => panic!("a Shadow cause, got {other:?}"),
        }
    }
}

/// A `compensate` declaration's `activate first (p)` trigger parameter
/// shadowing a native `predicate` declaration of the package (FR-113
/// "Refusals": a native declaration is checked the same way an alias
/// is, `ShadowedDeclaration::Function`).
#[trace("TC-512")]
#[test]
fn a_trigger_parameter_shadowing_a_native_function_declaration_refuses() {
    let source = format!(
        "predicate forward using v(): Boolean {{ true }}\n{}",
        recovery_flow("Main::Committed", "").replacen(
            "(trigger: Boolean)",
            "(forward: Boolean)",
            1
        )
    );
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    let (name, shadowed) = shadow_cause(&found);
    assert_eq!(name, "forward");
    assert!(
        matches!(shadowed, ShadowedDeclaration::Function),
        "{shadowed:?}"
    );
}

/// A `retry (a, b)` parameter shadowing another binder visible where it
/// is declared: `Undo`'s own record binder, renamed `shared`, and
/// `retry`'s first parameter, also renamed `shared` -- both inside the
/// same `compensate` declaration, the retry parameter one scope level
/// inside the declaration's own record binder.
#[trace("TC-512")]
#[test]
fn a_retry_parameter_shadowing_its_declarations_own_record_binder_refuses() {
    let source = recovery_flow("Main::Committed", "")
        .replacen("as (failure: Boolean)", "as (shared: Boolean)", 1)
        .replacen("current_attempt: M::Try", "shared: M::Try", 1);
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    let (name, shadowed) = shadow_cause(&found);
    assert_eq!(name, "shared");
    assert!(
        matches!(shadowed, ShadowedDeclaration::Binder(_)),
        "{shadowed:?}"
    );
}

/// A bare `commit` control's own record binder shadowing the unit's `v`
/// profile alias, spreading FR-113-AC-5's coverage across a binder
/// position AC-5's own fixture does not exercise.
#[trace("TC-512")]
#[test]
fn a_commit_controls_record_binder_shadowing_an_alias_refuses() {
    let source = recovery_flow("Main::Committed", "").replacen(
        "as (committed: Boolean)",
        "as (v: Boolean)",
        1,
    );
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    let (name, shadowed) = shadow_cause(&found);
    assert_eq!(name, "v");
    assert!(
        matches!(shadowed, ShadowedDeclaration::ProfileAlias),
        "{shadowed:?}"
    );
}

/// FR-113-AC-6 (TC-512 step 3, exactly): `effect Applied of Absent` (a
/// missing member, TC-512's own step-3 fixture, physically inside
/// `run`) combined with the shadowing `forward` capture (TC-512 step
/// 2's own fixture: `Undo` binds `forward` as its record binder, and a
/// `capture forward = ...;` inside it shadows that binder), physically
/// inside `compensate Undo`, so earlier in source. Both refusals
/// report, ordered by source position (SR-766 FND-004: this fixture
/// used to differ from TC-512 step 3's own procedure -- a
/// `Main::Missing` anchor and a `v`-named record binder shadowing the
/// unit's own alias, neither of which step 3 names -- so it is now the
/// same fixture step 3 describes). Checking it twice gives the same
/// refusals in the same order, and no checked protocol node emits (the
/// full pipeline refuses).
#[trace("TC-512", "FR-113-AC-6")]
#[test]
fn a_missing_anchor_and_a_later_shadowing_binder_report_in_source_order() {
    let source = recovery_flow("Main::Committed", "")
        .replacen("effect Applied of Tried", "effect Applied of Absent", 1)
        .replacen("as (failure: Boolean)", "as (forward: Boolean)", 1)
        .replacen(
            "clock \"ticks\" {\nactivate first",
            "clock \"ticks\" {\ncapture forward: Boolean = true;\nactivate first",
            1,
        );
    let (text, refusals_a) = refusals(&source);
    let (_, refusals_b) = refusals(&source);
    assert_eq!(refusals_a, refusals_b, "checking twice must agree");
    assert_eq!(refusals_a.len(), 2, "{refusals_a:?}");

    let shadow_span = text.find("capture forward").unwrap();
    let missing_span = text.find("effect Applied of Absent").unwrap();
    assert!(
        shadow_span < missing_span,
        "fixture sanity: the shadowing capture must come first in source"
    );
    let causes = protocol_causes(&refusals_a);
    assert!(
        matches!(causes[0], ProtocolAnchorCause::Shadow { .. }),
        "the earliest refusal must be the Shadow one, {:?}",
        refusals_a[0]
    );
    assert!(
        matches!(causes[1], ProtocolAnchorCause::Missing { .. }),
        "the later refusal must be the Missing one, {:?}",
        refusals_a[1]
    );
}

/// FR-113-AC-12 (TC-512): finish reads the root output, so its binder cannot
/// reuse a run binder that remains active there.
#[trace("TC-512", "FR-113-AC-12")]
#[test]
fn a_finish_binder_conflicting_with_root_output_refuses() {
    let source = recovery_flow("Main::Committed", "").replacen(
        "as (outcome: Boolean)",
        "as (tried: Boolean)",
        1,
    );
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    let (name, shadowed) = shadow_cause(&found);
    assert_eq!(name, "tried");
    assert!(
        matches!(shadowed, ShadowedDeclaration::Binder(_)),
        "{shadowed:?}"
    );
}

/// FR-113-AC-8 (TC-512): a compensation trigger and a later event binder in
/// a disjoint run scope may reuse the same name.
#[trace("TC-512", "FR-113-AC-8")]
#[test]
fn a_later_event_binder_reuses_a_compensation_trigger_name() {
    let source = recovery_flow("Main::Committed", "")
        .replacen("(trigger: Boolean)", "(f: Boolean)", 1)
        .replacen(
            "event Recovered by R for Undo as (notice: Boolean)",
            "event Fail by R for Undo as (f: Boolean)",
            1,
        );
    let graph = checks(&source);
    assert_eq!(graph.protocols().len(), 1);
}

/// FR-113-AC-9 (TC-512): an enclosing binder remains active in a nested
/// control, so the child's same-named binder refuses.
#[trace("TC-512", "FR-113-AC-10")]
#[test]
fn a_nested_sequences_binder_reusing_an_outer_siblings_name_refuses() {
    let inner = "sequence Inner {\n\
         event Recovered2 by R for Undo as (tried: Boolean) { true };\n\
         }\n";
    let source = recovery_flow("Main::Committed", inner);
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    let (name, shadowed) = shadow_cause(&found);
    assert_eq!(name, "tried");
    assert!(
        matches!(shadowed, ShadowedDeclaration::Binder(_)),
        "{shadowed:?}"
    );
}

/// FR-113-AC-9 (TC-512): a capture in the trigger's activation scope cannot
/// reuse the active trigger binder's name.
#[trace("TC-512", "FR-113-AC-9")]
#[test]
fn a_capture_cannot_reuse_its_active_trigger_binders_name() {
    let source = recovery_flow("Main::Committed", "").replacen(
        "activate first (trigger: Boolean) when { true } { }",
        "activate first (f: Boolean) when { true } { capture f: Boolean = f; }",
        1,
    );
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].cause.code().as_str(), "ambiguous_declaration");
    assert_eq!(found[0].cause.cause(), Some("ambiguous-name"));
    let (name, shadowed) = shadow_cause(&found);
    assert_eq!(name, "f");
    assert!(matches!(shadowed, ShadowedDeclaration::Binder(_)));
}

/// FR-113-AC-13 (TC-512): sibling case scopes are disjoint, so their local
/// record binders may reuse a name.
#[trace("TC-512", "FR-113-AC-13")]
#[test]
fn sibling_case_binders_of_one_name_are_out_of_scope_for_each_other() {
    let choice = "choice Which by R visible () {\n\
         case left when { true } sequence Left {\n\
         event LeftSeen by R for Undo as (dup: Boolean) { true };\n\
         }\n\
         case right when { false } sequence Right {\n\
         event RightSeen by R for Undo as (dup: Boolean) { true };\n\
         }\n\
         }\n";
    let source = recovery_flow("Main::Committed", choice);
    let graph = checks(&source);
    assert_eq!(graph.protocols().len(), 1);
}

/// FR-113-AC-11 (TC-512): a sequence threads a nested child's output into
/// the next sibling, so the later binder cannot reuse that active name.
#[trace("TC-512", "FR-113-AC-11")]
#[test]
fn a_later_sequence_step_cannot_reuse_a_nested_steps_active_binder() {
    let main_extra = "sequence Inner {\n\
         event Nested by R for Undo as (shared: Boolean) { true };\n\
         }\n\
         event After by R for Undo as (shared: Boolean) { true };\n";
    let source = recovery_flow("Main::Committed", main_extra);
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    let (name, shadowed) = shadow_cause(&found);
    assert_eq!(name, "shared");
    assert!(
        matches!(shadowed, ShadowedDeclaration::Binder(_)),
        "{shadowed:?}"
    );
}

/// SR-765 FND-002: the protocol's own `over (p)` input parameter and
/// `activation on each (p)` parameter are binders too, visible
/// everywhere in the protocol (`BinderKind::Input` and
/// `BinderKind::ActivationParameter`); a record binder reusing either
/// name refuses.
#[trace("TC-510", "FR-113")]
#[test]
fn a_record_binder_shadowing_the_protocols_own_input_parameter_refuses() {
    let source = recovery_flow("Main::Committed", "").replacen(
        "as (tried: Boolean)",
        "as (input: Boolean)",
        1,
    );
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    let (name, shadowed) = shadow_cause(&found);
    assert_eq!(name, "input");
    assert!(
        matches!(shadowed, ShadowedDeclaration::Binder(_)),
        "{shadowed:?}"
    );
}

#[trace("TC-510", "FR-113")]
#[test]
fn a_record_binder_shadowing_the_protocols_own_activation_parameter_refuses() {
    let source = recovery_flow("Main::Committed", "")
        .replacen("on origin", "on each (activated: Boolean) when (true)", 1)
        .replacen("as (tried: Boolean)", "as (activated: Boolean)", 1);
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    let (name, shadowed) = shadow_cause(&found);
    assert_eq!(name, "activated");
    assert!(
        matches!(shadowed, ShadowedDeclaration::Binder(_)),
        "{shadowed:?}"
    );
}

/// SR-765 FND-003: a `dimension`/`unit` declaration participates in the
/// package-wide half of FR-113's no-shadowing rule the same way an
/// alias or a function does, not just `Scope::named_types`'s composites,
/// enums and object types.
#[trace("TC-512", "FR-113")]
#[test]
fn a_record_binder_shadowing_a_native_dimension_declaration_refuses() {
    let source = format!("dimension tried;\n{}", recovery_flow("Main::Committed", ""));
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    let (name, shadowed) = shadow_cause(&found);
    assert_eq!(name, "tried");
    assert!(
        matches!(shadowed, ShadowedDeclaration::Quantity),
        "{shadowed:?}"
    );
}

// SR-766 FND-001's model-alias regression (the `.chain(selections.
// models...)` half of `PackageDeclarations::alias_names`'s assembly)
// lives in `fsl-semantics/tests/it/model_operations.rs`
// (`a_models_own_alias_is_recorded_in_the_packages_alias_names`), whose
// existing `admit_and_assemble` pipeline admits a real domain package;
// this module's own `assemble` test helper has no such admission and
// refuses `UnadmittedModel` for any `model` declaration.

/// SR-766 FND-002: AC-5's own "two separate protocols" carve-out, tested
/// directly rather than only implicitly (the pre-existing test's second
/// protocol reused the alias `v` itself, so the carve-out was never
/// actually exercised on an ordinary, non-alias binder name). Two
/// protocols in one unit, each with a record binder named `shared` (not
/// an alias or any native declaration): neither refuses, since local
/// binder scopes are checked within their owning protocol declaration.
#[trace("TC-512", "FR-113-AC-5")]
#[test]
fn two_protocols_sharing_an_ordinary_binder_name_both_check() {
    let combined = shared_binder_protocols();
    let checked = checks(&combined);
    let names: Vec<&str> = checked
        .protocols()
        .iter()
        .map(|protocol| protocol.name.as_str())
        .collect();
    assert_eq!(names, ["First", "Second"]);
}
