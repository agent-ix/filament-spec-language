---
type: reference
---

# Producer replay context

FR-621's `replay_context` is a serialized reconstruction input carried by a
replayable refuted item's `filament-outcome/1` entry. An external agreement
consumer can recover the producer's inputs from JSON, authenticate the original
context, then run an independent native replay under its own accounting limits.
The consumer invokes the FSL executable; this transport requires no linked
Rust driver. This is the contract draft for QSL-2108, based on source head
`f89c2e03082342f690be623f6181977c4453d8ed`.

## Transport

`replay_context` is a closed FR-621-owned object with these required members:

| Member | Form and ownership |
| --- | --- |
| `request` | The complete original FSpec FR-336 replay request descriptor, using FR-862's existing mapping. Its `package` fragment retains the original raw package JSON bytes. This object is immutable producer context. |
| `source_bytes` | Array of closed `{bytes_path, bytes}` rows. `bytes_path` is exactly the token referenced by a `request.byte_provision` row. `bytes` is an array of JSON integers in 0–255, preserving every original byte. Tokens are unique and resolved only against this array, never as filesystem paths. |
| `documents` | Array of original domain-package, snapshot, initial-state, universe and observation documents, each represented as a JSON integer byte array. Identity comes from native document admission. |
| `family` | The actual existing family spelling of the item's resolved claim, already reported by item resolution; it selects that family's reader, not a new `ReplaySource` arm. |
| `payload` | Complete original counterexample in the existing family's wire form. When an envelope exists, its `family_payload` must equal this payload. |
| `envelope` | For a witness-envelope family: the original `WitnessEnvelope::to_packet()` members in the mapping below. `null` only for an existing native family form that uses no witness envelope. |
| `subject` | The selected family subject inputs in the mapping below; `null` only when that family requires none. |
| `certificate` | The complete original family certificate form when required by the selected replay arm; otherwise `null`. Its prior check status stays in the existing item certificate record. |

Raw package JSON must be embedded using retained raw JSON, rather than parsing
and re-emitting a different byte representation. `source_bytes` gives FR-621 an
in-document resolver for the existing `fsl_command::replay_request::read`
`provide(path, pointer)` callback. `documents` is passed to that same reader's
existing document provision argument. This defines ownership of bytes without
adding members to the closed FR-336 request. The descriptor's complete admitted
dependency selections and their source provision must be present; an identity
without its required recoverable input is insufficient.

Every required original source token must resolve exactly once. Duplicate tokens,
invalid bytes, absent source bytes/documents, malformed raw package JSON and
unknown members refuse reconstruction with a pointer into `replay_context`.
Byte arrays are data, not authority; native admission still checks their identities
and digests under caller-supplied limits. Neither paths nor backend names grant
admission authority. No schema or external artifact is copied into the repository.

## Envelope and family forms

The envelope is a closed object with exactly the existing `WitnessPacket<P>`
members: `obligation_identity`, `occurrence_key`, `clause_node`, `replay_target`,
`profile_selections`, `run_limits`, `declared_domains`, `backend`,
`trace_position`, `source`, and `family_payload`. Required packet members may not
be omitted. Explicit `trace_position: null` means the family has no position;
omission means missing input. Identities, targets, selections, limits, domains
and source use their existing request/envelope native wire mappings. Node keys
use the owning node-wire conversion with the genuine package graph; do not
derive checked identity from display text. `run_limits` is immutable original
proving-run accounting.

`WitnessEnvelope::to_packet()` (`fsl-replay/src/witness.rs:1191`) supplies the
original typed members; `WitnessEnvelope::reconstruct` admits the recovered
packet. This is currently an in-process packet API, not an existing JSON writer.
FR-621 owns the JSON composition and the corresponding reader added by the
future implementation. It must preserve packet information and missing/null
semantics rather than claiming serde support already exists.

| Existing family form | `family_payload` / source and selected subject |
| --- | --- |
| Model temporal | Actual FR-364 payload produced by `temporal_wire`; read with `read_temporal_counterexample<ModelTransition>`. The transcript in `request.source` is retained as transcript, never substituted for the typed payload. Subject has exactly `initial`, `universes`, `expansion`, `timed`, `temporal_selections`. |
| Protocol temporal | Actual payload from `protocol_temporal_wire`, including its complete protocol transition alphabet and subject. The FR-621 reader must restore `TemporalCounterexample<ProtocolTransition>`; today's generic temporal reader does not support this alphabet. Subject has exactly `model`, `protocol`, `over`, `roles`, `capacities`, `memory`, `max_store_buffer`, `max_messages`, `max_live_instances`, `temporal_selections`. |
| Hybrid | Preserve existing `ReplaySource::Hybrid` and its native counterexample form, the actual checked claim inputs and required certificate. No temporal witness envelope is fabricated. FR-621 carries the actual hybrid counterexample in its existing family form and restores it into the existing typed source after reading the descriptor. |
| Observed temporal, model graph, protocol analysis or value inputs | Use the existing native family/source form only when the resolved item actually has that form; do not coerce model/protocol temporal into observed-temporal or use the value-only plugin builder as a converter. An unimplemented family transport is explicitly unavailable. |

Model subject members correspond to `ModelTraceSubject`: `initial` contains the
selected document references in request order; `universes` contains each selected
population and its exact key set; `expansion` is `{max_candidates}`; `timed` is
`null` or `{model, exploration: {max_states, max_transitions}, max_clocks, digital}`.
Optional native values remain `null`, not inferred defaults. Counts/bounds use
canonical unsigned decimal strings unless the owning existing family form fixes
another spelling. Original universe document bytes remain in `documents`.
`temporal_selections` retains actual admitted clock-selection records; package
clock declarations still come from the complete package, not an invented
FR-336 `clocks` member.

Protocol subject `model` has exactly the model fields above excluding
`temporal_selections`, which is shared by the protocol subject. `over`, roles,
capacities and memory are the actual admitted `ProtocolTraceSubject` selections,
using the existing protocol counterexample/subject and analyze request mappings;
they are reconstructed against the admitted checked protocol. All selected
capacity, memory and dynamic-instance bounds are retained, including explicit
absence. `max_store_buffer`, `max_messages` and `max_live_instances` use canonical
unsigned decimal strings or `null`.

The protocol `roles` array contains exactly `{role, model}` rows and
`capacities` exactly `{name, value}` rows, preserving admitted selections;
capacity values use canonical decimal strings. `memory` contains every
`ResolvedMemory::parallels()` entry as `{parallel, model}` using the existing
memory-model spelling. `over` contains the checked population declaration key
and exact instance key, using the canonical-reference owner's conversion where
required. Selected `initial` references use the existing document reference
`{authority, identity}`. Universe rows retain the owning population declaration
and exact selected keys, and must agree with their original universe document.
Clock-selection rows retain the actual `TemporalClockSelection` members:
`source`, `declaration_span`, `clock`, `definition`, `trace`; existing package
source/span/clock/artifact mappings apply and explicit absent trace is `null`.

`ReplayRequestWire` has certificate and document payloads beyond FR-862's closed
descriptor, and the descriptor reader currently accepts only its existing source
forms. FR-621's sibling `certificate`, `documents` and family envelope compose
those missing inputs after descriptor decoding. The future family reader restores
existing native source types; it does not extend FR-336's `source` union.
If a native family has no current FR-336 source encoding, its complete descriptor
cannot be offered until that authoritative encoding exists. In particular,
today's FR-862 reader alone provides no hybrid descriptor codec. A fake witness
or placeholder input is forbidden: report context unavailable for that family.
The family payload/certificate composition is defined here; the missing
descriptor codec remains an explicit implementation dependency for review.

## Acquisition and member mapping

The producer captures acquisition handles before `command::analyze::settle_items`
narrows them to a `CheckedPackage`. The consumer owns the independent request.

| Existing owned request member | Producer source and owner | Required/conditional payload |
| --- | --- | --- |
| `package: ReplayPackage` | Acquisition's `EmittedUnit.package().bytes()` or package route's retained `package_bytes`; `ReplayPackage::read/admit` owns syntax/admission. | Required complete raw package; raw fragment in `request.package`. |
| `profile_selections` | Actual admitted package/request selections; family settlement. | Required array, legitimately empty only when native run selected none. |
| `dependencies` | Checked dependency selections, `CheckedUnit.library_source` and `shared_library_packages`; dependency acquisition. | Required per selected dependency, with recoverable actual library/source bytes; no identity-only authority. |
| `replay_target` | Resolved `ReplayShape.target`, occurrence/node and actual family selection; item resolver. | Existing native target and over/argument/occurrence bindings where applicable. |
| `source` | Actual family counterexample and envelope; `ItemCounterexample` and family settlement. | Existing descriptor source plus complete family payload; never a guessed conversion. |
| `certificate` | Actual `HybridReport.certificate.form`; hybrid settlement. | Sibling `certificate`, required for native arm that needs it; otherwise null. |
| `obligation_identity` | Checked family obligation and subject identity at settlement. | Required immutable identity; never guessed ordinal or claim-name identity. |
| `backend` | Originating terminal record / `ItemCounterexample.backend`. | Required original identity string; no execution authority. |
| `state_environment` | Actual admitted scalar request/family environment. | Required native entries, empty only for a genuinely empty environment. |
| `accounting_limits` | Original `AnalyzeLimits.accounting`; run owner. | Original in serialized descriptor; independent request gets consumer values after authentication. |
| `stage_limits` | Actual admitted `CallerLimits.bounds()` / family settings. | Original map; excludes forbidden replay-input/output settings. Consumer owns every accounting-counter overlay in independent execution. |
| `supplied_limits` | Actual request-admitted helper ceilings. | Native option with its existing default semantics; distinct from consumer accounting. |
| `object_environment` | Actual selected snapshot and `SubjectBinding.snapshots`. | Native option plus original snapshot bytes, when family uses one. Canonical conversion depends on QSL-2726. |
| `declared_domains` | Actual substituted proving domains and resolved proof bounds. | Required native array; empty only when the actual family contract has none. |
| `byte_provision` | Actual program/library `FormalSource` bytes through `SourceProvision.entries()`. | Every source label plus exact source bytes via `source_bytes` resolver. |
| `documents` | `SubjectBinding.packages/snapshots` and original selected universe/observation documents. | Sibling byte arrays, with selected references/key sets in subject. |

The plugin context at `proof_result/plugin_replay.rs:87` demonstrates retention
of emitted packages, dependency locks and actual source bytes. Its `request`
method remains a value-only precedent. Temporal request/envelope composition is
at `analyze/clause_item.rs:581`; model and protocol subject types are at
`model_check/replay.rs:99` and `:156`. Existing family writers/readers are
`temporal_wire`, `protocol_temporal_wire`, `read_temporal_counterexample` in
`outcome/counterexample.rs`. Missing JSON composition/readers are implementation
work, not evidence that these APIs already serialize the complete context.

## Authentication and independent accounting

The reader first admits the complete original bytes and authenticates the
original descriptor against the original envelope: checked package/obligation,
target, occurrence, profiles, backend, declared domains, original accounting,
selected subject and family payload must agree under native rules. A mismatch
refuses. Original packet and descriptor remain immutable.

Only after this succeeds may the consumer construct a separate execution
request. The consumer supplies `accounting_limits`, all accounting entries in
`stage_limits`, and its replay-input byte ceiling. The producer's original
values remain in the authenticated context; no recorded envelope field is
changed to satisfy equality. Non-accounting proving settings and supplied-value
ceilings retain their separate meanings. The native family execution boundary
must distinguish authenticated proving context from effective execution
accounting; equality checks must not accidentally compare execution overlays
against an immutable original `run_limits`.

`replay.reproduced` is history, never independent recheck evidence. Missing
context, refusal, replay disagreement, unsupported transport and internal fault
remain explicit. None rewrites the producer's terminal record, proof/refutation,
certificate status, category or exit code. Non-replayable terminal outcomes
carry no invented successful replay context.

## Canonical-reference dependency

QSL-2726 owns canonical-reference conversion. Where a family payload or selected
subject requires canonical object references, this composition uses that owning
conversion and is unavailable until it exists. Native references do not confer
canonical authority. This contract adds neither a reference codec nor a copied
schema, fallback format or compatibility layer. Root owns the single spec review
and consumer agreement before implementation.
