// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared lock-row mutations used to verify catalog-bound package admission.

use serde_json::Value;

pub(crate) struct CatalogRefusalCase {
    pub(crate) package: Value,
    pub(crate) code: &'static str,
    pub(crate) cause: &'static str,
    pub(crate) path: String,
}

pub(crate) fn replace_definition_pair(
    value: &mut Value,
    old_authority: &str,
    old_identity: &str,
    new_authority: &str,
    new_identity: &str,
) -> usize {
    match value {
        Value::Object(object) => {
            let matches = object.get("authority").and_then(Value::as_str) == Some(old_authority)
                && object.get("identity").and_then(Value::as_str) == Some(old_identity);
            if matches {
                object.insert("authority".to_owned(), new_authority.into());
                object.insert("identity".to_owned(), new_identity.into());
            }
            usize::from(matches)
                + object
                    .values_mut()
                    .map(|child| {
                        replace_definition_pair(
                            child,
                            old_authority,
                            old_identity,
                            new_authority,
                            new_identity,
                        )
                    })
                    .sum::<usize>()
        }
        Value::Array(values) => values
            .iter_mut()
            .map(|child| {
                replace_definition_pair(
                    child,
                    old_authority,
                    old_identity,
                    new_authority,
                    new_identity,
                )
            })
            .sum(),
        _ => 0,
    }
}

pub(crate) fn catalog_refusal_cases(original: &Value, index: usize) -> Vec<CatalogRefusalCase> {
    let floor_authority = original["lock"]["definition_selections"][index]["authority"]
        .as_str()
        .expect("floor law authority");
    let mut cases = Vec::new();
    for (authority, identity, cause, row) in [
        (
            floor_authority,
            "filament.value.integer-division.ceiling/v1",
            "unsupported-selection",
            index,
        ),
        ("agent-ix", "ext.retention", "unsupported-selection", index),
        (
            "another-authority",
            "filament.value.integer-division.floor/v1",
            "unsupported-selection",
            index,
        ),
        (
            floor_authority,
            "filament.value.complete/v1",
            "wrong-selection-role",
            index,
        ),
    ] {
        let mut package = original.clone();
        package["lock"]["definition_selections"][index]["authority"] = authority.into();
        package["lock"]["definition_selections"][index]["identity"] = identity.into();
        cases.push(CatalogRefusalCase {
            package,
            code: "unknown_profile",
            cause,
            path: format!("/lock/definition_selections/{row}"),
        });
    }

    let mut two_defects = original.clone();
    two_defects["lock"]["definition_selections"][0]["identity"] =
        "test:uncatalogued-first-row".into();
    two_defects["lock"]["definition_selections"][index]["authority"] = "another-authority".into();
    cases.push(CatalogRefusalCase {
        package: two_defects,
        code: "unknown_profile",
        cause: "unsupported-selection",
        path: "/lock/definition_selections/0".to_owned(),
    });

    let mut paired_law = original.clone();
    let floor_identity = "filament.value.integer-division.floor/v1";
    let ceiling_identity = "filament.value.integer-division.ceiling/v1";
    paired_law["lock"]["definition_selections"][index]["identity"] = ceiling_identity.into();
    let replaced = replace_definition_pair(
        &mut paired_law,
        floor_authority,
        floor_identity,
        floor_authority,
        ceiling_identity,
    );
    assert!(
        replaced >= 1,
        "operation law identity is mutated with the lock row"
    );
    cases.push(CatalogRefusalCase {
        package: paired_law,
        code: "invalid_package",
        cause: "operation-law-mismatch",
        path: "/operation/laws/0/definition".to_owned(),
    });

    cases
}
