// SPDX-License-Identifier: AGPL-3.0-or-later
//! The plugin host of these tests: it starts the provider at the path the
//! test names, speaks `filament.plugin-wire/v1` (FSpec FR-462) with it and
//! implements FSL's [`HybridProcessExchange`] over the child process, the
//! part FR-750 leaves to the driver's plugin host (FR-635). The frame limits,
//! the grace period and the deadline are the run's `ProviderLimits`.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use filament_exact::{Cancel, CancelCause};
use fsl_analyze::analyze::{HybridExchangeError, HybridProcessExchange};
use fsl_analyze::provider::ProviderLimits;
use fsl_replay::ToolFailure;
use quire_canonical::Limits;
use serde_json::{json, Value};

const PROTOCOL: &str = "filament.plugin-wire/v1";
const KINDS: [&str; 6] = ["hello", "request", "result", "cancel", "shutdown", "fault"];
const POLL: Duration = Duration::from_millis(5);

/// What the reader thread learned from the provider's standard output.
enum Frame {
    /// One accepted envelope.
    Envelope { id: u64, kind: String, body: Value },
    /// The standard output ended.
    Closed,
    /// The frame broke FR-462's failure table.
    Refused(ToolFailure),
}

/// The host of one provider executable and the script it runs.
#[derive(Debug)]
pub struct StdioHost {
    program: PathBuf,
    script: PathBuf,
    limits: ProviderLimits,
    /// Whether the child has been waited on successfully before returning.
    pub reaped: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Result frames observed before deciding the exchange's return value.
    pub results: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl StdioHost {
    pub fn new(program: PathBuf, script: PathBuf, limits: ProviderLimits) -> Self {
        Self {
            program,
            script,
            limits,
            reaped: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            results: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    fn failure_after_end(child: &mut Child) -> ToolFailure {
        match child.wait() {
            Ok(status) if status.success() => ToolFailure::MissingTerminalRecord,
            Ok(status) if status.code().is_some() => ToolFailure::NonzeroExit,
            _ => ToolFailure::Crash,
        }
    }
}

impl HybridProcessExchange for StdioHost {
    fn exchange(&self, request: &[u8], cancel: &Cancel) -> Result<Vec<u8>, HybridExchangeError> {
        let body: Value =
            serde_json::from_slice(request).map_err(|_| ToolFailure::MalformedBody)?;
        let deadline_ms = body["request"]["deadline_ms"]
            .as_u64()
            .ok_or(ToolFailure::MalformedBody)?;
        let mut child = Command::new(&self.program)
            .arg(&self.script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| ToolFailure::Crash)?;
        let mut stdin = child.stdin.take().ok_or(ToolFailure::Crash)?;
        let stdout = child.stdout.take().ok_or(ToolFailure::Crash)?;
        let stderr = child.stderr.take().ok_or(ToolFailure::Crash)?;
        let limit = self.limits.max_stderr_bytes;
        let drain = std::thread::spawn(move || {
            // Standard error is opaque text that changes no settlement.
            let mut kept = Vec::new();
            let _ = stderr.take(limit).read_to_end(&mut kept);
            kept
        });
        let frames = spawn_reader(stdout, self.limits);
        let started = Instant::now();
        let deadline = Duration::from_millis(deadline_ms);
        let result = self.converse(
            &mut child, &mut stdin, &frames, &body, started, deadline, cancel,
        );
        drop(stdin);
        if result.is_err() || child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
        }
        if child.wait().is_ok() {
            self.reaped
                .store(true, std::sync::atomic::Ordering::Release);
        }
        let _ = drain.join();
        match cancel.cause() {
            Some(source) => Err(HybridExchangeError::Cancelled { source }),
            None => result,
        }
    }
}

impl StdioHost {
    #[allow(clippy::too_many_arguments)]
    fn converse(
        &self,
        child: &mut Child,
        stdin: &mut impl Write,
        frames: &Receiver<Frame>,
        request: &Value,
        started: Instant,
        deadline: Duration,
        cancel: &Cancel,
    ) -> Result<Vec<u8>, HybridExchangeError> {
        match self.next(child, frames, started, deadline, cancel) {
            Next::Frame(Frame::Envelope { kind, .. }) if kind == "hello" => {}
            Next::Frame(Frame::Envelope { .. }) => return Err(ToolFailure::UnknownFrameKind.into()),
            // A cancel before the hello leaves nothing to ask the provider to
            // stop: the host ends it and settles the item cancelled.
            Next::Cancelled(cause) => {
                return Err(HybridExchangeError::Cancelled { source: cause });
            }
            other => return self.settle_without_frame(child, other),
        }
        write_frame(stdin, 1, "request", request);
        let mut cancelled_at = None;
        loop {
            match self.next(child, frames, started, deadline, cancel) {
                Next::Frame(Frame::Envelope { id, kind, body }) => {
                    if kind != "result" {
                        return Err(ToolFailure::UnknownFrameKind.into());
                    }
                    if id != 1 {
                        return Err(ToolFailure::UnknownRequestId.into());
                    }
                    self.results
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    // FSpec FR-462 "Cancellation": a cancelled request settles
                    // `incomplete`/`cancelled`, whatever a late result says.
                    if let Some(cause) = cancel.cause() {
                        write_frame(stdin, 2, "shutdown", &json!({}));
                        return Err(HybridExchangeError::Cancelled { source: cause });
                    }
                    write_frame(stdin, 2, "shutdown", &json!({}));
                    return match child.wait() {
                        Ok(status) if status.success() => Ok(encode(&body)),
                        Ok(status) if status.code().is_some() => {
                            Err(ToolFailure::NonzeroExit.into())
                        }
                        _ => Err(ToolFailure::Crash.into()),
                    };
                }
                Next::Cancelled(cause) => match cancelled_at {
                    None => {
                        write_frame(stdin, 2, "cancel", &json!({"id": 1}));
                        cancelled_at = Some((Instant::now(), cause));
                    }
                    Some((at, cause))
                        if at.elapsed() >= Duration::from_millis(self.limits.grace_ms) =>
                    {
                        return Err(HybridExchangeError::Cancelled { source: cause });
                    }
                    Some(_) => {}
                },
                Next::Frame(Frame::Closed) if cancelled_at.is_some() => {
                    if let Some((_, source)) = cancelled_at {
                        return Err(HybridExchangeError::Cancelled { source });
                    }
                }
                other => return self.settle_without_frame(child, other),
            }
        }
    }

    /// FR-462 "Failures": what ends a conversation without a result.
    fn settle_without_frame(
        &self,
        child: &mut Child,
        next: Next,
    ) -> Result<Vec<u8>, HybridExchangeError> {
        match next {
            Next::Frame(Frame::Refused(failure)) => Err(failure.into()),
            Next::Frame(Frame::Closed) => Err(Self::failure_after_end(child).into()),
            Next::TimedOut => Err(HybridExchangeError::TimedOut),
            Next::Frame(Frame::Envelope { .. }) | Next::Cancelled(_) => {
                Err(ToolFailure::EnvelopeMember.into())
            }
        }
    }

    fn next(
        &self,
        _child: &mut Child,
        frames: &Receiver<Frame>,
        started: Instant,
        deadline: Duration,
        cancel: &Cancel,
    ) -> Next {
        loop {
            if let Some(cause) = cancel.cause() {
                // Observe cancellation before queued bytes or the provider's
                // independent deadline, while still draining an acknowledgement.
                if let Ok(frame) = frames.recv_timeout(POLL) {
                    return match frame {
                        Frame::Envelope { .. } | Frame::Closed => Next::Frame(frame),
                        Frame::Refused(_) => Next::Cancelled(cause),
                    };
                }
                return Next::Cancelled(cause);
            }
            if started.elapsed() >= deadline {
                return Next::TimedOut;
            }
            match frames.recv_timeout(POLL) {
                Ok(frame) => return Next::Frame(frame),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return Next::Frame(Frame::Closed),
            }
        }
    }
}

enum Next {
    Frame(Frame),
    TimedOut,
    Cancelled(CancelCause),
}

fn encode(value: &Value) -> Vec<u8> {
    quire_canonical::to_vec(value, Limits::new(u64::MAX)).expect("the value is canonical JSON")
}

fn write_frame(stdin: &mut impl Write, id: u64, kind: &str, body: &Value) {
    let bytes = encode(&json!({"protocol": PROTOCOL, "id": id, "kind": kind, "body": body}));
    // A provider that has already ended is read from its standard output.
    let _ = stdin
        .write_all(&(bytes.len() as u64).to_be_bytes())
        .and_then(|()| stdin.write_all(&bytes))
        .and_then(|()| stdin.flush());
}

/// Read frames until the output ends or one is refused (FSpec FR-462
/// "Framing" and "Envelope"; the envelope is checked before its body).
fn spawn_reader(mut stdout: impl Read + Send + 'static, limits: ProviderLimits) -> Receiver<Frame> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut total = 0_u64;
        loop {
            let frame = read_one(&mut stdout, limits, &mut total);
            let last = !matches!(frame, Frame::Envelope { .. });
            if sender.send(frame).is_err() || last {
                return;
            }
        }
    });
    receiver
}

fn read_one(stdout: &mut impl Read, limits: ProviderLimits, total: &mut u64) -> Frame {
    let mut length = [0_u8; 8];
    let mut have = 0;
    while have < length.len() {
        match stdout.read(&mut length[have..]) {
            Ok(0) if have == 0 => return Frame::Closed,
            Ok(0) | Err(_) => return Frame::Refused(ToolFailure::MalformedFrame),
            Ok(count) => have += count,
        }
    }
    let length = u64::from_be_bytes(length);
    if length > limits.max_frame_bytes {
        return Frame::Refused(ToolFailure::FrameLength {
            limit: limits.max_frame_bytes,
            value: length,
        });
    }
    *total += length;
    if *total > limits.max_total_bytes {
        return Frame::Refused(ToolFailure::TotalBytes {
            limit: limits.max_total_bytes,
            value: *total,
        });
    }
    let mut bytes = vec![0_u8; usize::try_from(length).expect("a frame under the limit fits")];
    if stdout.read_exact(&mut bytes).is_err() {
        return Frame::Refused(ToolFailure::MalformedFrame);
    }
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return Frame::Refused(ToolFailure::MalformedFrame);
    };
    if encode(&value) != bytes {
        return Frame::Refused(ToolFailure::NoncanonicalFrame);
    }
    let Some(members) = value.as_object() else {
        return Frame::Refused(ToolFailure::EnvelopeMember);
    };
    if members.len() != 4
        || ["protocol", "id", "kind", "body"]
            .iter()
            .any(|m| !members.contains_key(*m))
    {
        return Frame::Refused(ToolFailure::EnvelopeMember);
    }
    if members["protocol"] != PROTOCOL {
        return Frame::Refused(ToolFailure::UnknownProtocol);
    }
    let kind = members["kind"].as_str().unwrap_or_default();
    if !KINDS.contains(&kind) {
        return Frame::Refused(ToolFailure::UnknownFrameKind);
    }
    let Some(id) = members["id"].as_u64() else {
        return Frame::Refused(ToolFailure::EnvelopeMember);
    };
    Frame::Envelope {
        id,
        kind: kind.to_owned(),
        body: members["body"].clone(),
    }
}
