// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1000 (TC-2835): a reference literal stays pending until a subject is
//! bound; the resolver then decides fixed or created-object, refuses an
//! unknown key and a created-object literal in a slot that bars model state,
//! and evaluation reads a created-object literal by a per-state key lookup.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, counter_request_with, counter_snapshot, label, request_members, Workspace,
};

/// One analyze run over the Counter subject whose initial state holds `a`
/// and whose universe holds `a` and `b`, with `body` and the item `claim`.
fn over_wide_universe(name: &str, body: &str, claim: &str) -> crate::analyze_cli_support::Run {
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
        "profile p = \"filament.protocol.finite-global/v1\";\n\
         record Pick {{ target: Reference<M::Counter>; }}\n\
         protocol L using p over (c: M::Counter) on origin {{\n\
         role R on M::Counter;\nrole S on M::Counter;\n\
         channel ch from R to S carries Pick ordering unordered delivery \
         exactly-once-premise capacity 2 overflow reject;\n\
         run sequence Main {{\n\
         send Go via ch as (go: Pick) {{ true }};\n\
         receive Got via ch of Go as (got: Pick) {{ true }};\n\
         choice Decide by S visible (got.target) {{ \
         case yes when {{ got.target = M::Counter#\"{key}\" }} sequence Yes {{ }} \
         case no when {{ not (got.target = M::Counter#\"{key}\") }} sequence No {{ }} }} }}\n\
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
#[ignore = "QSL-2166: no binding-time reference-literal resolution exists yet"]
fn a_reference_literal_resolves_at_subject_binding() {
    let fixed = over_wide_universe(
        "reflit-fixed",
        &temporal("F", "always holds(c.value <= 3 or c = M::Counter#\"a\")"),
        "F",
    );
    assert_eq!(fixed.exit, 0, "{}", fixed.document());
    let unknown = over_wide_universe(
        "reflit-unknown",
        &temporal("U", "always holds(c = M::Counter#\"zz\" or true)"),
        "U",
    );
    let text = unknown.document().to_string();
    assert_eq!(unknown.exit, 20, "{text}");
    assert!(
        text.contains("missing-name") && text.contains("7a7a"),
        "{text}"
    );
    let created = over_wide_universe("reflit-created-guard", &guarded_protocol("b"), "L");
    let text = created.document().to_string();
    assert!(text.contains("operator-ineligible"), "{text}");
    let admitted = over_wide_universe("reflit-fixed-guard", &guarded_protocol("a"), "L");
    let text = admitted.document().to_string();
    assert!(!text.contains("operator-ineligible"), "{text}");
}

/// FR-1000-AC-5: a created-object literal with no member is undefined
/// `absent-key`; a fixed literal is decided.
#[trace("TC-2835", "FR-1000-AC-5", "FSpec-FR-480-AC-4")]
#[test]
#[ignore = "QSL-2166: a created-object reference literal is not a per-state key lookup yet"]
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

/// FR-1000-AC-8: a fixed literal is admitted in a hyper atom over two
/// components.
#[trace("TC-2957", "FR-1000-AC-8")]
#[test]
#[ignore = "QSL-2166: a fixed reference literal in a hyper atom is not bound per component yet"]
fn a_fixed_literal_is_admitted_in_a_hyper_atom() {
    let run = over_wide_universe(
        "reflit-hyper-fixed",
        &hyper(
            "HF",
            "always holds(c.value @ a <= 3 or c.value @ b <= 3 or c = M::Counter#\"a\")",
        ),
        "HF",
    );
    let text = run.document().to_string();
    assert_eq!(run.exit, 0, "{text}");
    assert!(!text.contains("operator-ineligible"), "{text}");
}

/// FR-1000-AC-9: a created-object literal is looked up in the component of
/// its innermost enclosing `@ t`, whatever the atom's other reads.
#[trace("TC-2957", "FR-1000-AC-9")]
#[test]
#[ignore = "QSL-2166: a created-object literal in a hyper atom is not resolved in its enclosing component yet"]
fn a_created_object_literal_resolves_in_its_enclosing_component() {
    let cases = [
        (
            "single",
            "always holds((deref(M::Counter#\"b\").value = 0) @ a)",
        ),
        (
            "spanning",
            "always holds((deref(M::Counter#\"b\").value) @ a = c.value @ b)",
        ),
    ];
    for (name, formula) in cases {
        let run = over_wide_universe(
            &format!("reflit-hyper-created-{name}"),
            &hyper("HC", formula),
            "HC",
        );
        let text = run.document().to_string();
        assert!(text.contains("absent-key"), "{text}");
        assert!(!text.contains("operator-ineligible"), "{text}");
    }
}

/// FR-1000-AC-10: a created-object literal that no `@ t` encloses refuses at
/// binding, even when the atom reads a component elsewhere; a fixed one there
/// is admitted.
#[trace("TC-2957", "FR-1000-AC-10")]
#[test]
#[ignore = "QSL-2166: a created-object literal outside every `@ t` is not refused yet"]
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
        assert!(
            text.contains("operator-ineligible") && text.contains("Counter") && text.contains("62"),
            "{text}"
        );
        let admitted = over_wide_universe(
            &format!("reflit-hyper-{name}-fixed"),
            &hyper("HS", &formula.replace('K', "a")),
            "HS",
        );
        let text = admitted.document().to_string();
        assert!(!text.contains("operator-ineligible"), "{text}");
    }
}

/// FR-1000-AC-11: hyper letters over components whose binding tables
/// disagree on a literal refuse before any state is observed.
#[trace("TC-2957", "FR-1000-AC-11")]
#[test]
#[ignore = "QSL-2166: hyper letters do not compare their components' reference bindings yet"]
fn hyper_letters_refuse_disagreeing_reference_bindings() {
    use crate::hyper_world::{Vault, World};
    use fsl_eval::hyper::CheckedHyperLetters;
    use fsl_eval::simulation::model::ModelSystem;

    let declaration = "hyper W using inf over behaviours (v: V::Vault) \
                       forall trace a of V forall trace b of V \
                       { always holds(v.l @ a = v.l @ b or v = V::Vault#\"w\") }";
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
        text.contains("ReferenceBindingsDisagree") && text.contains("77"),
        "{text}"
    );
    assert!(CheckedHyperLetters::new(
        &fixed.package,
        &fixed.clause,
        vec![&fixed_system, &fixed_system],
    )
    .is_ok());
}
