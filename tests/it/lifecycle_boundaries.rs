// SPDX-License-Identifier: AGPL-3.0-or-later
//! Caller-owned bounds and cancellation on actual lifecycle operations.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};

use filament_exact::{Cancel, CancelCause};
use fsl_foundation::diagnostic::{LimitExceeded, LimitKind, Locus, StageFailure};
use fsl_foundation::{Setting, SettingLimits, SourceIdentity};
use fsl_replay::spine::{
    self, CheckedUnit, DependencyInput, FrontEndFailure, PackageLimits, ParseRequest,
    ParsedUnitSource, SpineLimits,
};
use ix_trace_rs::trace;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    function f using v(): Integer pure { 1 + 1 + 1 }\n";

fn parse(
    text: &str,
    limits: SpineLimits,
    cancel: &Cancel,
) -> Result<ParsedUnitSource, FrontEndFailure> {
    spine::parse(
        &ParseRequest {
            source: &SourceIdentity::new("agent-ix", "lifecycle-boundaries"),
            path: "lifecycle-boundaries.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        cancel,
    )
    .map(|parsed| parsed.into_value())
}

fn check(
    parsed: &ParsedUnitSource,
    limits: SpineLimits,
    cancel: &Cancel,
) -> Result<CheckedUnit, FrontEndFailure> {
    let packages = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("empty domain provision");
    let models = spine::select(parsed, &packages, limits.model, &Cancel::new())
        .expect("a pure-value source needs no model")
        .into_value();
    spine::check(
        parsed,
        "lifecycle-boundaries".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        cancel,
    )
    .map(|checked| checked.into_value())
}

fn checked() -> CheckedUnit {
    let parsed = parse(UNIT, SpineLimits::default(), &Cancel::new()).expect("source parses");
    check(&parsed, SpineLimits::default(), &Cancel::new()).expect("sum checks")
}

fn limit(failure: FrontEndFailure) -> LimitExceeded {
    match failure {
        StageFailure::Limit(limit) | StageFailure::LimitWithPayload(limit, _) => limit,
        other => panic!("expected a reached caller limit: {other:?}"),
    }
}

#[test]
#[trace("FR-255-AC-3")]
fn changing_one_compile_setting_preserves_other_bounds_and_emit_rejects_source_settings() {
    let original = SpineLimits::default();
    let rows = original.bounds();
    let baseline: BTreeMap<_, _> = rows.iter().copied().collect();
    assert_eq!(
        rows.len(),
        baseline.len(),
        "each compiler setting has one owner"
    );
    assert!(baseline.contains_key(&Setting::S1InputBytes));
    assert!(baseline.contains_key(&Setting::S3Nodes));
    assert!(baseline.contains_key(&Setting::EmitOutputBytes));

    for &setting in baseline.keys() {
        for bound in [0, 7, baseline[&setting]] {
            let mut changed = original;
            assert!(
                changed.set_bound(setting, bound),
                "{} is owned",
                setting.name()
            );
            let mut expected = baseline.clone();
            expected.insert(setting, bound);
            let actual: BTreeMap<_, _> = changed.bounds().into_iter().collect();
            assert_eq!(actual, expected, "only {} changes", setting.name());
        }
    }
    let mut maximum_sequence = original;
    assert!(maximum_sequence.set_bound(Setting::ModelSequenceMaximum, u64::from(u32::MAX)));
    let mut expected = baseline;
    expected.insert(Setting::ModelSequenceMaximum, u64::from(u32::MAX));
    assert_eq!(
        maximum_sequence
            .bounds()
            .into_iter()
            .collect::<BTreeMap<_, _>>(),
        expected
    );
    let original_emit = PackageLimits::default();
    let mut unchanged_emit = original_emit;
    assert!(!unchanged_emit.set_bound(Setting::S1InputBytes, 0));
    assert_eq!(unchanged_emit, original_emit);
}

#[test]
#[trace("FR-255-AC-1", "FR-277-AC-1")]
fn source_byte_ceiling_counts_utf8_bytes_and_accepts_the_exact_bound() {
    let text = format!("{UNIT}// café ☃\n");
    let bytes = u64::try_from(text.len()).expect("small input");
    assert!(text.chars().count() < text.len());
    let mut below = SpineLimits::default();
    assert!(below.set_bound(Setting::S1InputBytes, bytes - 1));
    let denied = limit(parse(&text, below, &Cancel::new()).expect_err("one missing byte refuses"));
    assert_eq!(denied.setting(), Setting::S1InputBytes);
    assert_eq!(denied.kind(), LimitKind::InputBytes);
    assert_eq!(denied.configured_bound(), bytes - 1);
    assert_eq!(denied.actual(), u128::from(bytes));
    let mut exact = below;
    assert!(exact.set_bound(Setting::S1InputBytes, bytes));
    let admitted = parse(&text, exact, &Cancel::new()).expect("the byte ceiling is inclusive");
    check(&admitted, exact, &Cancel::new()).expect("the complete UTF-8 source checks");
}

#[test]
#[trace("FR-255-AC-2", "FR-277-AC-1")]
fn the_fifth_expression_node_names_its_s3_setting_and_source_locus() {
    let parsed = parse(UNIT, SpineLimits::default(), &Cancel::new()).expect("source parses");
    let mut four = SpineLimits::default();
    assert!(four.set_bound(Setting::S3Nodes, 4));
    let denied = limit(check(&parsed, four, &Cancel::new()).expect_err("five nodes exceed four"));
    assert_eq!(denied.setting(), Setting::S3Nodes);
    assert_eq!(denied.kind(), LimitKind::NodeCount);
    assert_eq!(denied.configured_bound(), 4);
    assert_eq!(denied.actual(), 5);
    let Some(Locus::Region(region)) = denied.locus() else {
        panic!("the denied expression has an authored source locus: {denied:?}");
    };
    let last_literal = UNIT
        .rfind('1')
        .expect("the independent fifth expression node");
    assert_eq!(
        usize::try_from(region.start()).expect("small source"),
        last_literal
    );
    assert_eq!(
        usize::try_from(region.end()).expect("small source"),
        last_literal + 1
    );
    let rendered = denied.to_string();
    assert!(
        rendered.contains("S3 node limit 4 reached (5)"),
        "{rendered}"
    );
    assert!(rendered.contains("--limit s3.nodes="), "{rendered}");
    let mut five = four;
    assert!(five.set_bound(Setting::S3Nodes, 5));
    check(&parsed, five, &Cancel::new()).expect("the exact five-node bound admits");
}

#[test]
#[trace("FR-255-AC-1", "FR-277-AC-1")]
fn package_output_ceiling_is_inclusive_and_never_returns_a_truncated_artifact() {
    let checked = checked();
    let complete = spine::package(&checked, PackageLimits::default(), &Cancel::new())
        .expect("the real checked source emits")
        .into_value();
    let bytes = complete.package().bytes();
    let length = u64::try_from(bytes.len()).expect("small package");
    let denied = limit(
        spine::package(
            &checked,
            PackageLimits::default().with_output_bytes(length - 1),
            &Cancel::new(),
        )
        .expect_err("one byte below the actual canonical artifact denies emission"),
    );
    assert_eq!(denied.setting(), Setting::EmitOutputBytes);
    assert_eq!(denied.kind(), LimitKind::InputBytes);
    assert_eq!(denied.configured_bound(), length - 1);
    assert_eq!(denied.actual(), u128::from(length));
    let exact = spine::package(
        &checked,
        PackageLimits::default().with_output_bytes(length),
        &Cancel::new(),
    )
    .expect("the exact byte bound admits the whole artifact")
    .into_value();
    assert_eq!(exact.package().bytes(), bytes);
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("complete JSON artifact");
    assert!(
        value
            .get("semantic_graph")
            .and_then(|graph| graph.get("nodes"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|nodes| !nodes.is_empty()),
        "the complete artifact retains its semantic graph"
    );
}

struct ChargeCancellation {
    handle: Cancel,
    observed: Arc<AtomicU32>,
    _owner: Arc<OnceLock<Cancel>>,
}

fn at_charge(charge: u32, cause: CancelCause) -> ChargeCancellation {
    let slot = Arc::new(OnceLock::<Cancel>::new());
    let held = Arc::downgrade(&slot);
    let observed = Arc::new(AtomicU32::new(0));
    let count = Arc::clone(&observed);
    let cancel = Cancel::observing(move || {
        if count.fetch_add(1, Ordering::SeqCst) + 1 == charge {
            let slot = held.upgrade().expect("the observer's owner is live");
            slot.get()
                .expect("handle installed before execution")
                .cancel(cause);
        }
    });
    slot.set(cancel.clone()).expect("one handle installed");
    ChargeCancellation {
        handle: cancel,
        observed,
        _owner: slot,
    }
}

#[test]
#[trace("FR-276")]
fn parse_cancellation_at_a_real_charge_never_returns_recovered_or_partial_forms() {
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        for charge in [1, 4, 8] {
            let cancellation = at_charge(charge, cause);
            let result = parse(UNIT, SpineLimits::default(), &cancellation.handle);
            assert!(
                matches!(result, Err(StageFailure::Cancelled(found)) if found == cause),
                "charge {charge}: {result:?}"
            );
            assert_eq!(cancellation.handle.tripped(), Some(cause));
            let observed = cancellation.observed.load(Ordering::SeqCst);
            assert!(
                (charge..=charge + 1).contains(&observed),
                "stopped after {observed} polls at {charge}"
            );
        }
    }
}

#[test]
#[trace("FR-276")]
fn checking_cancellation_at_a_real_charge_never_returns_a_checked_unit() {
    let parsed = parse(UNIT, SpineLimits::default(), &Cancel::new()).expect("source parses");
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        for charge in [1, 2, 4] {
            let cancellation = at_charge(charge, cause);
            let result = check(&parsed, SpineLimits::default(), &cancellation.handle);
            assert!(
                matches!(result, Err(StageFailure::Cancelled(found)) if found == cause),
                "charge {charge}: {result:?}"
            );
            assert_eq!(cancellation.handle.tripped(), Some(cause));
            let observed = cancellation.observed.load(Ordering::SeqCst);
            assert!(
                (charge..=charge + 1).contains(&observed),
                "stopped after {observed} polls at {charge}"
            );
        }
    }
}

#[test]
#[trace("FR-276")]
fn emission_cancellation_at_its_first_charge_produces_no_package() {
    let checked = checked();
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let cancellation = at_charge(1, cause);
        let result = spine::package(&checked, PackageLimits::default(), &cancellation.handle);
        assert!(
            matches!(result, Err(StageFailure::Cancelled(found)) if found == cause),
            "{result:?}"
        );
        assert_eq!(cancellation.handle.tripped(), Some(cause));
        assert!((1..=2).contains(&cancellation.observed.load(Ordering::SeqCst)));
    }
}

#[test]
#[trace("FR-276-AC-2")]
#[ignore = "QSL-2720: S3 checking continues charging after its deadline; strict next-charge oracle"]
fn a_deadline_from_another_thread_stops_two_hundred_thousand_declarations_at_the_next_charge() {
    let mut text = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n"
        .to_owned();
    for index in 0..200_000 {
        use std::fmt::Write;
        writeln!(text, "enum E{index} {{ A, B }}").expect("String writes are infallible");
    }
    let source_bytes = u64::try_from(text.len()).expect("input length");
    let mut limits = SpineLimits::default();
    for (setting, bound) in [
        (Setting::S1InputBytes, source_bytes),
        (Setting::S1Tokens, source_bytes + 1),
        (Setting::S1Nodes, source_bytes * 4),
        (Setting::S1WorkUnits, 1_000_000_000),
        (Setting::S3Nodes, 4_000_000),
        (Setting::S3InputBytes, source_bytes),
    ] {
        assert!(limits.set_bound(setting, bound));
    }
    let parsed = parse(&text, limits, &Cancel::new()).expect("the declared input provision fits");
    let (trigger_tx, trigger_rx) = mpsc::sync_channel::<()>(0);
    let trigger_owner = Arc::new(trigger_tx);
    let trigger = Arc::downgrade(&trigger_owner);
    let (ack_tx, ack_rx) = mpsc::sync_channel::<()>(0);
    let ack = Mutex::new(ack_rx);
    let observed = Arc::new(AtomicU32::new(0));
    let count = Arc::clone(&observed);
    let cancel = Cancel::observing(move || {
        if count.fetch_add(1, Ordering::SeqCst) + 1 == 1_000 {
            trigger
                .upgrade()
                .expect("the checking call is live")
                .send(())
                .expect("deadline listener");
            ack.lock()
                .expect("observer lock")
                .recv()
                .expect("deadline was recorded");
        }
    });
    let deadline = cancel.clone();
    let listener = std::thread::spawn(move || {
        if trigger_rx.recv().is_ok() {
            deadline.cancel(CancelCause::Deadline);
            ack_tx.send(()).expect("the observer is waiting");
        }
    });
    let result = check(&parsed, limits, &cancel);
    drop(trigger_owner);
    listener.join().expect("deadline listener completes");
    match result {
        Err(StageFailure::Cancelled(CancelCause::Deadline)) => {}
        Err(other) => panic!("checking did not retain the deadline: {other:?}"),
        Ok(_) => panic!("checking returned a package after the requested deadline"),
    }
    let charges = observed.load(Ordering::SeqCst);
    assert!(
        (1_000..=1_001).contains(&charges),
        "deadline reached at charge1000, observed {charges}"
    );
    assert_eq!(cancel.tripped(), Some(CancelCause::Deadline));
}
