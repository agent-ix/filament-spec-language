// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-144 and FR-145 "Unavailable occurrences" (FSpec TC-1482,
//! TC-1483): a supplied `Sequence<Int[0, 9]>` holding an unavailable
//! occurrence, written `{"type":"unavailable"}` at its position, replays
//! from a witness entry; a short-circuiting query completes only when its
//! deciding occurrence precedes every unavailable one, every other form and
//! sequence equality meet it as incomplete, and `size`, `dropFirst` and the
//! `removeOne` that removes before it read no element value.

use ix_trace_rs::trace;

use super::*;

/// The witness value text of a `Sequence<Int[0, 9]>`: `Some(n)` an integer
/// occurrence, `None` an unavailable one.
fn sequence(items: &[Option<i64>]) -> String {
    let elements: Vec<String> = items
        .iter()
        .map(|item| match item {
            Some(value) => format!(r#"{{"type":"integer","value":"{value}"}}"#),
            None => r#"{"type":"unavailable"}"#.to_owned(),
        })
        .collect();
    format!(
        r#"{{"elements":[{}],"type":"sequence"}}"#,
        elements.join(",")
    )
}

const U: Option<i64> = None;

/// `body`'s function `p` replayed from a witness whose entries are `texts`,
/// one per parameter in declared order.
fn replay_witness(body: &str, texts: &[String]) -> Result<ReplayResult, ReplayRefusal> {
    let source = format!("language \"ix:native\" edition \"1-draft\";\n{PROFILE}{body}");
    let compiled = spine(&source, &BTreeMap::new());
    let entries: Vec<String> = texts
        .iter()
        .enumerate()
        .map(|(index, text)| format!("{}={text}", crate::pn(&parameter(&compiled, "p", index))))
        .collect();
    let transcript = format!("<<<assertion|p|c|{}>>>", entries.join(";"));
    let wire = request(
        source.as_bytes(),
        &compiled,
        name(&["p"]),
        ReplaySource::Witness(Witness::parse(transcript).expect("the transcript parses")),
    );
    replay(wire, ReplayLimits::default())
}

fn category(body: &str, texts: &[String]) -> Category {
    replay_witness(body, texts)
        .unwrap_or_else(|refusal| panic!("{body}: the replay runs, got {refusal:?}"))
        .category()
}

const SEQ: &str = "Sequence<Int[0, 9]>[0, 3]";

fn predicate(expression: &str) -> String {
    format!("type N = Int[0, 3];\ntype S = Int[0, 27];\nfunction p using v(s: {SEQ}): Boolean pure {{ {expression} }}\n")
}

/// FSpec FR-144-AC-29: a sequence holding an unavailable occurrence is
/// admitted with that occurrence counted (`size` 2), and an `unavailable`
/// entry with another member is refused.
#[trace("FSpec-TC-1482", "FSpec-FR-144-AC-29")]
#[test]
#[ignore = "STD-652: the unavailable sequence occurrence wire entry is not implemented yet"]
fn an_unavailable_occurrence_is_admitted_at_its_position() {
    // `size(s) = 9` is false over the two-occurrence sequence.
    assert_eq!(
        category(&predicate("size(s) = 9"), &[sequence(&[Some(1), U])]),
        Category::Violation
    );
    let extra = r#"{"elements":[{"type":"unavailable","value":"1"}],"type":"sequence"}"#;
    assert!(replay_witness(&predicate("size(s) = 9"), &[extra.to_owned()]).is_err());
}

/// FSpec FR-144-AC-30: sequence equality with an unavailable occurrence is
/// a completed `false` on unequal counts or an earlier unequal pair, and
/// incomplete otherwise.
#[trace("FSpec-TC-1482", "FSpec-FR-144-AC-30")]
#[test]
#[ignore = "STD-652: sequence equality over an unavailable occurrence is not implemented yet"]
fn sequence_equality_meets_an_unavailable_occurrence_in_order() {
    let body = format!("function p using v(s: {SEQ}, t: {SEQ}): Boolean pure {{ s = t }}\n");
    let pair = |s: &[Option<i64>], t: &[Option<i64>]| category(&body, &[sequence(s), sequence(t)]);
    assert_eq!(
        pair(&[Some(1), U], &[Some(1), Some(2), Some(3)]),
        Category::Violation
    );
    assert_eq!(
        pair(&[Some(2), U], &[Some(1), Some(5)]),
        Category::Violation
    );
    assert_eq!(
        pair(&[Some(1), U], &[Some(1), Some(2)]),
        Category::Incomplete
    );
    assert_eq!(
        pair(&[U, Some(2)], &[Some(1), Some(3)]),
        Category::Incomplete
    );
}

/// FSpec FR-145-AC-13: `contains`, `exists` and `forall` complete when the
/// deciding occurrence precedes the unavailable one and are incomplete when
/// the unavailable one comes first.
#[trace("FSpec-TC-1483", "FSpec-FR-145-AC-13")]
#[test]
#[ignore = "STD-652: short-circuiting queries over an unavailable occurrence are not implemented yet"]
fn short_circuiting_queries_stop_before_or_at_the_unavailable_occurrence() {
    for expression in [
        "not contains(s, 1)",
        "not exists(x in s: x = 1)",
        "forall(x in s: x = 2)",
    ] {
        assert_eq!(
            category(&predicate(expression), &[sequence(&[Some(1), Some(1), U])]),
            Category::Violation,
            "{expression} on [1, 1, u]"
        );
        assert_eq!(
            category(&predicate(expression), &[sequence(&[U, Some(1)])]),
            Category::Incomplete,
            "{expression} on [u, 1]"
        );
    }
    assert_eq!(
        category(
            &predicate("forall(x in s: x = 1)"),
            &[sequence(&[Some(1), U])]
        ),
        Category::Incomplete
    );
}

/// FSpec FR-145-AC-14: whole-source forms are incomplete at the unavailable
/// occurrence; `removeOne` before it, `dropFirst` and `size` complete.
#[trace("FSpec-TC-1483", "FSpec-FR-145-AC-14")]
#[test]
#[ignore = "STD-652: whole-source collection forms over an unavailable occurrence are not implemented yet"]
fn whole_source_forms_are_incomplete_and_value_free_forms_complete() {
    for expression in [
        "size(filter(x in s: x > 0)) = 9",
        "size(map(x in s: x)) = 9",
        "count<N>(x in s: x > 0) = 3",
        "sum<S>(x in s: x) = 27",
    ] {
        for items in [[U, Some(1)], [Some(1), U]] {
            assert_eq!(
                category(&predicate(expression), &[sequence(&items)]),
                Category::Incomplete,
                "{expression} on {items:?}"
            );
        }
    }
    let completed = |expression: &str, items: &[Option<i64>]| {
        category(&predicate(expression), &[sequence(items)])
    };
    assert_eq!(
        completed("size(removeOne(s, 1)) = 9", &[Some(1), U]),
        Category::Violation
    );
    assert_eq!(
        completed("size(removeOne(s, 1)) = 9", &[U, Some(1)]),
        Category::Incomplete
    );
    assert_eq!(
        completed("not contains(dropFirst(s), 1)", &[U, Some(1)]),
        Category::Violation
    );
    assert_eq!(
        completed("size(dropFirst(s)) = 9", &[Some(1), U]),
        Category::Violation
    );
    assert_eq!(completed("size(s) = 9", &[Some(1), U]), Category::Violation);
}

/// FSpec FR-145-AC-15: the incomplete outcome is charged the visit of the
/// unavailable occurrence and nothing after it.
#[trace("FSpec-TC-1483", "FSpec-FR-145-AC-15")]
#[test]
#[ignore = "STD-652: the unavailable-occurrence incomplete outcome and its charges are not implemented yet"]
fn the_incomplete_outcome_charges_the_visit_and_no_retain() {
    let early = replay_witness(
        &predicate("forall(x in s: x = 2)"),
        &[sequence(&[U, Some(1)])],
    )
    .expect("the replay runs");
    let late = replay_witness(
        &predicate("forall(x in s: x = 2)"),
        &[sequence(&[Some(2), U])],
    )
    .expect("the replay runs");
    assert_eq!(early.category(), Category::Incomplete);
    assert_eq!(late.category(), Category::Incomplete);
    // One visit is charged at `u` on `[u, 1]`; on `[2, u]` the visit and the
    // predicate of `2` come first.
    assert!(early.charges().work_units > 0);
    assert!(late.charges().work_units > early.charges().work_units);
}
