// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-640-AC-8: the replay recompile's clocks come from the proving
//! package's `temporal_bindings` rows.

use super::ReplayPackage;
use filament_contract_model::ClockConfiguration;
use ix_trace_rs::trace;

fn package(rows: &str) -> ReplayPackage {
    let bytes = format!(r#"{{"name":"p","temporal_bindings":{rows}}}"#);
    ReplayPackage::read(bytes.as_bytes(), u64::MAX).expect("JSON")
}

/// Each row's `clock_name` names a member whose value is the row's
/// `clock`; a row two declarations share gives one member, and a row bound
/// by a model binding name is derived from the model and gives none.
#[trace("TC-2182", "FR-640-AC-8")]
#[test]
fn the_rows_give_one_member_per_trace_subject_clock() {
    let clocks = package(
        r#"[
        {"declaration":"a","occurrence":0,"clock_name":"plant-ms",
         "clock":{"kind":"timestamped_event","timestamp_unit":"ms"}},
        {"declaration":"b","occurrence":0,"clock_name":"plant-ms",
         "clock":{"kind":"timestamped_event","timestamp_unit":"ms"}},
        {"declaration":"c","occurrence":0,"clock_name":"model-steps",
         "clock":{"kind":"event_position","sequence_authority":"m"}},
        {"declaration":"d","occurrence":0,"clock_name":"sample",
         "clock":{"kind":"fixed_sample","epoch":{"kind":"integer","decimal":"0"},
                  "period":{"kind":"rational","numerator":"1","denominator":"2"},"unit":"s"}}
        ]"#,
    )
    .clocks()
    .expect("well-formed rows");
    assert_eq!(
        clocks.get("plant-ms"),
        Some(&ClockConfiguration::TimestampedEvent {
            timestamp_unit: "ms".into()
        })
    );
    assert!(matches!(
        clocks.get("sample"),
        Some(ClockConfiguration::FixedSample { unit, .. }) if &**unit == "s"
    ));
    assert_eq!(clocks.get("model-steps"), None);
    assert_eq!(
        package("[]").clocks().expect("no rows"),
        crate::spine::RequestClocks::default()
    );
}

/// A row whose `clock` is not a configuration refuses at the row's pointer.
#[trace("FR-640-AC-8")]
#[test]
fn a_malformed_row_refuses_at_its_pointer() {
    let refusal = package(
        r#"[{"declaration":"a","occurrence":0,"clock_name":"plant-ms",
            "clock":{"kind":"sundial"}}]"#,
    )
    .clocks()
    .expect_err("an unknown kind");
    assert_eq!(
        refusal.to_string(),
        super::field_refusal(
            "/package/temporal_bindings/0/clock",
            super::PackageMemberCause::WrongKind
        )
        .to_string()
    );
}

fn graph(modes: &str) -> ReplayPackage {
    let bytes = format!(r#"{{"name":"p","semantic_graph":{{"nodes":{modes}}}}}"#);
    ReplayPackage::read(bytes.as_bytes(), u64::MAX).expect("JSON")
}

/// FR-098-AC-34, FR-952: the recompile mode is the mode the package's
/// `function` nodes record; one `kernel` function selects `Kernel`, a
/// package with only `linked` functions (or none) selects `Linked`, and a
/// non-function node's members never select a mode.
#[trace("TC-1784", "FR-098-AC-34")]
#[test]
fn the_recompile_mode_is_the_recorded_function_mode() {
    use filament_semantic_value::checking::CheckMode;
    assert_eq!(
        graph(
            r#"[{"node_tag":"function","check_mode":"linked"},
               {"node_tag":"function","check_mode":"kernel"}]"#
        )
        .check_mode(),
        CheckMode::Kernel
    );
    assert_eq!(
        graph(r#"[{"node_tag":"function","check_mode":"linked"}]"#).check_mode(),
        CheckMode::Linked
    );
    assert_eq!(
        graph(r#"[{"node_tag":"expression","check_mode":"kernel"}]"#).check_mode(),
        CheckMode::Linked
    );
    assert_eq!(package("[]").check_mode(), CheckMode::Linked);
}

/// A package keeps the members replay reads after the read and the facts of
/// its function nodes, not its semantic graph: the descriptor and the
/// function modes read the same as from the whole document, and a root that
/// is not an object is still refused by the descriptor.
#[trace("TC-1784", "FR-098-AC-34")]
#[test]
fn a_package_keeps_the_replay_members_and_the_function_facts_only() {
    let bytes = br#"{"name":"p","lock":{"sources":[{"authority":"a","identity":"i"}]},
        "semantic_graph":{"nodes":[
          {"node_tag":"function","node_id":{"path":[1]},"declaration":{"qualified_name":["f"]},"check_mode":"kernel"},
          {"node_tag":"expression","body":{"term":"literal","value":1.0}}]},
        "extra":[1,2,3]}"#;
    let package = ReplayPackage::read(bytes, u64::MAX).expect("JSON");
    let (name, sources) = package.name_and_sources().expect("the descriptor");
    assert_eq!((name.as_str(), sources.len()), ("p", 1));
    assert_eq!(package.functions().len(), 1);
    assert_eq!(package.functions()[0].position, 0);
    assert_eq!(package.functions()[0].name.as_deref(), Some("f"));
    assert_eq!(package.functions()[0].mode.as_deref(), Some("kernel"));
    assert_eq!(package.bytes(), &bytes[..]);
    assert!(super::member(package.document.root(), "semantic_graph").is_none());
    assert!(super::member(package.document.root(), "extra").is_none());

    let not_an_object = ReplayPackage::read(b"[1]", u64::MAX).expect("JSON");
    assert!(not_an_object.name_and_sources().is_err());
}

/// The clock members a package keeps answer exactly what the whole document
/// would: the program rows, the dependency rows, their refusal pointers and
/// the program-source test, while the semantic graph and any other root
/// member are not held.
#[trace("TC-2182", "FR-640-AC-8")]
#[test]
fn a_package_keeps_the_clock_and_lock_members_it_reads() {
    use filament_contract_model::ClockConfiguration;
    use fsl_package::ClockSelectionCause;
    let bytes = br#"{"name":"p",
        "temporal_bindings":[{"declaration":"a","occurrence":0,"clock_name":"plant-ms",
            "clock":{"kind":"timestamped_event","timestamp_unit":"ms"}}],
        "lock":{"sources":[{"authority":"a","identity":"i"}],
            "dependency_clock_selections":[{"clock_name":"dep-ms",
                "clock":{"kind":"timestamped_event","timestamp_unit":"s"}}]},
        "semantic_graph":{"nodes":[{"node_tag":"expression"}]},
        "extra":[1,2,3]}"#;
    let package = ReplayPackage::read(bytes, u64::MAX).expect("JSON");
    let clock = |unit: &str| ClockConfiguration::TimestampedEvent {
        timestamp_unit: unit.into(),
    };
    let expected = crate::spine::RequestClocks::for_replay(
        ("a".to_owned(), "i".to_owned()),
        std::collections::BTreeMap::from([("plant-ms".to_owned(), clock("ms"))]),
        std::collections::BTreeMap::from([("dep-ms".to_owned(), clock("s"))]),
    )
    .expect("the rows are valid");
    assert_eq!(package.clocks().expect("well-formed rows"), expected);
    let root = "/package/lock/dependency_clock_selections";
    assert_eq!(
        package.dependency_clock_pointer("dep-ms", ClockSelectionCause::ClockKindMismatch),
        Some(format!("{root}/0/clock/kind"))
    );
    assert_eq!(
        package.dependency_clock_pointer("dep-ms", ClockSelectionCause::UnusedClockSelection),
        Some(format!("{root}/0"))
    );
    assert_eq!(
        package.dependency_clock_pointer("other", ClockSelectionCause::MissingClockSelection),
        Some(root.to_owned())
    );
    let program =
        fsl_foundation::source::provenance::RawSourceRef::new("a".to_owned(), "i".to_owned())
            .expect("a source");
    assert!(package.is_program_source(&program));
    assert!(super::member(package.document.root(), "semantic_graph").is_none());
    assert!(super::member(package.document.root(), "extra").is_none());
}

/// A clock configuration nested far past any configuration's shape refuses at
/// its row's pointer on a small stack, in either clock list.
#[trace("TC-2182", "FR-640-AC-8")]
#[test]
fn a_deeply_nested_clock_refuses_at_its_pointer_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for depth in [1_000, 16_000, 100_000] {
                let deep = format!("{}0{}", r#"{"a":"#.repeat(depth), "}".repeat(depth));
                let program = package(&format!(
                    r#"[{{"declaration":"a","occurrence":0,"clock_name":"plant-ms","clock":{deep}}}]"#
                ));
                assert_eq!(
                    program.clocks().expect_err("no configuration").pointer(),
                    Some("/package/temporal_bindings/0/clock"),
                    "depth {depth}"
                );
                let bytes = format!(
                    r#"{{"lock":{{"dependency_clock_selections":[{{"clock_name":"dep-ms","clock":{deep}}}]}}}}"#
                );
                let dependency = ReplayPackage::read(bytes.as_bytes(), u64::MAX).expect("JSON");
                assert_eq!(
                    dependency.clocks().expect_err("no configuration").pointer(),
                    Some("/package/lock/dependency_clock_selections/0/clock"),
                    "depth {depth}"
                );
            }
        })
        .expect("spawn the small-stack control")
        .join()
        .expect("no deep clock overflows");
}
