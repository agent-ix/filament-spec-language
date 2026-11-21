---
id: FR-644
title: "Stream observations through the monitor command"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-950
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: references
  - target: ix://agent-ix/filament-specification/FR-114
    type: implements
  - target: ix://agent-ix/filament-specification/FR-301
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-853
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: depends_on
---
# FR-644: Stream observations through the monitor command

## Description

FSL's library operation `command` SHALL accept a `native-monitor/1` request and
a stream of observation records, run them through FR-643's `MonitorSession`, and
hand the caller one monitor event per clause status change and a summary at the
end. The driver's `monitor` verb reaches it (FR-287): it reads records from a file
or standard input as they arrive and writes each event as one JSON line. `monitor`
is therefore both a stepping library API (FR-643) and a streaming command; it no
longer exits `unsupported` with no output.

## Use case

`tail -f plant.jsonl | quire monitor monitor.json` prints a line
when a clause's status changes, with `settled` `true` on the line where a
clause settles; a consumer that acts only on settled lines acts once per
clause, and a pending clause's provisional line (`settled` `false`) never
counts as its verdict. When the log ends the command
prints a summary and exits 10 if any clause was violated.

## Inputs

The request file, a closed JSON envelope `{"format": "native-monitor/1",
"request": {...}}`, whose request holds:

- `models`, `program`, `libraries` and `clocks`: as in a `1-draft` native-run/1
  request (FR-100, FR-640); the command compiles the program once;
- `integer_division`: optional, the integer-division law name for the program,
  also admitted on each `libraries` entry, as in a native-run/1 request
  ([FR-767](FR-767-supply-the-integer-division-law-from-a-command-option-or-request-member.md));
- `clauses`: a non-empty array of `{"name": N, "over": R}`, `over` as FR-641
  states for snapshot positions and absent for samples;
- `trace`: `{"kind": "finite"|"progressed"|"timestamped"|"timed", "lower_boundary":
  B, "binding": T, "cover": C}`, the members FR-641 gives that kind (no positions);
- `ledger`: optional, an FR-641 ledger file;
- `state`: optional, the path of a `filament.monitor-state/1` checkpoint to resume
  from (FR-643);
- `observations`: optional, a map from key to snapshot file, for records that name
  a snapshot by key.
- `records`: optional, the relative path of a JSON Lines record file. With it,
  `command::run` over the request reads that file as the whole record stream,
  runs it to its end and returns one `filament.monitor-run/1` result document
  `{"format", "events": [...], "summary"}` holding every event in order and the
  summary, with the summary's exit code. This is the batch form of the same
  stream: a file of records gives the events a live stream of the same records
  gives.
- `items`: optional, a non-empty array of FSpec FR-520 analysis items, with
  `subject`, `documents` and `deadline_ms` optional beside it, each member
  read as FR-620 reads it in a `native-analyze/1` request. These are the
  monitor request's provider-routed items ("Provider-routed items" below).

The record stream is JSON Lines: one closed JSON object per line, each exactly
one of

- `{"observation": P, "stamp": S, "order_key": {"authority": A, "key": Z}, "missing": [M, ...]}`: one
  position, `P` an FR-641 position (an `observations` key, an inline snapshot
  document object `{"snapshot": {...}}`, or `{"sample": R}`), `S` an integer tick
  or an exact rational string as the kind requires, `stamp` and `order_key`
  optional as the kind allows; `missing` optional on a `timestamped` observation
  only, absent meaning an empty list, each `M` exactly FR-641's closed
  `{"position": N, "atom": N}` entry (non-negative JSON integers representable
  in `u64`), passed to FR-643 unchanged;
- `{"progress": G}`: a progress assertion (FR-641's `G`);
- `{"through": X}`: for the `timed` kind only, FR-642's completeness stamp,
  FR-643's `assert_through`;
- `{"checkpoint": PATH}`: write the session's checkpoint to `PATH`;
- `{"end": true}`: the last record; end of input without it ends the stream
  the same way. It ends the stream and adds no closure: the trace's closure
  comes from its kind, its latest progress assertion and its latest
  `through` (FR-643 "Trace end"). A caller closes a progressed trace with a
  `progress` record whose scope is `closed`.

In process: `command::monitor(path: &Path, records: impl Iterator<Item =
Result<Vec<u8>, std::io::Error>>, sink: &mut dyn FnMut(&MonitorEvent), limits:
RunLimits, cancel: &Cancel) -> Result<MonitorSummary, Box<RunError>>`. It also
takes the started process providers as `providers` and the run's
`ProviderOptions` as `options`, by FR-635's rule for every provider-routing
entry (FR-635 "One rule for every provider-routing entry"), and the caller's
negotiator as `negotiate: &dyn Negotiate`, the one `analyze_with_providers`
and `run_with_providers` take (FR-961 "The negotiator is a parameter").

### Provider-routed items

A provider receives a checked package and request items (FSpec FR-331); no
FR-331 member carries an observation record. The verdict of a monitored
clause over the observed stream is therefore always the session's (FR-643),
and the `clauses` never route to a provider. What a monitor request routes
is its `items`: claims of the same compiled program decided over the
program's behaviours, such as a clause monitored at run time that is also
proved over its model. `monitor` SHALL settle them exactly as
`analyze_with_providers` settles the items of the equivalent
`native-analyze/1` request (below), over the registry `Engines::fsl()` and `providers`: route
each item (FR-620), negotiate it with `negotiate`, settle the items
`Supported` on a linked engine in process, and send the items `Supported`
on a provider through `fsl_analyze::provider::settle_items`, one FR-950
`ProviderRequest` per backend whose `package` is the admitted bytes of this
monitor's own compile, by one `ExternalProvider::settle` call per request
under `cancel`. A provider's `proved` is recorded `trusted`, its `refuted`
only after S6a replay reproduces its counterexample, and its `unknown` as
`inconclusive` `solver-inconclusive` (FR-290, FR-635).

The equivalent `native-analyze/1` request carries the monitor request's
`items`, `subject`, `documents` and `deadline_ms`, and selects the program
the monitor compiled. A monitor request without `clocks` selects it as the
analyze request's `program`, with the monitor request's `models`,
`libraries` and `integer_division`. A monitor request with `clocks` selects
it as `package` (FR-850): the handoff directory of the `native-compile/1`
request with the monitor request's `models`, `program`, `libraries`,
`integer_division` and `clocks` and a `handoff` member (FR-665), with the
same `models`.

The items SHALL settle after the program compiles and before the first
record is read, so a refused item is known before any event, and every
event and the summary's clause entries SHALL be those of the same request
without `items`. Once `cancel` fires no further provider request is sent and
every unsettled item settles `incomplete` with cause `cancelled{source}`. A
request without `items` calls neither the negotiator nor any provider.

### Decision: a clocked monitor's items are compared with the package route

A `native-analyze/1` request has no `clocks` member (FSpec FR-520), and none
is added: a clock selection is a compile input whose result the checked
package records as its `temporal_bindings` rows (FR-640), so a run or an
analysis of a compiled package never carries it again. A monitor program
that reads an external clock therefore has no source-route analyze
request, but it has a package-route one over the package of the same
compile, and that package is exactly the bytes the monitor's providers
receive. Comparing with it keeps every item family of the monitor's
program, temporal clauses on a clock included, under the one equality, where
a request over only the unit's value declarations would drop the clauses a
monitor is most likely to also prove over its model.

## Outputs

- One `filament.monitor-event/1` object per clause whose status changed at a
  record, in `clauses` order: `{"format", "record", "position", "clause",
  "settled", "category", "basis", "disposition", "settlement"}`, `settlement`
  present exactly when `settled` is `true`, where `settled` is `true`
  exactly when the clause's status is `Settled` after the record, `record` is the zero-based line
  number, `position` the trace position, and `disposition` FR-267's clause-run
  disposition for that clause, written as FR-267 writes it. The `disposition`
  is that of the clause's report, `B(k)` or `B(s)` (FR-643 "Batch
  equality"), and carries no member of the reading that settled it. An
  event whose `settled` is `true` also carries `settlement`, FR-643's
  `Settlement` as the closed JSON object `{"step", "trigger", "basis",
  "horizon", "position", "through", "cause"}` with each member present exactly
  when FR-643 "Settlement" gives it, `cause` (`requested` or `deadline`)
  present exactly when `trigger` is `cancelled` (FR-853), decimal positions and counts as strings and
  `position` `null` where the reading names none, for example
  `{"step": "3", "trigger": "decided", "basis": "decisive-witness",
  "horizon": "2", "position": "2"}`; an event whose `settled` is `false`
  carries no `settlement`.
- At the end, one `filament.monitor-summary/1` object: each clause's final
  `settled` (`true` for a clause settled before the end, `false` for one
  settled by the end itself), category, basis and disposition, and
  `settlement` with the event's shape, present for every clause: a clause
  settled before the end carries the `settlement` of the event that settled
  it, and a clause settled by the end itself carries `{"step": "<n>",
  "trigger": "finish"}`, `n` the number of counted calls made before
  `finish` (FR-643 "Counted calls": refused records do not count), and a clause
  still open when the stream is cancelled carries `{"step": "<n>",
  "trigger": "cancelled", "cause": "<cause>"}` (FR-853); the number of records
  read; `cancelled`, the handle's cause `requested` or `deadline`, present
  exactly when the stream was cancelled (FR-853); and the final ledger exactly when the kind carries progress (`progressed` or
  `timestamped`), so a `finite` stream's summary carries no ledger. A
  request carrying `items` adds the summary member `items`, one record per
  item in request order, each the item record FR-621 writes for that item in
  the outcome of the equivalent `native-analyze/1` request ("Provider-routed
  items"), and the `registry` member that outcome carries (FR-635); a
  request without `items` carries neither member.
- The exit code: FR-285's code of the most severe final category, a settled
  item's category counted beside the clauses', in the order
  internal failure (30), refusal (20), violation or undefined (10),
  indeterminate (11), unsupported (21), incomplete (22), and otherwise 0.

## Behavior

- `command::monitor` SHALL write nothing to the process streams; it hands each
  event to `sink` before reading the next record, so a driver that writes events
  as they come gives a live monitor.
- The events SHALL be exactly the status changes FR-643's `step` reports (a
  change of the clause's settled status, category or disposition), so the events plus the
  summary agree with a batch run over the same records in FR-643's "Batch
  equality" sense (FSpec FR-114): a pending clause's latest event carries the
  batch disposition over the records so far, a settled clause's carries
  the batch disposition over the records that decided it, and a clause
  settled on a limit carries its `incomplete` limit report instead, which no
  batch run produces (FR-643 "Settlement").
- The observation reader SHALL reject a `missing` value that is `null` or not
  an array, and an entry with an unknown, duplicate, absent or `null` member,
  or a `position` or `atom` of the wrong JSON type or outside `u64`, as a
  malformed record naming the offending member (for example
  `missing[0].atom`). The reader SHALL likewise refuse a repeated top-level
  `missing` member. These refusals produce the existing `invalid-request` refusal event
  at stage `request`, leave the session unchanged and allow the next record.
  The reader SHALL refuse a `missing` member on a non-observation record as
  an unknown member. When an observation is not `timestamped`, the reader SHALL
  refuse its `missing` member, even for an empty array. Position and atom admission remains the
  session's rule in
  [FR-643](FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md).
- A malformed record (not one closed object of the forms above, an observation
  key `observations` lacks, a stamp of the wrong form for the kind) SHALL produce
  a `filament.monitor-event/1` with `category` `refusal`, `invalid-request`,
  naming the record and member, and SHALL leave the session unchanged; the stream
  continues. A step refused by FR-643 (order, admission, contradiction) is reported
  the same way with FR-643's record. The summary counts refused records.
- A request-file defect, a compile refusal, an `open` refusal or a `resume`
  refusal SHALL return the command-error envelope before any record is read, as
  FR-100 refuses a run request, exit 20.
- The driver's `monitor` verb SHALL take the request file operand and `--input
  <file>` (default standard input), write each event and the summary as one JSON
  line to standard output, and follow FR-287's stream, usage and exit rules.
  It SHALL accept `--provider-config` and `--provider-limit` as every
  provider-routing verb does (FR-287, FSpec FR-561), passing the processes it
  started as `providers` and the resolved limits as
  `options.provider_limits`, and its production negotiator as `negotiate`;
  it refuses neither option on `monitor`.
- Every event and summary `command::monitor` hands back shall validate
  against its published schema.
- The driver and `command::run` shall encode every event and summary
  through FR-286's one encoder, `fsl_foundation::canonical_json::encode`.

### Published schemas

The event and the summary are published as two JSON Schemas (draft
2020-12) beside the types that build them in `fsl-command`:

| Object | File | `$id` | Constant |
| --- | --- | --- | --- |
| `filament.monitor-event/1` | `fsl-command/schemas/filament-monitor-event-1.schema.json` | `urn:filament:monitor-event:1` | `fsl_command::command::monitor::MONITOR_EVENT_SCHEMA_1` |
| `filament.monitor-summary/1` | `fsl-command/schemas/filament-monitor-summary-1.schema.json` | `urn:filament:monitor-summary:1` | `fsl_command::command::monitor::MONITOR_SUMMARY_SCHEMA_1` |

Each constant is its file's text (`include_str!`), so the driver's tests of
`fsl monitor` (driver FR-025) validate each line it writes against the same
bytes. A refusal or failure the stream ends with is a `filament-outcome/1`
document and validates against FR-286's `urn:filament:outcome:1`.

- Each schema describes every member at every depth and closes every object
  (`unevaluatedProperties: false`). The event's `settlement` is required
  exactly when `settled` is `true` and absent when it is `false`, and its
  `cause` is required exactly when `trigger` is `cancelled`. The summary's
  `ledger` is present exactly for a `progressed` or `timestamped` stream,
  its `cancelled` exactly when the stream was cancelled, and its `items` and
  `registry` together or not at all.
- `disposition` is described by a `$defs` entry transcribed from FR-267's
  clause-run disposition spelling, for an event that names a clause. A
  refusal event names none: its `clause` is `null`, `settled` `false`,
  `basis` `unavailable`, and its `disposition` is one of two closed shapes.
  A malformed record gives the request refusal `{"stage": "request",
  "category": "refusal", "cause": "invalid-request", "member", "message"}`,
  `member` the record member it names; a step FR-643 refuses (order,
  admission, contradiction) gives FR-643's refusal record, the
  `filament-outcome/1` document of the refusal, a `$ref` to
  `urn:filament:outcome:1`. The schema selects the shape by `clause`:
  `null` admits only these two, a clause name only FR-267's disposition. The summary's `items` entries and
  `registry` are FR-621's analyze item and FR-635's registry, and are a
  `$ref` into `urn:filament:outcome:1`, so one description serves the
  monitor summary and the analyze document.
- Each schema is versioned with its object's format as FR-286 "Published
  schema" states for the outcome document: an added optional member updates
  the `/1` file in the same change as the type; a change an existing reader
  cannot read is `/2` with its own file and `$id`.

The schemas are checked against a golden corpus,
`fsl-command/tests/fixtures/monitor-golden/`, one JSON Lines file per
stream an acceptance criterion of this requirement runs (AC-1, AC-2, AC-6,
AC-7, AC-8 and a cancelled stream), holding the canonical encoding of each
event and of the summary, in order, as the driver writes them. The corpus
test reruns each stream, compares each line with the file, checks each line
is its own RFC 8785 re-encoding, and validates each event line against
`MONITOR_EVENT_SCHEMA_1` and the summary line against
`MONITOR_SUMMARY_SCHEMA_1`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-644-AC-1 | `command::monitor` over FR-643-AC-1's three clauses and five snapshot records ending `{"end": true}` hands the sink one event per status change, at the record that caused it, and returns a summary whose dispositions equal FR-283's batch `monitor` over the closed trace; the exit code is 10. | Test (TC-1668) |
| FR-644-AC-2 | A record with an unknown member between two good records gives one `refusal` event naming that record, and the events and summary for the good records equal those of the stream without it. | Test (TC-1668) |
| FR-644-AC-3 | A `{"checkpoint": PATH}` record after record 2, then a second `command::monitor` call whose request names that file as `state` and whose stream holds records 3 to 5, gives the events of records 3 to 5 that the uninterrupted stream gives. | Test (TC-1669) |
| FR-644-AC-4 | A request naming a clause the program lacks returns the command-error envelope, exit 20, and the sink receives nothing; `command::monitor` writes no byte to stdout or stderr. | Test (TC-1669) |
| FR-644-AC-5 | Through `command::run`, a `native-monitor/1` request whose `records` file holds sample observations for two infinite-trace clauses over a record trace subject, `Reaches` (`eventually holds(r.level = 2)`) and `Never` (`always holds(r.level < 2)`), over levels 0, 1, 2, 1 then `{"end": true}` returns a `filament.monitor-run/1` document in which, for each clause after each observation record `k`: the `category` of the clause's latest event equals the `category` of the clause-run result of the native-run/1 request over the `finite` trace of samples 0 to `k`; that event's `disposition` equals the clause-run disposition over samples 0 to `j`, `j` the record of the latest event (`Never` settles `violation` at record 2, its event `settled` `true`, so from record 2 on `j` is 2 and its disposition carries the three-sample counterexample while the run over four samples carries four; `Reaches` is `pending` with events `settled` `false` at records 0 and 1 and settles `success` at record 2, its witness and good prefix (FR-643-AC-1, AC-6), its event `settled` `true`, so from record 2 on `j` is 2); the summary's `settled` and disposition for each clause equal its latest event's, both `settled` `true`; the summary's `settlement` is the settling event's for each clause, `step` `"3"` and `trigger` `decided`; and the exit code is 10 (`Never` is violated). | Test (TC-1668) |
| FR-644-AC-6 | A `timed` stream of FR-643-AC-10's `Ack` with the one record `req` at 0, followed by `{"through": 250}`, gives, at the `through` record, an event for `Ack` with `settled` `true` and category `violation`; a `{"through": X}` record on a `finite` stream, and a record `{"end": "closed"}`, each give a `refusal` event, `invalid-request`, naming `through` and `end` respectively, and leave the session unchanged. | Test (TC-2324) |
| FR-644-AC-7 | Through `command::run`, a `finite` stream of FR-643-AC-7's `E` over levels 0, 1, 2 then `{"end": true}` gives at record 2 an event with `settled` `true`, a `disposition` equal to the native-run/1 clause run over samples 0 to 2 (category `success`, basis `closed-scope`, no `finite_window`), and `settlement` `{"step": "3", "trigger": "decided", "basis": "decisive-witness", "horizon": "2", "position": "2"}`; the events at records 0 and 1 carry `settled` `false` and no `settlement`; the summary entry carries the same `settlement`, and the summary has no `ledger`. | Test (TC-2340) |
| FR-644-AC-8 | `command::monitor` over AC-1's stream, its unit adding `function Even using v(n: Integer): Boolean pure { n * 2 - n = n }`, with `items` `[{"claim": "Even", "kind": "value-validity", "backend": "fsl.test-recording-provider"}]` and the recording provider (advertising `value-validity` `unbounded`, answering `proved`) in `providers`, calls the provider's `settle` exactly once, before the sink receives its first event, with one `ProviderRequest` holding the items of `Even` whose `package` decodes to the monitor's admitted package; the summary's `items` and `registry` equal those of `analyze_with_providers` over the equivalent `native-analyze/1` request, which, since AC-1's request carries `clocks`, is the `package` request over the handoff directory of the `native-compile/1` request with AC-8's `models`, `program` and `clocks` ("Provider-routed items"), each item `proved` with certification `trusted`; the events and the summary's clause entries equal those of the same stream without `items`, and the exit code is 10. | Test (TC-2984) |
| FR-644-AC-9 | AC-1's stream with the recording provider in `providers` and no `items` calls neither the negotiator nor the provider, and its events and summary are byte-equal to the stream's with `providers` empty. | Test (TC-2984) |
| FR-644-AC-10 | AC-8's request whose item names the unregistered backend `fsl.absent` settles that item `invalid-request` `unknown-backend` in the summary's `items`, calls no provider, still streams every record with AC-1's events, and exits 20; AC-8's request with a provider answering `unknown` settles the item `inconclusive` `solver-inconclusive` and exits 10 (the violation outranks it), and with every clause successful exits 22; two providers of one `BackendId` refuse `invalid-request` naming `providers`, exit 20, before any record is read and with no event. | Test (TC-2984) |
| FR-644-AC-11 | AC-8's request with `cancel` fired before `monitor` is called settles the item `incomplete` with cause `cancelled{requested}` and sends no provider request; the driver's `fsl monitor monitor.json --provider-config providers.json --input records.jsonl` over AC-8's request and a configured test plugin writes AC-8's events and a summary carrying its `items`, and does not refuse `--provider-config`. | Test (TC-2984) |
| FR-644-AC-12 | Every line of `fsl-command/tests/fixtures/monitor-golden/` equals the canonical encoding of the event or summary its stream gives, is its own RFC 8785 re-encoding, and validates against `MONITOR_EVENT_SCHEMA_1` or `MONITOR_SUMMARY_SCHEMA_1`. AC-7's record-2 event fails validation with `settlement` removed (an error at the root pointer, keyword `required`), with `/settlement/trigger` set to `"decide"` (an error at `/settlement/trigger`) and with a member `extra` added (an error at the root pointer); AC-7's record-0 event with a `settlement` added fails at the root pointer; AC-2's refusal event validates, and fails with `/disposition/member` removed (an error at `/disposition`) and with `clause` set to a clause name while its `disposition` stays the request refusal (an error at `/disposition`); AC-6's `through` refusal event validates; AC-7's summary with a `ledger` added fails, and AC-8's summary with `registry` removed fails, each at the root pointer. | Test (TC-3115) |

| FR-644-AC-13 | Stream FR-643-AC-13's timestamped observations with `missing: [{"position": 18, "atom": A}]` on observation 18. Events and final summary match the session and batch reports, including `MissingValuation { position: 18, atom: A }` and exit 11. An omitted `missing` and `missing: []` give identical events and summary for the same timestamped stream. | Test |
| FR-644-AC-14 | Between good timestamped observations, supply records with `missing: null`, a non-array value, entries with absent, unknown, duplicate or null members, negative, fractional, string or overflowing-u64 `position` or `atom`, and a duplicate top-level `missing`. Each yields one stage-`request` `invalid-request` refusal event naming the member and changes no session state; subsequent good records give the events and final clause reports of the stream without the refused records. A `missing` member on a progress record, or on a non-timestamped observation even as `[]`, is likewise refused naming `missing`. | Test |

## Dependencies

- [FR-643](FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md):
  the session the command drives.
- [FR-287](FR-287-reach-fsl-through-the-driver-cli.md),
  [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): the verb and
  exit codes; [FR-267](FR-267-write-run-results-as-native-run-result-2.md): the
  disposition encoding.
- FSpec FR-114, FR-301.
