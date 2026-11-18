// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1033 (TC-2958): a `native-analyze/1` request selects a protocol
//! subject through `subject.protocol`, reads the `memory` member, requests
//! race freedom over a `parallel`, and refuses bad selections and bindings.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, litmus_request, Workspace};
use crate::litmus::{sb_run, sb_unit};

fn binding() -> Value {
    json!({"type": "model-binding", "selection": "test/litmus"})
}

fn sb_subject() -> Value {
    json!({"protocol": {"protocol": "SB", "roles": {"w": binding(), "c": binding()}}})
}

/// FR-1033-AC-1: the claim holds over `SB` under `sc` and is refuted under a
/// `tso` selection, the counterexample replayed.
#[trace("TC-2958", "FR-1033-AC-1")]
#[trace("FSpec-TC-2005", "FSpec-FR-520-AC-6")]
#[test]
#[ignore = "QSL-2126: subject.protocol and the memory member are not read by fsl analyze yet"]
fn a_protocol_subject_is_decided_under_the_selected_memory_model() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let ws = Workspace::new("protocol-subject-sc");
    let sc = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": sb_subject(), "items": [{"claim": "NoBothZero"}]}),
    ));
    assert_eq!(label(&sc.item(0)), "proved", "{}", sc.document());
    let ws = Workspace::new("protocol-subject-tso");
    let tso = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": sb_subject(), "memory": [{"parallel": "SB::Both", "model": "tso"}],
            "items": [{"claim": "NoBothZero"}]}),
    ));
    assert_eq!(label(&tso.item(0)), "refuted", "{}", tso.document());
    assert_eq!(tso.item(0)["replay"]["reproduced"], true);
    assert_eq!(tso.exit, 10);
    assert_eq!(
        tso.item(0)["request"]["temporal"]["subject"]["protocol"]["protocol"],
        "SB",
        "{}",
        tso.document()
    );
    assert_ne!(sc.item(0)["request"], tso.item(0)["request"]);
}

/// FR-1033-AC-2: a race-freedom item over two unsynchronised non-atomic
/// writes is refuted naming both; naming another protocol is unknown-claim.
#[trace("TC-2958", "FR-1033-AC-2")]
#[test]
#[ignore = "QSL-2126: race-freedom items are not reachable through fsl analyze yet"]
fn a_race_freedom_item_over_a_parallel_is_refuted() {
    let body = sb_unit(&sb_run("memory tso", "", "acquire"));
    let ws = Workspace::new("protocol-race");
    let run = analyze(&litmus_request(
        &ws,
        &body,
        json!({"items": [{"claim": "SB::Both", "temporal": {"item_kind": "race-freedom"}}]}),
    ));
    assert_eq!(label(&run.item(0)), "refuted", "{}", run.document());
    let race = &run.item(0)["counterexample"]["counterexample"]["kind"];
    assert!(race.to_string().contains("race"), "{}", run.document());
}

/// FR-1033-AC-4 (negative): an unknown model and a selection naming no
/// outermost `parallel` refuse the whole request.
#[trace("TC-2958", "FR-1033-AC-4")]
#[test]
#[ignore = "QSL-2126: the memory member is refused as unread by fsl analyze today"]
fn bad_memory_selections_refuse_the_request() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    for (n, memory) in [
        json!([{"parallel": "SB::Both", "model": "pso"}]),
        json!([{"parallel": "SB::Both::Sx", "model": "tso"}]),
        json!([{"parallel": "Nope::Both", "model": "tso"}]),
        json!([{"parallel": "SB::Both", "model": "tso"}, {"parallel": "SB::Both", "model": "ra"}]),
    ]
    .into_iter()
    .enumerate()
    {
        let ws = Workspace::new(&format!("protocol-memory-{n}"));
        let run = analyze(&litmus_request(
            &ws,
            &body,
            json!({"subject": sb_subject(), "memory": memory, "items": [{"claim": "NoBothZero"}]}),
        ));
        assert_eq!(run.exit, 20, "{}", run.document());
        assert!(
            run.document()["items"].as_array().is_some_and(Vec::is_empty),
            "{}",
            run.document()
        );
        assert!(
            !run.document().to_string().contains("not read by this build"),
            "{}",
            run.document()
        );
    }
}

/// FR-1033-AC-5 (negative): a protocol the package does not declare
/// declines the item; an unknown binding member refuses the request.
#[trace("TC-2958", "FR-1033-AC-5")]
#[test]
#[ignore = "QSL-2126: subject.protocol is not read by fsl analyze yet"]
fn bad_protocol_bindings_decline_or_refuse() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let ws = Workspace::new("protocol-binding-missing");
    let run = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": {"protocol": {"protocol": "Missing"}}, "items": [{"claim": "NoBothZero"}]}),
    ));
    assert_eq!(label(&run.item(0)), "declined", "{}", run.document());
    let ws = Workspace::new("protocol-binding-extra");
    let run = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": {"protocol": {"protocol": "SB", "extra": 1}},
            "items": [{"claim": "NoBothZero"}]}),
    ));
    assert_eq!(run.exit, 20, "{}", run.document());
}

/// FR-1033-AC-6 and FR-225-AC-4: FR-225-AC-2's three unfenced stores under
/// `tso` with `max_store_buffer` 2 and the state constraint `Zero` settle
/// `ConstraintReached` with no counterexample, stating the store-buffer
/// bound as reached; without `Zero` they settle `MemoryBoundReached`.
#[trace("TC-2958", "FR-1033-AC-6")]
#[trace("TC-670", "FR-225-AC-4")]
#[test]
#[ignore = "QSL-2126: constrained weak-memory items are not reachable through fsl analyze yet"]
fn a_constrained_store_buffer_run_settles_constraint_reached() {
    use crate::litmus::attempt;
    let run = format!(
        "parallel Both {{ branch left sequence L {{ {} {} {} }} \
         branch right sequence R {{ {} {} {} }} }} join all [left, right];",
        attempt("Sx", "w", "Cell::set", "release", "IsX, StoreOne"),
        attempt("Sy", "w", "Cell::setw", "release", "IsXw, FlagOne"),
        attempt("Sz", "w", "Cell::set", "release", "IsY, StoreOne"),
        attempt("Lx", "c", "Core::read1", "acquire", "ReadsX1, Read1"),
        attempt("Ly", "c", "Core::read2", "acquire", "ReadsX2, ReadFlag2"),
        attempt("Lz", "c", "Core::read3", "acquire", "ReadsY3, Read3"),
    );
    let body = format!(
        "{}constraint Zero using v on M::Cell at current {{ self.v <= 0 }}\n\
         temporal Holds using t0 over (k: M::Core) clock \"model-steps\" on origin \
         {{ always holds(k.r1 <= 3) }}\n",
        crate::litmus::contracts() + &crate::litmus::protocol("SB", &run)
    );
    let memory = json!([{"parallel": "SB::Both", "model": "tso"}]);
    let item = |constraint: bool| {
        let mut item = json!({"claim": "Holds", "temporal": {"limits": {"max_store_buffer": 2}}});
        if constraint {
            item["reductions"] = json!({"constraint": "Zero"});
        }
        item
    };
    let ws = Workspace::new("protocol-constraint-reached");
    let run_with = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": sb_subject(), "memory": memory, "items": [item(true)]}),
    ));
    let record = &run_with.item(0)["record"];
    assert_eq!(label(&run_with.item(0)), "inconclusive", "{}", run_with.document());
    assert!(record.to_string().contains("constraint-reached"), "{record}");
    assert!(run_with.item(0).get("counterexample").is_none(), "{record}");
    assert!(record.to_string().contains("max_store_buffer"), "{record}");
    let ws = Workspace::new("protocol-memory-bound-reached");
    let run_without = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": sb_subject(), "memory": memory, "items": [item(false)]}),
    ));
    assert!(
        run_without.item(0)["record"].to_string().contains("memory-bound-reached"),
        "{}",
        run_without.document()
    );
}
