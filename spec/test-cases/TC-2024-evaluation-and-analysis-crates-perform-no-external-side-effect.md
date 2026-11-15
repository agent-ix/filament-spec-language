---
id: TC-2024
title: "Evaluation and analysis crates perform no external side effect"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-139
    type: verifies
---
# TC-2024: Evaluation and analysis crates perform no external side effect

## Description

Verify that the crates that evaluate, analyse and replay have no path to an external side effect. Scope: NFR-139-AC-1 and NFR-139-AC-2.

## Test Procedure

| # | Change | Expected |
| --- | --- | --- |
| 1 | scan every shipped `.rs` file under `src` of `fsl-semantics`, `fsl-package`, `fsl-eval`, `fsl-analyze`, `fsl-route`, `fsl-replay`, excluding `tests.rs` and `tests` directories | no `std::process`, `std::net`, `Command::new` or `TcpStream` |
| 2 | read those crates' manifests | no dependency providing process spawning, networking or an effect adapter |
| 3 | explore and simulate a protocol whose `attempt` writes a model field | only the returned in-memory states change; no file is written outside the caller's output |

## Expected Results

- Row 1 names each offending file and line; row 3 is an inspection.
