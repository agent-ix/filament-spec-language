// SPDX-License-Identifier: AGPL-3.0-or-later
//! A scripted process hybrid provider (FR-750, FSpec FR-462). It speaks
//! `filament.plugin-wire/v1` on its standard input and output: it sends its
//! `hello`, reads the host's `request` frame, answers it by the script its
//! one argument names, and exits after the host's `shutdown`.
//!
//! The script is a JSON object: `mode` (`answer`, `silent`, `crash`,
//! `exit-nonzero`, `garbage`, `noncanonical`, `foreign-protocol`, `oversize`
//! `hang`, `no-hello` or `late-result`), `outcome` (the FSpec FR-331 outcome stage an `answer` returns
//! as its `result` body), `stderr` (text written to standard error) and
//! `size` (the length an `oversize` frame announces) and `record_request` (a
//! path the request body is written to).

use std::io::{Read, Write};
use std::process::ExitCode;

use serde_json::{json, Value};

const PROTOCOL: &str = "filament.plugin-wire/v1";

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: fsl-hybrid-test-provider <script.json>");
        return ExitCode::from(2);
    };
    let script: Value = match std::fs::read(&path)
        .map_err(|error| error.to_string())
        .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|error| error.to_string()))
    {
        Ok(script) => script,
        Err(error) => {
            eprintln!("the script {path} is not read: {error}");
            return ExitCode::from(2);
        }
    };
    match run(&script) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run(script: &Value) -> Result<(), String> {
    if let Some(text) = script["stderr"].as_str() {
        eprintln!("{text}");
    }
    let mode = script["mode"].as_str().unwrap_or("answer");
    if mode == "silent" {
        return Ok(());
    }
    let mut output = std::io::stdout().lock();
    let mut input = std::io::stdin().lock();
    if mode == "no-hello" {
        // A provider that never speaks first.
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }
    write_frame(
        &mut output,
        &frame(
            0,
            "hello",
            json!({"manifest": {"backend": "hybrid-process"}}),
        ),
    )?;
    let request = read_frame(&mut input)?;
    let id = request["id"]
        .as_u64()
        .ok_or("the request frame has no id")?;
    if let Some(path) = script["record_request"].as_str() {
        let body =
            quire_canonical::to_vec(&request["body"], quire_canonical::Limits::new(u64::MAX))
                .map_err(|error| format!("{error:?}"))?;
        std::fs::write(path, body).map_err(|error| error.to_string())?;
    }
    match mode {
        "answer" => {
            write_frame(&mut output, &frame(id, "result", script["outcome"].clone()))?;
            // The host's `shutdown`, then exit.
            let _ = read_frame(&mut input);
            Ok(())
        }
        "crash" => std::process::abort(),
        "exit-nonzero" => std::process::exit(3),
        "garbage" => {
            output
                .write_all(&5_u64.to_be_bytes())
                .and_then(|()| output.write_all(b"xxxxx"))
                .and_then(|()| output.flush())
                .map_err(|error| error.to_string())?;
            Ok(())
        }
        "noncanonical" => {
            let spaced = serde_json::to_vec_pretty(&frame(id, "result", script["outcome"].clone()))
                .map_err(|error| error.to_string())?;
            output
                .write_all(&(spaced.len() as u64).to_be_bytes())
                .and_then(|()| output.write_all(&spaced))
                .and_then(|()| output.flush())
                .map_err(|error| error.to_string())?;
            Ok(())
        }
        "foreign-protocol" => {
            let mut foreign = frame(id, "result", script["outcome"].clone());
            foreign["protocol"] = json!("other/v1");
            write_frame(&mut output, &foreign)
        }
        "oversize" => {
            let size = script["size"].as_u64().ok_or("no size")?;
            output
                .write_all(&size.to_be_bytes())
                .and_then(|()| output.flush())
                .map_err(|error| error.to_string())?;
            Ok(())
        }
        "late-result" => {
            // Answers only after the host's `cancel` frame arrives.
            let _ = read_frame(&mut input);
            write_frame(&mut output, &frame(id, "result", script["outcome"].clone()))?;
            let _ = read_frame(&mut input);
            Ok(())
        }
        "hang" => loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        },
        other => Err(format!("unknown mode {other}")),
    }
}

fn frame(id: u64, kind: &str, body: Value) -> Value {
    json!({"protocol": PROTOCOL, "id": id, "kind": kind, "body": body})
}

fn write_frame(output: &mut impl Write, frame: &Value) -> Result<(), String> {
    let bytes = quire_canonical::to_vec(frame, quire_canonical::Limits::new(u64::MAX))
        .map_err(|error| format!("{error:?}"))?;
    output
        .write_all(&(bytes.len() as u64).to_be_bytes())
        .and_then(|()| output.write_all(&bytes))
        .and_then(|()| output.flush())
        .map_err(|error| error.to_string())
}

fn read_frame(input: &mut impl Read) -> Result<Value, String> {
    let mut length = [0_u8; 8];
    input
        .read_exact(&mut length)
        .map_err(|error| error.to_string())?;
    let length = usize::try_from(u64::from_be_bytes(length)).map_err(|error| error.to_string())?;
    let mut bytes = vec![0_u8; length];
    input
        .read_exact(&mut bytes)
        .map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}
