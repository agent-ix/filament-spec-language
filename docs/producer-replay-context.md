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
| `request` | Closed FR-621 descriptor containing exactly `package`, `profile_selections`, `dependencies`, `replay_target`, `obligation_identity`, `backend`, `state_environment`, `accounting_limits`, `stage_limits`, `supplied_limits`, `object_environment`, `declared_domains`, `byte_provision`. Each uses the existing FR-336 member's mapping; original raw package JSON is preserved. Source data is supplied by `payload` and, where applicable, `envelope.source`. This composition is immutable original request data, not a standalone FR-336 request object. |
| `source_bytes` | Array of closed `{bytes_path, bytes}` rows. `bytes_path` is exactly the token referenced by a `request.byte_provision` row. `bytes` is an array of JSON integers in 0–255, preserving every original byte. Tokens are unique and resolved only against this array, never as filesystem paths. |
| `documents` | Array of closed `{kind, selection, bytes}` records. `kind` is `domain-package`, `snapshot`, `universe`, `observation`, or `dependency-package`; `selection` is the selected library identity for a dependency package, otherwise null. `bytes` is the complete original integer byte array. The record carries an owning admission role, not authority: document/package identity and selected-library agreement are checked natively. |
| `family` | FR-621 dispatch discriminator: `model-temporal`, `protocol-temporal`, or `hybrid`, checked against the resolved native claim family. It selects an existing family replay entry, not a new `ReplaySource` arm or a replacement terminal-result kind. |
| `payload` | Complete original counterexample in the existing family's wire form. When an envelope exists, its `family_payload` must equal this payload. |
| `envelope` | For a witness-envelope family: the original `WitnessEnvelope::to_packet()` members in the mapping below. `null` only for an existing native family form that uses no witness envelope. |
| `subject` | The selected family subject inputs in the mapping below; `null` only when that family requires none. |
| `certificate` | The complete original family certificate form when required by the selected replay arm; otherwise `null`. Its prior check status stays in the existing item certificate record. |

Raw package JSON must be embedded using retained raw JSON, rather than parsing
and re-emitting a different byte representation. The FR-621 native reader owns
the composition codec. It reads the common member mappings and resolves every
`byte_provision.bytes_path` only from `source_bytes`, with the same source-label
semantics as the existing reader's provision callback. It supplies `documents`
to native admission. It constructs `ReplayRequestWire` directly from these
admitted members plus the actual family source and certificate; it never sends
an invented source kind or fake transcript to the closed FR-336 reader.
The descriptor's complete admitted
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
| Hybrid | `payload` is exactly `HybridCounterexample::to_wire(model, locate)`: `{type: "hybrid-counterexample", kind, trace}` including formula or undefined-evaluation kind. `envelope` is null. The FR-621 reader restores `ReplaySource::Hybrid` through the existing hybrid counterexample reader over the admitted graph/model and restores the actual certificate. Its subject mapping is specified below. |

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

For hybrid, `subject` is a closed object with `claim`, `model`, `proof_bounds`,
`budget`, `options`, `deadline_ms`. `claim` is the original checked occurrence/
locus emitted with the provider item; `model` is that claim's admitted model
identity, not an independently chosen model. `proof_bounds` retains the
item's actual substituted bounds. `budget` retains the resolved original
`SolverBudget`, `options` the original string map and `deadline_ms` the original
optional deadline. These come from `HybridSolveRequest` and `HybridItem`, not
from the returned verdict. The complete model's modes, variables, initial
conditions, flows, guards, reset terms and checked claim are recovered from
the admitted package graph; they are not copied into a second model schema.
The actual trace includes its original start, jumps, optional before states,
delta, loop and end. Its state times and enclosures use the existing hybrid
rational/interval mapping. No initial state is guessed from the final state.
The following closed mappings define the hybrid identity and bounds fields:

| Field | Exact JSON and native resolution |
| --- | --- |
| `claim` | Exactly `{node: N, occurrence: {role: R, ordinal: O}}`, the shape emitted by `analyze/hybrid_process.rs::compose` at line 147. `N` is the JSON `CheckedNodeId` returned by the original package's `WireGraph::wire_value(original_occurrence.node())`; use that actual encoder, not a printed NodeKey or a numeric operand slot. Its owning mapping is the existing closed CheckedNodeId `{package, path, ordinal}` form. `R` is the original occurrence role string and must be `"claim"`; `O` is the original nonnegative JSON integer ordinal. |
| `model` | A JSON string equal to `graph.hybrid_models()[checked_claim.model.get()].name.as_str()`. It is a checked assertion about the model selected by the claim, never a lookup key that may select another model. No model-name-only resolution is permitted. |
| `proof_bounds` | An array whose rows have exactly `{domain, kind, bound}`, using the existing `declared_domain` codec at `fsl-command/src/replay_request.rs:494`. Rows retain the actual `HybridItem.proof_bounds` order and must decode to the same ordered `ProofBound` values as `request.declared_domains`; a duplicate domain or unequal value refuses. No `ProofBound::to_wire` API is assumed. |

For `claim`, resolve `N` with the admitted package's `WireGraph::key_of_value`,
obtain its document-held graph position with `index_of`, and require exact
roundtrip equality with `wire_value`. Require the original package row at that
position to have semantic form `temporal_clause` and to contain exactly the
supplied occurrence in its occurrences array. Then find exactly one
`graph.hybrid_claim_sites()` row whose `identity` and `claim` equal that resolved
node and occurrence, as `hybrid_item.rs:70` does; use the same position in
`graph.hybrid_claims()`. Resolve its `model` index through `hybrid_models()` and
compare the model string above. Require the resolved occurrence to equal the
original request obligation and selected target under native identity rules.
Absent, ambiguous, wrong-form, wrong-role, ordinal or model mismatch refuses
at the offending subject field. A dependency node cannot silently select a
different package's local claim. The existing package reader's `claim_position`
at `hybrid/process_request.rs:591` supplies the document-held resolution rule;
the FR-621 builder must preserve it rather than reinterpret node-local slots.

Each proof-bound `domain` is exactly one of
`{kind: "node", node: N, path: [U32String, ...]}` or
`{kind: "population", member_type: N, ordinal: U32String}`. Its node references
resolve with that same admitted `WireGraph` codec. `kind` is null or exactly
`{kind: K}`, where `K` is `DomainKind::to_wire()`'s spelling: `collection`,
`population`, `integer`, `recursive`, `loop`, `infinite-trace`, or `quantity`.
`bound` is exactly `{kind: "cardinality", maximum: U64String}`,
`{kind: "depth", maximum: U64String}`,
`{kind: "integer_range", lower: IntegerString, upper: IntegerString}`, or
`{kind: "variants", members: [String, ...]}`. Unsigned strings are canonical
decimal values in the stated range; integer strings use the existing admitted
integer codec. Use `FiniteBound` constructors and `ProofBound::new` for native
range/domain validation; reject extra/missing members, wrong types, unresolved
domain nodes and invalid ranges. After admission the values must agree with
the original checked item's substituted domains and obligation identity;
bounds never supply accounting authority. A genuinely empty bound vector is
`[]`, not inferred absence or a placeholder bounded extent.

`budget` is `{max_solver_time, max_solver_steps}` with the resolved original
positive counts or explicit null when native ownership left a ceiling unset;
the existing `SolverBudget` admitted count rules apply. `deadline_ms` is a
canonical unsigned decimal string or null. These proving/solver settings do
not set the consumer's replay meter.

The hybrid reader must admit actual domain documents and recursively selected
dependency package bytes, then resolve the original claim and its model. Use
`HybridProcessPackageContext`'s genuine package evidence/admission ownership
or the equivalent native package reader, never a trusted-name shortcut.
`read_plugin_hybrid_counterexample_over` reads the existing counterexample
entry shape `{counterexample: payload}` over that graph, checked claim and
emitted wire graph, preserving undefined-evaluation loci.
`hybrid_replay::admit` validates the trace. When required, `read_certificate`
uses `Expect::Witness(&trace)`; then the native hybrid replay arm runs its static
and exact checks with the consumer meter. The original certificate form and
status are not replaced by the independent check's status.

FR-621 owns new serialization and native reader/build work for both the complete
packet and the protocol transition alphabet. The existing generic temporal
reader's model/task restriction is not permission to coerce protocol transitions.
The reader must implement the inverse of `protocol_temporal_wire`, reconstruct
the admitted `ProtocolTraceSubject`, and dispatch through
`replay_protocol_trace_with_temporal_selections`. Model dispatch uses the
corresponding model replay entry and complete model subject. Hybrid dispatch
constructs the existing `ReplaySource::Hybrid` directly and uses native replay.
None requires changing FR-336's old source union. Other families are outside
this bounded transport until separately specified; native terminal outcomes
remain preserved for every item.

## Acquisition and member mapping

The producer captures acquisition handles before `command::analyze::settle_items`
narrows them to a `CheckedPackage`. The consumer owns the independent request.

| Existing owned request member | Producer source and owner | Required/conditional payload |
| --- | --- | --- |
| `package: ReplayPackage` | Acquisition's `EmittedUnit.package().bytes()` or package route's retained `package_bytes`; `ReplayPackage::read/admit` owns syntax/admission. | Required complete raw package; raw fragment in `request.package`. |
| `profile_selections` | Actual admitted package/request selections; family settlement. | Required array, legitimately empty only when native run selected none. |
| `dependencies` | Checked dependency selections, `CheckedUnit.library_source` and `shared_library_packages`; dependency acquisition. | Required per selected dependency, with actual recursive library package bytes in dependency-package document records and source bytes in source_bytes; no identity-only authority. |
| `replay_target` | Resolved `ReplayShape.target`, occurrence/node and actual family selection; item resolver. | Existing native target and over/argument/occurrence bindings where applicable. |
| `source` | Actual family counterexample and envelope; `ItemCounterexample` and family settlement. | Envelope's actual source and complete typed payload, or actual hybrid payload; assembled directly into existing native source types. |
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
| `documents` | `SubjectBinding.packages/snapshots`, selected universe/observation documents and actual recursively selected emitted dependency packages. | Sibling typed document records retain original bytes; selected references/key sets remain in subject. |

The plugin context at `proof_result/plugin_replay.rs:87` demonstrates retention
of emitted packages, dependency locks and actual source bytes. Its `request`
method remains a value-only precedent. Temporal request/envelope composition is
at `analyze/clause_item.rs:581`; model and protocol subject types are at
`model_check/replay.rs:99` and `:156`. Existing family writers/readers are
`temporal_wire`, `protocol_temporal_wire`, `read_temporal_counterexample` in
`outcome/counterexample.rs`. Missing JSON composition/readers are implementation
work, not evidence that these APIs already serialize the complete context.

## Authentication and independent accounting

The native command reader/dispatcher accepts a closed input object
`{format: "native-replay-context/1", context: C, effective_limits: L}` through
the existing replay command entry. `C` is exactly the eight-member object above.
`L` is closed `{replay_input_bytes, accounting_limits, stage_limits}`:
`replay_input_bytes` is the explicit consumer replay byte ceiling,
`accounting_limits` uses the native accounting-counter mapping, and
`stage_limits` contains only consumer accounting-setting entries. Count values
are canonical unsigned decimal strings. Unknown members, missing limits or
non-accounting stage entries refuse before dispatch. No implicit
`ReplayLimits::default()` substitutes for this explicit ceiling. Native byte
admission and reconstruction allocations are bounded by consumer limits even
while inspecting the original proving context.

The reader reconstructs the original `ReplayRequestWire` members directly:
the common thirteen descriptor members, source from the actual family's
envelope/payload, certificate from the sibling member and documents/source
provision from the admitted records. For model/protocol the actual envelope
source is reconstructed separately from its typed family payload. Hybrid uses
`ReplaySource::Hybrid`; it never creates a witness or a new FR-336 kind.
The original request is then admitted through native package/request and family
identity checks. No serialization trait is assumed on `ReplayRequestWire`.

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
The new native dispatcher passes both the authenticated original and a separate
execution budget to the selected family entry; its implementation must separate
original-context identity/parity validation from the independent meter. It may
not alter the envelope or bypass a mismatch by fabricating a second original.
Output remains a `filament-outcome/1` replay outcome with the native result,
refusal or fault and category/exit behavior; successful input parsing alone
does not mean the counterexample was reproduced.

## Outcome integration

The owner `OUTCOME_SCHEMA_1` (declared in `fsl-replay/src/outcome.rs:95`) must
admit optional item member `replay_context` with this exact composition. Existing
items without context remain valid. The producer writer must embed the context
as retained raw JSON, including the exact original `request.package` fragment.
The consumer retains this context as `RawValue` from original stdout and forwards
those bytes unchanged. Parsing or re-emitting the package through a JSON value
that changes its authenticated lexical bytes is forbidden.

The native command returns a fresh outcome with `format: "filament-outcome/1"`,
`operation: "replay"`, `last_stage: "S8"` for completed native replay and
`result: {kind: "replayed", results: [row]}`. It never returns the original
producer item as replay evidence. For hybrid, `row` is the existing
`ReplayResult::to_function_json` hybrid row: `arm: "hybrid"` and settlement
`reproduced-with-evaluated-witness`, `reproduced-with-validated-witness`,
`inconclusive`, or `incomplete`, with the native static/exact result,
counterexample/disagreement/limit fields from `result/wire/replay/arms.rs:72`.
The existing outcome writer owns its category and exit mapping.

For model/protocol the new command writer maps the fresh
`ModelTraceReplayResult.settlement` into one row with `arm` respectively
`"model-trace"` or `"protocol-trace"`, and these required fields:
`settlement`, `category`, `value`, `trace_position`, `undefined`,
`disagreement`, `limit`. `Reproduced` maps to settlement
`"reproduced-with-evaluated-witness"`, category `"success"`, actual trace
position (unsigned decimal string or null), and actual `Truth(b)` as JSON
Boolean `value`; reproduced undefined evaluation uses `value: null` and the
existing FR-364 undefined-evaluation form in `undefined`. Other nullable
fields are null. `Inconclusive(cause)` maps to `"inconclusive"` settlement and
category, with the native disagreement codec in `disagreement` and no value,
position or undefined result. `Stopped` maps to `"incomplete"` settlement and
category, and native reached-limit/value/count data in `limit`, with other
nullable fields null. No produced row means reconstruction/refusal/fault,
reported through the native refusal/fault outcome and diagnostics; it is never
converted to a reproduction row. A serialization fault writes no partial row.
The consumer decides independent reproduction from this fresh arm/settlement
and native payload, never from the producer's `replay.reproduced`, a zero exit
code, or category `success` alone.

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
