---
id: TC-3019
title: "A native-run request runs over a selected checked package"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1039
    type: verifies
---
# TC-3019: A native-run request runs over a selected checked package

## Description

Verify that a package-only `native-run/1` request runs from the package's
own records, never reads or compiles source, and refuses a package that
cannot run with the code and locus stated. Scope: FR-1039-AC-1 to
FR-1039-AC-6.

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
   members over the source.

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
  FR-1039-AC-6 states; none is the not-implemented refusal.
