---
id: FR-991
title: "Publish in-process replay of a global-conformance counterexample"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-896
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-895
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-217
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-990
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-433
    type: implements
  - target: ix://agent-ix/filament-specification/FR-059
    type: depends_on
---
# FR-991: Publish in-process replay of a global-conformance counterexample

## Description

`fsl-analyze` SHALL publish, in the public module
`fsl_analyze::protocol_analysis::conformance`, a function that replays the
counterexample of a refuted `global-conformance` result (FR-990) in
process. The function starts from the protocol subject a library already
holds, and needs no source text and no `ReplayRequestWire`. It is the same
replay the engine runs before it settles `refuted`, so a library that
carries FSL's result can confirm the refutation without recompiling the
unit.

```rust
pub fn replay_counterexample(
    subject: &fsl_eval::model_subject::ProtocolSubject<'_>,
    counterexample: &ConformanceCounterexample,
    limits: fsl_replay::ReplayLimits,
    cancel: &fsl_replay::Cancel,
) -> Result<ConformanceReplay, ConformanceReplayRefusal>;

pub enum ConformanceCounterexample {
    /// Over every behaviour: FSpec FR-433's protocol counterexample.
    Protocol(fsl_replay::TemporalCounterexample<fsl_eval::simulation::protocol::ProtocolTransition>),
    /// Over a recording: the FSpec FR-737 recording document's bytes and the
    /// index of the first violating observation in causal order.
    Recording { recording: Vec<u8>, trace_position: usize },
}

pub enum ConformanceReplay {
    /// `reproduced-with-evaluated-witness`.
    Reproduced,
    /// `inconclusive`, cause `replay-parity`, naming what differed.
    ReplayParity { detail: String },
}

pub struct ConformanceReplayRefusal { /* private fields */ }
impl ConformanceReplayRefusal {
    /// The FSpec catalog code, e.g. `invalid_runtime_input`.
    pub fn code(&self) -> filament_semantic_value::catalog::CatalogCode;
    /// The catalog cause, e.g. `invalid-value` or `missing-name`.
    pub fn cause(&self) -> &str;
    /// The step or observation index the refusal names, when it names one.
    pub fn position(&self) -> Option<usize>;
}
```

`subject` is the subject `protocol_subject` (FR-205) builds from the
package. For a checked handoff, that package is `CompiledHandoff::checked()`
and `CompiledHandoff::packages()` (FR-992), together with the item's role
bindings, memory selection, initial states and universes. `limits` are the
replay run's limits, and `cancel` stops it.

### Contract

- **Over every behaviour.** The function builds the subject's protocol
  system under the counterexample's resolved memory models and
  `max_live_instances`, and re-executes the counterexample by FSpec FR-433's
  replay rules 1 to 6, through the re-execution FR-217 uses. Agreement
  returns `Reproduced`. An enabled step at a deadlock's last state, an
  intended state, a different blocked list, or a formula that holds at
  `trace_position` returns `ReplayParity`. A step that is not enabled, a loop
  that does not close, an unfair lasso, or a wire-shape defect is a refusal,
  as FR-433 states.
- **Over a recording.** The function admits the recording against the
  subject's protocol as FR-895 admits it. It then assesses the observations
  up to and including `trace_position`, in causal order, by FR-895's
  obligations. The replay returns `Reproduced` when that prefix violates an
  obligation and no earlier observation does, and `ReplayParity` otherwise.
  An admission defect is a refusal, with FR-895's code and cause.
- **The refusal carries its code.** Every refusal of the protocol system's
  build, of admission, and of replay is a `ConformanceReplayRefusal` with
  the FSpec catalog code and cause that the `analyze` item of the same
  failure reports. Its `position` is the index that the failure names. A
  library therefore maps an engine refusal to its own disposition by code,
  never by message text.
- **Determinism and isolation.** The function reads no engine state and no
  file. Two counterexamples that differ but are both valid for one item
  both return `Reproduced`. Replaying the same inputs twice gives equal
  results.
- **Agreement with the engine.** The `replay` member of a refuted
  `global-conformance` item is this function's result over that item's
  counterexample and subject.

### Decision: replay from the subject, not from source

A library that consumes FSL's protocol analysis holds the checked package
and the subject built from it, and not the request wire with source text
that `replay_protocol_trace` recompiles from. Without an in-process entry,
such a library cannot confirm a `refuted` result. It has to either trust the
label or redo FSL's replay itself. Realizability already replays from its
subject in process. Global conformance uses the same shape, with one enum
for its two counterexample forms, because FR-896 gives the claim one
meaning and two evidence sources. Publishing the refusal's catalog code
makes FSL's refusal readable outside the crate in the form the `analyze`
item reports it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-991-AC-1 | `fsl_analyze::protocol_analysis::conformance` publishes `replay_counterexample`, `ConformanceCounterexample`, `ConformanceReplay` and `ConformanceReplayRefusal` with the signatures above. | Test (TC-2802) |
| FR-991-AC-2 | The deadlock counterexample of the order fixture without its `receive` (FR-896-AC-2) replays to `Reproduced` over the subject built from that fixture's checked package, with no source text supplied. The item's `replay` member reports the same result. | Test (TC-2802) |
| FR-991-AC-3 | The same counterexample with its last step's transition identity changed is refused, `code()` `invalid_runtime_input`, `cause()` `invalid-value` and `position()` that step's index. With an extra step appended that is enabled at the deadlock state, it returns `ReplayParity`. | Test (TC-2802) |
| FR-991-AC-4 | FR-895-AC-5's recording, with `trace_position` at the unexpected `Tell` send, returns `Reproduced`. With `trace_position` one earlier it returns `ReplayParity`. A recording naming the undeclared role `Courier` is refused `invalid_runtime_input`, cause `missing-name`. | Test (TC-2803) |

## Dependencies

- [FR-896](FR-896-decide-a-global-conformance-claim-over-the-protocol-system-or-a-recording.md)
  and [FR-990](FR-990-report-per-obligation-and-global-conformance-results.md):
  the claim and the result whose counterexample this replays.
- [FR-895](FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md):
  admission and obligations over a recording.
- [FR-217](FR-217-replay-a-protocol-counterexample.md): counterexample
  re-execution.
- [FR-992](FR-992-publish-the-source-provision-of-a-compiled-handoff.md):
  the package a handoff library builds the subject from.
- FSpec FR-433 and FR-059.
