// SPDX-License-Identifier: AGPL-3.0-or-later
use fsl_proptest_policy::{config_with_overrides, SeedPersistence};
use proptest::test_runner::{TestCaseError, TestError, TestRunner};
use std::cell::{Cell, RefCell};

fn sequence(cases: Option<&str>, seed: Option<&str>) -> Vec<u32> {
    let values = RefCell::new(Vec::new());
    let mut config = config_with_overrides(32, cases, seed).unwrap();
    config.failure_persistence = None;
    TestRunner::new(config)
        .run(&(0u32..1_000_000), |value| {
            values.borrow_mut().push(value);
            Ok(())
        })
        .unwrap();
    values.into_inner()
}

/// Provenance: QSL-2054. Overrides are scoped inputs, never global env mutation.
#[test]
fn config_is_deterministic_under_fixed_seed() {
    let first = sequence(None, None);
    assert_eq!(first, sequence(None, None));
    assert_ne!(first, sequence(None, Some("987654321")));
    assert_eq!(sequence(Some("17"), None).len(), 17);
    assert!(config_with_overrides(32, Some("0"), None).is_err());
    assert!(config_with_overrides(32, None, Some("invalid")).is_err());
}

/// Provenance: QSL-2054. A real failing property shrinks and persists a replay seed.
#[test]
fn failing_property_shrinks_and_persists() {
    let directory = std::env::temp_dir().join(format!("fsl-property-seeds-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let mut config = config_with_overrides(1, None, None).unwrap();
    config.failure_persistence = Some(Box::new(SeedPersistence::new(&directory)));
    config.source_file = Some("false_property.rs");
    let calls = Cell::new(0);
    let first = Cell::new(None);
    let outcome = TestRunner::new(config).run(&(1u32..1_000_000), |value| {
        if first.get().is_none() {
            first.set(Some(value));
        }
        calls.set(calls.get() + 1);
        Err(TestCaseError::fail("deliberately false"))
    });
    let Err(TestError::Fail(_, minimal)) = outcome else {
        panic!("false property must fail");
    };
    assert!(minimal < first.get().unwrap());
    assert!(
        calls.get() <= 258,
        "bounded shrinking: {} calls",
        calls.get()
    );
    let text = std::fs::read_to_string(directory.join("false_property.txt")).unwrap();
    let seeds = text
        .lines()
        .map(|line| {
            line.parse::<proptest::test_runner::PersistedSeed>()
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(seeds.len(), 1);
    std::fs::remove_dir_all(directory).unwrap();
}
