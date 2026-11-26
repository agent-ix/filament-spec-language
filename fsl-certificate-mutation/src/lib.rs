// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 single-mutation checks shared by certificate checker test suites.
#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ix_trace_rs::implements;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Number, Value};

/// The kind of one local certificate mutation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MutationKind {
    ValuePlus,
    ValueMinus,
    RowDropped,
    RowDuplicated,
    KeySwapped,
    RankPlus,
    RankMinus,
    PolicyChoiceChanged,
    SideFlipped,
    IdentityChanged,
    SourceByteChanged,
}

/// A mutation's stable member path and kind, used by diagnostics and exceptions.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MutationId {
    pub member: String,
    pub kind: MutationKind,
}

/// A reasoned exception for a mutation that remains a valid proof.
#[derive(Clone, Debug)]
pub struct AllowedMutation {
    pub id: MutationId,
    pub reason: String,
}

/// A checker route's fixture registration.
pub struct Fixture<'a> {
    pub route: &'a str,
    pub name: &'a str,
    pub document: Value,
    pub allowed_to_survive: Vec<AllowedMutation>,
}

/// The mutation rejected by a checker closure, or its accepted value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Verdict {
    Rejected,
    Accepted,
}

/// Harness registration errors, including unlisted survivors and stale exceptions.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HarnessError {
    #[error("route {route}, fixture {fixture}: accepted fixture was rejected")]
    FixtureRejected { route: String, fixture: String },
    #[error(
        "route {route}, fixture {fixture}, member {member}, mutation {mutation:?}: changed certificate was accepted"
    )]
    UnexpectedSurvivor {
        route: String,
        fixture: String,
        member: String,
        mutation: MutationKind,
    },
    #[error(
        "route {route}, fixture {fixture}: allowed mutation at {member} ({mutation:?}) has no reason"
    )]
    MissingReason {
        route: String,
        fixture: String,
        member: String,
        mutation: MutationKind,
    },
    #[error(
        "route {route}, fixture {fixture}: allowed mutation at {member} ({mutation:?}) is stale"
    )]
    StaleAllowed {
        route: String,
        fixture: String,
        member: String,
        mutation: MutationKind,
    },
    #[error("route {route} has no accepted fixture")]
    MissingRoute { route: String },
    #[error(
        "route {route}, fixture {fixture}, mutation {member} ({mutation:?}): mutated value could not be adapted"
    )]
    AdapterFailed {
        route: String,
        fixture: String,
        member: String,
        mutation: MutationKind,
    },
    #[error("canonical certificate could not be encoded: {0}")]
    Canonical(String),
}

#[derive(Clone, Debug)]
struct Mutant {
    id: MutationId,
    value: Value,
}

/// Canonical RFC 8785 bytes for a mutated document, checked by every byte-route closure.
#[implements("FR-1051-AC-11")]
pub fn canonical_bytes(value: &Value) -> Result<Vec<u8>, HarnessError> {
    quire_canonical::to_vec(value, quire_canonical::Limits::new(16 * 1024 * 1024))
        .map_err(|error| HarnessError::Canonical(error.to_string()))
}

/// Apply every generated single mutation to a canonical document and require rejection.
#[implements("FR-1051-AC-11")]
pub fn check_document(
    fixture: Fixture<'_>,
    mut check: impl FnMut(&[u8]) -> Verdict,
) -> Result<usize, HarnessError> {
    let route = fixture.route.to_owned();
    let name = fixture.name.to_owned();
    let baseline = canonical_bytes(&fixture.document)?;
    if check(&baseline) != Verdict::Accepted {
        return Err(HarnessError::FixtureRejected {
            route,
            fixture: name,
        });
    }
    let mutants = mutations(&fixture.document);
    check_mutants(route, name, fixture.allowed_to_survive, mutants, |value| {
        canonical_bytes(value).ok().map(|bytes| check(&bytes))
    })
}

/// Apply every single mutation through a typed JSON adapter, then invoke its typed check entry.
#[implements("FR-1051-AC-11")]
pub fn check_typed<T>(
    route: &str,
    fixture_name: &str,
    fixture: &T,
    allowed_to_survive: Vec<AllowedMutation>,
    check: impl FnMut(&T) -> Verdict,
) -> Result<usize, HarnessError>
where
    T: Clone + Serialize + DeserializeOwned,
{
    check_typed_with_adapter(
        route,
        fixture_name,
        fixture,
        allowed_to_survive,
        |typed| serde_json::to_value(typed).ok(),
        |document| serde_json::from_value(document.clone()).ok(),
        check,
    )
}

/// Apply the mutation set through route-owned conversions for a typed-only certificate.
#[implements("FR-1051-AC-11")]
pub fn check_typed_with_adapter<T>(
    route: &str,
    fixture_name: &str,
    fixture: &T,
    allowed_to_survive: Vec<AllowedMutation>,
    to_document: impl Fn(&T) -> Option<Value>,
    from_document: impl Fn(&Value) -> Option<T>,
    mut check: impl FnMut(&T) -> Verdict,
) -> Result<usize, HarnessError> {
    let route = route.to_owned();
    let fixture_name = fixture_name.to_owned();
    let value = to_document(fixture).ok_or_else(|| HarnessError::AdapterFailed {
        route: route.clone(),
        fixture: fixture_name.clone(),
        member: "$".into(),
        mutation: MutationKind::IdentityChanged,
    })?;
    if check(fixture) != Verdict::Accepted {
        return Err(HarnessError::FixtureRejected {
            route,
            fixture: fixture_name,
        });
    }
    let mutants = mutations(&value);
    check_mutants(
        route.clone(),
        fixture_name.clone(),
        allowed_to_survive,
        mutants,
        |value| from_document(value).map(|typed| check(&typed)),
    )
}

/// Require one or more accepted fixtures for each named route.
#[implements("FR-1051-AC-13")]
pub fn require_routes<'a>(
    routes: &[&'a str],
    registered: impl IntoIterator<Item = &'a str>,
) -> Result<(), HarnessError> {
    let registered = registered.into_iter().collect::<BTreeSet<_>>();
    for route in routes {
        if !registered.contains(route) {
            return Err(HarnessError::MissingRoute {
                route: (*route).to_owned(),
            });
        }
    }
    Ok(())
}

#[implements("FR-1051-AC-11", "FR-1051-AC-12")]
fn check_mutants(
    route: String,
    fixture: String,
    allowed: Vec<AllowedMutation>,
    mutants: Vec<Mutant>,
    mut check: impl FnMut(&Value) -> Option<Verdict>,
) -> Result<usize, HarnessError> {
    let mut allowed_by_id = allowed
        .into_iter()
        .map(|entry| (entry.id.clone(), entry))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    for mutant in &mutants {
        seen.insert(mutant.id.clone());
        let Some(verdict) = check(&mutant.value) else {
            return Err(HarnessError::AdapterFailed {
                route,
                fixture,
                member: mutant.id.member.clone(),
                mutation: mutant.id.kind.clone(),
            });
        };
        match verdict {
            Verdict::Rejected => {
                if allowed_by_id.remove(&mutant.id).is_some() {
                    return Err(HarnessError::StaleAllowed {
                        route,
                        fixture,
                        member: mutant.id.member.clone(),
                        mutation: mutant.id.kind.clone(),
                    });
                }
            }
            Verdict::Accepted => {
                if let Some(entry) = allowed_by_id.remove(&mutant.id) {
                    if entry.reason.trim().is_empty() {
                        return Err(HarnessError::MissingReason {
                            route,
                            fixture,
                            member: mutant.id.member.clone(),
                            mutation: mutant.id.kind.clone(),
                        });
                    }
                } else {
                    return Err(HarnessError::UnexpectedSurvivor {
                        route,
                        fixture,
                        member: mutant.id.member.clone(),
                        mutation: mutant.id.kind.clone(),
                    });
                }
            }
        }
    }
    if let Some((id, _)) = allowed_by_id.into_iter().next() {
        return Err(HarnessError::StaleAllowed {
            route,
            fixture,
            member: id.member,
            mutation: id.kind,
        });
    }
    let _ = seen;
    Ok(mutants.len())
}

#[implements("FR-1051-AC-11")]
fn mutations(document: &Value) -> Vec<Mutant> {
    let mut out = Vec::new();
    walk(document, document, "$", &mut out);
    row_key_swaps(document, &mut out);
    out
}

#[implements("FR-1051-AC-11")]
fn walk(root: &Value, value: &Value, path: &str, out: &mut Vec<Mutant>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let child_path = format!("{path}/{key}");
                if key == "side" || key == "kind" {
                    if let Some(next) = flip_side(child) {
                        push(root, out, &child_path, MutationKind::SideFlipped, next);
                    }
                }
                if key.contains("identity") || key == "identity" {
                    if let Some(next) = change_identity(child) {
                        push(root, out, &child_path, MutationKind::IdentityChanged, next);
                    }
                }
                if key.contains("source") {
                    if let Some(next) = change_source(child) {
                        push(
                            root,
                            out,
                            &child_path,
                            MutationKind::SourceByteChanged,
                            next,
                        );
                    }
                }
                if key == "rank" || key == "order" {
                    if let Some(next) = add_integer(child, 1) {
                        push(root, out, &child_path, MutationKind::RankPlus, next);
                    }
                    if let Some(next) = add_integer(child, -1) {
                        push(root, out, &child_path, MutationKind::RankMinus, next);
                    }
                } else if key == "value" || key == "count" {
                    if let Some(next) = add_integer(child, 1) {
                        push(root, out, &child_path, MutationKind::ValuePlus, next);
                    }
                    if let Some(next) = add_integer(child, -1) {
                        push(root, out, &child_path, MutationKind::ValueMinus, next);
                    }
                }
                if key == "choice" || key == "transition" || key == "action" {
                    if let Some(next) = change_choice(child) {
                        push(
                            root,
                            out,
                            &child_path,
                            MutationKind::PolicyChoiceChanged,
                            next,
                        );
                    }
                }
                if let Value::Array(rows) = child {
                    for (index, row) in rows.iter().enumerate() {
                        if row.is_object() {
                            let row_path = format!("{child_path}/{index}");
                            let mut dropped = value.clone();
                            if let Some(array) =
                                pointer_mut(&mut dropped, &child_path).and_then(Value::as_array_mut)
                            {
                                array.remove(index);
                            }
                            out.push(Mutant {
                                id: MutationId {
                                    member: row_path.clone(),
                                    kind: MutationKind::RowDropped,
                                },
                                value: dropped,
                            });
                            let mut duplicated = value.clone();
                            if let Some(array) = pointer_mut(&mut duplicated, &child_path)
                                .and_then(Value::as_array_mut)
                            {
                                array.insert(index, row.clone());
                            }
                            out.push(Mutant {
                                id: MutationId {
                                    member: row_path,
                                    kind: MutationKind::RowDuplicated,
                                },
                                value: duplicated,
                            });
                        }
                    }
                }
                walk(root, child, &child_path, out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                walk(root, child, &format!("{path}/{index}"), out);
            }
        }
        _ => {}
    }
}

#[implements("FR-1051-AC-11")]
fn row_key_swaps(document: &Value, out: &mut Vec<Mutant>) {
    let mut arrays = Vec::new();
    collect_row_arrays(document, "$", &mut arrays);
    for (path, rows) in arrays {
        let key_name = ["state", "key", "product_key"].into_iter().find(|name| {
            rows.iter()
                .all(|row| row.get(*name).is_some_and(Value::is_string))
        });
        let Some(key_name) = key_name else {
            continue;
        };
        for left in 0..rows.len() {
            for right in left + 1..rows.len() {
                if rows[left][key_name] == rows[right][key_name] {
                    continue;
                }
                let mut changed = document.clone();
                let Some(target_rows) =
                    pointer_mut(&mut changed, &path).and_then(Value::as_array_mut)
                else {
                    continue;
                };
                let left_key = target_rows[left][key_name].clone();
                target_rows[left][key_name] = target_rows[right][key_name].clone();
                target_rows[right][key_name] = left_key;
                target_rows
                    .sort_by(|a, b| decoded_key(&a[key_name]).cmp(&decoded_key(&b[key_name])));
                if changed != *document {
                    out.push(Mutant {
                        id: MutationId {
                            member: format!("{path}/{left}+{right}/{key_name}"),
                            kind: MutationKind::KeySwapped,
                        },
                        value: changed,
                    });
                }
            }
        }
    }
}

fn collect_row_arrays<'a>(value: &'a Value, path: &str, out: &mut Vec<(String, &'a Vec<Value>)>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let child_path = format!("{path}/{key}");
                if let Value::Array(rows) = child {
                    if rows.len() >= 2 && rows.iter().all(Value::is_object) {
                        out.push((child_path.clone(), rows));
                    }
                }
                collect_row_arrays(child, &child_path, out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_row_arrays(child, &format!("{path}/{index}"), out);
            }
        }
        _ => {}
    }
}

fn decoded_key(value: &Value) -> Vec<u8> {
    value
        .as_str()
        .and_then(|text| URL_SAFE_NO_PAD.decode(text).ok())
        .unwrap_or_else(|| value.as_str().unwrap_or_default().as_bytes().to_vec())
}

fn flip_side(value: &Value) -> Option<Value> {
    match value.as_str()? {
        "lower" => Some(Value::String("upper".into())),
        "upper" => Some(Value::String("lower".into())),
        _ => None,
    }
}

fn change_identity(value: &Value) -> Option<Value> {
    match value {
        Value::String(text) => Some(Value::String(format!("{text}#mutation"))),
        Value::Object(object) => {
            let mut changed = object.clone();
            let (key, value) = changed.iter_mut().find(|(key, value)| {
                key.to_ascii_lowercase().contains("id") && value.is_string()
            })?;
            let _ = key;
            *value = Value::String(format!("{}#mutation", value.as_str()?));
            Some(Value::Object(changed))
        }
        _ => None,
    }
}

fn change_source(value: &Value) -> Option<Value> {
    if let Some(text) = value.as_str() {
        let mut bytes = text.as_bytes().to_vec();
        if bytes.is_empty() {
            bytes.push(b' ');
        } else {
            bytes[0] ^= 1;
        }
        return Some(Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    }
    let Value::Object(object) = value else {
        return None;
    };
    let Value::Array(bytes) = object.get("bytes")? else {
        return None;
    };
    let mut changed = object.clone();
    let Value::Array(changed_bytes) = changed.get_mut("bytes")? else {
        return None;
    };
    match changed_bytes.first_mut() {
        Some(Value::Number(number)) => {
            let byte = number.as_u64()?;
            *changed_bytes.first_mut()? = Value::Number(Number::from(byte ^ 1));
        }
        Some(Value::String(text)) => {
            let mut bytes = text.as_bytes().to_vec();
            if bytes.is_empty() {
                bytes.push(b' ');
            } else {
                bytes[0] ^= 1;
            }
            *changed_bytes.first_mut()? =
                Value::String(String::from_utf8_lossy(&bytes).into_owned());
        }
        _ if bytes.is_empty() => changed_bytes.push(Value::Number(Number::from(0))),
        _ => return None,
    }
    Some(Value::Object(changed))
}

#[implements("FR-1051-AC-11")]
fn add_integer(value: &Value, delta: i64) -> Option<Value> {
    if let Some(number) = value.as_i64() {
        return number
            .checked_add(delta)
            .map(|next| Value::Number(Number::from(next)));
    }
    if let Some(text) = value.as_str() {
        return add_decimal(text, delta).map(Value::String);
    }
    if let Value::Object(object) = value {
        if let Some(Value::String(numerator)) = object.get("numerator") {
            let number = add_decimal(numerator, delta)?;
            let mut changed = object.clone();
            changed.insert("numerator".into(), Value::String(number));
            return Some(Value::Object(changed));
        }
        if let Some(number) = object.get("value").and_then(Value::as_i64) {
            let mut changed: Map<String, Value> = object.clone();
            changed.insert(
                "value".into(),
                Value::Number(Number::from(number.checked_add(delta)?)),
            );
            return Some(Value::Object(changed));
        }
    }
    None
}

fn add_decimal(value: &str, delta: i64) -> Option<String> {
    if delta != 1 && delta != -1 {
        return None;
    }
    let (negative, digits) = value
        .strip_prefix('-')
        .map_or((false, value), |digits| (true, digits));
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let increment = (delta == 1) != negative;
    if !negative && !increment && digits.bytes().all(|byte| byte == b'0') {
        return Some("-1".to_owned());
    }
    let mut bytes = digits.bytes().collect::<Vec<_>>();
    if increment {
        let mut carry = true;
        for digit in bytes.iter_mut().rev() {
            if !carry {
                break;
            }
            if *digit < b'9' {
                *digit += 1;
                carry = false;
            } else {
                *digit = b'0';
            }
        }
        if carry {
            bytes.insert(0, b'1');
        }
    } else {
        let mut borrow = true;
        for digit in bytes.iter_mut().rev() {
            if !borrow {
                break;
            }
            if *digit > b'0' {
                *digit -= 1;
                borrow = false;
            } else {
                *digit = b'9';
            }
        }
        if borrow {
            return None;
        }
        while bytes.len() > 1 && bytes.first() == Some(&b'0') {
            bytes.remove(0);
        }
    }
    let result = String::from_utf8(bytes).ok()?;
    let is_zero = result.bytes().all(|byte| byte == b'0');
    Some(if negative && !is_zero {
        format!("-{result}")
    } else {
        result
    })
}

fn change_choice(value: &Value) -> Option<Value> {
    match value {
        Value::String(text) => Some(Value::String(format!("{text}#mutation"))),
        Value::Number(number) => number
            .as_i64()?
            .checked_add(1)
            .map(|next| Value::Number(Number::from(next))),
        _ => None,
    }
}

fn push(root: &Value, out: &mut Vec<Mutant>, member: &str, kind: MutationKind, replacement: Value) {
    let mut value = root.clone();
    if let Some(slot) = pointer_mut(&mut value, member) {
        *slot = replacement;
    } else {
        return;
    }
    out.push(Mutant {
        id: MutationId {
            member: member.to_owned(),
            kind,
        },
        value,
    });
}

fn pointer_mut<'a>(value: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let path = path.strip_prefix('$').unwrap_or(path);
    value.pointer_mut(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct TypedFixture {
        identity: String,
        values: Vec<Row>,
        side: String,
    }

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct Row {
        state: String,
        value: i64,
        rank: i64,
    }

    fn fixture() -> Value {
        json!({
            "identity": "fixture-1",
            "side": "lower",
            "values": [
                {"state": "eyJhIjoxfQ", "value": 1, "rank": 1},
                {"state": "eyJhIjoyfQ", "value": 2, "rank": 0}
            ],
            "policy": [{"state": "eyJhIjoxfQ", "choice": "take-a"}],
            "source": "source bytes"
        })
    }

    /// Trace: FR-1051-AC-11, FR-1051-AC-12.
    #[trace("FR-1051-AC-11", "FR-1051-AC-12")]
    #[test]
    fn canonical_documents_generate_ordered_single_mutations_and_fail_on_a_weakened_check() {
        let doc = fixture();
        let mut weak_document = doc.clone();
        weak_document["values"][0]["value"] = json!(2);
        let err = check_document(
            Fixture {
                route: "byte-route",
                name: "accepted",
                document: doc.clone(),
                allowed_to_survive: Vec::new(),
            },
            |bytes| {
                let value: Value = serde_json::from_slice(bytes).expect("canonical JSON parses");
                if value == doc || value == weak_document {
                    Verdict::Accepted
                } else {
                    Verdict::Rejected
                }
            },
        )
        .expect_err("weakened checker accepting a changed value must fail");
        assert!(matches!(
            err,
            HarnessError::UnexpectedSurvivor {
                mutation: MutationKind::ValuePlus,
                ..
            }
        ));

        let mutants = mutations(&doc);
        assert!(
            mutants
                .iter()
                .any(|mutant| mutant.id.kind == MutationKind::RowDropped)
        );
        assert!(
            mutants
                .iter()
                .any(|mutant| mutant.id.kind == MutationKind::RowDuplicated)
        );
        assert!(
            mutants
                .iter()
                .any(|mutant| mutant.id.kind == MutationKind::KeySwapped)
        );
        for mutant in mutants
            .iter()
            .filter(|mutant| mutant.id.kind == MutationKind::KeySwapped)
        {
            let rows = mutant.value["values"].as_array().expect("row array");
            let decoded = rows
                .iter()
                .map(|row| decoded_key(&row["state"]))
                .collect::<Vec<_>>();
            assert!(
                decoded.windows(2).all(|pair| pair[0] < pair[1]),
                "rows sorted by decoded key bytes"
            );
        }
        assert_eq!(
            canonical_bytes(&doc).expect("canonical encoding"),
            quire_canonical::to_vec(&doc, quire_canonical::Limits::new(4096))
                .expect("canonical encoding")
        );
    }

    /// Trace: FR-1051-AC-11, FR-1051-AC-12.
    #[trace("FR-1051-AC-11", "FR-1051-AC-12")]
    #[test]
    fn typed_adapter_runs_the_same_mutations_and_detects_a_weakened_check() {
        let fixture = TypedFixture {
            identity: "typed-fixture".into(),
            values: vec![
                Row {
                    state: "eyJhIjoxfQ".into(),
                    value: 1,
                    rank: 1,
                },
                Row {
                    state: "eyJhIjoyfQ".into(),
                    value: 2,
                    rank: 0,
                },
            ],
            side: "lower".into(),
        };
        let mut weak_fixture = fixture.clone();
        weak_fixture.values[0].value = 2;
        let err = check_typed(
            "typed-route",
            "typed-accepted",
            &fixture,
            Vec::new(),
            |value| {
                if value == &fixture || value == &weak_fixture {
                    Verdict::Accepted
                } else {
                    Verdict::Rejected
                }
            },
        )
        .expect_err("weakened typed checker accepting a changed value must fail");
        assert!(matches!(
            err,
            HarnessError::UnexpectedSurvivor {
                mutation: MutationKind::ValuePlus,
                ..
            }
        ));
    }

    /// Trace: FR-1051-AC-13.
    #[trace("FR-1051-AC-13")]
    #[test]
    fn every_required_route_must_have_a_registration() {
        let err = require_routes(&["measure", "bounds"], ["measure"])
            .expect_err("missing route registration");
        assert_eq!(
            err,
            HarnessError::MissingRoute {
                route: "bounds".into()
            }
        );
        require_routes(&["measure", "bounds"], ["measure", "bounds"])
            .expect("all routes registered");
    }

    /// Trace: FR-1051-AC-12.
    #[trace("FR-1051-AC-12")]
    #[test]
    fn allowed_mutations_require_a_live_mutation_and_a_reason() {
        let document = fixture();
        let mut plus_one = document.clone();
        plus_one["values"][0]["value"] = json!(2);
        let mutation_id = MutationId {
            member: "$/values/0/value".into(),
            kind: MutationKind::ValuePlus,
        };
        let error = check_document(
            Fixture {
                route: "value-route",
                name: "missing-reason",
                document: document.clone(),
                allowed_to_survive: vec![AllowedMutation {
                    id: mutation_id.clone(),
                    reason: String::new(),
                }],
            },
            |bytes| {
                let value: Value = serde_json::from_slice(bytes).expect("canonical JSON");
                if value == document || value == plus_one {
                    Verdict::Accepted
                } else {
                    Verdict::Rejected
                }
            },
        )
        .expect_err("an allowed mutation without a reason must fail");
        assert!(matches!(error, HarnessError::MissingReason { .. }));

        let error = check_document(
            Fixture {
                route: "value-route",
                name: "stale-allowance",
                document: document.clone(),
                allowed_to_survive: vec![AllowedMutation {
                    id: mutation_id,
                    reason: "The check was tightened.".into(),
                }],
            },
            |bytes| {
                let value: Value = serde_json::from_slice(bytes).expect("canonical JSON");
                if value == document {
                    Verdict::Accepted
                } else {
                    Verdict::Rejected
                }
            },
        )
        .expect_err("a rejected allowance must be stale");
        assert!(matches!(error, HarnessError::StaleAllowed { .. }));
    }
}
