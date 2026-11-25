---
id: TC-3170
title: "Request refusals name every missing member and accepted format, and every example is read back"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1047
    type: verifies
---
# TC-3170: Request refusals name every missing member and accepted format, and every example is read back

## Description

Verify FR-1047-AC-1 to FR-1047-AC-8 and FR-1047-AC-10 to FR-1047-AC-16
through `fsl_command::command` in process: missing members listed at once as
JSON pointers with one-of groups, the accepted formats of each entry family,
the export member's shape, `request_example`'s documents fed back through
their entries, and which refusal a request with no run member gets. Each
test asserts what its criterion states, the exact `message` string and
`details` value where the criterion gives them, and is tagged
`#[trace("TC-3170", "FR-1047-AC-n")]` with the criterion it backs.

## Test Procedure

1. In an empty temporary directory, write each document of FR-1047-AC-1 to
   AC-4 (including `{"format":"bogus/1"}`, an FR-661 `argv` document, the
   empty `native-monitor/1` and `native-replay/1` requests, and the nested
   `models` / `program` case) and call the entry the criterion names; read
   the refusal's code, stage, exit, `message` and `details`.
2. For each entry and document of FR-1047-AC-5 (run, run-reader, analyze,
   compile, monitor and export families), write the document and call the
   named entry.
3. Remove `export` from FR-1015's counter export request (TC-3020) and call
   `export_with_limits_and_overrides` with `ExportSelection::Ordinary`; call
   it again with `ExportSelection::Evidence` over the same request; then call
   it with `Ordinary` over the request whose `export` lacks `profile`.
4. Call `request_example` for the seven inputs of FR-1047-AC-7 twice each;
   write each result to `request.json` in a fresh empty directory and call
   its entry (the monitor example through `monitor` with an empty record
   stream; for the replay example, again with an empty `entry.fsl` written
   beside `request.json`); then, for each top-level member of its `request`, write the
   example without that member and call the entry again.
5. Call `run_with_providers` over the monitor example with `records`
   absent, as in step 1's empty monitor request, to confirm the
   monitor-run reading lists `/request/records`.
6. Call `request_example("native-export/1", None)` and
   `request_example("native-compile/1", Some(ExportSelection::Ordinary))`.
7. In an empty directory, call `run_with_providers`, `prepare_run` and
   `read_clause_run_request` over FR-1047-AC-10's program request and package
   request, then over the program request without `models`, then over the
   package request with a `libraries` entry.
8. Make each call of FR-1047-AC-11 over both of AC-10's requests, in an
   empty directory.
9. Make each call of FR-1047-AC-12 and of FR-1047-AC-13 in an empty
   directory, adding one member at a time.
10. Make each call of FR-1047-AC-14 in an empty directory, changing the
    program request one way at a time.
11. In a build with `quire-extraction`, make each call of FR-1047-AC-15.
12. Make each call of FR-1047-AC-16, those over AC-15's request in a build
    with `quire-extraction`.

## Expected Results

- Steps 1 to 3 give exactly the codes, stages, exits, messages and details
  their criteria state; no message or details value holds `line` or
  `column`, and no refusal is `unknown_wire`. `{"format":"bogus/1"}` gets
  the format refusal, the `argv` document is not refused by FR-1047, and the
  `export` lacking `profile` keeps FR-1015's refusal.
- In step 4 every result is `Ok`, the two calls give equal bytes, the text
  is two-space indented with no trailing newline, its only members are
  `format` and `request` in member order, and no call over the unmodified
  example refuses at stage `envelope`, by FR-1047, or for an unknown,
  repeated or wrong-kind member. The run, compile, analyze, monitor and
  export examples refuse only at stage `file` naming a file the example
  names, the evidence example settles `unimplemented-meaning`, and the
  replay example refuses at stage `request`, exit 20, with the read
  refusal naming `entry.fsl`; with an empty `entry.fsl` beside it the entry
  is not refused `invalid-request` and not with a read refusal naming `entry.fsl`, and refuses by FR-098's recompile or
  package admission, never as `invalid_package` for `/package/lock/sources`
  and never as `missing_import`.
  Each one-member removal refuses
  `invalid-request` at stage `request` with `details.missing` holding that
  member's pointer, or its group's pointer array for `program` and `call`.
- Step 5 refuses with `/request/records` as the one missing entry.
- Step 6 gives `UnknownFormat` and `SelectionNeedsRunFormat` with the exact
  `Display` texts, `known` list and `code()` of FR-1047-AC-8.
- Step 7 gives the messages and `details` of FR-1047-AC-10, at stage
  `request`, exit 20, with no file read.
- Steps 8 to 11 give the refusal or limit outcome their criteria state, with
  no `details.missing`, never AC-10's message, and no file read other than
  the request.
- Step 12 gives AC-10's refusal for each call's source, with no file read
  other than the request.
