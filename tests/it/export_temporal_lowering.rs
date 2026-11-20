// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-813 lowering of temporal clauses and fairness (FSL FR-1015,
//! FR-1016, FR-1017) over FR-756-AC-1's counter: the exact text of the
//! clause's definitions in each target, and the clause shapes the TLA+
//! encoding does not cover.

use crate::plan_export_surface::{counter_request, COUNTER_UNIT};
use ix_trace_rs::trace;
use serde_json::{json, Value};

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
    let directory = tempfile::tempdir().unwrap();
    let path = counter_request(
        directory.path(),
        json!({"profile": target.profile(), "obligations": ["T"]}),
    );
    let unit = COUNTER_UNIT.replace(
        "model Counter",
        "profile t = \"filament.temporal.infinite-trace/v1\";\nmodel Counter",
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
    let (exit, value) = export(body, target);
    assert_eq!(exit, 0, "{value}");
    value["export"]["target_text"]
        .as_str()
        .expect("target text")
        .to_owned()
}

/// The definitions the clause adds, without their `fsl:` comments: from the
/// first of the clause's own definitions to the end of the text.
fn clause_text(body: &str, target: Target) -> String {
    let text = exported_text(body, target);
    let starts: &[&str] = match target {
        Target::Tla => &["vars_T", "Spec_T", "THEOREM"],
        Target::Lean => &["def a_T_0"],
    };
    let from = starts
        .iter()
        .find_map(|start| text.find(start))
        .expect("the clause's definitions");
    text[from..]
        .lines()
        .filter(|line| !line.starts_with("\\* fsl: ") && !line.starts_with("-- fsl: "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// FSpec FR-813-AC-1: `always holds(c.value <= 5)` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-1")]
fn always_tla() {
    assert_eq!(
        clause_text("always holds(c.value <= 5)", Target::Tla,),
        r#"THEOREM Th_T == Spec => \A p_c \in DOMAIN v_Counter : [](((v_Counter[p_c].f_value <= 5)))

===="#
    );
}

/// FSpec FR-813-AC-1: `always holds(c.value <= 5)` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-1")]
fn always_lean() {
    assert_eq!(
        clause_text("always holds(c.value <= 5)", Target::Lean,),
        r#"def a_T_0 (s : State) (p_c : List Nat) : Prop :=
  (((F_get_Counter p_c s.v_Counter).f_value <= 5))

def holds_T (b : Behaviour) (p_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (0 <= (i + d1) /\ a_T_0 (b.st (Int.toNat (i + d1))) p_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall p_c, F_has_Counter p_c (b.st 0).v_Counter -> holds_T b p_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-2: `holds(c.value >= 1) until holds(c.value >= 3)` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-2")]
fn until_tla() {
    assert_eq!(
        clause_text("holds(c.value >= 1) until holds(c.value >= 3)", Target::Tla,),
        r#"vars_T == <<vars, m_T>>

TypeOK_T ==
  /\ m_T \in [DOMAIN v_Counter -> {"run", "acc", "rej"}]

Init_T ==
  /\ Init
  /\ TypeOK_T
  /\ m_T = [p_c \in DOMAIN v_Counter |-> IF ((v_Counter[p_c].f_value >= 3)) THEN "acc" ELSE IF ~((v_Counter[p_c].f_value >= 1)) THEN "rej" ELSE "run"]

Mon_T ==
  /\ m_T' = [p_c \in DOMAIN v_Counter |-> IF m_T[p_c] /= "run" THEN m_T[p_c] ELSE IF ((v_Counter'[p_c].f_value >= 3)) THEN "acc" ELSE IF ~((v_Counter'[p_c].f_value >= 1)) THEN "rej" ELSE "run"]

Next_T == Next /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T == \A p_c \in DOMAIN v_Counter : m_T[p_c] = "acc"

THEOREM Th_T == Spec_T => []<>accept_T

===="#
    );
}

/// FSpec FR-813-AC-2: `holds(c.value >= 1) until holds(c.value >= 3)` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-2")]
fn until_lean() {
    assert_eq!(
        clause_text(
            "holds(c.value >= 1) until holds(c.value >= 3)",
            Target::Lean,
        ),
        r#"def a_T_0 (s : State) (p_c : List Nat) : Prop :=
  (((F_get_Counter p_c s.v_Counter).f_value >= 1))

def a_T_1 (s : State) (p_c : List Nat) : Prop :=
  (((F_get_Counter p_c s.v_Counter).f_value >= 3))

def holds_T (b : Behaviour) (p_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 0 <= d1 /\ (0 <= (i + d1) /\ a_T_1 (b.st (Int.toNat (i + d1))) p_c) /\ (forall e2 : Int, 0 <= e2 -> e2 < d1 -> (0 <= (i + e2) /\ a_T_0 (b.st (Int.toNat (i + e2))) p_c)))

def Ob_T : Prop :=
  forall b : Behaviour, forall p_c, F_has_Counter p_c (b.st 0).v_Counter -> holds_T b p_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-3: `always[0, 3] holds(c.value <= 5)` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn interval_always_tla() {
    assert_eq!(
        clause_text("always[0, 3] holds(c.value <= 5)", Target::Tla,),
        r#"vars_T == <<vars, m_T, n_T>>

TypeOK_T ==
  /\ m_T \in [DOMAIN v_Counter -> {"run", "acc", "rej"}]
  /\ n_T \in 0..4

Init_T ==
  /\ Init
  /\ TypeOK_T
  /\ m_T = [p_c \in DOMAIN v_Counter |-> IF (0 <= 0 /\ 0 <= 3) /\ ~((v_Counter[p_c].f_value <= 5)) THEN "rej" ELSE "run"]
  /\ n_T = 0

Mon_T ==
  /\ m_T' = [p_c \in DOMAIN v_Counter |-> IF m_T[p_c] = "rej" THEN "rej" ELSE IF (0 <= n_T' /\ n_T' <= 3) /\ ~((v_Counter'[p_c].f_value <= 5)) THEN "rej" ELSE "run"]
  /\ n_T' = IF n_T < 4 THEN n_T + 1 ELSE 4

Next_T == Next /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T == \A p_c \in DOMAIN v_Counter : m_T[p_c] /= "rej"

THEOREM Th_T == Spec_T => []accept_T

===="#
    );
}

/// FSpec FR-813-AC-3: `always[0, 3] holds(c.value <= 5)` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn interval_always_lean() {
    assert_eq!(
        clause_text("always[0, 3] holds(c.value <= 5)", Target::Lean,),
        r#"def a_T_0 (s : State) (p_c : List Nat) : Prop :=
  (((F_get_Counter p_c s.v_Counter).f_value <= 5))

def holds_T (b : Behaviour) (p_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> d1 <= 3 -> (0 <= (i + d1) /\ a_T_0 (b.st (Int.toNat (i + d1))) p_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall p_c, F_has_Counter p_c (b.st 0).v_Counter -> holds_T b p_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-6: `eventually always holds(c.value <= 5)` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
fn co_buchi_tla() {
    assert_eq!(
        clause_text("eventually always holds(c.value <= 5)", Target::Tla,),
        r#"vars_T == <<vars, m_T>>

TypeOK_T ==
  /\ m_T \in [DOMAIN v_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ TypeOK_T
  /\ m_T = [p_c \in DOMAIN v_Counter |-> ((v_Counter[p_c].f_value <= 5))]

Mon_T ==
  /\ m_T' = [p_c \in DOMAIN v_Counter |-> ((v_Counter'[p_c].f_value <= 5))]

Next_T == Next /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T == \A p_c \in DOMAIN v_Counter : m_T[p_c]

THEOREM Th_T == Spec_T => <>[]accept_T

===="#
    );
}

/// FSpec FR-813-AC-6: `eventually always holds(c.value <= 5)` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
fn co_buchi_lean() {
    assert_eq!(
        clause_text("eventually always holds(c.value <= 5)", Target::Lean,),
        r#"def a_T_0 (s : State) (p_c : List Nat) : Prop :=
  (((F_get_Counter p_c s.v_Counter).f_value <= 5))

def holds_T (b : Behaviour) (p_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 0 <= d1 /\ (forall d2 : Int, 0 <= d2 -> (0 <= ((i + d1) + d2) /\ a_T_0 (b.st (Int.toNat ((i + d1) + d2))) p_c)))

def Ob_T : Prop :=
  forall b : Behaviour, forall p_c, F_has_Counter p_c (b.st 0).v_Counter -> holds_T b p_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-1: `always (holds(c.value >= 2) implies once holds(c.value = 1))` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-1")]
fn once_tla() {
    assert_eq!(
        clause_text(
            "always (holds(c.value >= 2) implies once holds(c.value = 1))",
            Target::Tla,
        ),
        r#"vars_T == <<vars, m_T_h0>>

TypeOK_T ==
  /\ m_T_h0 \in [DOMAIN v_Counter -> BOOLEAN]

Init_T ==
  /\ Init
  /\ TypeOK_T
  /\ m_T_h0 = [p_c \in DOMAIN v_Counter |-> ((((0 <= 1) /\ (1 <= 9)) /\ (v_Counter[p_c].f_value = 1)) \/ FALSE)]

Mon_T ==
  /\ m_T_h0' = [p_c \in DOMAIN v_Counter |-> ((((0 <= 1) /\ (1 <= 9)) /\ (v_Counter'[p_c].f_value = 1)) \/ m_T_h0[p_c])]

Next_T == Next /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

THEOREM Th_T == Spec_T => \A p_c \in DOMAIN v_Counter : []((((v_Counter[p_c].f_value >= 2)) => m_T_h0[p_c]))

===="#
    );
}

/// FSpec FR-813-AC-1: `always (holds(c.value >= 2) implies once holds(c.value = 1))` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-1")]
fn once_lean() {
    assert_eq!(
        clause_text(
            "always (holds(c.value >= 2) implies once holds(c.value = 1))",
            Target::Lean,
        ),
        r#"def a_T_0 (s : State) (p_c : List Nat) : Prop :=
  (((F_get_Counter p_c s.v_Counter).f_value >= 2))

def a_T_1 (s : State) (p_c : List Nat) : Prop :=
  (((0 <= 1) /\ (1 <= 9)) /\ ((F_get_Counter p_c s.v_Counter).f_value = 1))

def holds_T (b : Behaviour) (p_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> ((0 <= (i + d1) /\ a_T_0 (b.st (Int.toNat (i + d1))) p_c) -> (exists d2 : Int, 0 <= d2 /\ (0 <= ((i + d1) - d2) /\ a_T_1 (b.st (Int.toNat ((i + d1) - d2))) p_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall p_c, F_has_Counter p_c (b.st 0).v_Counter -> holds_T b p_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-2: `holds(c.value <= 4) release holds(c.value <= 5)` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-2")]
fn release_tla() {
    assert_eq!(
        clause_text(
            "holds(c.value <= 4) release holds(c.value <= 5)",
            Target::Tla,
        ),
        r#"vars_T == <<vars, m_T>>

TypeOK_T ==
  /\ m_T \in [DOMAIN v_Counter -> {"run", "rel", "rej"}]

Init_T ==
  /\ Init
  /\ TypeOK_T
  /\ m_T = [p_c \in DOMAIN v_Counter |-> IF ~((v_Counter[p_c].f_value <= 5)) THEN "rej" ELSE IF ((v_Counter[p_c].f_value <= 4)) THEN "rel" ELSE "run"]

Mon_T ==
  /\ m_T' = [p_c \in DOMAIN v_Counter |-> IF m_T[p_c] = "rej" THEN "rej" ELSE IF m_T[p_c] = "run" /\ ~((v_Counter'[p_c].f_value <= 5)) THEN "rej" ELSE IF m_T[p_c] = "run" /\ ((v_Counter'[p_c].f_value <= 4)) THEN "rel" ELSE m_T[p_c]]

Next_T == Next /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T == \A p_c \in DOMAIN v_Counter : m_T[p_c] /= "rej"

THEOREM Th_T == Spec_T => []accept_T

===="#
    );
}

/// FSpec FR-813-AC-2: `holds(c.value <= 4) release holds(c.value <= 5)` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-2")]
fn release_lean() {
    assert_eq!(
        clause_text(
            "holds(c.value <= 4) release holds(c.value <= 5)",
            Target::Lean,
        ),
        r#"def a_T_0 (s : State) (p_c : List Nat) : Prop :=
  (((F_get_Counter p_c s.v_Counter).f_value <= 4))

def a_T_1 (s : State) (p_c : List Nat) : Prop :=
  (((F_get_Counter p_c s.v_Counter).f_value <= 5))

def holds_T (b : Behaviour) (p_c : List Nat) (i : Int) : Prop :=
  Not (exists d1 : Int, 0 <= d1 /\ Not ((0 <= (i + d1) /\ a_T_1 (b.st (Int.toNat (i + d1))) p_c)) /\ (forall e2 : Int, 0 <= e2 -> e2 < d1 -> Not ((0 <= (i + e2) /\ a_T_0 (b.st (Int.toNat (i + e2))) p_c))))

def Ob_T : Prop :=
  forall b : Behaviour, forall p_c, F_has_Counter p_c (b.st 0).v_Counter -> holds_T b p_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-3: `eventually[1, 3] holds(c.value = 2)` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn interval_eventually_tla() {
    assert_eq!(
        clause_text("eventually[1, 3] holds(c.value = 2)", Target::Tla,),
        r#"vars_T == <<vars, m_T, n_T>>

TypeOK_T ==
  /\ m_T \in [DOMAIN v_Counter -> {"run", "acc", "rej"}]
  /\ n_T \in 0..4

Init_T ==
  /\ Init
  /\ TypeOK_T
  /\ m_T = [p_c \in DOMAIN v_Counter |-> IF (1 <= 0 /\ 0 <= 3) /\ (((0 <= 2) /\ (2 <= 9)) /\ (v_Counter[p_c].f_value = 2)) THEN "acc" ELSE IF 0 >= 3 THEN "rej" ELSE "run"]
  /\ n_T = 0

Mon_T ==
  /\ m_T' = [p_c \in DOMAIN v_Counter |-> IF m_T[p_c] /= "run" THEN m_T[p_c] ELSE IF (1 <= n_T' /\ n_T' <= 3) /\ (((0 <= 2) /\ (2 <= 9)) /\ (v_Counter'[p_c].f_value = 2)) THEN "acc" ELSE IF n_T' >= 3 THEN "rej" ELSE "run"]
  /\ n_T' = IF n_T < 4 THEN n_T + 1 ELSE 4

Next_T == Next /\ Mon_T

Spec_T == Init_T /\ [][Next_T]_vars_T /\ WF_vars_T(Next_T)

accept_T == \A p_c \in DOMAIN v_Counter : m_T[p_c] = "acc"

THEOREM Th_T == Spec_T => []<>accept_T

===="#
    );
}

/// FSpec FR-813-AC-3: `eventually[1, 3] holds(c.value = 2)` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-3")]
fn interval_eventually_lean() {
    assert_eq!(
        clause_text("eventually[1, 3] holds(c.value = 2)", Target::Lean,),
        r#"def a_T_0 (s : State) (p_c : List Nat) : Prop :=
  (((0 <= 2) /\ (2 <= 9)) /\ ((F_get_Counter p_c s.v_Counter).f_value = 2))

def holds_T (b : Behaviour) (p_c : List Nat) (i : Int) : Prop :=
  (exists d1 : Int, 1 <= d1 /\ d1 <= 3 /\ (0 <= (i + d1) /\ a_T_0 (b.st (Int.toNat (i + d1))) p_c))

def Ob_T : Prop :=
  forall b : Behaviour, forall p_c, F_has_Counter p_c (b.st 0).v_Counter -> holds_T b p_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-4: `fair strong Counter::Counter::inc; always eventually holds(c.value = 2)` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-4")]
fn strong_fairness_tla() {
    assert_eq!(
        clause_text(
            "fair strong Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Tla,
        ),
        r#"Spec_T == Spec /\ SF_vars((\E p_self \in DOMAIN v_Counter : op_inc(p_self)))

THEOREM Th_T == Spec_T => \A p_c \in DOMAIN v_Counter : [](<>((((0 <= 2) /\ (2 <= 9)) /\ (v_Counter[p_c].f_value = 2))))

===="#
    );
}

/// FSpec FR-813-AC-4: `fair strong Counter::Counter::inc; always eventually holds(c.value = 2)` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-4")]
fn strong_fairness_lean() {
    assert_eq!(
        clause_text(
            "fair strong Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Lean,
        ),
        r#"def a_T_0 (s : State) (p_c : List Nat) : Prop :=
  (((0 <= 2) /\ (2 <= 9)) /\ ((F_get_Counter p_c s.v_Counter).f_value = 2))

def holds_T (b : Behaviour) (p_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (exists d2 : Int, 0 <= d2 /\ (0 <= ((i + d1) + d2) /\ a_T_0 (b.st (Int.toNat ((i + d1) + d2))) p_c)))

def fair_T (b : Behaviour) : Prop :=
  ((forall i : Nat, exists j : Nat, i <= j /\ (exists t, exists p_self, op_inc (b.st j) t p_self)) -> (forall i : Nat, exists j : Nat, i <= j /\ (exists p_self, op_inc (b.st j) (b.st (j + 1)) p_self)))

def Ob_T : Prop :=
  forall b : Behaviour, fair_T b -> forall p_c, F_has_Counter p_c (b.st 0).v_Counter -> holds_T b p_c 0

end M_test_u3A_w21_u3A_unit_u2E_native"#
    );
}

/// FSpec FR-813-AC-4: `fair weak each Counter::Counter::inc; always eventually holds(c.value = 2)` as tla2 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-4")]
fn weak_each_fairness_tla() {
    assert_eq!(
        clause_text(
            "fair weak each Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Tla,
        ),
        r#"Spec_T == Spec /\ \A p_self \in DOMAIN v_Counter : WF_vars((op_inc(p_self)))

THEOREM Th_T == Spec_T => \A p_c \in DOMAIN v_Counter : [](<>((((0 <= 2) /\ (2 <= 9)) /\ (v_Counter[p_c].f_value = 2))))

===="#
    );
}

/// FSpec FR-813-AC-4: `fair weak each Counter::Counter::inc; always eventually holds(c.value = 2)` as lean4 text.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-4")]
fn weak_each_fairness_lean() {
    assert_eq!(
        clause_text(
            "fair weak each Counter::Counter::inc; always eventually holds(c.value = 2)",
            Target::Lean,
        ),
        r#"def a_T_0 (s : State) (p_c : List Nat) : Prop :=
  (((0 <= 2) /\ (2 <= 9)) /\ ((F_get_Counter p_c s.v_Counter).f_value = 2))

def holds_T (b : Behaviour) (p_c : List Nat) (i : Int) : Prop :=
  (forall d1 : Int, 0 <= d1 -> (exists d2 : Int, 0 <= d2 /\ (0 <= ((i + d1) + d2) /\ a_T_0 (b.st (Int.toNat ((i + d1) + d2))) p_c)))

def fair_T (b : Behaviour) : Prop :=
  (forall p_self, F_has_Counter p_self (b.st 0).v_Counter -> (forall i : Nat, (forall j : Nat, i <= j -> (exists t, op_inc (b.st j) t p_self)) -> (exists j : Nat, i <= j /\ (op_inc (b.st j) (b.st (j + 1)) p_self))))

def Ob_T : Prop :=
  forall b : Behaviour, fair_T b -> forall p_c, F_has_Counter p_c (b.st 0).v_Counter -> holds_T b p_c 0

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

/// FSpec FR-813-AC-1/AC-2/AC-6 and FR-811: each lowered clause is
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
                assert!(text[start..end].contains('T'), "{}", &text[start..end]);
            }
        }
    }
}

/// FSpec FR-811 "Decision": a future operator nested in another future
/// operator has no deterministic TLA+ monitor in this build, so the TLA+
/// export stops as a mapper failure that names the operator, while the Lean
/// text states it by direct recursion.
#[test]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-2")]
fn a_nested_future_operator_is_a_tla_mapper_failure_and_a_lean_definition() {
    let body = "always (holds(c.value >= 1) until holds(c.value >= 3))";
    let (exit, value) = export(body, Target::Tla);
    assert_eq!(exit, 30, "{value}");
    let message = value["message"].as_str().unwrap();
    assert!(message.contains("temporal/until"), "{message}");
    let lean = exported_text(body, Target::Lean);
    assert!(lean.contains("def holds_T"), "{lean}");
}
