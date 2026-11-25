---
id: FR-563
title: "Run the parity gate over every non-interpreter backend and fail on any difference from the interpreter"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-294
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-295
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-296
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-560
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-350
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-353
    type: references
  - target: "ix://agent-ix/filament-specification/FR-336"
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: "ix://agent-ix/filament-specification/FR-306"
    type: depends_on
---
# FR-563: Run the parity gate over every non-interpreter backend and fail on any difference from the interpreter

## Description

ADR-029 EB-5 and FR-295 require each non-interpreter backend to run the
conformance corpus and generated programs against the interpreter, with any
difference failing the gate. This requirement specifies the runner that does
it and its failure policy.

### The runner

The runner is a library operation in `fsl-conformance`, an FSL crate outside
the qualified core (ADR-029 CB-3 item 3, as amended):

```rust
pub fn run_parity(
    backends: &[(ExecutionBackendChoice, &dyn SuppliedBackend, Option<&BackendCoverage>)],
    inputs: &ParityInputs,
    limits: &ParityLimits,
    cancel: &Cancel,
) -> Result<ParityReport, ParityRefusal>;
```

`ParityInputs` names the conformance corpus location (read in place, as
FR-350 reads it), the generator seed (default FR-295's
`0x51534C2D4A495431`) and count (default 10,000). The reference is always
the interpreter: a test may substitute a reference only through
`run_parity_with_reference`, which exists only under `fsl-conformance`'s
`test-support` feature, and `make parity` never enables it. `BackendCoverage`
is the seam type FR-560 defines in `fsl-eval`, and `fsl-conformance` does not
depend on `fsl-jit`. `make parity` runs the same operation over `fsl-aot`, an FSL crate
(Overlap). `make parity` runs `cargo xtask parity`, which links every
non-interpreter backend the FSL workspace has and calls `run_parity` with the
defaults.

### Parity cases

A parity case is one source unit, one entry and a list of argument tuples
(each tuple the values of the entry's parameters in order, or, for a corpus
prefix call, of a leading part of them). The generated
units carry the tuples FR-295 generates. For the corpus, the runner reads the
vectors in place (FR-350), selects those whose subject consumer type is
`runtime` and whose operation is `run` (FR-100) with a function entry, and
builds one case from each it can build: the unit is the source files the vector's
`input.files` names, the entry is the function the vector names, and the
vector's one decoded argument assignment, in parameter order through the same
input adapters FR-350 uses, is the case's one tuple. A vector of another
consumer type or operation (`compile`, `format`, `replay`), or whose entry is
a clause, is no parity case and is counted in the report as not selected,
and is never a finding. The case's id is the vector id.

A call that supplies fewer arguments than the function declares is a prefix
call (FSpec FR-336): it is a parity case whose tuple holds the supplied
prefix, and the runner compares both sides' results for it, a `CallFailure`
included.

A selected vector from which the runner cannot build a case is unbuildable.
The report lists it with its vector id and one of these causes, counts it,
and runs every other case:

| Cause | When | Excused by an expected refusal |
| --- | --- | --- |
| `UnreadableFile` | A file its `input.files` names cannot be read. The cause names the path. | No |
| `UnsafePath` | A path in its `input.files` is absolute or parent-traversing. That path is never read. The cause names it. | No |
| `MissingMember` | The vector or its call file lacks a member or file kind the case needs, or the member has another shape: among them no `input.files` array, no source file, no call file, and a call file without the function or the argument array. | No |
| `DuplicateFile` | It names more than one call file; or it names more than one source file and a later step of its build fails, whatever cause that step would otherwise give. | No |
| `InvalidCall` | Its call file is not a JSON document, is not an FSpec FR-336 `$defs.CallInput` document in a way no other cause names, or holds an accounting value the input adapters refuse. The cause names the path. | No |
| `InvalidObservations` | An object context or population file is not an identified state snapshot. | No |
| `UndecodableArgument` | The input adapters cannot decode an argument: a malformed tagged value or unknown tag, a reference or population name that resolves to nothing admitted, or a population binding that conflicts with an earlier one. The cause names its position. | No |
| `ExtraArgument` | Its call supplies more arguments than the function declares, which FSpec FR-336 makes an input-adapter refusal. The cause names the first position with no parameter. | No |
| `Fault` | A step of its build ends in an internal fault, or in another outcome FR-350 settles `tool-failure` that no other cause names. | No |
| `Uncompilable` | Parsing, model admission, checking or packaging its source with its `model` files ends in an FSL typed refusal, a reached limit included. | Yes |
| `RefusedObservations` | Admission of its object environment ends in the owner's typed refusal or a reached limit. | Yes |
| `UnknownFunction` | Its call names a function the source does not declare. | Yes |
| `RefusedArgument` | The owner's admission of a decoded argument ends in a typed refusal or a reached limit. The cause names its position. | Yes |

A vector with several defects has one cause, the first the runner meets in
this order: the `input.files` member; each file in listed order, its path and
then its bytes; the number of source and call files; the call file's
document, the shape of its argument values included; compilation; the object
context and population files and then their admission; the called function;
and each argument in position order. `DuplicateFile` replaces the cause of a
later step as its row states.

A `runtime`/`run` vector is left unselected for a clause entry only when its
call file is read and names a clause; one whose entry cannot be read is
unbuildable. An unbuildable vector is not counted as not selected.
Cancellation while a case is built stops the run with no report and makes no
vector unbuildable.

An unbuildable vector is also an `unbuildable-case` finding unless it is
excused. It is excused only when it expects a refusal and its cause is one
that FR-350 settles by an FSL typed refusal, a cause marked Yes. A cause
marked No is a defect of the vector's input or of its decoding, or an internal
fault, which FR-350 never passes; it is a finding whatever the vector
expects. A vector expects a refusal when its `expected.disposition`, the
expected disposition FR-350 compares against, in FSpec FR-336's closed
vocabulary of `value` and `refusal`, is `refusal`. Neither the vector's
`class` nor the refusal's `code` or `cause` decides this. A vector that
expects a refusal and does build is a parity case like any other.

For each backend and each corpus or generated unit, the runner prepares the
unit's entries on the backend and on the reference, then calls each entry
with each argument tuple on both, twice on the backend.

### What fails the gate

Each of these is a finding in the report, and any finding fails the gate:

| Finding | When |
| --- | --- |
| `difference` | The backend's result differs from the reference's in any of: `Ok` against `Err`; the `Evaluation` outcome kind; the completed value; the typed cause; the `Location`; the loss records; an `Incomplete`'s limit or charge point; a `CallFailure`'s variant or payload. A backend `Fault` where the reference returned anything else is a `difference`. |
| `nondeterministic` | The backend's two calls with the same inputs return different results. |
| `coverage-gap` | The backend supplied a coverage (FR-560) and listed unsupported with an `Untranslatable` cause an entry whose constructs all lie in it. An entry listed with an `EnvironmentRefused` cause is reported with its `UnsupportedEntry` and not compared, and is no `coverage-gap`. |
| `prepare-failed` | The backend's `prepare` returned a `PrepareFailure` for a unit the reference prepared. |
| `empty-comparison` | No entry was compared for a backend: every entry was unsupported. |
| `reference-fault` | The interpreter returned `Fault` for an entry, from a call or from `prepare` of the entry's unit (one finding for each entry of that unit): the oracle itself has a defect. The entry is not compared on any backend. |
| `reference-prepare-limit` | The interpreter's `prepare` of a unit returned `PrepareFailure::Limit`: one finding for each entry of that unit, carrying the reached limit (FR-294, naming its setting). No entry of the unit is compared on any backend. |
| `generator-exhausted` | Checking refused more generated draws than `ParityLimits.max_discarded` (default 10,000) before the generator yielded the requested count. The finding names, by index and source bytes, the refused draw that passed the bound, and the runner draws no further unit. |
| `unbuildable-case` | A selected corpus vector is unbuildable and is not excused (Parity cases). |

`reference-fault`, `reference-prepare-limit`, `generator-exhausted` and
`unbuildable-case` are no one backend's defect: each is recorded for every
supplied backend.

An entry the backend lists unsupported outside its coverage is reported with
its `UnsupportedEntry` and not compared (FR-295). Every finding carries the
FR-295 report fields: the seed, the unit's index or corpus vector id, the
unit's source bytes, the entry, the arguments and both results. A
`generator-exhausted`, `reference-prepare-limit` or `unbuildable-case`
finding has no result. `generator-exhausted` also has no entry or arguments;
`reference-prepare-limit` carries the entry's first argument tuple; and
`unbuildable-case` carries its cause, no arguments, and the source bytes and
entry only as far as the runner read them.

### Failure policy

- No allow-list, tolerance, expected-difference record, retry or skip
  exists. The runner has no option, environment variable or setting that
  excuses a finding. A vector's own expected refusal is no such setting: it is
  the corpus's oracle, and the vector it covers is still listed and counted.
- `ParityLimits.max_discarded` bounds generation and excuses no finding.
  `cargo xtask parity` and `cargo xtask qualify` run
  `ParityLimits::default()`, and no option or environment variable sets it.
- The runner runs every unit after a finding, except that after
  `generator-exhausted` it draws no further generated unit. The report holds
  every finding up to `ParityLimits.max_reported` (default 100) and the total
  count.
- `ParityRefusal` stops the run before any unit runs, with no verdict:
  `empty-scope` when no non-interpreter backend is supplied; an unreadable
  corpus; an unknown option of `cargo xtask parity`. Cancellation stops the
  run with no verdict.
- To the runner, an unreadable corpus is a document-level failure and nothing
  else: `complete-v1.json` is absent, cannot be read, is not JSON or has no
  `vectors` array, or a vector has no id or two vectors share one. The runner
  does not repeat FR-353's corpus preflight, which governs `qualify` before
  any part runs. A problem confined to one vector never refuses the run
  (FR-350 Isolation); for a selected vector it is an unbuildable case. A
  reference `prepare` limit or fault and generator exhaustion are findings,
  never refusals.
- `make parity` exits 0 when the report has no finding, 1 when it has any,
  and 2 on a `ParityRefusal`, the exit convention of FR-353.
- `make ci` lists `parity` as a prerequisite in the change that adds
  `fsl-jit` to the workspace.

## Inputs

The backends with their optional coverages; the corpus location, seed and
count; `ParityLimits`; `&Cancel`.

## Outputs

`ParityReport`: per backend, the compared count, the unsupported entries and
the findings; the count of corpus vectors not selected as parity cases; and
the unbuildable vectors, each with its id and cause, with their count; or
`ParityRefusal`.

## Behavior

- The runner shall compare every entry each backend prepares with the
  reference's result for the same entry, arguments and limits, over the
  corpus and the generated units.
- When a backend result differs from the reference's in any compared member,
  the runner shall record a `difference` naming the member.
- When a backend's two calls with equal inputs return different results, the
  runner shall record `nondeterministic`.
- If a backend lists unsupported with an `Untranslatable` cause an entry
  whose constructs all lie in the coverage it supplied, then the runner shall record `coverage-gap`.
- If a backend's `prepare` fails for a unit the reference prepared, then the
  runner shall record `prepare-failed`.
- If a backend compares no entry, then the runner shall record
  `empty-comparison`.
- If the interpreter returns `Fault` for an entry, from a call or from
  `prepare` of its unit, then the runner shall record `reference-fault` for
  that entry.
- If the interpreter's `prepare` of a unit returns `PrepareFailure::Limit`,
  then the runner shall record `reference-prepare-limit` for each entry of
  that unit.
- If checking refuses more than `max_discarded` generated draws, then the
  runner shall record `generator-exhausted` and draw no further unit.
- If a selected corpus vector is unbuildable, then the runner shall list it
  with its vector id and cause, count it and run every other case.
- If a path in a selected vector's `input.files` is absolute or
  parent-traversing, then the runner shall not read that path.
- If an unbuildable vector expects no refusal, then the runner shall record
  `unbuildable-case`.
- If an unbuildable vector's cause is not `Uncompilable`,
  `RefusedObservations`, `UnknownFunction` or `RefusedArgument`, then the
  runner shall record `unbuildable-case`.
- When a selected corpus vector's call supplies fewer arguments than its
  function declares, the runner shall compare both sides' results for the
  supplied prefix.
- If the run is cancelled while a case is built, then the runner shall return
  no report.
- If the corpus has a document-level failure, then the runner shall refuse the
  corpus as unreadable before running any unit.
- If no non-interpreter backend is supplied, then the runner shall refuse
  with `empty-scope` before running any unit.
- The runner shall offer no setting that allows, skips, retries or tolerates
  a finding.
- `make parity` shall exit 0 with no finding, 1 with any finding and 2 on a
  refusal.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-563-AC-1 | A test backend that delegates to the interpreter passes the gate over the corpus and the default generated units: the report has no finding and a compared count above zero, and the report's exit code is 0. | Test (TC-1386) |
| FR-563-AC-2 | Each of eight test backends that differs from the interpreter in exactly one member on one entry (`Ok` against `Err`, outcome kind, completed value, typed cause, `Location`, loss records, `Incomplete` charge point, `CallFailure` payload) fails with one `difference` naming that member and carrying the seed, unit index, source bytes, entry, arguments and both results; a backend returning `Fault` on one entry fails with a `difference`. | Test (TC-1386) |
| FR-563-AC-3 | A test backend whose result alternates between two values on repeated calls fails with `nondeterministic` naming the entry and arguments. | Test (TC-1386) |
| FR-563-AC-4 | A test backend whose coverage lists `let` but which lists every entry using `let` unsupported fails with one `coverage-gap` per such entry; the same backend with `let` absent from its coverage passes, listing those entries with their `UnsupportedEntry`. | Test (TC-1387) |
| FR-563-AC-5 | A test backend whose `prepare` returns `PrepareFailure::Limit` for one generated unit fails with `prepare-failed` naming that unit; a backend that lists every entry unsupported and supplies no coverage fails with `empty-comparison`. | Test (TC-1387) |
| FR-563-AC-6 | Under `test-support`, a reference that returns `Fault` for one entry gives one `reference-fault` finding naming the entry, exit 1; a run with no non-interpreter backend refuses `empty-scope` before any unit runs, exit 2; a corpus location that does not exist refuses, exit 2. | Test (TC-1387) |
| FR-563-AC-7 | A backend with 150 `difference` findings yields a report holding 100 findings and the total 150 at the default `max_reported`, and the run reaches every unit. | Test (TC-1386) |
| FR-563-AC-8 | `run_parity`'s inputs and `cargo xtask parity`'s options hold no member that allows, skips, retries or tolerates a finding, and none selects a reference; `run_parity_with_reference` does not exist without the `test-support` feature (a `compile_fail` doctest with a compiling control under the feature); `cargo xtask parity --no-such-option` and `cargo xtask parity --max-discarded 1` each refuse, exit 2, naming the option; no option or environment variable of `cargo xtask parity` or `cargo xtask qualify` sets a `ParityLimits` member. | Test (TC-1387) and Inspection |
| FR-563-AC-9 | Over a corpus fixture holding a `runtime`/`run` vector for `function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }` with the argument assignment `x` = 4, a `compile` vector and a `format` vector, the runner builds exactly one parity case, id the `run` vector's, entry `inc`, one tuple `(4)`; the report counts two vectors not selected and no finding for them; a test backend that returns 6 for `inc(4)` fails with one `difference` carrying that vector id and the tuple `(4)`. | Test (TC-1386) |
| FR-563-AC-10 | Over fixture corpora each derived from one valid corpus by a single document-level defect (`complete-v1.json` absent; a directory in its place; not JSON; no `vectors` array; a vector with no id; two vectors with one id), `run_parity` refuses the corpus as unreadable, exit 2, and calls no backend's `prepare`; over the valid corpus it returns a report. | Test (TC-1387) |
| FR-563-AC-11 | Over a corpus with no document-level defect that holds AC-9's `inc` vector and sixteen more `runtime`/`run` vectors, each expecting a value and each with one defect (an absent source file, listed first; an absent call file; a parent-traversing and an absolute source path, each to an existing valid source outside the corpus directory; no call file; two call files; a call file that is not JSON; a call file with an unknown member; an `object-context` file that is not JSON; an integer argument whose `decimal` is not an integer; two arguments for one parameter; a source that does not compile; a `model` file that is not a model document; a well-formed state snapshot that admission refuses; a call naming a function the source does not declare; an argument naming a population that its model declares `extent: open` and its snapshot lists, which binding admission refuses `incomplete_population`), a run with two delegating backends and generated-unit count 3 does not refuse: the report lists exactly those sixteen by vector id with, in that order, causes `UnreadableFile` naming its path (twice), `UnsafePath` naming its path (twice), `MissingMember`, `DuplicateFile`, `InvalidCall` (twice), `InvalidObservations`, `UndecodableArgument` naming position 0, `ExtraArgument` naming position 1, `Uncompilable` (twice), `RefusedObservations`, `UnknownFunction` and `RefusedArgument` naming position 0, counts 16, counts none of them not selected, gives each backend exactly sixteen findings, all `unbuildable-case`, one per vector, each with the seed, its vector id and cause, no arguments and no result (the non-compiling source's finding with that source's bytes and the called function, the absent source's with no source bytes and no entry), compares `inc` and the three generated units, and exits 1. | Test (TC-1387) |
| FR-563-AC-12 | AC-11's corpus with each of the sixteen defective vectors expecting a refusal (`expected.disposition` `refusal`) lists and counts the same sixteen, gives each backend exactly eleven findings, all `unbuildable-case`, for the vectors whose causes are `UnreadableFile`, `UnsafePath`, `MissingMember`, `DuplicateFile`, `InvalidCall`, `InvalidObservations`, `UndecodableArgument` and `ExtraArgument`, and exits 1; with only `inc` and the other five, each expecting a refusal, it lists and counts the five, holds no finding and exits 0; among those five causes, an unbuildable vector of class `refusal` that expects a value is an `unbuildable-case` finding, and one of class `positive` that expects a refusal is not; a vector expecting a refusal whose source does not compile and whose integer argument is malformed is `UndecodableArgument`, one naming two source files of which one does not compile is `DuplicateFile`, and each is a finding; a vector that expects a refusal and builds is compared, and a backend altering its result fails with `difference`; every internal fault of a build step is `Fault`, which no expected refusal excuses. | Test (TC-1387) and Inspection |
| FR-563-AC-13 | For one seed and count, let N, above zero, be the draws a run with `max_discarded` at `u64::MAX` discards. With `max_discarded` N the run reaches the count with no finding; with N − 1 each of two delegating backends holds one `generator-exhausted` finding carrying the index and source bytes of the N-th refused draw and no entry, arguments or result, the report's unit count falls short of a full run's, and the run exits 1 without refusing. `ParityLimits::default().max_discarded` is 10,000. | Test (TC-1387) |
| FR-563-AC-14 | Under `test-support`, with two delegating backends over a corpus of two `run` vectors, a reference whose `prepare` returns `PrepareFailure::Limit` for the first vector's unit alone gives each backend one `reference-prepare-limit` finding naming that vector, its entry and the limit's setting, compares the second vector, and exits 1 without refusing; the same limit for one generated unit of two or more entries gives each backend one such finding per entry; each such finding carries its entry's first argument tuple and no result; a reference whose `prepare` returns `Fault` for the first vector's unit alone gives each backend one `reference-fault` finding for its entry and compares the second. | Test (TC-1387) |
| FR-563-AC-15 | A `runtime`/`run` vector that expects a value and whose call supplies one argument for a function of two parameters is not listed unbuildable: each side is called with the one-value tuple; a delegating backend returns the reference's `CallFailure::Input` arity refusal (declared 2, supplied 1), the call is counted compared and there is no finding; a backend returning a completed value for it fails with one `difference` naming `Ok` against `Err` and carrying the one-value tuple. | Test (TC-1387) |
| FR-563-AC-16 | Over a corpus whose one vector's source does not compile, with generated-unit count 0 and a `Cancel::observing` handle that is cancelled at its first charge, which falls while that vector is built, `run_parity` returns the cancellation refusal and no report, and calls no backend's `prepare`. | Test (TC-1387) |

## Dependencies

- ADR-029 EB-3, EB-5: the reference and the parity evidence.
- [FR-294](FR-294-run-every-execution-backend-behind-one-seam.md), [FR-296](FR-296-select-the-execution-backend-in-the-request.md): the seam and the supplied backends.
- [FR-295](FR-295-return-interpreter-outcomes-from-jit-and-aot.md): the compared members, the corpus and the generator.
- [FR-560](FR-560-translate-the-jit-coverage-set-and-report-every-other-construct-untranslatable.md): the JIT coverage.
- [FR-350](FR-350-run-the-conformance-corpus-against-fsl.md): reading the corpus in place.
- [FR-100](FR-100-run-a-named-function-through-the-spine.md): the `run` operation whose vectors become parity cases.
- FSpec FR-306-AC-1: parity over the conformance corpus and generated
  programs.

## Overlap

`xtask parity` runs `run_parity` over `fsl-aot`, an FSL crate, and gates the
AOT backend on it, constructing it with the fixture generator of `fsl-conformance` and
the toolchain the run names (ADR-029 EB-6); the gate never calls the driver's or CG's `generate` (ADR-011 FB-11). The driver depends on `fsl-conformance` for the CLI half of the
matrix and for the real-generator AOT parity run: the driver repository owns that run, composing `fsl-aot` with its real generator and comparing it with the interpreter over the same parity cases (the driver's requirement tracked as IR-1540). FSL's own run, with the fixture generator, qualifies the backend's build-and-run harness, and the two runs together satisfy FR-295 and ADR-029 EB-5 for AOT.

## References

- QSL-393 (V1-A06a).
