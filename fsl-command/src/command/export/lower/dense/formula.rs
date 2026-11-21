// SPDX-License-Identifier: AGPL-3.0-or-later
//! TLA+ temporal statements over the positions of a dense-time behaviour.

use crate::command::export::lower::temporal::{ClauseLow, Tf};
use crate::command::export::lower::{defer_at, define, mangle, Definition, Lower};

fn direct(
    clause: &ClauseLow,
    tf: &Tf,
    state: &impl Fn(&Tf, bool) -> Lower<String>,
) -> Lower<String> {
    let child = |tf: &Tf| direct(clause, tf, state);
    Ok(match tf {
        Tf::Always(iv, inner) if !iv.is_some() => format!("[](tick \\/ {})", child(inner)?),
        Tf::Eventually(iv, inner) if !iv.is_some() => format!("<>(~tick /\\ {})", child(inner)?),
        Tf::Not(inner) => format!("~({})", child(inner)?),
        Tf::And(a, b) => format!("({} /\\ {})", child(a)?, child(b)?),
        Tf::Or(a, b) => format!("({} \\/ {})", child(a)?, child(b)?),
        Tf::Implies(a, b) => format!("({} => {})", child(a)?, child(b)?),
        Tf::Const(_) | Tf::Atom(_) => state(tf, false)?,
        _ => {
            return defer_at(
                clause.refuse_at.unwrap_or(clause.position),
                super::super::temporal::CONSTRUCT,
                "a timed formula outside the direct always and eventually forms",
            );
        }
    })
}

pub(in crate::command::export::lower) fn tla_clause(
    clause: &ClauseLow,
    ids: &str,
    ok: &str,
    state: impl Fn(&Tf, bool) -> Lower<String>,
    out: &mut Vec<Definition>,
) -> Lower<()> {
    let theorem = mangle("Th_", &clause.name);
    let key = &clause.binder;
    let formula = direct(clause, &clause.tf, &state)?;
    out.push(define(
        clause.position,
        Some(theorem.clone()),
        format!(
            "THEOREM {theorem} == Spec => ((NZ /\\ []<>~tick) => \\A {key} \\in {ids} : [](tick \\/ {ok}({key})) /\\ {formula})"
        ),
        Some(&clause.name),
    ));
    Ok(())
}
