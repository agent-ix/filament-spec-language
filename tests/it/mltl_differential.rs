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
