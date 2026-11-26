// SPDX-License-Identifier: AGPL-3.0-or-later
//! The spec-consistency reading of FR-1051 (AC-9, AC-15): the requirement's
//! classes, trusted base, exception rows, routes and producer-built types,
//! compared with the check's data. Test code: nothing else reads the tables.

use std::collections::{BTreeMap, BTreeSet};

use super::{Extent, Policy};

/// The text of the section headed `heading`, up to the next heading.
fn section<'a>(text: &'a str, heading: &str) -> &'a str {
    let Some(start) = text.find(&format!("\n{heading}\n")) else {
        return "";
    };
    let body = &text[start + heading.len() + 2..];
    &body[..body.find("\n#").unwrap_or(body.len())]
}

/// The body rows of the section's table, as trimmed cells.
fn rows(section: &str) -> Vec<Vec<&str>> {
    section
        .lines()
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}

/// Every backtick-quoted name of `text`.
fn names(text: &str) -> BTreeSet<String> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// `text` without its parentheses, which describe and do not narrow.
fn without_parentheses(text: &str) -> String {
    let mut depth = 0_u32;
    text.chars()
        .filter(|&character| {
            match character {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    return false;
                }
                _ => {}
            }
            depth == 0
        })
        .collect()
}

fn owned(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

fn compare(
    differences: &mut Vec<String>,
    subject: &str,
    spec: &BTreeSet<String>,
    data: &BTreeSet<String>,
) {
    for missing in spec.difference(data) {
        differences.push(format!(
            "{subject}: FR-1051 has {missing}, the data does not"
        ));
    }
    for extra in data.difference(spec) {
        differences.push(format!("{subject}: the data has {extra}, FR-1051 does not"));
    }
}

/// The route ids of a Routes cell: `R-3`, `R-1 to R-16` or `R-17, R-18`.
fn route_ids(cell: &str) -> Vec<String> {
    let number = |id: &str| {
        id.trim()
            .strip_prefix("R-")
            .and_then(|number| number.parse::<u32>().ok())
    };
    cell.split(',')
        .flat_map(|part| {
            let (first, last) = part.split_once(" to ").unwrap_or((part, part));
            match (number(first), number(last)) {
                (Some(first), Some(last)) => (first..=last).map(|id| format!("R-{id}")).collect(),
                _ => vec![format!("unreadable route cell {part}")],
            }
        })
        .collect()
}

fn class_differences(differences: &mut Vec<String>, text: &str, policy: &Policy) {
    let classes = &policy.classes;
    let by_file = format!("{}/src", classes.by_file);
    let mut checker = owned(classes.checker_files);
    for (file, item) in classes.checker_items {
        checker.insert((*file).to_owned());
        checker.insert((*item).to_owned());
    }
    checker.extend(owned(classes.checker_crates));
    let mut producer = owned(classes.producer_crates);
    producer.insert(by_file);
    let data = BTreeMap::from([
        ("Checker code", checker),
        ("Settling code", owned(classes.settling_files)),
        ("Trusted-base crates", owned(classes.trusted_crates)),
        ("Out of scope", owned(classes.out_of_scope_files)),
        ("Producer code", producer),
        ("Tools", owned(classes.tool_crates)),
    ]);
    let mut spec: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for row in rows(section(text, "### Classes of code")) {
        if let [class, members] = row.as_slice() {
            spec.insert(class, names(&without_parentheses(members)));
        }
    }
    let all: BTreeSet<&str> = spec.keys().chain(data.keys()).copied().collect();
    let empty = BTreeSet::new();
    for class in all {
        compare(
            differences,
            &format!("class \"{class}\""),
            spec.get(class).unwrap_or(&empty),
            data.get(class).unwrap_or(&empty),
        );
    }
}

fn base_differences(differences: &mut Vec<String>, text: &str, policy: &Policy) {
    let mut seen = BTreeSet::new();
    for row in rows(section(text, "### Trusted base")) {
        let [id, _, modules, extent] = row.as_slice() else {
            continue;
        };
        let id = (*id).to_owned();
        seen.insert(id.clone());
        let Some(data) = policy.base.iter().find(|row| row.id == id) else {
            differences.push(format!("base row {id}: not in the data"));
            continue;
        };
        compare(
            differences,
            &format!("base row {id}"),
            &names(&without_parentheses(modules)),
            &owned(data.modules),
        );
        let stated = [
            ("whole crate", Extent::WholeCrate),
            ("module", Extent::Module),
            ("type items", Extent::TypeItems),
            ("the items the check's data records", Extent::Recorded),
            ("data items only", Extent::DataItems),
        ]
        .into_iter()
        .find(|(start, _)| extent.starts_with(start))
        .map(|(_, extent)| extent);
        if stated != Some(data.extent) {
            differences.push(format!(
                "base row {id}: extent \"{extent}\" is not the data's {:?}",
                data.extent
            ));
        }
        let excepted: BTreeSet<String> = extent
            .split_whitespace()
            .map(|word| word.trim_matches(','))
            .filter(|word| word.starts_with("S-"))
            .map(str::to_owned)
            .collect();
        compare(
            differences,
            &format!("base row {id} exceptions"),
            &excepted,
            &owned(data.except),
        );
    }
    for row in policy.base {
        if !seen.contains(row.id) {
            differences.push(format!("base row {}: not in FR-1051", row.id));
        }
    }
}

fn exception_differences(differences: &mut Vec<String>, text: &str, policy: &Policy) {
    let mut seen = BTreeSet::new();
    for row in rows(section(text, "### Recorded exceptions")) {
        let [id, rule, concerns, ..] = row.as_slice() else {
            continue;
        };
        seen.insert((*id).to_owned());
        let Some(data) = policy.exceptions.iter().find(|row| row.id == *id) else {
            differences.push(format!("exception row {id}: not in the data"));
            continue;
        };
        if *rule != data.rule.as_str() {
            differences.push(format!(
                "exception row {id}: rule \"{rule}\" is not the data's \"{}\"",
                data.rule.as_str()
            ));
        }
        compare(
            differences,
            &format!("exception row {id}"),
            &names(concerns),
            &data
                .concerns
                .iter()
                .map(|(name, _)| (*name).to_owned())
                .collect(),
        );
    }
    for row in policy.exceptions {
        if !seen.contains(row.id) {
            differences.push(format!("exception row {}: not in FR-1051", row.id));
        }
    }
}

/// Every difference between FR-1051's text and `policy`.
pub(super) fn differences(text: &str, policy: &Policy) -> Vec<String> {
    let mut differences = Vec::new();
    class_differences(&mut differences, text, policy);
    base_differences(&mut differences, text, policy);
    exception_differences(&mut differences, text, policy);
    let routes: BTreeSet<String> = rows(section(text, "### Routes"))
        .iter()
        .filter_map(|row| row.first())
        .flat_map(|cell| route_ids(cell))
        .collect();
    compare(&mut differences, "routes", &routes, &owned(policy.routes));
    let objects = section(text, "### No producer objects");
    let bullet = objects
        .split("\n- ")
        .find(|bullet| bullet.starts_with("A check entry SHALL NOT take"))
        .unwrap_or_default();
    let types: BTreeSet<String> = names(bullet)
        .into_iter()
        .filter(|name| name.starts_with(|c: char| c.is_ascii_uppercase()))
        .collect();
    compare(
        &mut differences,
        "producer-built types",
        &types,
        &owned(policy.producer_types),
    );
    differences
}

/// AC-15: ADR-028 CE-7 says the checker shares the concrete run alone, and
/// does not say that the product construction is shared with EN-1.
pub(super) fn ce_7_difference(adr: &str) -> Option<String> {
    let narrowed = adr
        .lines()
        .find(|line| line.starts_with("| CE-7 |"))
        .is_some_and(|line| {
            line.contains("checker shares is the concrete run alone")
                && !line.contains("product construction is shared with EN-1")
        });
    (!narrowed).then(|| {
        "ADR-028 CE-7 does not limit what the checker shares to the concrete run alone".to_owned()
    })
}
