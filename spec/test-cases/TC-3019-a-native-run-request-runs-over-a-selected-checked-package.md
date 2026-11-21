---
id: TC-3019
title: "A native-run request runs over a selected checked package"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1039
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-026
    type: verifies
---
# TC-3019: A native-run request runs over a selected checked package

## Description

Verify that a package-only `native-run/1` request runs from the package's
own records, never discovers or compiles source, and refuses a package that
cannot run with the code and locus stated. Explicit caller-selected rendering
sources use the existing source-file intake and do not supply execution
semantics. Scope: FR-1039-AC-1 to AC-12, FR-100-AC-23 and FR-026-AC-8.

## Test Procedure

1. Compile `half` (`-7 div 2`) with `integer_division` `floor`, delete the
   source, and call it from the package file; repeat with `integer_division`;
   send a request with neither `program` nor `package`.
2. Run `Pick` from the handoff directory and from the package file with its
   `equality` file; run it from the package file alone, with the
   `equality.json` of another unit that holds an equality application, and as `{directory, equality}`; send a `clause`
   member over the package file.
3. Send a missing file, a v1 document, a package with an unknown member, no
   `models` entry for a spine-model package, and a spine `models` entry for
   the value package.
4. Export FR-1015-AC-4's counter from its package file and its handoff's
   `equality.json` (`{file, equality}`) with
   `source_package` `{"identity": "example/counter-unit", "version": "7"}`,
   the source removed; send it without `source_package`, with `null` (refused `wrong-value-kind`), with a
   missing file, and with each malformed `source_package` of FR-1039-AC-4;
   call `half` over the package with and without `source_package`.
5. Compile `Even` (FR-961-AC-2) and RR-12 (FR-062) to package files with
   handoffs, remove the sources, and run `Even` (as `{file, equality}`, and
   once as `{file}` alone) and `f` (`n = 4`, as `{file}`) with `call.requirements`
   `{}` over each package; run the same requests over the sources; call `g`
   over RR-12's package with `requirements` `{}`; send a call of `g` whose
   `items` names a claim of `f`; remove the row of `f`'s `+` record from the
   package and run `f` again.
6. Compile FR-931-AC-1's `Low` unit to a package file with a handoff,
   remove the source, and send the `tl_lowering` members of FR-1039-AC-6 over
   `{file, equality}`; send the same
   members over the source; send FR-1039-AC-6's `cross_evaluate` request
   over `{file, equality}` and over the handoff `{directory}`.
7. Remove the `Low` package's `temporal_clause_names`; separately set the
   `Low` node's `temporal_profile` law to `filament.temporal.unknown/v1`;
   send `{"clause": "Low"}` over each. Compile FR-931-AC-16's unit to a
   package and send `{"clause": "Low"}` over it and over the source.
8. Send a package request with the optional top-level `sources` absent,
   then `[]`, then the ordered array
   `[{"file":"unit.native","authority":"example","identity":"unit-1","document":"doc-1","formal_revision":1}]`.
   Exercise each carrier defect in the table below, with unavailable selected
   files to establish that structural refusal precedes I/O. Send duplicate
   native pairs with equal bytes, with different files, and with different
   valid formal metadata; then send distinct pairs. Send a source `program`
   request with `sources: []` and with a non-empty valid array, then omit it.
9. After valid package and model admission, select an unavailable source
   file; separately select files containing invalid UTF-8, BOM and NUL, and
   each blank native label. Use two distinct valid entries to observe one
   admission per selected source, in array order, before member execution.
   Vary `request_bytes`, `dependent_bytes` and the selected-file count as
   the resource rows below state, counting package, model and rendering
   files together. Start each invocation with fresh budgets.
10. Compile a call that produces a genuine located refusal and a temporal
    `holds` condition that produces a source-native refusal (the
    [FR-283](../functional/FR-283-monitor-a-supplied-trace-offline.md)
    condition-refusal fixture), recording the source route's code, cause,
    fields and span. Run the packages with explicit `sources` selecting
    those original bytes from a different authored file path. Independently
    omit the matching source, change its native pair, truncate its admitted
    UTF-8 text below the span, and place a span endpoint inside a UTF-8
    scalar. Change only valid formal metadata as a matching control.
11. Run [TC-1295](TC-1295-evaluate-the-finite-window-profile-on-the-spine.md)
    rows 43 and 44 for finite-window whole-atom text and missing-source
    parity. Repeat step 6's TL lowering with absent `sources`, `[]` and valid
    explicit caller sources; retain different source bytes in the handoff
    for the rendering tests.

The carrier and resource cases for steps 8 and 9 are:

For retained host failures, assert the outer `native-run-result/2.code`
separately from any native diagnostic code. The formal constructor refusal
has outer `invalid-identifier` and `details.code` `invalid_identifier`;
the file failure has outer `io-error` and its existing path/OS-code details.
The native catalog's underscore spellings are unchanged.

| Input variation | Expected diagnostic and point of failure |
| --- | --- |
| `sources` is `null`, `{}`, a string or a number | `invalid-request`, stage `request`, exit 20, `/request/sources`, before selected-file I/O |
| repeated top-level `sources` | same code/stage/exit and `/request/sources`, before I/O |
| entry is `null`, scalar or a positional array | same code/stage/exit, `/request/sources/0`, before I/O |
| independently omit each of the five required fields, add `text`, or repeat each field | same code/stage/exit, naming `/request/sources/0/<field>`, before I/O |
| non-string `file`, `authority`, `identity` or `document`; non-integer, negative, null or above-u64 `formal_revision` | same code/stage/exit, naming that field, before I/O |
| invalid formal document, or `formal_revision` zero | outer `invalid-identifier`, stage `request`, exit 20, naming the constructor-invalid field, before I/O; constructor `details.code` remains `invalid_identifier` |
| second entry repeats the first native pair, despite different path or formal metadata | `invalid-request`, stage `request`, exit 20, `/request/sources/1`, before I/O |
| `program` beside either empty or non-empty `sources` | `invalid-request`, stage `request`, exit 20, `/request/sources`, before I/O |
| valid package/model inputs, unavailable explicitly selected source | outer `io-error`, stage `file`, exit 20, retaining its file path |
| invalid UTF-8, BOM, NUL or a blank native label in selected source | unchanged FR-001 source/S0 diagnostic, exit 20, including original offered-source details |
| request JSON at `request_bytes`, then one byte over | at the bound, run; over it, `resource_exhausted`, incomplete, exit 22, naming `request_bytes` and its configured value |
| dependent file bytes including sources at `dependent_bytes`, then one byte over | at the bound, run; over it, `resource_exhausted`, incomplete, exit 22, naming `dependent_bytes` and its configured value |
| valid request with 64 selected files across all roles, then add a 65th via `sources` | 64 runs; 65 gives `resource_exhausted`, incomplete, exit 22, naming the selected-file maximum 64 |

## Expected Results

- Step 1: `-4`; `invalid-request` naming `integer_division`, no file opened;
  `invalid-request` naming `program`.
- Step 2: `candidate` 7, exit 0, from `{directory}` and from `{file,
  equality}`; `{file}` alone refuses `invalid_package`/`missing-file` at
  `/request/package/equality`, exit 20, with no synthesis outcome; the
  foreign witness refuses `invalid_package`/`refused-file` (`Unjoined`);
  `{directory, equality}` refuses `invalid-request` naming
  `package.equality`; the `clause` member refuses `invalid-request` with
  `requires` `directory`.
- Step 3: each refusal at stage `request`, exit 20, at its row's locus; the
  v1 document refuses `unknown_wire`/`unsupported-wire` at
  `/request/package/contract_version`.
- Step 4: the outcome's `package.source_package` equals the stated pair;
  each refusal is `invalid-request` with `invalid_runtime_input` and the
  cause and `details.path` FR-1039-AC-4 states, exit 20, no file opened;
  the call returns `-4` both times.
- Step 5: the package runs' `requirements`, `registry` and `execute`
  `items` equal the source runs' byte for byte; `g` writes `requirements`
  `[]`; the `items` request refuses `UnknownRequirementClaim` before any
  negotiation; `Even` from `{file}` alone refuses `invalid_package`/
  `missing-file` at `/request/package/equality`; the package without the row refuses at stage `request`, exit
  20, `invalid_package`/`invalid-value` at
  `/request/package/semantic_graph/nodes/{n}/body`, with no item settled.
- Step 6: each `tl_lowering` record over the package equals the source
  route's byte for byte; each refusal has the code, member and exit
  FR-1039-AC-6 states; none is the not-implemented refusal; each record
  carries the members FR-1039-AC-6 lists; the `cross_evaluate` request
  refuses `invalid-request` with `requires` `directory` over
  `{file, equality}` and returns the source record over `{directory}`.
- Step 7: `invalid_package`/`invalid-value` at
  `/request/package/temporal_clause_names` and `unknown_profile`/
  `unsupported-selection` at `/request/package/semantic_graph/nodes/{n}/body/operation/laws/0/definition`, each at stage `request`, exit 20, no
  record; FR-931-AC-16's package record equals the source record byte for
  byte, its atom `count<ix://I/Small>`.
- Steps 8 and 9: the table's diagnostic fields, stage, exit and intake
  precedence hold, with no member outcome for failures. Distinct native pairs
  remain in array order. Resource stops retain null diagnostic cause/locus
  and the existing applicable limit-raising option; fresh invocations have
  fresh budgets. Valid source requests without `sources` are unchanged.
- Step 10: package refusals preserve the source result's code, cause and
  fields; their locus `file` is the caller's new authored path and their span
  equals the source span. Invalid or missing matches give
  `invalid_runtime_input`/`missing-member` naming `sources`, stage `call`,
  exit 20, with the region's native identity and clause when applicable,
  no clause record or rendered member outcome. Valid changed formal metadata
  leaves native matching unchanged. None becomes an internal fault.
- Step 11: finite-window text is the exact whole `holds( p )` from caller
  bytes; direct adapters do no source-file intake after caller provisioning.
  Command intake reads only explicitly selected files and never compiles
  them or discovers handoff source bytes. TL atom text stays byte-equal to
  step 6's canonical lowering. Caller labels and formal metadata remain
  unauthenticated assertions; neither bytes nor a content hash validates them.
