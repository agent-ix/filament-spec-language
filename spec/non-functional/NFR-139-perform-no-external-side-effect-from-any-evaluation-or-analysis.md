---
id: NFR-139
org: agent-ix
title: "Perform no external side effect from any evaluation or analysis"
type: NFR
quality_attribute: safety
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-215
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-206
    type: references
  - target: ix://agent-ix/filament-specification/FR-182
    type: depends_on
---
# NFR-139: Perform no external side effect from any evaluation or analysis

## Statement

Every FSL evaluation, simulation, exploration, analysis and replay shall
observe or recommend only: it shall read its inputs and return typed
results, and shall perform no external business side effect, start no
process, open no network connection and invoke no effect adapter.

## Scope

This is the language's side of the observe-or-recommend boundary (FSpec
FR-182). FSL provides that boundary with the constructs it already has; it
adds no construct for it. A protocol `attempt`, `effect` or `commit` is a
modelled step: the protocol system applies it to the model state it holds in
memory (FR-206, FR-215) and to nothing outside. Every result FSL returns
(a verdict, a counterexample, a simulated behaviour, an enabled step) is a
recommendation in FSpec FR-182's sense. Executing an effect needs an
authority grant and an idempotency key, which only FSpec FR-182's effect
adapter holds, outside the language; no FSL crate depends on or calls one.

The rule covers the crates that evaluate and analyse: `fsl-semantics`,
`fsl-package`, `fsl-eval`, `fsl-analyze`, `fsl-route` and `fsl-replay`,
in their shipped source (test code is outside the rule). The command-line
front end and the language server read and write the files a user names and
are outside this rule.

## Rationale

An analysis that could act on the world would turn a counterexample search into a sequence of real business actions. Keeping every FSL result a recommendation is what lets a user run exploration and replay freely and leaves authority and idempotency with the one component that holds them.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Uses of process, network or effect-adapter APIs in the shipped source of the listed crates | 0 | 0 | Test |
| Files written outside the caller's output by an exploration or simulation | 0 | 0 | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-139-AC-1 | The shipped source of `fsl-semantics`, `fsl-package`, `fsl-eval`, `fsl-analyze`, `fsl-route` and `fsl-replay` (every `.rs` file under each crate's `src`, excluding files named `tests.rs` and files under a `tests` directory) contains no use of `std::process`, `std::net`, `Command::new` or `TcpStream`, and none of those crates' manifests names a dependency that provides process spawning, networking or an effect adapter. | Test (TC-2024) |
| NFR-139-AC-2 | Exploring a protocol whose `attempt` applies an operation that writes a model field, and simulating it, changes only the in-memory model state of the returned states and writes no file outside the caller's output. | Inspection (TC-2024) |

## Verification

TC-2024 scans the listed crates (row 1 and 2) and inspects an exploration and a simulation (row 3).
