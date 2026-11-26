// SPDX-License-Identifier: AGPL-3.0-or-later
//! The hand models of the timed defect tickets.

use serde_json::{json, Value};

use crate::timed_world::{self, World, DENSE};

pub(super) const PROFILES: &str = "profile ev = \"filament.temporal.timed/v1\";\n\
     profile inf = \"filament.temporal.infinite-trace/v1\";\n";

/// `probabilistic name` over every scheduler.
pub(super) fn every(name: &str, form: &str) -> String {
    format!(
        "probabilistic {name} using ev on M over (c: M::Call) under every scheduler on origin \
         {{ {form} }}\n"
    )
}

/// `probabilistic name` under the workload `workload`.
pub(super) fn under(workload: &str, name: &str, form: &str) -> String {
    format!(
        "probabilistic {name} using ev on M over (c: M::Call) under {workload} on origin \
         {{ {form} }}\n"
    )
}

/// [`under`] with the parameters a statistical test needs.
pub(super) fn sampled(workload: &str, name: &str, form: &str) -> String {
    format!(
        "probabilistic {name} using ev on M over (c: M::Call) under {workload} on origin \
         with alpha 0.1, beta 0.1, indifference 0.005 {{ {form} }}\n"
    )
}

/// The temporal declaration `name` of `formula`, which the falsifier reads.
pub(super) fn formula(name: &str, formula: &str) -> String {
    format!(
        "temporal {name} using ev over (c: M::Call) clock \"model-time\" on origin \
         {{ {formula} }}\n"
    )
}

fn model(document: &[u8], body: &str) -> World {
    timed_world::world(document, PROFILES, &format!("{DENSE}{body}"))
}

/// `document` with the Boolean parameter `lost` on its first operation.
fn with_lost(document: &[u8], operation: &str) -> Vec<u8> {
    let mut package: Value = serde_json::from_slice(document).expect("JSON");
    package["types"][1]["operations"][0]["params"] = json!([{
        "identity": timed_world::ident(&format!("Call/{operation}/lost")),
        "name": "lost",
        "typeRef": "ix://filament/native/Boolean",
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"source": {"sourceIdentity": timed_world::ident("spec"),
            "path": "spec.fspec", "startLine": 1, "startColumn": 1}},
    }]);
    serde_json::to_vec(&package).expect("serializes")
}

/// QSL-3324: `step` sets `late`, time bounded by 3 ms until it runs. With
/// `drawn`, the step's delay is the one-point distribution at 3 ms; without,
/// its guard holds it to 3 ms.
pub(super) fn one_step(drawn: bool, claims: &str) -> World {
    let (guard, delay) = if drawn {
        ("", "delay M::Call::step ~ discrete { 3 ms: 1 };\n")
    } else {
        (" and self.x >= 3 ms", "")
    };
    model(
        &timed_world::document(&["x"], &[("step", &["phase", "late", "x"])]),
        &format!(
            "pre Step using v on M::Call::step {{ self.phase = 0{guard} }}\n\
             post Stepped using v on M::Call::step {{ self.phase = 1 and self.late = true }}\n\
             {delay}\
             time invariant Wait when {{ self.phase = 0 }} {{ self.x <= 3 ms }}\n\
             workload One on M {{ weight Call::step = 1; }}\n\
             {claims}"
        ),
    )
}

/// QSL-3324's workload subject: `a` runs by 1 ms and resets `x`, `b` runs
/// 2 ms later. With `drawn`, `b` also declares that one delay.
pub(super) fn workload_pair(drawn: bool, claims: &str) -> World {
    let delay = if drawn {
        "delay M::Call::b ~ discrete { 2 ms: 1 };\n"
    } else {
        ""
    };
    model(
        &timed_world::document(&["x"], &[("a", &["phase", "x"]), ("b", &["phase"])]),
        &format!(
            "pre CanA using v on M::Call::a {{ self.phase = 0 }}\n\
             post DidA using v on M::Call::a {{ self.phase = 1 and self.x = 0 ms }}\n\
             pre CanB using v on M::Call::b {{ self.phase = 1 and self.x >= 2 ms }}\n\
             post DidB using v on M::Call::b {{ self.phase = 2 }}\n\
             {delay}\
             workload W on M {{ weight Call::a = 1; weight Call::b = 1; }}\n\
             time invariant WaitA when {{ self.phase = 0 }} {{ self.x <= 1 ms }}\n\
             time invariant WaitB when {{ self.phase = 1 }} {{ self.x <= 2 ms }}\n\
             {claims}"
        ),
    )
}

/// The two-step closed subject (FR-204's `Gap`, FR-243-AC-5): `first` runs
/// by 1 ms and resets `x`, `second` runs exactly 2 ms later, and time is free
/// afterwards.
pub(super) fn two_step_body(claims: &str) -> String {
    format!(
        "pre First using v on M::Call::first {{ self.phase = 0 }}\n\
         post FirstDone using v on M::Call::first {{ self.phase = 1 and self.x = 0 ms }}\n\
         pre Second using v on M::Call::second {{ self.phase = 1 and self.x >= 2 ms }}\n\
         post SecondDone using v on M::Call::second {{ self.phase = 2 }}\n\
         time invariant Early when {{ self.phase = 0 }} {{ self.x <= 1 ms }}\n\
         time invariant Exact when {{ self.phase = 1 }} {{ self.x <= 2 ms }}\n\
         {claims}"
    )
}

pub(super) fn two_step(claims: &str) -> World {
    model(
        &timed_world::document(
            &["x"],
            &[("first", &["phase", "x"]), ("second", &["phase"])],
        ),
        &two_step_body(claims),
    )
}

/// QSL-3324's `Retx`: `send` needs `x >= 1 ms`, resets `x`, loses one time
/// in ten, and must run by `x = 2 ms` until delivery (`late`).
pub(super) fn retx(claims: &str) -> World {
    let document = with_lost(
        &timed_world::document(&["x"], &[("send", &["late", "x"])]),
        "send",
    );
    model(
        &document,
        &format!(
            "pre Send using v on M::Call::send {{ not self.late and self.x >= 1 ms }}\n\
             post Sent using v on M::Call::send {{ self.late = not lost and self.x = 0 ms }}\n\
             random M::Call::send(lost) ~ {{ true: 1, false: 9 }};\n\
             time invariant Deadline when {{ not self.late }} {{ self.x <= 2 ms }}\n\
             {claims}"
        ),
    )
}

/// `send` runs exactly at `at` ms.
pub(super) fn send_at(at: u32, claims: &str) -> World {
    model(
        &timed_world::document(&["x"], &[("send", &["phase"])]),
        &format!(
            "pre Send using v on M::Call::send {{ self.phase = 0 and self.x >= {at} ms }}\n\
             post Sent using v on M::Call::send {{ self.phase = 1 }}\n\
             time invariant Wait when {{ self.phase = 0 }} {{ self.x <= {at} ms }}\n\
             {claims}"
        ),
    )
}

/// QSL-3444: `first` runs strictly inside (0, 1) ms and resets `x`; `third`
/// needs `x < 1 ms` and `y > 1 ms`. With `bounded`, time is bounded until
/// `third` runs.
pub(super) fn two_clocks(bounded: bool, claims: &str) -> World {
    let invariant = if bounded {
        "time invariant Bounded when { self.phase < 2 } { self.y <= 3 ms }\n"
    } else {
        ""
    };
    model(
        &timed_world::document(
            &["x", "y"],
            &[("first", &["phase", "x"]), ("third", &["phase", "late"])],
        ),
        &format!(
            "pre First using v on M::Call::first {{ self.phase = 0 and self.x > 0 ms and self.x < 1 ms }}\n\
             post FirstDone using v on M::Call::first {{ self.phase = 1 and self.x = 0 ms }}\n\
             pre Third using v on M::Call::third {{ self.phase = 1 and self.x < 1 ms and self.y > 1 ms }}\n\
             post ThirdDone using v on M::Call::third {{ self.phase = 2 and self.late = true }}\n\
             {invariant}\
             {claims}"
        ),
    )
}

/// QSL-3445: `first` runs at a time `t` in [0, 1] ms, resets `x` and draws
/// `lost`. When lost, `inflight` holds from `t` and `late` from 2 ms; when
/// not, `inflight` holds from 1 ms and `late` from `t + 2` ms.
pub(super) fn skew(claims: &str) -> World {
    let document = with_lost(
        &timed_world::document(
            &["x", "y"],
            &[
                ("first", &["phase", "inflight", "x"]),
                ("start", &["phase", "inflight"]),
                ("left", &["phase", "late"]),
                ("right", &["phase", "late"]),
            ],
        ),
        "first",
    );
    model(
        &document,
        &format!(
            "pre First using v on M::Call::first {{ self.phase = 0 }}\n\
             post FirstDone using v on M::Call::first {{ self.phase = 1 and self.inflight = lost and self.x = 0 ms }}\n\
             random M::Call::first(lost) ~ {{ true: 1, false: 1 }};\n\
             pre Start using v on M::Call::start {{ self.phase = 1 and not self.inflight and self.y >= 1 ms }}\n\
             post Started using v on M::Call::start {{ self.phase = 2 and self.inflight = true }}\n\
             pre Left using v on M::Call::left {{ self.phase = 1 and self.inflight and self.y >= 2 ms }}\n\
             post LeftDone using v on M::Call::left {{ self.phase = 3 and self.late = true }}\n\
             pre Right using v on M::Call::right {{ self.phase = 2 and self.x >= 2 ms }}\n\
             post RightDone using v on M::Call::right {{ self.phase = 3 and self.late = true }}\n\
             time invariant Initial when {{ self.phase = 0 }} {{ self.y <= 1 ms }}\n\
             time invariant Lost when {{ self.phase = 1 and self.inflight }} {{ self.y <= 2 ms }}\n\
             time invariant Kept when {{ self.phase = 1 and not self.inflight }} {{ self.y <= 1 ms }}\n\
             time invariant Started when {{ self.phase = 2 }} {{ self.x <= 2 ms }}\n\
             {claims}"
        ),
    )
}

/// TC-709's `Send` fixture (QSL-3447, QSL-3463): `send` takes `phase` to 1
/// after `send` ms and, with `reply`, `reply` takes it to 2 that long after.
pub(super) fn send_reply(send: u32, reply: Option<u32>, claims: &str) -> World {
    let mut operations: Vec<(&str, &[&str])> = vec![("send", &["phase"])];
    let mut body = format!(
        "pre Send using v on M::Call::send {{ self.phase = 0 }}\n\
         post Sent using v on M::Call::send {{ self.phase = 1 }}\n\
         delay M::Call::send ~ discrete {{ {send} ms: 1 }};\n"
    );
    let mut weights = String::from("weight Call::send = 1;");
    if let Some(reply) = reply {
        operations.push(("reply", &["phase"]));
        body.push_str(&format!(
            "pre Reply using v on M::Call::reply {{ self.phase = 1 }}\n\
             post Replied using v on M::Call::reply {{ self.phase = 2 }}\n\
             delay M::Call::reply ~ discrete {{ {reply} ms: 1 }};\n"
        ));
        weights.push_str(" weight Call::reply = 1;");
    }
    model(
        &timed_world::document(&["x"], &operations),
        &format!("{body}workload Even on M {{ {weights} }}\n{claims}"),
    )
}

/// QSL-3448: `send` is drawn at exactly 5 ms and time stops there until it
/// runs.
pub(super) fn send_by_five(claims: &str) -> World {
    model(
        &timed_world::document(&["x"], &[("send", &["phase"])]),
        &format!(
            "pre Send using v on M::Call::send {{ self.phase = 0 }}\n\
             post Sent using v on M::Call::send {{ self.phase = 1 }}\n\
             delay M::Call::send ~ discrete {{ 5 ms: 1 }};\n\
             time invariant Wait when {{ self.phase = 0 }} {{ self.x <= 5 ms }}\n\
             workload Even on M {{ weight Call::send = 1; }}\n\
             {claims}"
        ),
    )
}

/// QSL-3468: no operation is ever enabled and time stops at the bound
/// `invariant` states.
pub(super) fn time_locked(invariant: &str, claims: &str) -> World {
    model(
        &timed_world::document(&["x"], &[("step", &["late"])]),
        &format!(
            "pre Step using v on M::Call::step {{ self.late and not self.late }}\n\
             post Stepped using v on M::Call::step {{ self.late = true }}\n\
             time invariant Stop when {{ true }} {{ {invariant} }}\n\
             {claims}"
        ),
    )
}
