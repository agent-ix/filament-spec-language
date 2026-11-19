// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-623: the `reductions` member of an analyze item, admitted over what the
//! unit declares. The unit declares (a population annotated `symmetric`, a
//! `constraint` state clause) and the item selects, so no reduction is ever
//! applied because a unit happens to declare one.
//!
//! [`admit`] reads the document member into FSpec FR-382's wire form, each
//! population by its universe key and the classes sorted, and
//! [`wire`] writes that form back for the item's request echo.

use filament_checked::key::DeclarationKey;
use fsl_eval::simulation::model::{ModelSystem, PopulationUniverse};
use fsl_route::reductions::{Reductions, SymmetryDeclaration};
use serde_json::{json, Map, Value};

/// Why a `reductions` member is refused (FR-623 "Refusals").
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(super) enum ReductionIssue {
    /// A `constraint` that names no constraint clause:
    /// `invalid_runtime_input`/`invalid-value`, naming the name.
    #[error("{0} names no constraint clause")]
    UnknownConstraint(String),
    /// A malformed selection: `declined` with `invalid-input`.
    #[error("invalid reductions selection: {0}")]
    Malformed(String),
}

fn issue(reason: impl Into<String>) -> ReductionIssue {
    ReductionIssue::Malformed(reason.into())
}

/// The reductions `selection` selects over `system` and its `universes`.
///
/// # Errors
///
/// [`ReductionIssue`] for an unknown member, a malformed symmetry
/// declaration, `"declared"` over a unit that annotates no population
/// `symmetric`, a population or key the subject does not hold, or a
/// `constraint` that names no constraint clause.
pub(super) fn admit(
    selection: &Value,
    system: &ModelSystem<'_>,
    universes: &[PopulationUniverse],
) -> Result<Reductions, ReductionIssue> {
    let Some(members) = selection.as_object() else {
        return Err(issue("reductions is not an object"));
    };
    if let Some(unknown) = members
        .keys()
        .find(|name| !matches!(name.as_str(), "symmetry" | "partial_order" | "constraint"))
    {
        return Err(issue(format!("unknown member {unknown}")));
    }
    let mut reductions = Reductions::default();
    match members.get("symmetry") {
        None | Some(Value::Null) => {}
        Some(Value::String(word)) if word == "declared" => {
            reductions.symmetry = declared(system, universes)?;
            if reductions.symmetry.is_empty() {
                return Err(issue("no population is annotated symmetric"));
            }
        }
        Some(Value::Array(declarations)) => {
            for declaration in declarations {
                reductions.symmetry.push(explicit(declaration, universes)?);
            }
        }
        Some(_) => return Err(issue("symmetry is neither \"declared\" nor a list")),
    }
    match members.get("partial_order") {
        None | Some(Value::Null) => {}
        Some(Value::Bool(selected)) => reductions.partial_order = *selected,
        Some(_) => return Err(issue("partial_order is not a Boolean")),
    }
    match members.get("constraint") {
        None | Some(Value::Null) => {}
        Some(Value::String(name)) if system.has_constraint(name) => {
            reductions.constraint = Some(name.clone());
        }
        Some(Value::String(name)) => return Err(ReductionIssue::UnknownConstraint(name.clone())),
        Some(_) => return Err(issue("constraint is not a name")),
    }
    Ok(reductions)
}

/// `"declared"`: one class of every key of each population annotated
/// `symmetric`.
fn declared(
    system: &ModelSystem<'_>,
    universes: &[PopulationUniverse],
) -> Result<Vec<SymmetryDeclaration>, ReductionIssue> {
    let mut declarations = Vec::new();
    for population in system.populations().iter().filter(|p| p.symmetric) {
        let Some(universe) = universes
            .iter()
            .find(|universe| universe.population == population.key)
        else {
            continue;
        };
        let mut keys = universe.keys.clone();
        keys.sort();
        declarations.push(SymmetryDeclaration {
            population: population.key.clone(),
            classes: vec![keys],
        });
    }
    declarations.sort();
    Ok(declarations)
}

/// One `{population, classes}` declaration, the population written by its
/// declared name, with its classes sorted.
fn explicit(
    declaration: &Value,
    universes: &[PopulationUniverse],
) -> Result<SymmetryDeclaration, ReductionIssue> {
    let Some(members) = declaration.as_object() else {
        return Err(issue("a symmetry declaration is not an object"));
    };
    if let Some(unknown) = members
        .keys()
        .find(|name| !matches!(name.as_str(), "population" | "classes"))
    {
        return Err(issue(format!("unknown symmetry member {unknown}")));
    }
    let Some(name) = members.get("population").and_then(Value::as_str) else {
        return Err(issue("a symmetry declaration names no population"));
    };
    let matching: Vec<&PopulationUniverse> = universes
        .iter()
        .filter(|universe| population_named(&universe.population, name))
        .collect();
    let [universe] = matching.as_slice() else {
        return Err(issue(format!("{name} names no single population")));
    };
    let Some(classes) = members.get("classes").and_then(Value::as_array) else {
        return Err(issue("a symmetry declaration lists no classes"));
    };
    let mut sorted = Vec::with_capacity(classes.len());
    for class in classes {
        let Some(keys) = class.as_array() else {
            return Err(issue("a symmetry class is not a list"));
        };
        let mut class: Vec<String> = Vec::with_capacity(keys.len());
        for key in keys {
            let Some(key) = key.as_str() else {
                return Err(issue("a symmetry key is not text"));
            };
            if !universe.keys.iter().any(|held| held == key) {
                return Err(issue(format!("{key} is no key of {name}")));
            }
            if class.iter().any(|seen| seen == key) {
                return Err(issue(format!("{key} repeats in a class")));
            }
            class.push(key.to_owned());
        }
        class.sort();
        sorted.push(class);
    }
    sorted.sort();
    Ok(SymmetryDeclaration {
        population: universe.population.clone(),
        classes: sorted,
    })
}

/// Whether the declared `name` names the population `key`: its whole node or
/// its last path segment.
fn population_named(key: &DeclarationKey, name: &str) -> bool {
    key.node == name || key.node.rsplit('/').next() == Some(name)
}

/// FSpec FR-382's wire form of `reductions`, for the request echo.
pub(super) fn wire(reductions: &Reductions) -> Value {
    let mut member = Map::new();
    if !reductions.symmetry.is_empty() {
        member.insert(
            "symmetry".to_owned(),
            Value::Array(
                reductions
                    .symmetry
                    .iter()
                    .map(|declaration| {
                        json!({"population": declaration.population.node,
                            "classes": declaration.classes})
                    })
                    .collect(),
            ),
        );
    }
    if reductions.partial_order {
        member.insert("partial_order".to_owned(), json!(true));
    }
    if let Some(constraint) = &reductions.constraint {
        member.insert("constraint".to_owned(), json!(constraint));
    }
    Value::Object(member)
}
