---
id: FR-644
title: "Stream observations through the monitor command"
type: FR
relationships:
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

The record stream is JSON Lines: one closed JSON object per line, each exactly
one of

- `{"observation": P, "stamp": S, "order_key": {"authority": A, "key": Z}}`: one
  position, `P` an FR-641 position (an `observations` key, an inline snapshot
  document object `{"snapshot": {...}}`, or `{"sample": R}`), `S` an integer tick
  or an exact rational string as the kind requires, `stamp` and `order_key`
  optional as the kind allows;
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
takes the started process providers as `providers`, by FR-635's rule for
every provider-routing entry (FR-635 "One rule for every provider-routing
entry").

## Outputs

- One `filament.monitor-event/1` object per clause whose status changed at a
  record, in `clauses` order: `{"format", "record", "position", "clause",
  "settled", "category", "basis", "disposition"}`, where `settled` is `true`
  exactly when the clause's status is `Settled` after the record, `record` is the zero-based line
  number, `position` the trace position, and `disposition` FR-267's clause-run
  disposition for that clause, written as FR-267 writes it.
- At the end, one `filament.monitor-summary/1` object: each clause's final
  `settled` (`true` for a clause settled before the end, `false` for one
  settled by the end itself), category, basis and disposition, the number of records read, and the final
  ledger when the kind carries progress.
- The exit code: FR-285's code of the most severe final category, in the order
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
  batch disposition over the records so far, and a settled clause's carries
  the batch disposition over the records that decided it.
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

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-644-AC-1 | `command::monitor` over FR-643-AC-1's three clauses and five snapshot records ending `{"end": true}` hands the sink one event per status change, at the record that caused it, and returns a summary whose dispositions equal FR-283's batch `monitor` over the closed trace; the exit code is 10. | Test (TC-1668) |
| FR-644-AC-2 | A record with an unknown member between two good records gives one `refusal` event naming that record, and the events and summary for the good records equal those of the stream without it. | Test (TC-1668) |
| FR-644-AC-3 | A `{"checkpoint": PATH}` record after record 2, then a second `command::monitor` call whose request names that file as `state` and whose stream holds records 3 to 5, gives the events of records 3 to 5 that the uninterrupted stream gives. | Test (TC-1669) |
| FR-644-AC-4 | A request naming a clause the program lacks returns the command-error envelope, exit 20, and the sink receives nothing; `command::monitor` writes no byte to stdout or stderr. | Test (TC-1669) |
| FR-644-AC-5 | Through `command::run`, a `native-monitor/1` request whose `records` file holds sample observations for two infinite-trace clauses over a record trace subject, `Reaches` (`eventually holds(r.level = 2)`) and `Never` (`always holds(r.level < 2)`), over levels 0, 1, 2, 1 then `{"end": "open"}` returns a `filament.monitor-run/1` document in which, for each clause after each observation record `k`: the `category` of the clause's latest event equals the `category` of the clause-run result of the native-run/1 request over the `finite` trace of samples 0 to `k`; that event's `disposition` equals the clause-run disposition over samples 0 to `j`, `j` the record of the latest event (`Never` settles `violation` at record 2, its event `settled` `true`, so from record 2 on `j` is 2 and its disposition carries the three-sample counterexample while the run over four samples carries four; `Reaches` stays `pending`, its events `settled` `false`, and `j` is the record of its latest pending event); the summary's `settled` and disposition for each clause equal its latest event's, `Reaches` `pending`; and the exit code is 10 (`Never` is violated). | Test (TC-1668) |
| FR-644-AC-6 | A `timed` stream of FR-643-AC-10's `Ack` records followed by `{"through": 250}` gives, at the `through` record, an event for `Ack` with `settled` `true` and category `violation`; a `{"through": X}` record on a `finite` stream, and a record `{"end": "closed"}`, each give a `refusal` event, `invalid-request`, naming `through` and `end` respectively, and leave the session unchanged. | Test (TC-2324) |

## Dependencies

- [FR-643](FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md):
  the session the command drives.
- [FR-287](FR-287-reach-fsl-through-the-driver-cli.md),
  [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): the verb and
  exit codes; [FR-267](FR-267-write-run-results-as-native-run-result-2.md): the
  disposition encoding.
- FSpec FR-114, FR-301.
