// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-932 (TC-2604): `tl-mltl` and fsl-eval give one verdict on the bounded
//! past and future fragment, reached through `fsl-command`'s `tl_lowering`
//! cross-evaluation (FR-931), so no FSL crate but `fsl-command` names a TL
//! crate. The fsl-eval-against-reference half is
//! `fsl-eval/tests/it/mltl_differential.rs`.

use crate::command_harness::run_file;
use ix_trace_rs::trace;
use serde_json::{json, Value};

const PAST: &[&str] = &[
    "once[0,2] holds(c.level = 1)",
    "historically[1,2] holds(c.level = 0)",
    "historically[0,1] not holds(c.level = 0)",
    "holds(c.level = 0) since[0,2] holds(c.level = 1)",
    "holds(c.level = 0) triggered[1,2] holds(c.level = 1)",
    "once[1,1] holds(c.level = 0)",
    "once[1,1] not holds(c.level = 0)",
    "historically[0,2] holds(c.level = 0)",
    "historically[0,2] true",
    "true since[1,1] not holds(c.level = 0)",
];

const FUTURE: &[&str] = &[
    "eventually[0,2] holds(c.level = 1)",
    "always[1,2] holds(c.level = 0)",
    "holds(c.level = 0) until[0,2] holds(c.level = 1)",
    "holds(c.level = 0) release[0,1] holds(c.level = 1)",
];

const MIXED: &str = "eventually[1,1] once[1,1] not holds(c.level = 0)";

/// Every trace of length 1 to 4 over the values 0, 1 and 2.
fn traces() -> Vec<Vec<i64>> {
    let mut all = Vec::new();
    for len in 1..=4u32 {
        for code in 0..3i64.pow(len) {
            let mut code = code;
            let mut values = Vec::new();
            for _ in 0..len {
                values.push(code % 3);
                code /= 3;
            }
            all.push(values);
        }
    }
    all
}

/// Runs `C` (`body`, `on each`) over the samples `values`: through
/// `tl_lowering` with cross-evaluation when `lowering`, else as a clause run.
fn run(body: &str, values: &[i64], lowering: bool) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
         record Counter {{ level: Int[0, 9]; }}\n\
         temporal C using e over (c: Counter) clock \"events\" on each (q: Counter) {{ {body} }}\n"
    );
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let samples: Vec<Value> = values
        .iter()
        .map(|level| json!({"sample": {"level": {"integer": level.to_string()}}}))
        .collect();
    let trace = json!({"finite": samples});
    let mut request = json!({
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:mltl-differential", "document": "Mltl", "formal_revision": 1}},
        "clocks": {"events": {"kind": "event_position", "sequence_authority": "events"}},
    });
    if lowering {
        request["tl_lowering"] = json!({"clause": "C", "cross_evaluate": {"trace": trace}});
    } else {
        request["clause"] =
            json!({"selection": {"temporal": {"name": "C", "trace": trace}}, "observations": {}});
    }
    let path = directory.path().join("request.json");
    std::fs::write(
        &path,
        json!({"format": "native-run/1", "request": request}).to_string(),
    )
    .unwrap();
    let output = run_file(&path);
    let code = u8::try_from(output.status.code().expect("an exit code")).expect("a small code");
    let text = if output.stdout.is_empty() {
        output.stderr
    } else {
        output.stdout
    };
    (
        code,
        serde_json::from_slice(&text).expect("a JSON document"),
    )
}

/// FR-932-AC-2: every past and future case cross-evaluates with no fault
/// under its default correspondence; the mixed case is unsupported.
#[test]
#[trace("TC-2604", "FR-932-AC-2")]
fn fsl_eval_the_reference_and_tl_mltl_agree() {
    for (cases, correspondence) in [
        (PAST, "mltl.origin-complete-history/v1"),
        (FUTURE, "mltl.closed-trace/v1"),
    ] {
        for body in cases {
            for values in traces() {
                let (code, value) = run(body, &values, true);
                let record = &value["tl_lowering"];
                assert_eq!(code, 0, "{body} over {values:?}: {value}");
                assert_eq!(record["correspondence"], correspondence, "{body}");
                assert_eq!(
                    record["cross_evaluation"]["agree"], true,
                    "{body} {values:?}"
                );
                assert_eq!(record["cross_evaluation"]["faults"], json!([]));
            }
        }
    }
    let (code, value) = run(MIXED, &[0, 1], true);
    assert_eq!(code, 21, "{value}");
    assert_eq!(value["tl_lowering"]["disposition"], "unsupported");
    assert_eq!(value["tl_lowering"]["dimensions"], json!(["direction"]));
}

/// FR-932-AC-3: on `0, 0, 0`, `once[1,1] not p` holds at position 0 and
/// `historically[0,2] p` fails there, natively and in `tl-mltl`; clipping the
/// closed past window at the origin would give the opposite on both.
#[test]
#[trace("TC-2604", "FR-932-AC-3")]
fn the_pre_origin_reading_is_one_reading() {
    let values = [0, 0, 0];
    let clipped_once_not = |_i: usize| false; // window at 0 clipped to nothing
    let clipped_historically = |_i: usize| true; // window at 0 clipped to {0}
    for (body, native_holds_at_zero, clipped) in [
        (
            "once[1,1] not holds(c.level = 0)",
            true,
            clipped_once_not(0),
        ),
        (
            "historically[0,2] holds(c.level = 0)",
            false,
            clipped_historically(0),
        ),
    ] {
        assert_ne!(
            native_holds_at_zero, clipped,
            "the clipping reading disagrees"
        );
        let (_, value) = run(body, &values, false);
        let text = value.to_string();
        if native_holds_at_zero {
            assert!(!text.contains("\"position\":0"), "{body}: {text}");
        } else {
            assert!(text.contains("violation"), "{body}: {text}");
        }
        let (code, value) = run(body, &values, true);
        assert_eq!(code, 0, "{value}");
        assert_eq!(
            value["tl_lowering"]["cross_evaluation"]["agree"], true,
            "{body}"
        );
    }
}

/// FR-757-AC-2, FR-931-AC-4: through the `fsl-command` library entry, an atom
/// table that maps `p0` to the negated leaf renders through the record writer
/// as one `correspondence-loss` fault naming the clause, the trace, the
/// position and both verdicts, and the run's exit is 30; the faithful table
/// agrees.
#[test]
#[trace("TC-2107", "FR-757-AC-2")]
#[trace("TC-2603", "FR-931-AC-4")]
fn a_mis_mapped_atom_is_a_correspondence_loss() {
    let directory = tempfile::tempdir().unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
         record Counter { level: Int[0, 9]; }\n\
         temporal C using e over (c: Counter) clock \"events\" on origin \
         { always[0,2] holds(c.level = 1) }\n";
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let samples: Vec<Value> = [1, 1, 1]
        .iter()
        .map(|level: &i64| json!({"sample": {"level": {"integer": level.to_string()}}}))
        .collect();
    let request = json!({"format": "native-run/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:mltl-mismap", "document": "Mltl", "formal_revision": 1}},
        "clocks": {"events": {"kind": "event_position", "sequence_authority": "events"}},
        "clause": {"selection": {"temporal": {"name": "C", "trace": {"finite": samples}}},
            "observations": {}}}});
    let path = directory.path().join("request.json");
    std::fs::write(&path, request.to_string()).unwrap();
    let clause_run = fsl_command::command::read_clause_run_request(&path).expect("a clause run");
    let report = fsl_replay::spine::run_temporal_valuation(clause_run).expect("a report");
    let fsl_replay::spine::ClauseDisposition::TemporalValuation(valuation) = &report.disposition
    else {
        panic!("a valuation: {:?}", report.disposition);
    };
    let faithful = fsl_command::command::tl_record(
        valuation,
        unit,
        None,
        &fsl_command::command::AtomMapping::default(),
    )
    .expect("the clause lowers");
    assert_eq!(faithful.exit_code, 0, "{}", faithful.record);
    assert_eq!(faithful.record["cross_evaluation"]["agree"], true);

    let mut mapping = fsl_command::command::AtomMapping::default();
    mapping.negated.insert("p0".to_owned());
    let mismapped = fsl_command::command::tl_record(valuation, unit, None, &mapping)
        .expect("the clause lowers");
    assert_eq!(mismapped.exit_code, 30, "{}", mismapped.record);
    let cross = &mismapped.record["cross_evaluation"];
    assert_eq!(cross["agree"], false);
    assert_eq!(
        cross["faults"],
        json!([{"kind": "correspondence-loss", "clause": "C", "trace": 0, "position": 0,
            "native": true, "tl": false}])
    );
}

/// FR-931-AC-8: each supported lowering of the corpus carries the
/// classification read from its lowered formula, equal to the clause's own
/// (its default correspondence and its operators' direction); a named
/// online-prefix correspondence is carried as named; the mixed case has none.
#[test]
#[trace("TC-2979", "FR-931-AC-8")]
fn lowering_preserves_the_classification() {
    for (cases, correspondence, fragment) in [
        (PAST, "mltl.origin-complete-history/v1", "bounded-past"),
        (FUTURE, "mltl.closed-trace/v1", "bounded-future"),
    ] {
        for body in cases {
            for values in traces() {
                let (code, value) = run(body, &values, true);
                let record = &value["tl_lowering"];
                assert_eq!(code, 0, "{body} over {values:?}: {value}");
                assert_eq!(
                    record["classification"],
                    json!({"correspondence": correspondence, "fragment": fragment}),
                    "{body} over {values:?}"
                );
                assert_eq!(
                    record["classification"]["correspondence"], record["correspondence"],
                    "{body}"
                );
                assert_eq!(record["classification_fault"], Value::Null, "{body}");
            }
        }
    }

    let directory = tempfile::tempdir().unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
         record Counter { level: Int[0, 9]; }\n\
         temporal C using e over (c: Counter) clock \"events\" on origin \
         { always[0,3] holds(c.level <= 2) }\n";
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:mltl-classification", "document": "Mltl", "formal_revision": 1}},
        "clocks": {"events": {"kind": "event_position", "sequence_authority": "events"}},
        "tl_lowering": {"clause": "C", "correspondence": "mltl.online-prefix/v1"}}});
    let path = directory.path().join("request.json");
    std::fs::write(&path, request.to_string()).unwrap();
    let output = run_file(&path);
    assert_eq!(output.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&output.stdout).expect("a JSON document");
    assert_eq!(
        value["tl_lowering"]["classification"],
        json!({"correspondence": "mltl.online-prefix/v1", "fragment": "bounded-future"}),
        "{value}"
    );

    let (code, value) = run(MIXED, &[0, 1], true);
    assert_eq!(code, 21, "{value}");
    assert_eq!(value["tl_lowering"]["disposition"], "unsupported");
    assert_eq!(
        value["tl_lowering"]["classification"],
        Value::Null,
        "{value}"
    );
}

/// FR-931-AC-8, negative half: a lowered formula stamped with the wrong
/// correspondence at the `fsl-command` library entry (never through the
/// request) is a classification loss naming the clause and both
/// classifications, with no `classification`, and the run's exit is 30; the
/// faithful lowering carries the native classification.
#[test]
#[trace("TC-2979", "FR-931-AC-8")]
fn a_wrong_lowered_profile_is_a_classification_loss() {
    let directory = tempfile::tempdir().unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
         record Counter { level: Int[0, 9]; }\n\
         temporal C using e over (c: Counter) clock \"events\" on origin \
         { always[0,2] holds(c.level = 1) }\n";
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let samples: Vec<Value> = [1, 1, 1]
        .iter()
        .map(|level: &i64| json!({"sample": {"level": {"integer": level.to_string()}}}))
        .collect();
    let request = json!({"format": "native-run/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:mltl-classification-loss", "document": "Mltl", "formal_revision": 1}},
        "clocks": {"events": {"kind": "event_position", "sequence_authority": "events"}},
        "clause": {"selection": {"temporal": {"name": "C", "trace": {"finite": samples}}},
            "observations": {}}}});
    let path = directory.path().join("request.json");
    std::fs::write(&path, request.to_string()).unwrap();
    let clause_run = fsl_command::command::read_clause_run_request(&path).expect("a clause run");
    let report = fsl_replay::spine::run_temporal_valuation(clause_run).expect("a report");
    let fsl_replay::spine::ClauseDisposition::TemporalValuation(valuation) = &report.disposition
    else {
        panic!("a valuation: {:?}", report.disposition);
    };
    let native = json!({"correspondence": "mltl.closed-trace/v1", "fragment": "bounded-future"});

    let faithful = fsl_command::command::tl_record(
        valuation,
        unit,
        None,
        &fsl_command::command::AtomMapping::default(),
    )
    .expect("the clause lowers");
    assert_eq!(faithful.exit_code, 0, "{}", faithful.record);
    assert_eq!(faithful.record["classification"], native);

    let mapping = fsl_command::command::AtomMapping {
        lowered_correspondence: Some("mltl.origin-complete-history/v1".to_owned()),
        ..fsl_command::command::AtomMapping::default()
    };
    let lost = fsl_command::command::tl_record(valuation, unit, None, &mapping)
        .expect("the clause lowers");
    assert_eq!(lost.exit_code, 30, "{}", lost.record);
    assert_eq!(
        lost.record["classification"],
        Value::Null,
        "{}",
        lost.record
    );
    assert_eq!(
        lost.record["classification_fault"],
        json!({"kind": "classification-loss", "clause": "C", "native": native,
            "lowered": {"correspondence": "mltl.origin-complete-history/v1",
                "fragment": "bounded-future"}})
    );
}

const INFINITE_PROFILE: &str = "filament.temporal.infinite-trace/v1";

/// Every lasso of a prefix of 0 to 2 samples and a loop of 1 to 3, at most 3
/// samples in all, over the values 0, 1 and 2, as (prefix length, samples).
fn lassos() -> Vec<(usize, Vec<i64>)> {
    let mut all = Vec::new();
    for len in 1..=3u32 {
        for prefix in 0..usize::try_from(len).expect("a small lasso") {
            for code in 0..3i64.pow(len) {
                let mut code = code;
                let mut values = Vec::new();
                for _ in 0..len {
                    values.push(code % 3);
                    code /= 3;
                }
                all.push((prefix, values));
            }
        }
    }
    all
}

/// A `native-run/1` request over the unit holding `temporal C` (`body`,
/// `head`) under `profile`, with `extra` merged into the request.
fn request_over(profile: &str, head: &str, body: &str, extra: Value) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile e = \"{profile}\";\n\
         record Counter {{ level: Int[0, 9]; }}\n\
         temporal C using e over (c: Counter) clock \"events\" {head} {{ {body} }}\n"
    );
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let mut request = json!({
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:mltl-infinite", "document": "Mltl", "formal_revision": 1}},
        "clocks": {"events": {"kind": "event_position", "sequence_authority": "events"}},
    });
    for (key, value) in extra.as_object().expect("an object") {
        request[key] = value.clone();
    }
    let path = directory.path().join("request.json");
    std::fs::write(
        &path,
        json!({"format": "native-run/1", "request": request}).to_string(),
    )
    .unwrap();
    let output = run_file(&path);
    let code = u8::try_from(output.status.code().expect("an exit code")).expect("a small code");
    let text = if output.stdout.is_empty() {
        output.stderr
    } else {
        output.stdout
    };
    (
        code,
        serde_json::from_slice(&text).expect("a JSON document"),
    )
}

fn samples(values: &[i64]) -> Vec<Value> {
    values
        .iter()
        .map(|level| json!({"sample": {"level": {"integer": level.to_string()}}}))
        .collect()
}

/// The lasso trace member of `values` with a prefix of `prefix` samples.
fn lasso(prefix: usize, values: &[i64]) -> Value {
    json!({"lasso": {"prefix": samples(&values[..prefix]), "loop": samples(&values[prefix..])}})
}

/// Lowers `C` (`body`, `on each`, infinite-trace) with `lowering` as the
/// `tl_lowering` member.
fn lower_infinite(body: &str, lowering: Value) -> (u8, Value) {
    request_over(
        INFINITE_PROFILE,
        "on each (q: Counter)",
        body,
        json!({ "tl_lowering": lowering }),
    )
}

/// FR-932-AC-5: every past case of the corpus, under the infinite-trace
/// profile, lowers by default to `mltl.infinite-trace/v1` with the
/// bounded-past classification and cross-evaluates with no fault over every
/// small lasso; over the lasso with loop `0`, `once[1,1] not p` holds at
/// position 0 and `historically[0,2] p` fails there, natively and in
/// `tl-mltl`, the opposite of the clipping reading.
#[test]
#[trace("TC-3024", "FR-932-AC-5")]
#[ignore = "QSL-2505: fsl-command does not lower infinite-trace past clauses to mltl.infinite-trace/v1 yet"]
fn tl_mltl_agrees_on_every_lasso() {
    for body in PAST {
        for (prefix, values) in lassos() {
            let (code, value) = lower_infinite(
                body,
                json!({"clause": "C", "cross_evaluate": {"trace": lasso(prefix, &values)}}),
            );
            let record = &value["tl_lowering"];
            assert_eq!(code, 0, "{body} over {prefix} {values:?}: {value}");
            assert_eq!(record["correspondence"], "mltl.infinite-trace/v1", "{body}");
            assert_eq!(
                record["classification"],
                json!({"correspondence": "mltl.infinite-trace/v1", "fragment": "bounded-past"}),
                "{body}"
            );
            assert_eq!(
                record["cross_evaluation"]["agree"], true,
                "{body} over {prefix} {values:?}"
            );
            assert_eq!(record["cross_evaluation"]["faults"], json!([]));
        }
    }

    let clipped_once_not = false; // the window at 0 clipped to nothing
    let clipped_historically = true; // the window at 0 clipped to {0}
    for (body, holds_at_zero, clipped) in [
        ("once[1,1] not holds(c.level = 0)", true, clipped_once_not),
        (
            "historically[0,2] holds(c.level = 0)",
            false,
            clipped_historically,
        ),
    ] {
        assert_ne!(holds_at_zero, clipped, "the clipping reading disagrees");
        let (_, value) = request_over(
            INFINITE_PROFILE,
            "on origin",
            body,
            json!({"clause": {"selection": {"temporal": {"name": "C",
                "trace": lasso(0, &[0])}}, "observations": {}}}),
        );
        let text = value.to_string();
        if holds_at_zero {
            assert!(!text.contains("violat"), "{body}: {text}");
        } else {
            assert!(text.contains("violat"), "{body}: {text}");
        }
        let (code, value) = lower_infinite(
            body,
            json!({"clause": "C", "cross_evaluate": {"trace": lasso(0, &[0])}}),
        );
        assert_eq!(code, 0, "{value}");
        assert_eq!(
            value["tl_lowering"]["cross_evaluation"]["agree"], true,
            "{body}"
        );
    }
}

/// FR-931-AC-9: an infinite-trace closed-past clause lowers to
/// `mltl.infinite-trace/v1` by default and when named, cross-evaluates over a
/// lasso, and a correspondence that reads the other kind of trace reports
/// `profile`; the infinite-trace clauses outside the row stay unsupported.
#[test]
#[trace("TC-3023", "FR-931-AC-9")]
#[ignore = "QSL-2505: tl_lowering has no mltl.infinite-trace/v1 correspondence yet"]
fn an_infinite_trace_past_clause_lowers_to_the_infinite_trace_correspondence() {
    let back = "once[1,1] not holds(c.level = 0)";
    let (default_code, default) = lower_infinite(back, json!({"clause": "C"}));
    let (named_code, named) = lower_infinite(
        back,
        json!({"clause": "C", "correspondence": "mltl.infinite-trace/v1"}),
    );
    assert_eq!((default_code, named_code), (0, 0), "{default} {named}");
    assert_eq!(default, named);
    let record = &default["tl_lowering"];
    assert_eq!(record["disposition"], "supported", "{record}");
    assert_eq!(record["correspondence"], "mltl.infinite-trace/v1");
    assert_eq!(
        record["classification"],
        json!({"correspondence": "mltl.infinite-trace/v1", "fragment": "bounded-past"})
    );
    assert!(record["formula"].is_string(), "{record}");

    let (code, value) = lower_infinite(
        back,
        json!({"clause": "C", "cross_evaluate": {"trace": lasso(0, &[0])}}),
    );
    assert_eq!(code, 0, "{value}");
    let cross = &value["tl_lowering"]["cross_evaluation"];
    assert_eq!(cross["traces"], 1, "{cross}");
    assert!(
        cross["positions"].as_u64().is_some_and(|n| n >= 1),
        "{cross}"
    );
    assert_eq!(cross["agree"], true);
    assert_eq!(cross["faults"], json!([]));

    let (code, value) = lower_infinite(
        back,
        json!({"clause": "C", "correspondence": "mltl.origin-complete-history/v1"}),
    );
    assert_eq!(code, 21, "{value}");
    assert_eq!(value["tl_lowering"]["disposition"], "unsupported");
    assert_eq!(value["tl_lowering"]["classified"], "mltl.infinite-trace/v1");
    assert_eq!(value["tl_lowering"]["dimensions"], json!(["profile"]));

    let (code, value) = request_over(
        "filament.temporal.event-position.false-extension/v1",
        "on origin",
        "historically[0,2] holds(c.level <= 2)",
        json!({"tl_lowering": {"clause": "C", "correspondence": "mltl.infinite-trace/v1"}}),
    );
    assert_eq!(code, 21, "{value}");
    assert_eq!(
        value["tl_lowering"]["classified"],
        "mltl.origin-complete-history/v1"
    );
    assert_eq!(value["tl_lowering"]["dimensions"], json!(["profile"]));

    for body in [
        "once holds(c.level = 0)",
        "eventually[0,2] holds(c.level = 0)",
    ] {
        let (code, value) = lower_infinite(body, json!({"clause": "C"}));
        assert_eq!(code, 21, "{body}: {value}");
        assert_eq!(value["tl_lowering"]["disposition"], "unsupported");
        assert_eq!(
            value["tl_lowering"]["dimensions"],
            json!(["infinite-trace"])
        );
        assert_eq!(value["tl_lowering"]["classified"], Value::Null, "{body}");
    }
}

/// FR-931-AC-10: a trace form that does not match the correspondence
/// refuses `invalid-request` naming the trace member, before compiling when
/// the correspondence is named and after classification when it is not; an
/// empty loop refuses as FR-329 refuses it.
#[test]
#[trace("TC-3023", "FR-931-AC-10")]
#[ignore = "QSL-2505: tl_lowering does not admit a lasso cross-evaluation trace yet"]
fn a_trace_form_must_match_the_correspondence() {
    let back = "once[1,1] not holds(c.level = 0)";
    for (lowering, member) in [
        (
            json!({"clause": "C", "correspondence": "mltl.closed-trace/v1",
                "cross_evaluate": {"trace": lasso(0, &[0])}}),
            "tl_lowering.cross_evaluate.trace",
        ),
        (
            json!({"clause": "C", "correspondence": "mltl.infinite-trace/v1",
                "cross_evaluate": {"trace": {"finite": samples(&[0, 0])}}}),
            "tl_lowering.cross_evaluate.trace",
        ),
        (
            json!({"clause": "C", "correspondence": "mltl.infinite-trace/v1",
                "cross_evaluate": {"initial": "snapshot.json", "max_steps": 1}}),
            "tl_lowering.cross_evaluate.initial",
        ),
        (
            json!({"clause": "C", "cross_evaluate": {"trace": {"finite": samples(&[0, 0])}}}),
            "tl_lowering.cross_evaluate.trace",
        ),
    ] {
        let (code, value) = lower_infinite(back, lowering.clone());
        assert_eq!(code, 20, "{lowering}: {value}");
        let text = value.to_string();
        assert!(text.contains("invalid-request"), "{lowering}: {text}");
        assert!(text.contains(member), "{lowering}: {text}");
        assert_eq!(value["tl_lowering"], Value::Null, "{lowering}: {text}");
    }
    let (code, value) = lower_infinite(
        back,
        json!({"clause": "C", "cross_evaluate": {"trace": {"lasso": {"prefix": [], "loop": []}}}}),
    );
    assert_eq!(code, 20, "{value}");
    assert!(value.to_string().contains("invalid-value"), "{value}");
}

/// FR-931-AC-10, library half: through the `fsl-command` library entry, an
/// atom table that maps `p0` to the negated leaf over the infinite-trace
/// clause and the lasso with loop `0` is one `correspondence-loss` fault
/// naming the clause and trace 0, and the run's exit is 30; the faithful
/// table agrees.
#[test]
#[trace("TC-3023", "FR-931-AC-10")]
#[ignore = "QSL-2505: the fsl-command library entry does not cross-evaluate a lasso under mltl.infinite-trace/v1 yet"]
fn a_mis_mapped_atom_over_a_lasso_is_a_correspondence_loss() {
    let directory = tempfile::tempdir().unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile e = \"{INFINITE_PROFILE}\";\n\
         record Counter {{ level: Int[0, 9]; }}\n\
         temporal C using e over (c: Counter) clock \"events\" on each (q: Counter) \
         {{ once[1,1] not holds(c.level = 0) }}\n"
    );
    std::fs::write(directory.path().join("unit.native"), &unit).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:mltl-infinite-mismap", "document": "Mltl", "formal_revision": 1}},
        "clocks": {"events": {"kind": "event_position", "sequence_authority": "events"}},
        "clause": {"selection": {"temporal": {"name": "C", "trace": lasso(0, &[0])}},
            "observations": {}}}});
    let path = directory.path().join("request.json");
    std::fs::write(&path, request.to_string()).unwrap();
    let clause_run = fsl_command::command::read_clause_run_request(&path).expect("a clause run");
    let report = fsl_replay::spine::run_temporal_valuation(clause_run).expect("a report");
    let fsl_replay::spine::ClauseDisposition::TemporalValuation(valuation) = &report.disposition
    else {
        panic!("a valuation: {:?}", report.disposition);
    };
    let faithful = fsl_command::command::tl_record(
        valuation,
        &unit,
        None,
        &fsl_command::command::AtomMapping::default(),
    )
    .expect("the clause lowers");
    assert_eq!(faithful.exit_code, 0, "{}", faithful.record);
    assert_eq!(faithful.record["correspondence"], "mltl.infinite-trace/v1");
    assert_eq!(faithful.record["cross_evaluation"]["agree"], true);

    let mut mapping = fsl_command::command::AtomMapping::default();
    mapping.negated.insert("p0".to_owned());
    let mismapped = fsl_command::command::tl_record(valuation, &unit, None, &mapping)
        .expect("the clause lowers");
    assert_eq!(mismapped.exit_code, 30, "{}", mismapped.record);
    let cross = &mismapped.record["cross_evaluation"];
    assert_eq!(cross["agree"], false);
    // Natively `not p` fails at 1 (p holds at 0); with `p0` negated the TL
    // side reads p false at 0 and holds there. Position 0 reads the
    // pre-origin extension on both sides and agrees.
    assert_eq!(
        cross["faults"],
        json!([{"kind": "correspondence-loss", "clause": "C", "trace": 0, "position": 1,
            "native": false, "tl": true}])
    );
}
