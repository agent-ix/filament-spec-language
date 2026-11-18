// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-814 name mangling, the module name and the obligation records of
//! an export. Authored state clauses are resolved by name; this build lowers
//! none of them, so each requested clause is an `unrepresented` record that
//! names the construct, and the text is the module skeleton.

use super::super::{Result, RunCause};
use super::request::{EffectiveLimits, Request, Target};
use super::{Disposition, Mapped, Record};
use serde_json::{json, Value};

/// One authored state clause the program declares.
pub(super) struct ClauseFact {
    pub(super) name: String,
    pub(super) kind: &'static str,
}

/// FSpec FR-814's injective mangling of one authored name under `prefix`:
/// `_` becomes `__`, `::` becomes `_N_`, any other scalar outside ASCII
/// letters and digits becomes `_uXX_` (uppercase hexadecimal).
pub(super) fn mangle(prefix: &str, name: &str) -> String {
    let mut out = String::from(prefix);
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '_' => out.push_str("__"),
            ':' if chars.peek() == Some(&':') => {
                chars.next();
                out.push_str("_N_");
            }
            c if c.is_ascii_alphanumeric() => out.push(c),
            c => out.push_str(&format!("_u{:X}_", u32::from(c))),
        }
    }
    out
}

/// The module name of a package identity.
pub(super) fn module_name(identity: &str) -> String {
    mangle("M_", identity)
}

/// Map `clauses` named by `request` to records and the module skeleton.
///
/// # Errors
///
/// An obligation no clause of the program declares.
pub(super) fn run(
    identity: &str,
    clauses: &[ClauseFact],
    request: &Request,
    effective: &EffectiveLimits,
) -> Result<Mapped> {
    let chosen: Vec<&ClauseFact> = if request.obligations.is_empty() {
        clauses.iter().collect()
    } else {
        let mut chosen = Vec::new();
        for (index, name) in request.obligations.iter().enumerate() {
            let clause = clauses
                .iter()
                .find(|clause| &clause.name == name)
                .ok_or_else(|| RunCause::ClauseRequest {
                    member: format!("/request/export/obligations/{index}"),
                    reason: format!("the program declares no state clause `{name}`"),
                })?;
            chosen.push(clause);
        }
        chosen
    };
    let nodes = u64::try_from(chosen.len())
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    if nodes > effective.nodes {
        return Ok(Mapped::Incomplete {
            setting: "export.nodes",
        });
    }
    if effective.work_units == 0 {
        return Ok(Mapped::Incomplete {
            setting: "export.work_units",
        });
    }
    let module = module_name(identity);
    let mut text = String::new();
    match request.target {
        Target::Tla => {
            let head = format!("---- MODULE {module} ----\n");
            text.push_str(&head);
            text.push_str("====\n");
        }
        Target::Lean => {
            text.push_str(&format!("namespace {module}\nend {module}\n"));
        }
    }
    let records = chosen
        .iter()
        .map(|clause| Record {
            obligation: clause.name.clone(),
            disposition: Disposition::Unrepresented,
            conditions: Vec::new(),
            causes: vec![cause(request.target, clause)],
            regions: Vec::new(),
        })
        .collect();
    Ok(Mapped::Assembled {
        text,
        name_map: Vec::new(),
        records,
    })
}

fn cause(target: Target, clause: &ClauseFact) -> Value {
    json!({
        "kind": "target-unrepresentable",
        "target": target.profile(),
        "construct": clause.kind,
        "node": clause.name,
        "locus": format!("state clause `{}`", clause.name),
    })
}
