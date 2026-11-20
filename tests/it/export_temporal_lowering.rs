// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-813 lowering of temporal clauses and fairness (FSL FR-1015,
//! FR-1016, FR-1017) over FR-756-AC-1's counter: the exact text of the
//! clause's definitions in each target, and the clause shapes the TLA+
//! encoding does not cover.

use crate::plan_export_surface::{COUNTER_UNIT, counter_request};
use ix_trace_rs::trace;
use serde_json::{Value, json};

const HEAD: &str =
    "temporal T using t over (c: Counter::Counter) clock \"model-steps\" on origin {";

#[derive(Clone, Copy)]
enum Target {
    Tla,
    Lean,
}

impl Target {
    fn profile(self) -> &'static str {
        match self {
            Self::Tla => "filament.output.tla2/v1",
            Self::Lean => "filament.output.lean4/v1",
        }
    }
}

/// Exports the clause `body` of the counter to `target`: the exit code and
/// the result document.
fn export(body: &str, target: Target) -> (u8, Value) {
    export_from(body, target, None)
}

/// As [`export`], from an initial snapshot whose counter holds `start`.
fn export_from(body: &str, target: Target, start: Option<i64>) -> (u8, Value) {
    export_full(
        body,
        target,
        start,
        "filament.temporal.infinite-trace/v1",
        true,
    )
}

/// As [`export`], with no `initial` snapshot: the request fixes no keys.
fn export_unfixed(body: &str, target: Target) -> (u8, Value) {
    export_full(
        body,
        target,
        None,
        "filament.temporal.infinite-trace/v1",
        false,
    )
}

/// As [`export_from`], under the temporal profile `profile`.
fn export_full(
    body: &str,
    target: Target,
    start: Option<i64>,
    profile: &str,
    fixed: bool,
) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    let path = counter_request(
        directory.path(),
        json!({"profile": target.profile(), "obligations": ["T"]}),
    );
    if let Some(start) = start {
        let file = directory.path().join("counter-0.json");
        let snapshot = std::fs::read_to_string(&file).unwrap();
        let zero = "\"integer\":\"0\"";
        assert!(snapshot.contains(zero), "{snapshot}");
        std::fs::write(
            &file,
            snapshot.replace(zero, &format!("\"integer\":\"{start}\"")),
        )
        .unwrap();
    }
    if !fixed {
        let file = path.clone();
        let mut request: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        request["request"]["export"]
            .as_object_mut()
            .unwrap()
            .remove("initial");
        std::fs::write(&file, serde_json::to_vec(&request).unwrap()).unwrap();
    }
    let unit = COUNTER_UNIT.replace(
        "model Counter",
        &format!("profile t = \"{profile}\";\nmodel Counter"),
    );
    std::fs::write(
        directory.path().join("unit.native"),
        format!("{unit}{HEAD} {body} }}\n"),
    )
    .unwrap();
    match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
        Ok(result) => (
            result.exit_code,
            serde_json::from_str(&result.value.to_string()).unwrap(),
        ),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

fn exported_text(body: &str, target: Target) -> String {
    exported_text_from(body, target, None)
}

fn exported_text_from(body: &str, target: Target, start: Option<i64>) -> String {
    let (exit, value) = export_from(body, target, start);
    assert_eq!(exit, 0, "{value}");
    value["export"]["target_text"]
        .as_str()
        .expect("target text")
        .to_owned()
}

/// The definitions the clause adds, without their `fsl:` comments: from the
/// first of the clause's own definitions to the end of the text.
fn clause_text(body: &str, target: Target) -> String {
    clause_text_from(body, target, None)
}

fn clause_text_from(body: &str, target: Target, start: Option<i64>) -> String {
    clause_of(&exported_text_from(body, target, start), target)
}

/// As [`clause_text`], for a request that fixes no keys.
fn clause_text_unfixed(body: &str, target: Target) -> String {
    let (exit, value) = export_unfixed(body, target);
    assert_eq!(exit, 0, "{value}");
    clause_of(value["export"]["target_text"].as_str().unwrap(), target)
}

/// The clause's definitions of `text`, from the identity universe on.
fn clause_of(text: &str, target: Target) -> String {
    let starts: &[&str] = match target {
        Target::Tla => &["Ids_Counter ==", "ok_T"],
        Target::Lean => &["def Ids_Counter"],
    };
    let from = starts
        .iter()
        .filter_map(|start| text.find(start))
        .min()
        .expect("the clause's definitions");
    text[from..]
        .lines()
        .filter(|line| !line.starts_with("\\* fsl: ") && !line.starts_with("-- fsl: "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// FSpec FR-813-AC-1: always as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-1")]
fn always_tla() {
    assert_eq!(
        clause_text("always holds(c.value <= 5)", Target::Tla),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

THEOREM Th_T == Spec => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((v_Counter[o_c].fld_value <= 5)))

===="#
    );
}

/// FSpec FR-813-AC-1: always as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-1")]
fn always_lean() {
    assert_eq!(
        clause_text("always holds(c.value <= 5)", Target::Lean),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value <= 5))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-7: `always` from a request that fixes no keys as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn always_unfixed_keys_tla() {
    assert_eq!(
        clause_text_unfixed("always holds(c.value <= 5)", Target::Tla,),
        r#"Ids_Counter == Keys_Counter

ok_T(o_c) == o_c \in DOMAIN v_Counter

THEOREM Th_T == Spec => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((v_Counter[o_c].fld_value <= 5)))

===="#
    );
}

/// FSpec FR-813-AC-7: `always` from a request that fixes no keys as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn always_unfixed_keys_lean() {
    assert_eq!(
        clause_text_unfixed("always holds(c.value <= 5)", Target::Lean,),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value <= 5))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-7: until as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn until_tla() {
    assert_eq!(
        clause_text("holds(c.value >= 1) until holds(c.value >= 3)", Target::Tla),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T>>

TypeOK_T ==
  /\ m_T \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ m_T = [o_c \in Ids_Counter |-> IF ((v_Counter[o_c].fld_value >= 3)) THEN "acc" ELSE IF ~((v_Counter[o_c].fld_value >= 1)) THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF ((v_Counter'[o_c].fld_value >= 3)) THEN "acc" ELSE IF ~((v_Counter'[o_c].fld_value >= 1)) THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []<>accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-7: until as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn until_lean() {
    assert_eq!(
        clause_text(
            "holds(c.value >= 1) until holds(c.value >= 3)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 1))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 3))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 0 <= d1 /\ (0 <= (i + d1) /\ at_T_1 (b.st (Int.toNat (i + d1))) o_c) /\ (forall e2 : Int, 0 <= e2 -> e2 < d1 -> (0 <= (i + e2) /\ at_T_0 (b.st (Int.toNat (i + e2))) o_c)))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-7: `until` from a request that fixes no keys as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn until_unfixed_keys_tla() {
    assert_eq!(
        clause_text_unfixed("holds(c.value >= 1) until holds(c.value >= 3)", Target::Tla,),
        r#"Ids_Counter == Keys_Counter

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T>>

TypeOK_T ==
  /\ m_T \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ m_T = [o_c \in Ids_Counter |-> IF ((v_Counter[o_c].fld_value >= 3)) THEN "acc" ELSE IF ~((v_Counter[o_c].fld_value >= 1)) THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF ((v_Counter'[o_c].fld_value >= 3)) THEN "acc" ELSE IF ~((v_Counter'[o_c].fld_value >= 1)) THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []<>accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-7: `until` from a request that fixes no keys as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn until_unfixed_keys_lean() {
    assert_eq!(
        clause_text_unfixed(
            "holds(c.value >= 1) until holds(c.value >= 3)",
            Target::Lean,
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 1))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 3))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 0 <= d1 /\ (0 <= (i + d1) /\ at_T_1 (b.st (Int.toNat (i + d1))) o_c) /\ (forall e2 : Int, 0 <= e2 -> e2 < d1 -> (0 <= (i + e2) /\ at_T_0 (b.st (Int.toNat (i + e2))) o_c)))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-3: an `always` over steps 0 to 3 as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn interval_always_tla() {
    assert_eq!(
        clause_text("always[0, 3] holds(c.value <= 5)", Target::Tla),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, n_T_0, m_T>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T \in [Ids_Counter -> {"run", "rel", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T = [o_c \in Ids_Counter |-> IF (0 <= 0 /\ 0 <= 3) /\ ~((v_Counter[o_c].fld_value <= 5)) THEN "rej" ELSE IF 0 >= 3 THEN "rel" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (0 <= n_T_0' /\ n_T_0' <= 3) /\ ~((v_Counter'[o_c].fld_value <= 5)) THEN "rej" ELSE IF n_T_0' >= 3 THEN "rel" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] /= "rej"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-3: an `always` over steps 0 to 3 as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn interval_always_lean() {
    assert_eq!(
        clause_text("always[0, 3] holds(c.value <= 5)", Target::Lean),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value <= 5))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> d1 <= 3 -> (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-21: two interval operators, each with its own counter as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn two_intervals_tla() {
    assert_eq!(
        clause_text(
            "always[0, 3] holds(c.value >= 4) and eventually[1, 2] holds(c.value = 5)",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, n_T_0, m_T_0, n_T_1, m_T_1>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T_0 \in [Ids_Counter -> {"run", "rel", "rej"}]
  /\ n_T_1 \in 0..2
  /\ m_T_1 \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T_0 = [o_c \in Ids_Counter |-> IF (0 <= 0 /\ 0 <= 3) /\ ~((v_Counter[o_c].fld_value >= 4)) THEN "rej" ELSE IF 0 >= 3 THEN "rel" ELSE "run"]
  /\ n_T_1 = 0
  /\ m_T_1 = [o_c \in Ids_Counter |-> IF (1 <= 0 /\ 0 <= 2) /\ (((0 <= 5) /\ (5 <= 9)) /\ (v_Counter[o_c].fld_value = 5)) THEN "acc" ELSE IF 0 >= 2 THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T_0' = [o_c \in Ids_Counter |-> IF m_T_0[o_c] /= "run" THEN m_T_0[o_c] ELSE IF (0 <= n_T_0' /\ n_T_0' <= 3) /\ ~((v_Counter'[o_c].fld_value >= 4)) THEN "rej" ELSE IF n_T_0' >= 3 THEN "rel" ELSE "run"]
  /\ n_T_1' = IF n_T_1 < 2 THEN n_T_1 + 1 ELSE 2
  /\ m_T_1' = [o_c \in Ids_Counter |-> IF m_T_1[o_c] /= "run" THEN m_T_1[o_c] ELSE IF (1 <= n_T_1' /\ n_T_1' <= 2) /\ (((0 <= 5) /\ (5 <= 9)) /\ (v_Counter'[o_c].fld_value = 5)) THEN "acc" ELSE IF n_T_1' >= 2 THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T_0(o_c) == m_T_0[o_c] /= "rej"

accept_T_1(o_c) == m_T_1[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []accept_T_0(o_c) /\ []<>accept_T_1(o_c)

===="#
    );
}

/// FSpec FR-813-AC-21: two interval operators, each with its own counter as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn two_intervals_lean() {
    assert_eq!(
        clause_text(
            "always[0, 3] holds(c.value >= 4) and eventually[1, 2] holds(c.value = 5)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 4))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 5) /\ (5 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 5))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  ((forall d1 : Int, 0 <= d1 -> d1 <= 3 -> (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c)) /\ (exists d2 : Int, 1 <= d2 /\ d2 <= 2 /\ (0 <= (i + d2) /\ at_T_1 (b.st (Int.toNat (i + d2))) o_c)))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-6: `eventually always`, stated directly as `<>[]` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
fn co_buchi_tla() {
    assert_eq!(
        clause_text("eventually always holds(c.value <= 5)", Target::Tla),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

THEOREM Th_T == Spec => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ <>([](((v_Counter[o_c].fld_value <= 5))))

===="#
    );
}

/// FSpec FR-813-AC-6: `eventually always`, stated directly as `<>[]` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
fn co_buchi_lean() {
    assert_eq!(
        clause_text("eventually always holds(c.value <= 5)", Target::Lean),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value <= 5))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 0 <= d1 /\ (forall d2 : Int, 0 <= d2 -> (0 <= ((i + d1) + d2) /\ at_T_0 (b.st (Int.toNat ((i + d1) + d2))) o_c)))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-23: `once` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn once_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value >= 2) implies once holds(c.value = 1))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T_h0>>

TypeOK_T ==
  /\ m_T_h0 \in [Ids_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ m_T_h0 = [o_c \in Ids_Counter |-> (((0 <= 1) /\ (1 <= 9)) /\ (v_Counter[o_c].fld_value = 1))]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h0' = [o_c \in Ids_Counter |-> ((((0 <= 1) /\ (1 <= 9)) /\ (v_Counter'[o_c].fld_value = 1)) \/ m_T_h0[o_c])]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []((((v_Counter[o_c].fld_value >= 2)) => m_T_h0[o_c]))

===="#
    );
}

/// FSpec FR-813-AC-23: `once` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn once_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value >= 2) implies once holds(c.value = 1))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 2))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 1) /\ (1 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 1))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (exists d2 : Int, 0 <= d2 /\ 0 <= ((i + d1) - d2) /\ (0 <= ((i + d1) - d2) /\ at_T_1 (b.st (Int.toNat ((i + d1) - d2))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-7: `release` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn release_tla() {
    assert_eq!(
        clause_text(
            "holds(c.value <= 4) release holds(c.value <= 5)",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T>>

TypeOK_T ==
  /\ m_T \in [Ids_Counter -> {"run", "rel", "rej"}]

Init_T ==
  /\ Init
  /\ m_T = [o_c \in Ids_Counter |-> IF ~((v_Counter[o_c].fld_value <= 5)) THEN "rej" ELSE IF ((v_Counter[o_c].fld_value <= 4)) THEN "rel" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF ~((v_Counter'[o_c].fld_value <= 5)) THEN "rej" ELSE IF ((v_Counter'[o_c].fld_value <= 4)) THEN "rel" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] /= "rej"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-7: `release` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn release_lean() {
    assert_eq!(
        clause_text(
            "holds(c.value <= 4) release holds(c.value <= 5)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value <= 4))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value <= 5))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  Not (exists d1 : Int, 0 <= d1 /\ Not ((0 <= (i + d1) /\ at_T_1 (b.st (Int.toNat (i + d1))) o_c)) /\ (forall e2 : Int, 0 <= e2 -> e2 < d1 -> Not ((0 <= (i + e2) /\ at_T_0 (b.st (Int.toNat (i + e2))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-3: an `eventually` over steps 1 to 3 as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn interval_eventually_tla() {
    assert_eq!(
        clause_text("eventually[1, 3] holds(c.value = 2)", Target::Tla),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, n_T_0, m_T>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T = [o_c \in Ids_Counter |-> IF (1 <= 0 /\ 0 <= 3) /\ (((0 <= 2) /\ (2 <= 9)) /\ (v_Counter[o_c].fld_value = 2)) THEN "acc" ELSE IF 0 >= 3 THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (1 <= n_T_0' /\ n_T_0' <= 3) /\ (((0 <= 2) /\ (2 <= 9)) /\ (v_Counter'[o_c].fld_value = 2)) THEN "acc" ELSE IF n_T_0' >= 3 THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []<>accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-3: an `eventually` over steps 1 to 3 as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn interval_eventually_lean() {
    assert_eq!(
        clause_text("eventually[1, 3] holds(c.value = 2)", Target::Lean),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 2) /\ (2 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 2))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 1 <= d1 /\ d1 <= 3 /\ (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-4: strong fairness as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-4")]
fn strong_fairness_tla() {
    assert_eq!(
        clause_text(
            "fair strong Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

Spec_T == Init /\ [][Next]_vars /\ WF_vars(Next) /\ SF_vars(\E r_self \in Ids_Counter : op_inc(r_self))

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](<>((((0 <= 2) /\ (2 <= 9)) /\ (v_Counter[o_c].fld_value = 2))))

===="#
    );
}

/// FSpec FR-813-AC-4: strong fairness as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-4")]
fn strong_fairness_lean() {
    assert_eq!(
        clause_text(
            "fair strong Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 2) /\ (2 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 2))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (exists d2 : Int, 0 <= d2 /\ (0 <= ((i + d1) + d2) /\ at_T_0 (b.st (Int.toNat ((i + d1) + d2))) o_c)))

def fair_T (b : Behaviour) : Prop :=
  ((forall i : Nat, exists j : Nat, i <= j /\ (exists t, exists r_self, op_inc (b.st j) t r_self)) -> (forall i : Nat, exists j : Nat, i <= j /\ (exists r_self, op_inc (b.st j) (b.st (j + 1)) r_self)))

def Ob_T : Prop :=
  forall b : Behaviour, fair_T b -> forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-22: weak `each` fairness as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-22")]
fn weak_each_fairness_tla() {
    assert_eq!(
        clause_text(
            "fair weak each Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

Spec_T == Init /\ [][Next]_vars /\ WF_vars(Next) /\ \A r_self \in Ids_Counter : WF_vars(op_inc(r_self))

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](<>((((0 <= 2) /\ (2 <= 9)) /\ (v_Counter[o_c].fld_value = 2))))

===="#
    );
}

/// FSpec FR-813-AC-22: weak `each` fairness as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-22")]
fn weak_each_fairness_lean() {
    assert_eq!(
        clause_text(
            "fair weak each Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 2) /\ (2 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 2))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (exists d2 : Int, 0 <= d2 /\ (0 <= ((i + d1) + d2) /\ at_T_0 (b.st (Int.toNat ((i + d1) + d2))) o_c)))

def fair_T (b : Behaviour) : Prop :=
  (forall r_self, Ids_Counter r_self -> (forall i : Nat, (forall j : Nat, i <= j -> (exists t, op_inc (b.st j) t r_self)) -> (exists j : Nat, i <= j /\ (op_inc (b.st j) (b.st (j + 1)) r_self))))

def Ob_T : Prop :=
  forall b : Behaviour, fair_T b -> forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-22: weak `each` fairness over a monitored clause as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-22")]
fn monitored_each_fairness_tla() {
    assert_eq!(
        clause_text(
            "fair weak each Counter::Counter::inc; always eventually (holds(c.value = 2) since holds(c.value = 1))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T_h0>>

TypeOK_T ==
  /\ m_T_h0 \in [Ids_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ m_T_h0 = [o_c \in Ids_Counter |-> (((0 <= 1) /\ (1 <= 9)) /\ (v_Counter[o_c].fld_value = 1))]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h0' = [o_c \in Ids_Counter |-> ((((0 <= 1) /\ (1 <= 9)) /\ (v_Counter'[o_c].fld_value = 1)) \/ ((((0 <= 2) /\ (2 <= 9)) /\ (v_Counter'[o_c].fld_value = 2)) /\ m_T_h0[o_c]))]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T) /\ \A r_self \in Ids_Counter : WF_vars(op_inc(r_self))

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](<>(m_T_h0[o_c]))

===="#
    );
}

/// FSpec FR-813-AC-22: weak `each` fairness over a monitored clause as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-22")]
fn monitored_each_fairness_lean() {
    assert_eq!(
        clause_text(
            "fair weak each Counter::Counter::inc; always eventually (holds(c.value = 2) since holds(c.value = 1))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 2) /\ (2 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 2))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 1) /\ (1 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 1))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (exists d2 : Int, 0 <= d2 /\ (exists d3 : Int, 0 <= d3 /\ 0 <= (((i + d1) + d2) - d3) /\ (0 <= (((i + d1) + d2) - d3) /\ at_T_1 (b.st (Int.toNat (((i + d1) + d2) - d3))) o_c) /\ (forall e4 : Int, 0 <= e4 -> e4 < d3 -> (0 <= (((i + d1) + d2) - e4) /\ at_T_0 (b.st (Int.toNat (((i + d1) + d2) - e4))) o_c)))))

def fair_T (b : Behaviour) : Prop :=
  (forall r_self, Ids_Counter r_self -> (forall i : Nat, (forall j : Nat, i <= j -> (exists t, op_inc (b.st j) t r_self)) -> (exists j : Nat, i <= j /\ (op_inc (b.st j) (b.st (j + 1)) r_self))))

def Ob_T : Prop :=
  forall b : Behaviour, fair_T b -> forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-4: weak fairness without `each` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-4")]
fn weak_fairness_tla() {
    assert_eq!(
        clause_text(
            "fair weak Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

Spec_T == Init /\ [][Next]_vars /\ WF_vars(Next) /\ WF_vars(\E r_self \in Ids_Counter : op_inc(r_self))

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](<>((((0 <= 2) /\ (2 <= 9)) /\ (v_Counter[o_c].fld_value = 2))))

===="#
    );
}

/// FSpec FR-813-AC-4: weak fairness without `each` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-4")]
fn weak_fairness_lean() {
    assert_eq!(
        clause_text(
            "fair weak Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 2) /\ (2 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 2))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (exists d2 : Int, 0 <= d2 /\ (0 <= ((i + d1) + d2) /\ at_T_0 (b.st (Int.toNat ((i + d1) + d2))) o_c)))

def fair_T (b : Behaviour) : Prop :=
  (forall i : Nat, (forall j : Nat, i <= j -> (exists t, exists r_self, op_inc (b.st j) t r_self)) -> (exists j : Nat, i <= j /\ (exists r_self, op_inc (b.st j) (b.st (j + 1)) r_self)))

def Ob_T : Prop :=
  forall b : Behaviour, fair_T b -> forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-23: `historically` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn historically_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies historically holds(c.value >= 0))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T_h0>>

TypeOK_T ==
  /\ m_T_h0 \in [Ids_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ m_T_h0 = [o_c \in Ids_Counter |-> ~(((v_Counter[o_c].fld_value >= 0)))]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h0' = [o_c \in Ids_Counter |-> (~(((v_Counter'[o_c].fld_value >= 0))) \/ m_T_h0[o_c])]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((((0 <= 5) /\ (5 <= 9)) /\ (v_Counter[o_c].fld_value = 5)) => ~(m_T_h0[o_c])))

===="#
    );
}

/// FSpec FR-813-AC-23: `historically` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn historically_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies historically holds(c.value >= 0))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 5) /\ (5 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 5))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 0))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (forall d2 : Int, 0 <= d2 -> 0 <= ((i + d1) - d2) -> (0 <= ((i + d1) - d2) /\ at_T_1 (b.st (Int.toNat ((i + d1) - d2))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-23: `since` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn since_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies (holds(c.value >= 1) since holds(c.value = 0)))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T_h0>>

TypeOK_T ==
  /\ m_T_h0 \in [Ids_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ m_T_h0 = [o_c \in Ids_Counter |-> (((0 <= 0) /\ (0 <= 9)) /\ (v_Counter[o_c].fld_value = 0))]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h0' = [o_c \in Ids_Counter |-> ((((0 <= 0) /\ (0 <= 9)) /\ (v_Counter'[o_c].fld_value = 0)) \/ (((v_Counter'[o_c].fld_value >= 1)) /\ m_T_h0[o_c]))]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((((0 <= 2) /\ (2 <= 9)) /\ (v_Counter[o_c].fld_value = 2)) => m_T_h0[o_c]))

===="#
    );
}

/// FSpec FR-813-AC-23: `since` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn since_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies (holds(c.value >= 1) since holds(c.value = 0)))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 2) /\ (2 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 2))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 1))

def at_T_2 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 0) /\ (0 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 0))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (exists d2 : Int, 0 <= d2 /\ 0 <= ((i + d1) - d2) /\ (0 <= ((i + d1) - d2) /\ at_T_2 (b.st (Int.toNat ((i + d1) - d2))) o_c) /\ (forall e3 : Int, 0 <= e3 -> e3 < d2 -> (0 <= ((i + d1) - e3) /\ at_T_1 (b.st (Int.toNat ((i + d1) - e3))) o_c)))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-23: `triggered` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn triggered_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies (holds(c.value >= 0) triggered holds(c.value <= 5)))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T_h0>>

TypeOK_T ==
  /\ m_T_h0 \in [Ids_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ m_T_h0 = [o_c \in Ids_Counter |-> ~(((v_Counter[o_c].fld_value <= 5)))]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h0' = [o_c \in Ids_Counter |-> (~(((v_Counter'[o_c].fld_value <= 5))) \/ (~(((v_Counter'[o_c].fld_value >= 0))) /\ m_T_h0[o_c]))]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((((0 <= 2) /\ (2 <= 9)) /\ (v_Counter[o_c].fld_value = 2)) => ~(m_T_h0[o_c])))

===="#
    );
}

/// FSpec FR-813-AC-23: `triggered` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-23")]
fn triggered_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies (holds(c.value >= 0) triggered holds(c.value <= 5)))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 2) /\ (2 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 2))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 0))

def at_T_2 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value <= 5))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> Not (exists d2 : Int, 0 <= d2 /\ 0 <= ((i + d1) - d2) /\ Not ((0 <= ((i + d1) - d2) /\ at_T_2 (b.st (Int.toNat ((i + d1) - d2))) o_c)) /\ (forall e3 : Int, 0 <= e3 -> e3 < d2 -> Not ((0 <= ((i + d1) - e3) /\ at_T_1 (b.st (Int.toNat ((i + d1) - e3))) o_c))))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-7: `until` over steps 1 to 3 as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn bounded_until_tla() {
    assert_eq!(
        clause_text(
            "holds(c.value >= 1) until[1, 3] holds(c.value >= 3)",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, n_T_0, m_T>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T = [o_c \in Ids_Counter |-> IF (1 <= 0 /\ 0 <= 3) /\ ((v_Counter[o_c].fld_value >= 3)) THEN "acc" ELSE IF (1 <= 0) /\ ~((v_Counter[o_c].fld_value >= 1)) THEN "rej" ELSE IF 0 >= 3 THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (1 <= n_T_0' /\ n_T_0' <= 3) /\ ((v_Counter'[o_c].fld_value >= 3)) THEN "acc" ELSE IF (1 <= n_T_0') /\ ~((v_Counter'[o_c].fld_value >= 1)) THEN "rej" ELSE IF n_T_0' >= 3 THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []<>accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-7: `until` over steps 1 to 3 as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn bounded_until_lean() {
    assert_eq!(
        clause_text(
            "holds(c.value >= 1) until[1, 3] holds(c.value >= 3)",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 1))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 3))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 1 <= d1 /\ d1 <= 3 /\ (0 <= (i + d1) /\ at_T_1 (b.st (Int.toNat (i + d1))) o_c) /\ (forall e2 : Int, 1 <= e2 -> e2 < d1 -> (0 <= (i + e2) /\ at_T_0 (b.st (Int.toNat (i + e2))) o_c)))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-9: `once[2, *]` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-9")]
fn once_from_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies once[2, *] holds(c.value = 1))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T_c0>>

TypeOK_T ==
  /\ m_T_c0 \in [Ids_Counter -> -1..2]

Init_T ==
  /\ Init
  /\ m_T_c0 = [o_c \in Ids_Counter |-> IF (((0 <= 1) /\ (1 <= 9)) /\ (v_Counter[o_c].fld_value = 1)) THEN 0 ELSE -1]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_c0' = [o_c \in Ids_Counter |-> IF m_T_c0[o_c] = -1 THEN (IF (((0 <= 1) /\ (1 <= 9)) /\ (v_Counter'[o_c].fld_value = 1)) THEN 0 ELSE -1) ELSE IF m_T_c0[o_c] < 2 THEN m_T_c0[o_c] + 1 ELSE 2]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((((0 <= 5) /\ (5 <= 9)) /\ (v_Counter[o_c].fld_value = 5)) => m_T_c0[o_c] = 2))

===="#
    );
}

/// FSpec FR-813-AC-9: `once[2, *]` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-9")]
fn once_from_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies once[2, *] holds(c.value = 1))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 5) /\ (5 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 5))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 1) /\ (1 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 1))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (exists d2 : Int, 2 <= d2 /\ 0 <= ((i + d1) - d2) /\ (0 <= ((i + d1) - d2) /\ at_T_1 (b.st (Int.toNat ((i + d1) - d2))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-10: `since[2, *]` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-10")]
fn since_from_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies (holds(c.value >= 1) since[2, *] holds(c.value = 1)))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T_h0, m_T_r0>>

TypeOK_T ==
  /\ m_T_h0 \in [Ids_Counter -> BOOLEAN]
  /\ m_T_r0 \in [Ids_Counter -> [1..2 -> BOOLEAN]]

Init_T ==
  /\ Init
  /\ m_T_h0 = [o_c \in Ids_Counter |-> (((0 <= 1) /\ (1 <= 9)) /\ (v_Counter[o_c].fld_value = 1))]
  /\ m_T_r0 = [o_c \in Ids_Counter |-> [k \in 1..2 |-> FALSE]]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h0' = [o_c \in Ids_Counter |-> ((((0 <= 1) /\ (1 <= 9)) /\ (v_Counter'[o_c].fld_value = 1)) \/ (((v_Counter'[o_c].fld_value >= 1)) /\ m_T_h0[o_c]))]
  /\ m_T_r0' = [o_c \in Ids_Counter |-> [k \in 1..2 |-> IF k = 1 THEN m_T_h0[o_c] ELSE m_T_r0[o_c][k - 1]]]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((((0 <= 5) /\ (5 <= 9)) /\ (v_Counter[o_c].fld_value = 5)) => m_T_r0[o_c][2]))

===="#
    );
}

/// FSpec FR-813-AC-10: `since[2, *]` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-10")]
fn since_from_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies (holds(c.value >= 1) since[2, *] holds(c.value = 1)))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 5) /\ (5 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 5))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 1))

def at_T_2 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 1) /\ (1 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 1))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (exists d2 : Int, 2 <= d2 /\ 0 <= ((i + d1) - d2) /\ (0 <= ((i + d1) - d2) /\ at_T_2 (b.st (Int.toNat ((i + d1) - d2))) o_c) /\ (forall e3 : Int, 2 <= e3 -> e3 < d2 -> (0 <= ((i + d1) - e3) /\ at_T_1 (b.st (Int.toNat ((i + d1) - e3))) o_c)))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-8: `once[1, 3]` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-8")]
fn once_within_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies once[1, 3] holds(c.value = 1))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T_r0>>

TypeOK_T ==
  /\ m_T_r0 \in [Ids_Counter -> [1..3 -> BOOLEAN]]

Init_T ==
  /\ Init
  /\ m_T_r0 = [o_c \in Ids_Counter |-> [k \in 1..3 |-> FALSE]]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_r0' = [o_c \in Ids_Counter |-> [k \in 1..3 |-> IF k = 1 THEN (((0 <= 1) /\ (1 <= 9)) /\ (v_Counter[o_c].fld_value = 1)) ELSE m_T_r0[o_c][k - 1]]]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((((0 <= 5) /\ (5 <= 9)) /\ (v_Counter[o_c].fld_value = 5)) => ((\E k \in 1..3 : m_T_r0[o_c][k]))))

===="#
    );
}

/// FSpec FR-813-AC-8: `once[1, 3]` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-8")]
fn once_within_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 5) implies once[1, 3] holds(c.value = 1))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 5) /\ (5 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 5))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 1) /\ (1 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 1))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (exists d2 : Int, 1 <= d2 /\ d2 <= 3 /\ (0 <= ((i + d1) - d2) /\ at_T_1 (b.st (Int.toNat ((i + d1) - d2))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-10: `once[1, 1]` over a `since` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-10")]
fn nested_past_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies once[1, 1] (holds(c.value = 1) since holds(c.value = 0)))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T_h1, m_T_r0>>

TypeOK_T ==
  /\ m_T_h1 \in [Ids_Counter -> BOOLEAN]
  /\ m_T_r0 \in [Ids_Counter -> [1..1 -> BOOLEAN]]

Init_T ==
  /\ Init
  /\ m_T_h1 = [o_c \in Ids_Counter |-> (((0 <= 0) /\ (0 <= 9)) /\ (v_Counter[o_c].fld_value = 0))]
  /\ m_T_r0 = [o_c \in Ids_Counter |-> [k \in 1..1 |-> FALSE]]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_h1' = [o_c \in Ids_Counter |-> ((((0 <= 0) /\ (0 <= 9)) /\ (v_Counter'[o_c].fld_value = 0)) \/ ((((0 <= 1) /\ (1 <= 9)) /\ (v_Counter'[o_c].fld_value = 1)) /\ m_T_h1[o_c]))]
  /\ m_T_r0' = [o_c \in Ids_Counter |-> [k \in 1..1 |-> IF k = 1 THEN m_T_h1[o_c] ELSE m_T_r0[o_c][k - 1]]]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ [](((((0 <= 2) /\ (2 <= 9)) /\ (v_Counter[o_c].fld_value = 2)) => ((\E k \in 1..1 : m_T_r0[o_c][k]))))

===="#
    );
}

/// FSpec FR-813-AC-10: `once[1, 1]` over a `since` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-10")]
fn nested_past_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value = 2) implies once[1, 1] (holds(c.value = 1) since holds(c.value = 0)))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 2) /\ (2 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 2))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 1) /\ (1 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 1))

def at_T_2 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 0) /\ (0 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 0))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c) -> (exists d2 : Int, 1 <= d2 /\ d2 <= 1 /\ (exists d3 : Int, 0 <= d3 /\ 0 <= (((i + d1) - d2) - d3) /\ (0 <= (((i + d1) - d2) - d3) /\ at_T_2 (b.st (Int.toNat (((i + d1) - d2) - d3))) o_c) /\ (forall e4 : Int, 0 <= e4 -> e4 < d3 -> (0 <= (((i + d1) - d2) - e4) /\ at_T_1 (b.st (Int.toNat (((i + d1) - d2) - e4))) o_c))))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-21: `always[2, 3]` from a snapshot at value 4 as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn terminal_always_tla() {
    assert_eq!(
        clause_text_from("always[2, 3] holds(c.value = 5)", Target::Tla, Some(4),),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, n_T_0, m_T>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T \in [Ids_Counter -> {"run", "rel", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T = [o_c \in Ids_Counter |-> IF (2 <= 0 /\ 0 <= 3) /\ ~(((0 <= 5) /\ (5 <= 9)) /\ (v_Counter[o_c].fld_value = 5)) THEN "rej" ELSE IF 0 >= 3 THEN "rel" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (2 <= n_T_0' /\ n_T_0' <= 3) /\ ~(((0 <= 5) /\ (5 <= 9)) /\ (v_Counter'[o_c].fld_value = 5)) THEN "rej" ELSE IF n_T_0' >= 3 THEN "rel" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] /= "rej"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-21: `always[2, 3]` from a snapshot at value 4 as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn terminal_always_lean() {
    assert_eq!(
        clause_text_from("always[2, 3] holds(c.value = 5)", Target::Lean, Some(4),),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 5) /\ (5 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 5))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 2 <= d1 -> d1 <= 3 -> (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-21: `eventually[2, 3]` of the terminal value as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn terminal_eventually_tla() {
    assert_eq!(
        clause_text_from("eventually[2, 3] holds(c.value = 5)", Target::Tla, Some(4),),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, n_T_0, m_T>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T = [o_c \in Ids_Counter |-> IF (2 <= 0 /\ 0 <= 3) /\ (((0 <= 5) /\ (5 <= 9)) /\ (v_Counter[o_c].fld_value = 5)) THEN "acc" ELSE IF 0 >= 3 THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (2 <= n_T_0' /\ n_T_0' <= 3) /\ (((0 <= 5) /\ (5 <= 9)) /\ (v_Counter'[o_c].fld_value = 5)) THEN "acc" ELSE IF n_T_0' >= 3 THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []<>accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-21: `eventually[2, 3]` of the terminal value as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn terminal_eventually_lean() {
    assert_eq!(
        clause_text_from("eventually[2, 3] holds(c.value = 5)", Target::Lean, Some(4),),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 5) /\ (5 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 5))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 2 <= d1 /\ d1 <= 3 /\ (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-21: `eventually[2, 3]` of a value that is left as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn terminal_eventually_missed_tla() {
    assert_eq!(
        clause_text_from("eventually[2, 3] holds(c.value = 4)", Target::Tla, Some(4),),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, n_T_0, m_T>>

TypeOK_T ==
  /\ n_T_0 \in 0..3
  /\ m_T \in [Ids_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ n_T_0 = 0
  /\ m_T = [o_c \in Ids_Counter |-> IF (2 <= 0 /\ 0 <= 3) /\ (((0 <= 4) /\ (4 <= 9)) /\ (v_Counter[o_c].fld_value = 4)) THEN "acc" ELSE IF 0 >= 3 THEN "rej" ELSE "run"]
  /\ TypeOK_T

Mon_T ==
  /\ n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3
  /\ m_T' = [o_c \in Ids_Counter |-> IF m_T[o_c] /= "run" THEN m_T[o_c] ELSE IF (2 <= n_T_0' /\ n_T_0' <= 3) /\ (((0 <= 4) /\ (4 <= 9)) /\ (v_Counter'[o_c].fld_value = 4)) THEN "acc" ELSE IF n_T_0' >= 3 THEN "rej" ELSE "run"]

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == m_T[o_c] = "acc"

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ []<>accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-21: `eventually[2, 3]` of a value that is left as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-21")]
fn terminal_eventually_missed_lean() {
    assert_eq!(
        clause_text_from("eventually[2, 3] holds(c.value = 4)", Target::Lean, Some(4),),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((0 <= 4) /\ (4 <= 9)) /\ ((F_get_Counter o_c s.v_Counter).fld_value = 4))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 2 <= d1 /\ d1 <= 3 /\ (0 <= (i + d1) /\ at_T_0 (b.st (Int.toNat (i + d1))) o_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-6: a nested future operator as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
fn nested_future_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value >= 1) until holds(c.value >= 3))",
            Target::Tla
        ),
        r#"Ids_Counter == {<<99>>}

ok_T(o_c) == o_c \in DOMAIN v_Counter

vars_T == <<vars, m_T_g0, m_T_g1>>

TypeOK_T ==
  /\ m_T_g0 \in [Ids_Counter -> BOOLEAN]
  /\ m_T_g1 \in [Ids_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ \A o_c \in Ids_Counter : m_T_g0[o_c]
  /\ TypeOK_T

Mon_T ==
  /\ m_T_g0' \in [Ids_Counter -> BOOLEAN]
  /\ m_T_g1' \in [Ids_Counter -> BOOLEAN]
  /\ \A o_c \in Ids_Counter : (m_T_g0[o_c] <=> (m_T_g1[o_c] /\ m_T_g0'[o_c])) /\ (m_T_g1[o_c] <=> (((v_Counter[o_c].fld_value >= 3)) \/ (((v_Counter[o_c].fld_value >= 1)) /\ m_T_g1'[o_c])))

Next_T == (Next \/ (~ENABLED Next /\ UNCHANGED vars)) /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T(o_c) == []<>(~m_T_g1[o_c] \/ ((v_Counter[o_c].fld_value >= 3)))

THEOREM Th_T == Spec_T => \A o_c \in Ids_Counter : [](ok_T(o_c)) /\ accept_T(o_c)

===="#
    );
}

/// FSpec FR-813-AC-6: a nested future operator as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
fn nested_future_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value >= 1) until holds(c.value >= 3))",
            Target::Lean
        ),
        r#"def Ids_Counter (k : List Nat) : Prop := exists s : State, Init s /\ F_has_Counter k s.v_Counter

def ok_T (s : State) (o_c : List Nat) : Prop := F_has_Counter o_c s.v_Counter

def at_T_0 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 1))

def at_T_1 (s : State) (o_c : List Nat) : Prop :=
  (((F_get_Counter o_c s.v_Counter).fld_value >= 3))

def holds_T (b : Behaviour) (o_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (exists d2 : Int, 0 <= d2 /\ (0 <= ((i + d1) + d2) /\ at_T_1 (b.st (Int.toNat ((i + d1) + d2))) o_c) /\ (forall e3 : Int, 0 <= e3 -> e3 < d2 -> (0 <= ((i + d1) + e3) /\ at_T_0 (b.st (Int.toNat ((i + d1) + e3))) o_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall o_c, Ids_Counter o_c -> (forall j : Nat, ok_T (b.st j) o_c) /\ holds_T b o_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-5: the Lean file states obligations as propositions and
/// has no `theorem`, proof term or `sorry`, for every operator.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-5")]
fn the_lean_text_has_no_theorem_proof_or_sorry() {
    for body in [
        "always holds(c.value <= 5)",
        "holds(c.value >= 1) until holds(c.value >= 3)",
        "always (holds(c.value >= 2) implies once holds(c.value = 1))",
        "always (holds(c.value >= 2) implies historically holds(c.value >= 0))",
        "holds(c.value <= 4) release holds(c.value <= 5)",
        "fair weak each Counter::Counter::inc; eventually holds(c.value = 2)",
    ] {
        let text = exported_text(body, Target::Lean);
        for word in ["theorem", "THEOREM", "sorry", ":= by", "by simp"] {
            assert!(!text.contains(word), "{word}: {text}");
        }
        assert!(text.contains("def Ob_T : Prop :="), "{text}");
        assert!(text.is_ascii(), "{text}");
    }
}

/// FSpec FR-813-AC-1/AC-2/AC-6 and FR-811: each deterministic clause is
/// `preserved` with no cause and claims regions of the text that state it.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-1")]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-2")]
fn a_lowered_temporal_clause_is_preserved_and_claims_its_regions() {
    for target in [Target::Tla, Target::Lean] {
        for body in [
            "always holds(c.value <= 5)",
            "holds(c.value >= 1) until holds(c.value >= 3)",
            "eventually always holds(c.value <= 5)",
            "always[2, 3] holds(c.value = 5)",
        ] {
            let (exit, value) = export(body, target);
            assert_eq!(exit, 0, "{value}");
            let record = &value["export"]["records"][0];
            assert_eq!(record["disposition"], "preserved", "{value}");
            assert_eq!(record["causes"], json!([]), "{value}");
            let text = value["export"]["target_text"].as_str().unwrap();
            let regions = record["regions"].as_array().expect("regions");
            assert!(!regions.is_empty(), "{value}");
            for region in regions {
                let start = usize::try_from(region[0].as_u64().unwrap()).unwrap();
                let end = usize::try_from(region[1].as_u64().unwrap()).unwrap();
                let region = &text[start..end];
                assert!(region.contains('T') || region.contains("Ids_"), "{region}");
            }
        }
    }
}

/// FSpec FR-813-AC-6: a clause with no deterministic monitor exports to TLA+
/// `conditional` with `nondeterministic-monitor` naming the clause, and to
/// Lean `preserved`.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
fn a_nested_future_operator_is_conditional_in_tla_and_preserved_in_lean() {
    let body = "always (holds(c.value >= 1) until holds(c.value >= 3))";
    let (exit, value) = export(body, Target::Tla);
    assert_eq!(exit, 0, "{value}");
    let record = &value["export"]["records"][0];
    assert_eq!(record["disposition"], "conditional", "{value}");
    let condition = &record["conditions"][0];
    assert_eq!(condition["type"], "nondeterministic-monitor", "{value}");
    assert_eq!(condition["clause"], "T", "{value}");
    assert!(condition["node"].is_string(), "{value}");
    assert_eq!(record["conditions"].as_array().unwrap().len(), 1, "{value}");
    let (exit, value) = export(body, Target::Lean);
    assert_eq!(exit, 0, "{value}");
    assert_eq!(
        value["export"]["records"][0]["disposition"], "preserved",
        "{value}"
    );
}

/// FSpec FR-813-AC-3: the step counter of `always[0, 3]` is bounded by the
/// interval's upper bound, and the monitor advances on the terminal
/// stuttering step (`Next_T` stutters where `Next` is disabled), so inserted
/// stuttering steps leave the verdict unchanged.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn the_step_counter_is_bounded_and_stuttering_keeps_the_verdict() {
    let text = clause_text("always[0, 3] holds(c.value <= 5)", Target::Tla);
    assert!(text.contains("n_T_0 \\in 0..3"), "{text}");
    assert!(
        text.contains("Next_T == (Next \\/ (~ENABLED Next /\\ UNCHANGED vars)) /\\ Mon_T"),
        "{text}"
    );
    assert!(
        text.contains("n_T_0' = IF n_T_0 < 3 THEN n_T_0 + 1 ELSE 3"),
        "{text}"
    );
}

/// FSpec FR-813-AC-24 and FR-817-AC-16: a clause under a step-counting
/// profile other than `filament.temporal.infinite-trace/v1` is the typed
/// `unimplemented_capability` refusal naming the construct, exit 21, with a
/// node and a locus, never exit 30.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-24")]
fn a_clause_under_another_profile_is_an_unimplemented_capability() {
    for target in [Target::Tla, Target::Lean] {
        let (exit, value) = export_full(
            "always[0, 3] holds(c.value <= 5)",
            target,
            None,
            "filament.temporal.event-position.false-extension/v1",
            true,
        );
        assert_eq!(exit, 21, "{value}");
        let refusal = &value["export"]["refusal"];
        assert_eq!(refusal["code"], "unimplemented_capability", "{value}");
        assert_eq!(refusal["cause"], "unimplemented-meaning", "{value}");
        assert_eq!(refusal["construct"], "temporal/temporal_clause", "{value}");
        let family = if matches!(target, Target::Tla) {
            "tla"
        } else {
            "lean"
        };
        assert_eq!(
            refusal["capability"],
            format!("export:{family}:temporal/temporal_clause"),
            "{value}"
        );
        assert!(
            refusal["node"].is_string() && refusal["locus"].is_object(),
            "{value}"
        );
        assert_eq!(value["export"]["status"], "refused", "{value}");
    }
}

/// FSpec FR-813-AC-1/AC-7/AC-22: the identity universe is the snapshot's key
/// set, and no temporal definition holds `DOMAIN v_` outside `ok_T`.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-22")]
fn the_identity_universe_is_the_snapshot_key_set() {
    let text = exported_text("always holds(c.value <= 5)", Target::Tla);
    assert!(text.contains("Ids_Counter == {<<99>>}"), "{text}");
    assert!(!text.contains("CONSTANT"), "{text}");
    for body in [
        "holds(c.value >= 1) until holds(c.value >= 3)",
        "always (holds(c.value >= 2) implies once[1, 2] holds(c.value = 1))",
        "fair weak each Counter::Counter::inc; always eventually holds(c.value = 2)",
    ] {
        let clause = clause_text(body, Target::Tla);
        let outside: Vec<&str> = clause
            .lines()
            .filter(|line| line.contains("DOMAIN v_") && !line.starts_with("ok_T("))
            .collect();
        assert!(outside.is_empty(), "{outside:?}");
    }
}

/// FSpec FR-813-AC-7: a request that fixes no keys exports the constant
/// `Keys_Counter` with its assumption and `TypeOK` bound, its TLA+ record is
/// `conditional` with exactly the `bounded-identity-universe` condition, its
/// Lean record stays `preserved`, and `Seq(Nat)` is no quantifier range.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-7")]
fn an_unfixed_key_set_is_a_constant_and_a_conditional_record() {
    let body = "holds(c.value >= 1) until holds(c.value >= 3)";
    let (exit, value) = export_unfixed(body, Target::Tla);
    assert_eq!(exit, 0, "{value}");
    let text = value["export"]["target_text"].as_str().unwrap();
    assert!(text.contains("CONSTANTS Keys_Counter\n"), "{text}");
    assert!(
        text.contains("ASSUME Keys_Counter \\subseteq Seq(Nat) /\\ IsFiniteSet(Keys_Counter)"),
        "{text}"
    );
    assert!(
        text.contains("DOMAIN v_Counter \\subseteq Keys_Counter"),
        "{text}"
    );
    assert!(text.contains("Ids_Counter == Keys_Counter"), "{text}");
    for line in text.lines() {
        assert!(
            !(line.contains("\\in Seq(Nat)") || line.contains("[Seq(Nat) ->")),
            "{line}"
        );
    }
    let record = &value["export"]["records"][0];
    assert_eq!(record["disposition"], "conditional", "{value}");
    let conditions = record["conditions"].as_array().unwrap();
    assert_eq!(conditions.len(), 1, "{value}");
    assert_eq!(
        conditions[0]["type"], "bounded-identity-universe",
        "{value}"
    );
    assert_eq!(conditions[0]["type_name"], "Counter", "{value}");
    assert!(conditions[0]["node"].is_string(), "{value}");
    assert_eq!(conditions[0].as_object().unwrap().len(), 3, "{value}");
    let (exit, value) = export_unfixed(body, Target::Lean);
    assert_eq!(exit, 0, "{value}");
    assert_eq!(
        value["export"]["records"][0]["disposition"], "preserved",
        "{value}"
    );
}
