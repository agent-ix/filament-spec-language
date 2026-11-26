// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-862-AC-4: a complete `native-replay/1` request refuses a deeply nested
//! member value naming it, on a small stack (ADR-030 D-1).

use std::process::{Command, Stdio};

use fsl_command::command::{replay_with_limits, CommandLimits, RunCause, RunLimits};
use ix_trace_rs::trace;

/// A node id the package graph holds.
const ID: &str = r#"{"package":"p","path":["x"],"ordinal":0}"#;

/// The request member each case replaces, its value with `@` standing for the
/// deep tree, and the pointer the refusal names.
fn cases() -> Vec<(&'static str, String, &'static str)> {
    let target = |key: &str, over: &str| {
        format!(r#"{{"members":{{}},"arguments":false,"occurrence_key":{key},"over":{over}}}"#)
    };
    let witness = |value: String| {
        format!(r#"{{"kind":"input","assignments":[{{"parameter":{ID},"value":{value}}}]}}"#)
    };
    let at_value = "/source/assignments/0/value";
    vec![
        (
            "dependencies",
            r#"[{"identity":"x","sources":[],"refinement_internals":@}]"#.into(),
            "/dependencies/0/refinement_internals",
        ),
        (
            "package",
            r#"{"semantic_graph":{"nodes":[],"operand_rows":@}}"#.into(),
            "/package",
        ),
        (
            "package",
            r#"{"semantic_graph":{"nodes":[{"node_id":@}]}}"#.into(),
            "/package",
        ),
        (
            "obligation_identity",
            r#"{"node_id":@,"model":null,"ordinal":0}"#.into(),
            "/obligation_identity/node_id",
        ),
        (
            "replay_target",
            target(r#"{"node_id":@,"role":"r","ordinal":"0"}"#, "null"),
            "/replay_target/occurrence_key/node_id",
        ),
        (
            "replay_target",
            target("null", r#"{"population":@,"key":"k"}"#),
            "/replay_target/over/population",
        ),
        (
            "source",
            r#"{"kind":"input","assignments":[{"parameter":@,"value":null}]}"#.into(),
            "/source/assignments/0/parameter",
        ),
        (
            "source",
            witness(r#"{"kind":"text","value":@}"#.into()),
            at_value,
        ),
        (
            "source",
            witness(format!(
                r#"{{"kind":"enum","declaration":{ID},"member":@}}"#
            )),
            at_value,
        ),
        (
            "source",
            witness(format!(
                r#"{{"kind":"union","declaration":{ID},"member":@,"components":[]}}"#
            )),
            at_value,
        ),
        (
            "source",
            witness(format!(
                r#"{{"kind":"record","declaration":{ID},"fields":[{{"name":@,"slot":{{"kind":"absent"}}}}]}}"#
            )),
            at_value,
        ),
        (
            "source",
            witness(r#"{"kind":"sequence","elements":[{"kind":"boolean","value":@}]}"#.into()),
            at_value,
        ),
        (
            "source",
            witness(format!(
                r#"{{"kind":"reference","universe":{ID},"object_type":{ID},"identity":@}}"#
            )),
            at_value,
        ),
        (
            "source",
            witness(r#"{"kind":"rational","numerator":@,"denominator":"1"}"#.into()),
            at_value,
        ),
        ("source", r#"{"kind":@}"#.into(), "/source/kind"),
        ("stage_limits", r#"{"x":@}"#.into(), "/stage_limits/x"),
        (
            "accounting_limits",
            r#"{"x":@}"#.into(),
            "/accounting_limits/x",
        ),
        ("supplied_limits", "@".into(), "/supplied_limits"),
        ("object_environment", "@".into(), "/object_environment"),
        ("declared_domains", "[@]".into(), "/declared_domains/0"),
        (
            "byte_provision",
            r#"[{"bytes_path":@}]"#.into(),
            "/byte_provision/0/bytes_path",
        ),
        ("state_environment", "[@]".into(), "/state_environment/0"),
        (
            "profile_selections",
            r#"[{"profile":@,"value":"v"}]"#.into(),
            "/profile_selections/profile",
        ),
        ("backend", "@".into(), "/backend"),
    ]
}

/// The fourteen members of a request whose every member decodes, with
/// `member` replaced by `value`.
fn request(member: &str, value: &str) -> String {
    let package = format!(r#"{{"semantic_graph":{{"nodes":[{{"node_id":{ID}}}]}}}}"#);
    let obligation = format!(r#"{{"node_id":{ID},"model":null,"ordinal":0}}"#);
    let members = [
        ("profile_selections", "[]"),
        ("package", &package),
        ("dependencies", "[]"),
        (
            "replay_target",
            r#"{"members":{},"arguments":false,"occurrence_key":null,"over":null}"#,
        ),
        ("source", r#"{"kind":"claim_free"}"#),
        ("obligation_identity", &obligation),
        ("backend", r#""x""#),
        ("state_environment", "[]"),
        ("accounting_limits", "{}"),
        ("stage_limits", "{}"),
        ("supplied_limits", "null"),
        ("object_environment", "null"),
        ("declared_domains", "[]"),
        ("byte_provision", "[]"),
    ];
    assert!(members.iter().any(|(name, _)| *name == member));
    let body: Vec<String> = members
        .iter()
        .map(|(name, whole)| {
            let written = if *name == member { value } else { whole };
            format!(r#""{name}":{written}"#)
        })
        .collect();
    format!(
        r#"{{"format":"native-replay/1","request":{{{}}}}}"#,
        body.join(",")
    )
}

/// What a child process observed of the replay entry for the request `bytes`,
/// run on a 2 MiB stack: `<exit> <pointer>`, or why the child failed. Each
/// request has its own process, so an overflow fails its case alone and leaves
/// every other result standing.
fn observed(test: &str, bytes: &str) -> String {
    assert!(bytes.len() < RunLimits::default().request_bytes);
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("request.json");
    std::fs::write(&input, bytes).unwrap();
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("replay_depth::{test}"),
            "--test-threads=1",
            "--nocapture",
        ])
        .env(INPUT, &input)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&child.stdout);
    stdout
        .split_once("refused:")
        .and_then(|(_, rest)| rest.split_once(":end"))
        .filter(|_| child.status.success())
        .map_or_else(
            || format!("no refusal, {}", child.status),
            |(said, _)| said.to_owned(),
        )
}

const INPUT: &str = "FSL_DEEP_REPLAY_CHILD_INPUT";

/// In a child process: replay the request and say how it refused. `false` in
/// the parent.
fn answered_as_child() -> bool {
    let Some(input) = std::env::var_os(INPUT) else {
        return false;
    };
    let refusal = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let error = replay_with_limits(
                std::path::Path::new(&input),
                RunLimits::default(),
                &CommandLimits::replay(),
            )
            .expect_err("the request refuses");
            let pointer = match &error.cause {
                RunCause::ReplayRequest(defect) => defect.pointer().to_owned(),
                other => format!("{other}"),
            };
            format!("{} {pointer}", error.category().exit_code())
        })
        .expect("spawn the small-stack control")
        .join()
        .expect("the entry returns");
    println!("refused:{refusal}:end");
    true
}

fn deep(depth: usize) -> String {
    format!("{}0{}", "[".repeat(depth), "]".repeat(depth))
}

/// All fourteen members, one holding a deep tree, so the typed decode walks it.
#[test]
#[trace("FR-862-AC-4", "TC-2443")]
fn a_deep_value_at_any_replay_member_refuses_naming_it() {
    if answered_as_child() {
        return;
    }
    let mut failures = Vec::new();
    for (index, (member, value, pointer)) in cases().into_iter().enumerate() {
        for depth in [1_000, 16_000, 100_000] {
            let said = observed(
                "a_deep_value_at_any_replay_member_refuses_naming_it",
                &request(member, &value.replace('@', &deep(depth))),
            );
            if said != format!("20 {pointer}") {
                failures.push(format!(
                    "case {index} ({member}, {pointer}) depth {depth}: {said}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A deep unknown member of an otherwise empty request is read whole before
/// the request's defects are named.
#[test]
#[trace("FR-862-AC-4", "TC-2443")]
fn a_deep_unknown_member_of_an_empty_replay_request_refuses() {
    if answered_as_child() {
        return;
    }
    for depth in [1_000, 16_000, 100_000] {
        let bytes = format!(
            r#"{{"format":"native-replay/1","request":{{"z":{}}}}}"#,
            deep(depth)
        );
        let said = observed(
            "a_deep_unknown_member_of_an_empty_replay_request_refuses",
            &bytes,
        );
        assert!(
            said.starts_with(
                "20 invalid request: missing required members /request/profile_selections, "
            ) && said.ends_with(", /request/byte_provision"),
            "depth {depth}: {said}"
        );
    }
}
