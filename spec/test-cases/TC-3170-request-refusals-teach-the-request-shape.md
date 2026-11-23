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

Verify FR-1047-AC-1 to FR-1047-AC-8 through `fsl_command::command` in
process: missing members listed at once as JSON pointers with one-of groups,
the accepted formats of each entry family, the export member's shape, and
`request_example`'s documents fed back through their entries. Each test
asserts the exact `message` string and the exact `details` value, and is
tagged `#[trace("TC-3170", "FR-1047-AC-n")]` with the criterion it backs.

## Test Procedure

1. In an empty temporary directory, write each document of FR-1047-AC-1 to
   AC-4 and call the entry the criterion names; read the refusal's code,
   stage, exit, `message` and `details`.
2. For each pair of FR-1047-AC-5, write the document and call the named
   entry.
3. Remove `export` from FR-1015's counter export request (TC-3020) and call
   `export_with_limits_and_overrides` with `ExportSelection::Ordinary`; call
   it again with `ExportSelection::Evidence` over the same request.
4. Call `request_example` for the seven inputs of FR-1047-AC-7 twice each;
   write each result to `request.json` in a fresh empty directory and call
   its entry; then, for each top-level member of its `request`, write the
   example without that member and call the entry again.
5. Call `request_example("native-export/1", None)` and
   `request_example("native-compile/1", Some(ExportSelection::Ordinary))`.

## Expected Results

- Steps 1 to 3 give exactly the codes, stages, exits, messages and details
  their criteria state; no message or details value holds `line` or
  `column`, and no refusal is `unknown_wire`.
- In step 4 every result is `Ok`, the two calls give equal bytes, the text
  is two-space indented with no trailing newline, its only members are
  `format` and `request`, and no call over the unmodified example refuses at
  stage `envelope` or `request`. Each one-member removal refuses
  `invalid-request` at stage `request` with `details.missing` holding that
  member's pointer, or its group's pointer array for `program` and `call`.
- Step 5 gives `UnknownFormat` and `SelectionNeedsRunFormat` with the exact
  `Display` texts, `known` list and `code()` of FR-1047-AC-8.
