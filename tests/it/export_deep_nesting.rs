// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-030 D-1: `fsl export` lowers a source expression of any nesting depth.
//! Each shape is 1,000 levels deep and is exported to TLA+ and Lean 4 on a
//! 2 MiB caller thread. The expected texts are written from the FSpec FR-812
//! and FR-813 forms: a binary operator is `(left op right)`, a negation
//! prefixes its operand, and a chain associates to the left.

use crate::plan_export_surface::{counter_request, COUNTER_UNIT};
use ix_trace_rs::trace;
use serde_json::{json, Value};

const DEPTH: usize = 1_000;
const TLA: &str = "filament.output.tla2/v1";
const LEAN: &str = "filament.output.lean4/v1";

/// Runs `check` on a 2 MiB thread. A walk that recurses with the input
/// aborts the test process.
pub(crate) fn on_a_small_stack(check: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(check)
        .expect("spawn the small-stack thread")
        .join()
        .expect("the deep export succeeds");
}

/// Exports the obligation `name` of the counter's unit with `tail` appended,
/// and returns the exit code and the result.
fn export(tail: &str, name: &str, profile: &str) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    let path = counter_request(
        directory.path(),
        json!({"profile": profile, "obligations": [name]}),
    );
    let unit = COUNTER_UNIT.replace(
        "model Counter",
        "profile t = \"filament.temporal.infinite-trace/v1\";\nmodel Counter",
    );
    std::fs::write(
        directory.path().join("unit.native"),
        format!("{unit}{tail}"),
    )
    .unwrap();
    let result = fsl_command::command::run(&path, &crate::negotiation::Fr290)
        .unwrap_or_else(|error| panic!("the export runs: {error}"));
    let value = serde_json::from_str(&result.value.to_string()).unwrap();
    (result.exit_code, value)
}

/// The target text of a successful export.
fn exported(tail: &str, name: &str, profile: &str) -> String {
    let (exit, value) = export(tail, name, profile);
    assert_eq!(exit, 0, "{}", value["export"]["refusal"]);
    value["export"]["target_text"]
        .as_str()
        .expect("target text")
        .to_owned()
}

fn invariant(body: &str) -> String {
    format!("invariant Deep using m on Counter::Counter at current {{ {body} }}\n")
}

fn temporal(body: &str) -> String {
    format!(
        "temporal T using t over (c: Counter::Counter) clock \"model-steps\" on origin {{ {body} }}\n"
    )
}

/// `first op next op next ...` with `DEPTH` operators, left associated.
fn chain(first: &str, rest: &str) -> String {
    format!(
        "{}{first}{}",
        "(".repeat(DEPTH),
        format!("{rest})").repeat(DEPTH)
    )
}

/// The definition of the invariant `Deep` in each target, over the formula
/// `formula` of the object `r_self`.
fn assert_invariant(source: &str, tla: &str, lean: &str) {
    let text = exported(&invariant(source), "Deep", TLA);
    let line = format!("c_Deep == \\A r_self \\in DOMAIN v_Counter : ({tla})");
    assert!(
        text.lines().any(|found| found == line),
        "TLA+ c_Deep differs"
    );
    let text = exported(&invariant(source), "Deep", LEAN);
    let definition = format!(
        "def c_Deep (s : State) : Prop :=\n  forall r_self, F_has_Counter r_self s.v_Counter -> ({lean})\n"
    );
    assert!(text.contains(&definition), "Lean c_Deep differs");
}

const TLA_VALUE: &str = "v_Counter[r_self].fld_value";
const LEAN_VALUE: &str = "(F_get_Counter r_self s.v_Counter).fld_value";

#[test]
#[trace("FSpec-FR-812-AC-1")]
fn a_deep_and_chain_exports_to_both_targets() {
    on_a_small_stack(|| {
        let source = format!("self.value <= 5{}", " and self.value <= 5".repeat(DEPTH));
        let tla = format!("({TLA_VALUE} <= 5)");
        let lean = format!("({LEAN_VALUE} <= 5)");
        assert_invariant(
            &source,
            &chain(&tla, &format!(" /\\ {tla}")),
            &chain(&lean, &format!(" /\\ {lean}")),
        );
    });
}

#[test]
#[trace("FSpec-FR-812-AC-1")]
fn a_deep_not_nest_exports_to_both_targets() {
    on_a_small_stack(|| {
        let source = format!("{}(self.value <= 5)", "not ".repeat(DEPTH));
        assert_invariant(
            &source,
            &format!("{}({TLA_VALUE} <= 5)", "~".repeat(DEPTH)),
            &format!("{}({LEAN_VALUE} <= 5)", "Not ".repeat(DEPTH)),
        );
    });
}

#[test]
#[trace("FSpec-FR-812-AC-1")]
fn a_deep_sum_exports_to_both_targets() {
    on_a_small_stack(|| {
        let source = format!("self.value{} <= 5", " + 1".repeat(DEPTH));
        assert_invariant(
            &source,
            &format!("({} <= 5)", chain(TLA_VALUE, " + 1")),
            &format!("({} <= 5)", chain(LEAN_VALUE, " + 1")),
        );
    });
}

/// An operation clause is lowered on the same path as an invariant, and the
/// float scan of both renderers reads it.
#[test]
#[trace("FSpec-FR-812-AC-1")]
fn a_deep_operation_clause_exports_to_both_targets() {
    on_a_small_stack(|| {
        let body = format!("self.value < 5{}", " and self.value < 5".repeat(DEPTH));
        let tail = format!(
            "pre Deeply using v on Counter::Counter::inc {{ {body} }}\n{}",
            invariant("self.value <= 5")
        );
        for (profile, value) in [(TLA, TLA_VALUE), (LEAN, LEAN_VALUE)] {
            let text = exported(&tail, "Deep", profile);
            let atom = format!("({value} < 5)");
            let expected = chain(&atom, &format!(" /\\ {atom}"));
            assert!(text.contains(&expected), "{profile}: precondition differs");
        }
    });
}

/// A temporal atom over a deep expression, and deep temporal formulas.
#[test]
#[trace("FSpec-FR-813-AC-1")]
fn deep_temporal_formulas_export_to_both_targets() {
    on_a_small_stack(|| {
        let tla_atom = "(v_Counter[o_c].fld_value <= 5)";
        let lean_atom = "(0 <= (i + d1) /\\ at_T_0 (b.st (Int.toNat (i + d1))) o_c)";

        // A deep negation of one atom.
        let body = format!("always {}holds(c.value <= 5)", "not ".repeat(DEPTH));
        let text = exported(&temporal(&body), "T", TLA);
        let formula = format!("{}{tla_atom}{}", "~(".repeat(DEPTH), ")".repeat(DEPTH));
        assert!(
            text.contains(&format!("[](ok_T(o_c)) /\\ []({formula})")),
            "TLA+ negation differs"
        );
        let text = exported(&temporal(&body), "T", LEAN);
        let formula = format!("{}{lean_atom}{}", "Not (".repeat(DEPTH), ")".repeat(DEPTH));
        assert!(
            text.contains(&format!("(forall d1 : Int, 0 <= d1 -> {formula})")),
            "Lean negation differs"
        );

        // A deep conjunction of temporal atoms.
        let body = format!(
            "always (holds(c.value <= 5){})",
            " and holds(c.value <= 5)".repeat(DEPTH)
        );
        let text = exported(&temporal(&body), "T", TLA);
        let formula = chain(tla_atom, &format!(" /\\ {tla_atom}"));
        assert!(
            text.contains(&format!("[](ok_T(o_c)) /\\ []({formula})")),
            "TLA+ conjunction differs"
        );
        let text = exported(&temporal(&body), "T", LEAN);
        let atom = |index: usize| {
            format!("(0 <= (i + d1) /\\ at_T_{index} (b.st (Int.toNat (i + d1))) o_c)")
        };
        let mut formula = "(".repeat(DEPTH) + &atom(0);
        for index in 1..=DEPTH {
            formula.push_str(&format!(" /\\ {})", atom(index)));
        }
        assert!(
            text.contains(&format!("(forall d1 : Int, 0 <= d1 -> {formula})")),
            "Lean conjunction differs"
        );

        // A deep expression inside one atom.
        let body = format!("always holds(c.value{} <= 5)", " + 1".repeat(DEPTH));
        let text = exported(&temporal(&body), "T", TLA);
        let formula = format!("({} <= 5)", chain("v_Counter[o_c].fld_value", " + 1"));
        assert!(
            text.contains(&format!("[](ok_T(o_c)) /\\ []({formula})")),
            "TLA+ atom differs"
        );
        let text = exported(&temporal(&body), "T", LEAN);
        let formula = format!(
            "({} <= 5)",
            chain("(F_get_Counter o_c s.v_Counter).fld_value", " + 1")
        );
        assert!(
            text.contains(&format!(
                "def at_T_0 (s : State) (o_c : List Nat) : Prop :=\n  {formula}\n"
            )),
            "Lean atom differs"
        );
    });
}

/// Deep past operators: each `once` is its own history variable in TLA+, and
/// in Lean each one is an offset back from the position of the one outside it.
#[test]
#[trace("FSpec-FR-813-AC-23")]
fn deep_past_operators_export_to_both_targets() {
    on_a_small_stack(|| {
        let body = format!("{}holds(c.value <= 5)", "once ".repeat(DEPTH));
        let text = exported(&temporal(&body), "T", TLA);
        let last = DEPTH - 1;
        // The innermost history reads the atom; each outer one reads the
        // history inside it.
        assert!(
            text.contains(&format!(
                "m_T_h{last}' = [o_c \\in Ids_Counter |-> ((v_Counter'[o_c].fld_value <= 5) \\/ m_T_h{last}[o_c])]"
            )),
            "TLA+ innermost history differs"
        );
        assert!(
            text.contains("m_T_h0' = [o_c \\in Ids_Counter |-> (m_T_h1'[o_c] \\/ m_T_h0[o_c])]"),
            "TLA+ outermost history differs"
        );
        assert!(
            text.contains("[](ok_T(o_c)) /\\ m_T_h0[o_c]\n"),
            "TLA+ theorem differs"
        );
        let text = exported(&temporal(&body), "T", LEAN);
        assert!(
            text.contains("(exists d1 : Int, 0 <= d1 /\\ 0 <= (i - d1) /\\ (exists d2 : Int, 0 <= d2 /\\ 0 <= ((i - d1) - d2) /\\ "),
            "Lean past operators differ"
        );
        assert!(
            text.contains(&format!("(exists d{DEPTH} : Int, ")),
            "Lean innermost offset is missing"
        );
    });
}

/// A deep chain of `until` has no TLA+ monitor: the TLA+ export refuses it
/// with the typed capability, and Lean states it.
#[test]
#[trace("FSpec-FR-813-AC-2", "FSpec-FR-813-AC-24")]
fn a_deep_until_chain_reaches_an_outcome_in_both_targets() {
    on_a_small_stack(|| {
        let body = format!(
            "{}holds(c.value <= 5){}",
            "holds(c.value <= 5) until (".repeat(DEPTH),
            ")".repeat(DEPTH)
        );
        let (exit, value) = export(&temporal(&body), "T", TLA);
        assert_eq!(exit, 21, "{}", value["export"]["refusal"]["detail"]);
        assert_eq!(
            value["export"]["refusal"]["code"],
            "unimplemented_capability"
        );
        let text = exported(&temporal(&body), "T", LEAN);
        assert!(
            text.contains(&format!("(exists d{} : Int, ", 2 * DEPTH - 1)),
            "Lean innermost until is missing"
        );
    });
}
