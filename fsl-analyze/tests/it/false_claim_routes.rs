// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 route coverage of the false-claim corpus: the Routes table of the
//! requirement is read from the spec, and each route needs a corpus entry.
//! A seed's entry names the test that forges its certificate; a not-yet
//! route's entry would carry its hand-derived value (the requirement's
//! "not-yet" clause). The first test keeps the table and the registry
//! honest; the second is the coverage gate itself and names each route that
//! has no entry, which is every route outside the four seeds landed so far.

use std::collections::BTreeSet;

/// The routes with a corpus entry: route, seed ticket, and the test that
/// forges the certificate (or the witness) for it.
const CORPUS: &[(&str, &str, &str)] = &[
    (
        "R-18",
        "QSL-3444",
        "false_claim_timed::seed_qsl_3444_a_certificate_of_maximum_zero_is_rejected_for_the_strict_two_clock_claim",
    ),
    (
        "R-27",
        "QSL-3478",
        "zone_check::certificate::false_claim_tests::seed_qsl_3478_always_possible_forgery_is_rejected",
    ),
    (
        "R-28",
        "QSL-3478",
        "zone_check::certificate::false_claim_tests::seed_qsl_3478_unique_path_is_decided_violated",
    ),
    (
        "R-38",
        "QSL-923",
        "false_claim_hyper::seed_qsl_923_a_certificate_that_the_lasso_misses_the_acceptance_set_is_rejected",
    ),
];

fn spec() -> String {
    let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../spec/functional");
    let path = std::fs::read_dir(directory)
        .expect("the spec directory reads")
        .map(|entry| entry.expect("a directory entry").path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("FR-1051-"))
        })
        .expect("the FR-1051 spec exists");
    std::fs::read_to_string(path).expect("the FR-1051 spec reads")
}

/// The route numbers the Routes table names, in table order: `R-1 to R-16`
/// is a range, `R-17, R-18` a list and `R-32` one route.
fn table_routes() -> Vec<u32> {
    let spec = spec();
    let table = spec
        .split("### Routes")
        .nth(1)
        .and_then(|rest| rest.split("###").next())
        .expect("the spec has a Routes section");
    let number = |text: &str| -> u32 {
        text.trim()
            .strip_prefix("R-")
            .and_then(|digits| digits.parse().ok())
            .unwrap_or_else(|| panic!("a route id: {text:?}"))
    };
    let mut routes = Vec::new();
    for row in table.lines().filter(|line| line.starts_with("| R-")) {
        let ids = row.split('|').nth(1).expect("the id cell");
        for token in ids.split(',') {
            match token.split_once(" to ") {
                Some((first, last)) => routes.extend(number(first)..=number(last)),
                None => routes.push(number(token)),
            }
        }
    }
    routes
}

/// The Routes table names R-1 to R-62 once each, and every corpus entry
/// names a route of the table.
#[test]
fn the_routes_table_names_sixty_two_routes_and_the_corpus_names_only_them() {
    let routes = table_routes();
    assert_eq!(routes, (1..=62).collect::<Vec<_>>());
    let table: BTreeSet<String> = routes.iter().map(|route| format!("R-{route}")).collect();
    for (route, ticket, test) in CORPUS {
        assert!(
            table.contains(*route),
            "{route} is not a route of the table"
        );
        assert!(
            ticket.starts_with("QSL-"),
            "{route}: a seed names its ticket"
        );
        assert!(!test.is_empty(), "{route}: a seed names its test");
    }
    let named: BTreeSet<_> = CORPUS.iter().map(|(route, _, _)| *route).collect();
    assert_eq!(named.len(), CORPUS.len(), "a route has one corpus entry");
}

/// FR-1051-AC-16's gate: every route of the table has a corpus entry. It
/// fails naming each route without one.
#[test]
#[ignore = "QSL-3550: 58 of the 62 routes have no corpus entry yet"]
fn every_route_of_the_routes_table_has_a_corpus_entry() {
    let covered: BTreeSet<String> = CORPUS
        .iter()
        .map(|(route, _, _)| (*route).to_owned())
        .collect();
    let missing: Vec<String> = table_routes()
        .into_iter()
        .map(|route| format!("R-{route}"))
        .filter(|route| !covered.contains(route))
        .collect();
    assert!(
        missing.is_empty(),
        "routes with no corpus entry: {}",
        missing.join(", ")
    );
}
