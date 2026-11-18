// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1000 (TC-2835): a reference literal stays pending until a subject is
//! bound; the resolver then decides fixed or created-object, refuses an
//! unknown key and a created-object literal in a slot that bars model state,
//! and evaluation reads a created-object literal by a per-state key lookup.

use ix_trace_rs::trace;
use serde_json::{Value, json};

use crate::analyze_cli_support::{
    Workspace, analyze, counter_request, counter_request_with, counter_snapshot, label,
    request_members,
};
use crate::counter::{CONTRACT, unit_text};

/// One analyze run over the Counter subject whose initial state holds `a`
/// and whose universe holds `a` and `b`, with `body` and the item `claim`.
fn over_wide_universe(name: &str, body: &str, claim: &str) -> crate::analyze_cli_support::Run {
    over_wide_universe_with_prelude(name, "", body, claim)
}

fn over_wide_universe_with_prelude(
    name: &str,
    prelude: &str,
    body: &str,
    claim: &str,
) -> crate::analyze_cli_support::Run {
    let ws = Workspace::new(name);
    let mut wide: Value =
        serde_json::from_slice(&counter_snapshot(&[("a", 0), ("b", 0)])).expect("a snapshot");
    wide["identity"]["identity"] = json!("wide");
    let mut request = request_members(Some(json!([{"claim": claim}])));
    request["documents"] = json!([{"file": "start.json"}, {"file": "wide.json"}]);
    request["subject"] = json!({
        "initial_states": [{"authority": "test", "identity": "start"}],
        "universes": [{"authority": "test", "identity": "wide"}],
        "proof_bounds": []
    });
    let path = counter_request_with(&ws, body, request);
    if !prelude.is_empty() {
        let mut source = unit_text(&format!("{CONTRACT}{body}"));
        source = source.replacen("model M =", &format!("{prelude}model M ="), 1);
        ws.write("program.fsl", source.as_bytes());
    }
    ws.write("wide.json", wide.to_string().as_bytes());
    analyze(&path)
}

fn temporal(name: &str, formula: &str) -> String {
    format!(
        "temporal {name} using t0 over (c: M::Counter) clock \"model-steps\" on origin {{ {formula} }}\n"
    )
}

fn guarded_protocol(key: &str) -> String {
    format!(
        "record Pick {{ target: Reference<M::Counter>; }}\n\
         protocol L using p over (c: M::Counter) on origin {{\n\
         role R on M::Counter;\nrole S on M::Counter;\n\
         channel ch from R to S carries Pick ordering unordered delivery \
         exactly-once-premise capacity 2 overflow reject;\n\
         run sequence Main {{\n\
         send Go via ch as (go: Pick) {{ true }};\n\
         receive Got via ch of Go as (got: Pick) {{ true }};\n\
         choice Decide by S visible (got.target) {{ \
         case yes when {{ true or got.target = M::Counter#\"{key}\" }} sequence Yes {{ }} \
         case no when {{ false }} sequence No {{ }} }} }}\n\
         finish Done as (d: Boolean) {{ true }};\n}}\n"
    )
}

/// FR-1000-AC-4: a fixed literal is decided, an unknown key refuses before
/// any state, and a created-object literal in a `choice` guard refuses
/// `operator-ineligible` while a fixed one there is admitted.
#[trace(
    "TC-2835",
    "FR-1000-AC-4",
    "FSpec-TC-965",
    "FSpec-FR-480-AC-4",
    "FSpec-FR-480-AC-5"
)]
#[test]
fn a_reference_literal_resolves_at_subject_binding() {
    let fixed_workspace = Workspace::new("reflit-fixed");
    let fixed = analyze(&counter_request(
        &fixed_workspace,
        &temporal("F", "always holds(c = M::Counter#\"a\")"),
        json!([{"claim": "F"}]),
    ));
    assert_eq!(fixed.exit, 0, "{}", fixed.document());
    let unknown = over_wide_universe(
        "reflit-unknown",
        &temporal("U", "always holds(c = M::Counter#\"zz\" or true)"),
        "U",
    );
    let text = unknown.document().to_string();
    assert_eq!(unknown.exit, 20, "{text}");
    assert!(text.contains("missing-name"), "{text}");
    assert!(text.contains("Counter") && text.contains("7a7a"), "{text}");
    let created = over_wide_universe_with_prelude(
        "reflit-created-guard",
        "profile p = \"filament.protocol.finite-global/v1\";\n",
        &guarded_protocol("b"),
        "L",
    );
    let text = created.document().to_string();
    assert!(text.contains("operator-ineligible"), "{text}");
    let admitted = over_wide_universe_with_prelude(
        "reflit-fixed-guard",
        "profile p = \"filament.protocol.finite-global/v1\";\n",
        &guarded_protocol("a"),
        "L",
    );
    let text = admitted.document().to_string();
    assert!(!text.contains("operator-ineligible"), "{text}");
}

/// FR-1000-AC-4 (TC-2957): a call whose body contains a reference literal
/// still uses checked-source replay when a temporal counterexample is settled.
#[trace("TC-2957", "FR-1000-AC-4")]
#[test]
fn a_called_function_reference_literal_replays_from_checked_source() {
    let workspace = Workspace::new("reflit-function-replay");
    let body = format!(
        "function reference_a using v(): Reference<M::Counter> pure {{ M::Counter#\"a\" }}\n{}",
        temporal(
            "CalledReference",
            "always holds(c.value = 1 and c = reference_a())",
        ),
    );
    let run = analyze(&counter_request(
        &workspace,
        &body,
        json!([{"claim": "CalledReference"}]),
    ));
    assert_eq!(run.exit, 10, "{}", run.document());
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert!(!item["counterexample"].is_null(), "{item}");
}

/// FR-1000-AC-5: a created-object literal with no member is undefined
/// `absent-key`; a fixed literal is decided.
#[trace("TC-2835", "FR-1000-AC-5", "FSpec-FR-480-AC-4")]
#[test]
fn a_created_object_literal_is_a_per_state_lookup() {
    let created = over_wide_universe(
        "reflit-created",
        &temporal("B", "always holds(deref(M::Counter#\"b\").value = 0)"),
        "B",
    );
    assert!(
        created.document().to_string().contains("absent-key"),
        "{}",
        created.document()
    );
    let fixed = over_wide_universe(
        "reflit-fixed-deref",
        &temporal("A", "always holds(deref(M::Counter#\"a\").value <= 3)"),
        "A",
    );
    assert_eq!(label(&fixed.item(0)), "proved", "{}", fixed.document());
}

/// A hyper clause `name` over two traces of the Counter model with `formula`.
fn hyper(name: &str, formula: &str) -> String {
    format!(
        "hyper {name} using t0 over behaviours (c: M::Counter) \
         forall trace a of M forall trace b of M {{ {formula} }}\n"
    )
}

/// FR-1000-AC-10: a created-object literal that no `@ t` encloses refuses at
/// binding, even when the atom reads a component elsewhere; a fixed one there
/// is admitted.
#[trace("TC-2957", "FR-1000-AC-10")]
#[test]
fn a_created_object_literal_outside_every_indexed_read_refuses() {
    let cases = [
        ("bare", "always holds(c = M::Counter#\"K\")"),
        (
            "beside-a-read",
            "always holds(c.value @ a = 0 or c = M::Counter#\"K\")",
        ),
    ];
    for (name, formula) in cases {
        let refused = over_wide_universe(
            &format!("reflit-hyper-{name}"),
            &hyper("HS", &formula.replace('K', "b")),
            "HS",
        );
        let text = refused.document().to_string();
        assert_eq!(refused.exit, 20, "{text}");
        assert!(text.contains("operator-ineligible"), "{text}");
        assert!(text.contains("Counter") && text.contains("62"), "{text}");
        let admitted = over_wide_universe(
            &format!("reflit-hyper-{name}-fixed"),
            &hyper("HS", &formula.replace('K', "a")),
            "HS",
        );
        let text = admitted.document().to_string();
        assert!(!text.contains("operator-ineligible"), "{text}");
    }
}

/// FR-1000-AC-10: one called literal used both under `@ a` and outside every
/// indexed read stays barred, regardless of which use the graph visits first.
#[trace("TC-2957", "FR-1000-AC-10")]
#[test]
fn a_called_literal_is_barred_when_another_use_is_unindexed() {
    let body = format!(
        "function reference_b using v(): Reference<M::Counter> pure {{ M::Counter#\"b\" }}\n{}",
        hyper(
            "CalledUse",
            "always holds(c = reference_b() or (c = reference_b()) @ a)",
        ),
    );
    let refused = over_wide_universe("reflit-called-unindexed", &body, "CalledUse");
    let text = refused.document().to_string();
    assert_eq!(refused.exit, 20, "{text}");
    assert!(text.contains("operator-ineligible"), "{text}");
    assert!(text.contains("Counter") && text.contains("62"), "{text}");
}

/// FR-1000-AC-8: the specified two-component hyper clause admits and analyzes
/// its fixed reference literal without a binding refusal.
#[trace("TC-2957", "FR-1000-AC-8")]
#[test]
fn a_fixed_literal_hyper_clause_analyzes_without_binding_refusal() {
    let body = hyper(
        "HF",
        "always holds(c.value @ a <= 3 or c.value @ b <= 3 or c = M::Counter#\"a\")",
    );
    let run = over_wide_universe("reflit-hyper-fixed-analyze", &body, "HF");
    let text = run.document().to_string();
    assert_eq!(run.exit, 0, "{text}");
    assert!(!text.contains("operator-ineligible"), "{text}");
}

/// FR-1000-AC-11: hyper letters over components whose binding tables
/// disagree on a literal refuse before any state is observed.
#[trace("TC-2957", "FR-1000-AC-11")]
#[test]
fn hyper_letters_refuse_disagreeing_reference_bindings() {
    use crate::hyper_world::{Vault, World};
    use fsl_eval::hyper::CheckedHyperLetters;
    use fsl_eval::simulation::model::ModelSystem;

    let declaration = "hyper W using inf over behaviours (v: V::Vault) \
                       forall trace a of V forall trace b of V \
                       { always holds((v = V::Vault#\"w\") @ a or v.l @ a = v.l @ b) }";
    let fixed = World::with_vaults(Vault::Secure, &[("v", (0, 0)), ("w", (0, 0))], declaration);
    let created = World::with_snapshots(
        Vault::Secure,
        vec![vec![("v", (0, 0))]],
        &["v", "w"],
        declaration,
    );
    let fixed_system = ModelSystem::new(fixed.subject().simulation).expect("the subject admits");
    let created_system =
        ModelSystem::new(created.subject().simulation).expect("the subject admits");
    let refusal = CheckedHyperLetters::new(
        &fixed.package,
        &fixed.clause,
        vec![&fixed_system, &created_system],
    )
    .err();
    let text = format!("{refusal:?}");
    assert!(
        text.contains("ReferenceBindingsDisagree")
            && text.contains("Vault")
            && text.contains("77")
            && text.contains("[0, 1]")
            && text.contains("Fixed")
            && text.contains("CreatedObject"),
        "{text}"
    );
    assert!(
        CheckedHyperLetters::new(
            &fixed.package,
            &fixed.clause,
            vec![&fixed_system, &fixed_system],
        )
        .is_ok()
    );
}
