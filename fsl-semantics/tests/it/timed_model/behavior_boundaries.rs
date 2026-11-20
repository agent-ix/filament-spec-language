// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact clock expressions retain comparisons, branches and source loci.

use super::*;

#[trace("FR-230-AC-4")]
#[test]
fn fractional_disjunction_retains_both_clock_parts_and_their_data_guard() {
    let body = format!(
        "{UNITS}time Config dense unit ms;\n\
         pre Split using v on Config::Call::bump {{ \
           (self.x < rational(1, 2) ms or self.x > rational(5, 2) ms) and self.ready \
         }}\n"
    );
    let graph = check(&rpc_document(&["x"]), &body).expect("both convex parts check");
    assert_eq!(
        parts(timed(&graph), 0),
        [
            (
                vec!["ready".to_owned()],
                vec![("x".to_owned(), ClockOp::Less, exact(1, 2))]
            ),
            (
                vec!["ready".to_owned()],
                vec![("x".to_owned(), ClockOp::Greater, exact(5, 2))]
            ),
        ]
    );
}

#[trace("FR-230")]
#[test]
fn reversing_a_clock_comparison_mirrors_all_five_relations_exactly() {
    let document = rpc_document(&["x"]);
    for (written, expected) in [
        ("<", ClockOp::Greater),
        ("<=", ClockOp::GreaterOrEqual),
        ("=", ClockOp::Equal),
        (">=", ClockOp::LessOrEqual),
        (">", ClockOp::Less),
    ] {
        let body = format!(
            "{UNITS}time Config dense unit ms;\n\
             pre Mirror using v on Config::Call::bump {{ rational(3, 2000) s {written} self.x }}\n"
        );
        let graph = check(&document, &body).expect("a constant can precede the clock");
        assert_eq!(
            parts(timed(&graph), 0),
            [(Vec::new(), vec![("x".to_owned(), expected, exact(3, 2))])],
            "written relation {written} must be mirrored after exact unit conversion"
        );
    }
}

#[trace("FR-230-AC-9", "FR-230-AC-10")]
#[test]
fn nested_clock_selection_and_predicate_calls_keep_each_selected_place() {
    let body = format!(
        "{UNITS}time Config dense unit ms;\n\
         function choose using v(a: Clock, b: Clock, left: Boolean): Clock pure {{ \
           if left then a else b }}\n\
         predicate below using v(c: Clock, b: Quantity<ms>): Boolean {{ c < b }}\n\
         pre Selected using v on Config::Call::bump {{ \
           below(choose(self.x, self.y, self.ready), rational(7, 2) ms) }}\n"
    );
    let graph = check(&rpc_document(&["x", "y"]), &body)
        .expect("nested clock helpers expand at their call site");
    assert_eq!(
        shown_parts(timed(&graph), 0),
        [
            (
                Some("ready".to_owned()),
                vec![("x".to_owned(), ClockOp::Less, exact(7, 2))]
            ),
            (
                Some("not ready".to_owned()),
                vec![("y".to_owned(), ClockOp::Less, exact(7, 2))]
            ),
        ]
    );
    let names = declared_names(&graph);
    assert!(!names.iter().any(|name| name == "choose" || name == "below"));
}

#[trace("FR-230-AC-2")]
#[test]
fn a_negative_fractional_clock_bound_refuses_at_its_written_quantity() {
    let document = rpc_document(&["x"]);
    for comparison in ["<", "<=", "=", ">=", ">"] {
        let body = format!(
            "{UNITS}time Config dense unit ms;\n\
             pre Negative using v on Config::Call::bump {{ self.x {comparison} rational(-1, 2000) s }}\n"
        );
        let (cause, text) = refused(&document, &body);
        assert_eq!(cause.code(), Code::InvalidTimedModel);
        assert_eq!(cause.tag(), "negative-clock-constant");
        assert_eq!(
            text, "rational(-1, 2000) s",
            "written comparison {comparison}"
        );
    }
}
