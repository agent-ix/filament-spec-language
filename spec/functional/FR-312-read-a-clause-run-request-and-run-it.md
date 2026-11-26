---
id: FR-312
title: "Read a clause-run request and run it through run_clause"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/US-014
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-031
    type: traces_to
  - target: ix://agent-ix/filament-specification/FR-301
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-330
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-031
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: references
---
# FR-312: Read a clause-run request and run it through run_clause

## Description

FSL SHALL provide `fsl_replay::spine::read_clause_run_request`, which reads a
clause-run request file and the files it names into FR-109's
`ClauseRunRequest`. The request shape is FSL's. The `run` command
([FR-100](FR-100-run-a-named-function-through-the-spine.md)) and the
orchestrating driver (agent-ix/filament-driver, ADR-011 T-13) expose it: each
passes the request file to this reader, calls
`fsl_replay::spine::run_clause` (FR-109) with the result, and writes the
report as FR-267's `native-run-result/2` clause-run document. Neither forms
a `ClauseRunRequest` any other way.

FR-100's clause runner is this requirement's reader plus `run_clause`
(FR-109); every artifact names the change that lands it as "the change that
lands FR-100's clause runner". That change is ADR-011 M-6c's deletion of
native `run` and `native-run-result/1` (ADR-031 R-1).

## Inputs

A native-run/1 request file for a `1-draft` program (FR-100's closed
envelope, source selections and intake limits) that carries `clause` in
place of `call`. `clause` is a closed object:

- `selection`: exactly one of
  - `{"clause": {"name": N, "observation": O, ...}}`, FR-106's
    `ClauseSelection`, where `O` names the snapshot or invocation the clause
    reads by its key in `observations`. The permitted member sets are exactly
    `{name, observation, anchor}` and optionally `self` for a current
    snapshot, `{name, observation, parameters}` and optionally `self` for a
    pre snapshot, and `{name, observation}` for an invocation. `self` is
    FR-106's `ObjectRef`; it is absent when the clause's receiver is a state
    machine, which has no object identity, and an object receiver whose
    request omits it is refused by FR-106 check 2, not by the reader.
    `anchor` is FR-106's `{kind, name}`, and `parameters` is FR-106's
    parameter map (including `{}` when empty). An invocation document
    carries its own `self` and call context;
  - `{"frame": {"operation": Op, "invocation": K}}`, FR-115's frame
    selection, which also evaluates the operation's own `requires` and
    `ensures` (FR-702);
  - `{"function": {"name": N, "arguments": A, "snapshot": K}}`, FR-109's
    `Function` selection, with `A` an array of `{parameter, value}` entries
    in any order. Each `value` uses FR-100's public command encoding:
    FSpec FR-181's tagged canonical form for a value type, or FR-650's
    `{"population": P}` selection for a population parameter. A
    `Reference<T>` argument uses FSpec FR-181's tagged form
    `{"type":"reference","universe":U,"object_type":T,"identity":O}`;
    `U` and `T` are structural declaration node ids and `O` is the lowercase
    hexadecimal object identity. The reader resolves it in selected snapshot
    `K` under FR-106 and FR-650;
  - `{"context": {...}}`, FR-652's state-machine context selection
    (FR-106 `Context`);
  - `{"temporal": {"name": N, "trace": T, "over": R, ...}}`, FR-330's
    `Temporal` selection. `T` is exactly one of FR-641's trace kinds:
    `{"finite": [K, ...]}` (`TraceInput::Finite`), `{"lasso": {"prefix": [K,
    ...], "loop": [K, ...]}}` (`TraceInput::Lasso`), `model`, `progressed`,
    `timestamped` and `timed` ([FR-641](FR-641-carry-every-temporal-trace-kind-and-the-progress-ledger-on-the-run-request.md),
    [FR-642](FR-642-run-a-timed-clause-over-a-recorded-timed-trace.md)), each
    position `K` an `observations` key or an FR-641 record sample, in
    position order. The selection may also carry FR-641's `ledger` and FR-985's correction members `relation`, `evidence_ledger` and `admit_late`. `R` is FR-330's `{"reference": {"population": P,
    "key": K}}`, with `P` the population's declaration identity text and
    `K` the object's key. The permitted member sets are exactly
    `{name, trace, over, ledger, relation, evidence_ledger, admit_late}`
    (`over` as FR-641 requires it, the others optional, the last three on a
    `timestamped` trace only, FR-985), one FR-641 member for `trace`, and `{prefix, loop}` for
    `lasso`;
- `observations`: an object mapping each key to the file of a
  `filament.state.snapshot/v1` or `filament.state.invocation/v1` document
  (FR-106);
- `accounting`: optional, a closed object whose members are any subset of
  the ten `filament.value.accounting/v1` counter names, each a JSON integer
  from 0 to `u64::MAX`, with the same defaults as FR-100's
  `call.accounting`; a counter supplied here overrides only that counter's
  default. A direct `work_units` member of `clause` is not accepted;
- `observation_limits`: optional, FR-106's `ObservationLimits` overrides.

The request's `models` and `libraries` are FR-100's and give FR-109's
package and dependency inputs.

## Outputs

- `read_clause_run_request` returns `Result<ClauseRunRequest,
  ClauseRunRequestRefusal>`.
- `command`'s `run` returns the report as one `native-run-result/2`
  clause-run result document (FR-267), with FR-285's exit code of the
  disposition's category (FR-109, FSpec FR-301), and writes nothing to the
  process streams; the driver writes that document to stdout (FR-287,
  Overlap).

## Behavior

- The clause request reader SHALL retain the existing selection shapes and
  FR-109's Boolean `function` claim semantics. A `function` selection's
  request-file arguments SHALL use FR-100's public command encoding: FSpec
  FR-181's tagged canonical values and FR-650's `{"population": P}`
  selection for a population parameter. The reader SHALL admit every
  argument kind admitted by a value call and resolve references and
  population selections in the selection's snapshot, applying the same
  typed admission and refusal behavior.
  A Reference-result function belongs to the value `run` boundary; this
  reader SHALL NOT select it as a Boolean claim or form a default model
  context to stand in for admission.
- The reader SHALL accept FR-652's `context` selection as a `selection`.
  FR-652's `value_invariants` selection and FR-651's population selector
  produce several items and are carried by an FR-651 `items` request; in a
  `clause` request they refuse `invalid-request` naming the member.
- The reader SHALL decode `clause` under FR-100's closed-envelope rules:
  unknown and duplicate members and positional record arrays refuse
  `invalid-request` at stage `request`, naming the member.
- If a `1-draft` request carries more than one of `call`, `clause` and
  `items` (FR-651), or none, then the reader SHALL refuse `invalid-request`
  at stage `request`.
- When `fsl-replay` is built with `quire-extraction` and the request's
  `program` carries `extraction`, the reader SHALL decode `extraction` and
  `clauses`, check the request's mode, build and validate the clause-only
  Quire context, extract the body through FR-030 and put the extracted
  source in the `ClauseRunRequest` as its unit, as FR-031 states. Its mode,
  context and extraction refusals are FR-031's. When `fsl-replay` is built
  without that feature, the reader SHALL refuse `extraction` as an unknown
  member.
- If `selection` holds no variant, more than one, or a variant whose
  members are not the closed set above, then the reader SHALL refuse
  `invalid-request` at stage `request`, naming the member.
- If a selection names an observation key `observations` does not hold,
  then the reader SHALL refuse `invalid-request` at stage `request`, naming
  the key.
- For a clause selection, the reader SHALL inspect the named document's
  format and observation role. From a current snapshot it SHALL construct
  FR-106 `Current` with the request's `self` and `anchor`; FR-106 checks the
  anchor against the snapshot. From a pre snapshot it SHALL construct
  `PreCall` with the request's `self` and `parameters`. From an invocation
  it SHALL construct `Invocation` using the document reference; FR-106 reads
  `self` and parameters from that invocation. A directly selected post
  snapshot, an unrecognized document format or observation role, a missing
  required member, or a member outside that role's permitted set SHALL
  refuse `invalid-request` at stage `request`, naming `observation` for an
  unsupported document or role and the member for a member defect, before
  `run_clause`.
- For a temporal selection, the reader SHALL construct FR-330's
  `Temporal{name, trace, over}` with one FR-106 `DocumentRef` per trace
  key, in the order the request lists the keys; a key listed twice names
  the same document at both positions. Each key SHALL name a
  `filament.state.snapshot/v1` document whose observation role is `current`;
  an invocation document or another role refuses `invalid-request` at
  stage `request`, naming `trace`. Every other trace kind, its record
  samples and the `ledger` file SHALL be read as FR-641 states; a `trace`
  member that is not one of FR-641's kinds, or a kind missing a required
  member, refuses `invalid-request` at stage `request`, naming the member. The reader checks no trace length: an empty `finite` array or
  an empty `loop` reaches FR-330, which refuses it at stage `evaluate`.
- The reader SHALL read each observation file once, under FR-100's
  `dependent_bytes` limit and relative to the request file's directory.
- The reader SHALL pass every byte it read into the `ClauseRunRequest`;
  `run_clause` reads no path (FR-109).
- When the reader admits a request, the run command SHALL run it through
  `run_clause` exactly once.
- The run command SHALL write every compile, selection, admission or
  evaluation result as the report FR-109 gives, never as a command-error
  envelope.
- If the reader refuses a request, then the run command SHALL return an
  outcome carrying FR-267's command-error envelope as its refusal envelope
  and no result document, with the refusal code's exit code.

### The observation size limit has one name and one result shape

FR-255 names the observation document byte ceiling `observation.input_bytes`
(`ObservationLimits::document_bytes`). A caller sets it in two places, and
both are the same setting:

- the request member `clause.observation_limits.document_bytes`;
- the run command's caller limits: the setting and value of the driver's
  `fsl run --limit observation.input_bytes=<n>` operand, passed to the run
  entry in `RequestOverrides::limits` beside the request file, as `clocks`
  is (FR-640, FR-869).

The two SHALL give the same run. A run in which either sets the ceiling to
`B` SHALL produce the same outcome document, byte for byte, as the run in
which the other sets it to `B`. A request that sets
`clause.observation_limits.document_bytes` while the caller also names an
observation setting in `RequestOverrides::limits` SHALL refuse `invalid-request` at stage
`request` naming `observation_limits`, as `clocks` given twice is refused, so
no run depends on which of two bounds wins.

An observation file longer than the ceiling SHALL settle the clause item
`incomplete` with FR-255 Behavior 1's `stage_limit_exceeded` record: `kind`
`input bytes`, `bound` the configured ceiling, `actual` the file's byte count,
and `setting` `observation.input_bytes`, exit 22. The file is not read past
the ceiling. This is the one shape of the limit at every entry point: the
request member, the caller limits and the default ceiling all produce it,
and none produces a top-level `resource_exhausted` diagnostic or a refusal
envelope instead. The rendered hint SHALL name the setting the `--limit`
option accepts, `--limit observation.input_bytes=<n>`, and the request
member `clause.observation_limits.document_bytes`, and SHALL never name
`observation.document_bytes`, which is no setting name and which the
settings operation refuses as an unknown setting.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-312-AC-1 | Through `run`, FR-108's healthy-parent case as a clause-run request (`ParentOrder` over its current snapshot) returns one `native-run-result/2` result document with `success`, `truth: true` and `basis`, exit 0; violating-parent returns `violation`, exit 10. | Test |
| FR-312-AC-2 | A request carrying both `call` and `clause`, one carrying neither, one whose `selection` holds both `clause` and `function`, one with an unknown member in `clause`, and one whose selection names an observation key `observations` lacks each refuse `invalid-request` at stage `request`, exit 20, no result document, naming the member or key. | Test |
| FR-312-AC-3 | Observation files whose total size exceeds `dependent_bytes` stop at the intake limit and settle `incomplete` with `resource_exhausted` (FR-026 Outputs), and the same request with the limit raised to their size runs. | Test |
| FR-312-AC-4 | A `function` selection of `sameIdentity` with object arguments, and a `frame` selection, each give the report FR-109 and FR-115 give for the same `ClauseRunRequest` built directly in code. | Test |
| FR-312-AC-5 | A current clause request with `self` and `anchor` constructs FR-106 `Current` and checks the supplied anchor against the snapshot; a pre clause request with `parameters` and, for an object receiver, `self` constructs `PreCall`; a current or pre request without `self` constructs the selection with no `self`, and admission refuses it as `wrong-observation` at stage `admit` when the receiver is an object; a pre request without `self` for a state machine's precondition admits, and a state machine's invariant selected by `Current` is refused by FR-106-AC-46 (it is selected by `Context`); an invocation clause request constructs `Invocation` and reads `self` from that document. Omitting `anchor` from a current request or `parameters` from a pre request, or adding a role-forbidden member, refuses `invalid-request` at stage `request`, naming that member, before `run_clause`. Selecting a post snapshot or a document with an unrecognized format or observation role refuses `invalid-request` at stage `request`, naming `observation`. | Test |
| FR-312-AC-6 | A clause request's `accounting` object accepts all ten counter names and passes their values unchanged to `ClauseRunRequest.accounting`; omitted `accounting` uses the FR-100 defaults. A direct `work_units` member of `clause` refuses `invalid-request` at stage `request`, exit 20, no result document. | Test (TC-891) |
| FR-312-AC-7 | Through `run`, a request selecting `{"temporal": {"name": "Reaches", "trace": {"lasso": {"prefix": [], "loop": ["s0", "s1"]}}, "over": {"reference": {"population": P, "key": "c1"}}}}` over FR-330-AC-2's `Counter` snapshots `s0` (`c.value` 0), `s1` (1) and `s2` (2) returns the `/2` document FR-267-AC-13 states, exit 10; with loop `["s0", "s1", "s2"]` it returns `success`, exit 0, byte-identical to the report of the same `ClauseRunRequest` built in process. A trace key naming an invocation document or a pre snapshot, a `trace` of `{"model": {}}`, a `trace` holding both `finite` and `lasso`, and an unknown member in `temporal` each refuse `invalid-request` at stage `request`, exit 20, no result document, naming `trace` or the member. A `finite` array of zero keys reaches FR-330 and reports stage `evaluate`, `refusal`, `invalid_runtime_input`/`invalid-value`. | Test (TC-1160) |
| FR-312-AC-8 | Through `run`, a request whose `temporal.trace` is `{"timestamped": {}}` refuses `invalid-request` at stage `request`, exit 20, naming the missing `observations` member of `trace.timestamped`, and no result document is written; a complete `timestamped` trace is read as FR-641 states (FR-641-AC-1). | Test (TC-1160) |
| FR-312-AC-9 | Through `run`, FR-108's healthy-parent clause-run request whose snapshot file is `n` bytes, with `clause.observation_limits.document_bytes` `n - 1` and a `dependent_bytes` above `n`, exits 22 with one clause item settled `incomplete` whose cause is `stage_limit_exceeded` with `kind` `input bytes`, `bound` `n - 1`, `actual` `n` and `setting` `observation.input_bytes`; the document carries no `resource_exhausted` diagnostic and no refusal envelope; the rendered hint contains `--limit observation.input_bytes=` and not `observation.document_bytes`; at `document_bytes` `n` the clause runs and exits 0. | Test (TC-2982) |
| FR-312-AC-10 | The AC-9 request without `observation_limits`, run with `RequestOverrides::limits` naming `observation.input_bytes` `n - 1`, gives an outcome document byte-equal to AC-9's; the AC-9 request run with that override as well refuses `invalid-request` at stage `request` naming `observation_limits`, exit 20, before any observation file is read. | Test (TC-2982) |
| FR-312-AC-11 | Through `run`, a `function` selection of `sameIdentity` over FR-650-AC-4's distinct-identities snapshot accepts each argument `value` as FSpec FR-181's tagged canonical `reference` form: `root` and `child` produce `violation`, `truth: false`, exit 10, and `child` for both arguments produces `success`, `truth: true`, exit 0. A `function nonempty using v(p: Population<Config::ConfigVersion>[4]): Boolean pure { size(allInstances<Config::ConfigVersion>(p)) > 0 }` with `p`'s `value` `{"population": P}` admits the population from that same snapshot and reports `success`, `truth: true`, exit 0; a `P` the snapshot does not hold refuses at stage `admit` with `invalid_runtime_input`/`wrong-role-mapping`, exit 20. The values use FR-100's public command encoding, and each request's result equals the report for the same admitted `ClauseRunRequest` built directly. | Test (TC-1706) |

The existing AC-1 through AC-5 test-case association remains TC-891;
FR-312-AC-11 is associated with TC-1706. Method-only Verification cells do
not replace the existing trace tags.

| FR-312-AC-12 | The reader accepts FR-652's `context` selection. `value_invariants` and FR-651 population-selection requests are admitted through the supported `items` form; either selection placed in a `clause` request refuses `invalid-request` at stage `request`, naming the member. | Test (TC-891) |
| FR-312-AC-13 | In a `clause` request, an unknown member, duplicate member, or positional record array refuses `invalid-request` at stage `request`, naming the offending member or position; no `ClauseRunRequest` is returned. | Test (TC-891) |
| FR-312-AC-14 | A request carrying more than one of `call`, `clause` and `items`, or carrying none of them, refuses `invalid-request` at stage `request` before any run; a request carrying exactly one supported form proceeds to that form's reader. | Test (TC-891) |
| FR-312-AC-15 | With `quire-extraction` enabled, the reader decodes a valid program `extraction` and its clauses, checks the mode and context, and puts the extracted source in `ClauseRunRequest.unit`; with the feature disabled, the same `extraction` member is refused as unknown. | Test (TC-891) |
| FR-312-AC-16 | A selection with no variant, multiple variants, or a variant whose member set is not the allowed closed set refuses `invalid-request` at stage `request`, naming the variant or member; a selection naming an observation key absent from `observations` refuses at the same stage, naming that key. | Test (TC-891) |
| FR-312-AC-17 | For a clause selection, a post snapshot, an unrecognized document format or observation role, a missing required member, or a role-forbidden member refuses `invalid-request` at stage `request` before `run_clause`; the refusal names `observation` for an unsupported document or role and the offending member for a member defect. | Test (TC-891) |
| FR-312-AC-18 | For a temporal selection, a trace key naming an invocation or non-current observation, a trace kind outside FR-641's kinds, or a kind missing a required member refuses `invalid-request` at stage `request`, naming `trace` or the missing member. Each trace key must name a current snapshot document. | Test (TC-1160) |
| FR-312-AC-19 | The reader reads each observation file once, relative to the request file's directory, under `dependent_bytes`, and passes every byte it read into `ClauseRunRequest`; the request handed to `run_clause` contains no path from which it could reread the file. | Test (TC-891) |
| FR-312-AC-20 | An admitted request invokes `run_clause` exactly once. Compile, selection, admission and evaluation outcomes are written as FR-109's report; a reader refusal returns FR-267's command-error envelope with the refusal code's exit code and no result document. | Test (TC-891) |
## Dependencies

- [FR-109](FR-109-run-a-state-clause-through-the-spine.md): `ClauseRunRequest`,
  `run_clause` and `ClauseRunReport`.
- [FR-100](FR-100-run-a-named-function-through-the-spine.md): the request
  envelope, source selections, `models`, `libraries`, intake limits and the
  `run` command.
- [FR-106](FR-106-admit-snapshots-and-invocations.md): observation documents
  and `ObservationLimits`; FR-115 for the frame selection.
- [FR-330](FR-330-run-a-temporal-clause-through-the-spine.md): the
  `Temporal` selection, its trace input and `over` binding.
- [FR-267](FR-267-write-run-results-as-native-run-result-2.md): the `/2`
  clause-run document and the command-error envelope.
- FSpec FR-301 (exit codes).

## Status

QSL-665's typed-call clarification is proposed; its consumer qualification
is UNRUN. The accounting setting member is owned by QSL-656; this
clarification does not duplicate that contract or define a compatibility
member.

The deletion of native `run` (ADR-031 R-1) also removes the per-file content
checks the native-v1 request selections carry (FR-026,
FR-027, FR-100): a request names its files, and the command reads their
bytes as they are.

## References

- Owning ticket: Linear QSL-389 (plan slice E19-CLI).
