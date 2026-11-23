// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2999: a driver outside the crate builds the separate program and
//! dependency clock namespaces of a proving package through
//! `fsl_replay::RequestClocks::for_replay`.

use std::collections::BTreeMap;

use filament_contract_model::ClockConfiguration;
use fsl_replay::{InvalidClock, RequestClocks};

fn clock(unit: &str) -> ClockConfiguration {
    ClockConfiguration::TimestampedEvent {
        timestamp_unit: unit.into(),
    }
}

/// The program rows answer `get` and `iter`; the dependency rows are a
/// separate namespace, so a name only the dependencies hold is not read as a
/// program member.
#[test]
fn for_replay_keeps_program_and_dependency_rows_apart() {
    let clocks = RequestClocks::for_replay(
        ("test".to_owned(), "program".to_owned()),
        BTreeMap::from([("shared".to_owned(), clock("ms"))]),
        BTreeMap::from([
            ("shared".to_owned(), clock("s")),
            ("dep-only".to_owned(), clock("s")),
        ]),
    )
    .expect("well-formed rows");
    assert_eq!(clocks.get("shared"), Some(&clock("ms")));
    assert_eq!(clocks.get("dep-only"), None);
    assert_eq!(
        clocks.iter().collect::<Vec<_>>(),
        vec![("shared", &clock("ms"))]
    );
    assert!(!clocks.is_empty());
}

/// Rows of either namespace that are not well formed are refused by name.
#[test]
fn for_replay_refuses_a_malformed_row_in_either_namespace() {
    let refused = |program: BTreeMap<String, ClockConfiguration>,
                   dependencies: BTreeMap<String, ClockConfiguration>| {
        RequestClocks::for_replay(("a".to_owned(), "i".to_owned()), program, dependencies)
            .expect_err("a malformed row refuses")
    };
    assert_eq!(
        refused(
            BTreeMap::from([(String::new(), clock("ms"))]),
            BTreeMap::new()
        ),
        InvalidClock {
            clock: String::new(),
            field: "name"
        }
    );
    assert_eq!(
        refused(
            BTreeMap::new(),
            BTreeMap::from([(String::new(), clock("s"))])
        ),
        InvalidClock {
            clock: String::new(),
            field: "name"
        }
    );
}
