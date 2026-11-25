---
id: SR-3112004
title: "spec-review of QSL-3112 parity unbuildable cases and refusals (filament-spec-language PR 3887)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR 3887 head 993d45edabf9f133c25f16dcf6d46854f4c372f3; spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md; spec/functional/FR-567-qualify-fsl-on-the-final-path-the-driver-calls.md; spec/functional/FR-295-return-interpreter-outcomes-from-jit-and-aot.md; spec/test-cases/TC-1387-the-parity-runner-fails-coverage-gaps-and-refuses-without-an-oracle.md; spec/test-cases/TC-1396-qualify-covers-the-corpus-the-lifecycle-matrix-and-parity.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-563
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-567
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-295
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-350
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-353
    type: references
---

# Spec review of QSL-3112 parity unbuildable cases and refusals (filament-spec-language PR 3887)

## Summary

Ticket: QSL-3112. One spec-review pass over `git diff origin/main...HEAD` of
PR 3887 at head 993d45edabf9f133c25f16dcf6d46854f4c372f3 (FR-563, FR-567,
FR-295, TC-1387, TC-1396). Checked against FR-350 (Settlement, Isolation,
adapters), FR-353 (preflight, exit contract), FR-560, FR-565, FR-295, FSpec
FR-336 (filament-specification origin/main 691ac5a6: call input file, tagged
input value, path rules) and the lead's rulings: parity's unreadable-corpus
refusal is document-level only; a per-vector problem is listed and counted,
never dropped, an unsafe path never read; any defect FR-350 settles as
`tool-failure` is always an `unbuildable-case` finding, even when the vector
expects a refusal; only refusals FSL itself produces are exempt when
`expected.disposition` is `refusal`.

FR-350's settlement was read in the spec and in the runner that implements it
on main (`xtask/src/qualification/adapters.rs`: `Failure::decode` errors become
`VectorOutcome::ToolFailure`; owner admission refusals become
`Partition::Result`). PR 3800 (`fsl-conformance/src/parity.rs`
`build_corpus_case`, `excused_by_expected_refusal`) was read only for
feasibility. `quire validate` over the five changed files exits 0 (quire
0.52.1, engine 0.62.2). No cargo or Python was run.

Methods: spec-review (this file). `code-review` not run: the diff holds no
code. Criterion-strength analysis (Jev) not run; criterion failability was
checked by hand.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `InvalidCall` (call file not JSON) is in the exempt set: the rule at lines 101-104 makes only `UnreadableFile`, `UnsafePath` and `MissingMember` always findings, and AC-12 and TC-1387 step 7 expect no finding for the `InvalidCall` vector when it expects a refusal. A call file is a corpus-owned `$defs.CallInput` document (FSpec FR-336); one that is not JSON is "not a valid document of its kind", a FR-350 `tool-failure` with cause `corpus-decode` (FR-350 Settlement; main's runner validates it through `document(call_bytes, schema)`). Per the lead ruling it must always be an `unbuildable-case` finding. Move `InvalidCall` into the always-finding set in the description, the Behavior bullet at lines 214-215, AC-12 and TC-1387 step 7. | spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md:86 |
| FND-002 | high | `UnadaptableArgument` conflates two FR-350 settlements and AC-11/AC-12 exempt the wrong one. AC-11's fixture, an integer argument whose `decimal` is not an integer, is a malformed decimal that FSpec FR-336-AC-6 says the schema refuses; main's runner settles it as `Failure::decode` at `/arguments/<i>`, a `tool-failure`. So this vector must be a finding even when it expects a refusal, but AC-12 counts it among the exempt seven. Only an owner admission refusal of a well-formed value (for example a population binding refused by FR-084/FR-106 admission, which main's runner settles as an FSL result) is an FSL-produced refusal. Split the cause, e.g. `UndecodableArgument` (corpus-decode: malformed tagged value, unknown tag, unresolvable reference node id, unadmitted population name; always a finding) and `RefusedArgument` (owner admission refusal; exempt when a refusal is expected). PR 3800 maps every `corpus_input_value` error to one cause, so it needs the same split. | spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md:93 |
| FND-003 | high | `RefusedObservations` conflates a corpus-decode defect with FSL's admission refusal, and AC-11/AC-12 exempt the decode case. AC-11's fixture is an `object-context` file that is not JSON. Main's FR-350 runner settles an object-context or population file that is not an identified snapshot as `Failure::decode` ("object context is not an identified snapshot"), a `tool-failure`; only `observation::AdmissionFailure::Refused` (FR-084/FR-089/FR-106 admission) is an FSL result, and `AdmissionFailure::Fault` is a `tool-failure`. So the non-JSON object-context vector must be a finding whatever it expects. Split the cause (e.g. `InvalidObservations`, always a finding, and `RefusedObservations`, the owner's typed refusal, exempt when a refusal is expected), and give AC-12 a fixture for the refused kind (a well-formed snapshot that admission refuses). PR 3800 maps every `admit_corpus_object_environment` error to one cause. | spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md:90 |
| FND-004 | high | `ArgumentCount` with more arguments than parameters is a FR-350 `tool-failure`, but it is exempt. FSpec FR-336 (Call input file) says an item with no declared parameter is an input-adapter refusal, and main's runner settles it as `Failure::decode` at `/arguments/<i>` ("argument has no declared parameter"), i.e. `tool-failure`. AC-11's fixture ("two arguments for one parameter") is exactly that case, and AC-12 counts it exempt. It must be an always-finding cause. | spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md:92 |
| FND-005 | medium | `ArgumentCount` with fewer arguments than parameters contradicts FSpec FR-336: "A shorter array remains a prefix call and the typed operation settles the unbound declared parameters." FR-350 (main's runner) runs such a vector through `spine::run` and settles FSL's typed result. FR-563 makes it unbuildable and a finding unless a refusal is expected, so if FR-100 settles a prefix call with a value outcome, a vector that passes the `corpus` part fails the `parity` part. State how a prefix call is handled: either build it as a parity case with the prefix tuple, if the seam allows, or state that it is settled by FSL's own typed result and classify it with the FSL-produced refusals (exempt when a refusal is expected), with an AC. | spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md:92 |
| FND-006 | medium | No cause excludes an FSL internal fault, which FR-350 settles as `tool-failure` ("panics, returns an internal fault"). `Uncompilable` ("do not compile") and `RefusedObservations` read as covering a compile or admission `Fault` too, which would then be exempt when a refusal is expected (main's runner maps `Category::InternalFailure` and `AdmissionFailure::Fault` to `ToolFailure`; PR 3800 maps compile `Err(())` wholesale to `Uncompilable`). State that an internal fault while building a case is always an `unbuildable-case` finding (or its own cause), and that only a typed refusal is exempt. Preferably replace the fixed exempt list with the rule itself: a cause is exempt only when FR-350 would settle the vector by an FSL typed refusal, never when FR-350 settles it `tool-failure`. | spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md:101 |
| FND-007 | medium | New behaviour with no AC that can fail: "Cancellation while a case is built stops the run and makes no vector unbuildable" (lines 98-99). No AC of FR-563 cancels during case building, so an implementation that records the vector being built as unbuildable (or keeps going) passes every AC. Add a clause to AC-11 or AC-14 (cancel during the build of a vector: no verdict, no unbuildable listing for it). | spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md:98 |
| FND-008 | low | Several new report-shape rules are asserted by no AC: an `unbuildable-case` finding carries its cause, no arguments and the source bytes and entry only as far as read (lines 141-143); `reference-prepare-limit` carries the entry's first argument tuple (AC-14 checks vector, entry and setting only); `generator-exhausted` has no entry or arguments; and `cargo xtask parity` has no option or environment variable setting `max_discarded` (lines 151-153; FR-567-AC-7 covers only `qualify --count`). Add these to AC-11, AC-13, AC-14 and AC-8. | spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md:138 |

## Verdict

Changes requested (head 993d45edabf9f133c25f16dcf6d46854f4c372f3): FND-001 to
FND-004 are high and contradict FR-350 and the lead's ruling, because four of
AC-11's twelve fixtures (`InvalidCall`, the malformed-decimal
`UnadaptableArgument`, the non-JSON object-context `RefusedObservations`, the
extra-argument `ArgumentCount`) are FR-350 `tool-failure`s that AC-12 exempts.
After the fix, AC-12's refusal-expecting run should give nine findings per
backend (two `UnreadableFile`, two `UnsafePath`, `MissingMember`,
`InvalidCall`, the undecodable argument, the invalid object context, the extra
argument), and the exempt set should be FSL typed refusals only (`Uncompilable`
twice, `UnknownFunction`, plus new fixtures for a refused object environment and
a refused argument). Update the TC-1387 step 7 expectations to match.

Verified clean: parity's unreadable-corpus refusal is document-level only
(lines 162-169) and FR-567 keeps FR-353's full preflight governing `qualify`,
consistent with FR-353 and with main's preflight, which refuses an absolute or
non-normal input path (so FR-567-AC-8's parent-traversing case refusing exit 2
is correct). Per-vector problems are listed, counted, not counted as not
selected, and never refuse the run (FR-350 Isolation); the unsafe path is never
read (Behavior bullet and AC-11's paths to existing files outside the corpus
make that failable). `UnreadableFile` and `MissingMember` always being findings
agrees with FR-350 and FR-353 ("a missing file named by an otherwise valid
vector is the per-vector tool-failure"). Nothing is weakened: AC-1 to AC-9 are
unchanged, AC-6's reference-fault, empty-scope and missing-corpus refusals
stand, and the no-skip rule is kept, with the expected-refusal exemption stated
as the corpus oracle rather than a setting and the exempt vectors still listed
and counted. The expected-disposition test uses FSpec FR-336's closed
`value`/`refusal` vocabulary and ignores class, code and cause, per the ruling.
`generator-exhausted` (FR-295 edit, AC-13 with N and N - 1),
`reference-prepare-limit` and prepare-time `reference-fault` (AC-14) are
failable and recorded for every backend. FR-567-AC-7 reuses no retired id
(FR-567-AC-6 was deleted earlier). FR-560 and FR-565 are untouched and not
contradicted. `quire validate` exits 0.

## Coverage

- FR-563 (examined)
- FR-563-AC-6 (examined)
- FR-563-AC-8 (examined)
- FR-563-AC-10 (examined)
- FR-563-AC-11 (examined)
- FR-563-AC-12 (examined)
- FR-563-AC-13 (examined)
- FR-563-AC-14 (examined)
- FR-567 (examined)
- FR-567-AC-7 (examined)
- FR-567-AC-8 (examined)
- FR-295 (examined)
- TC-1387 (examined)
- TC-1396 (examined)
- FR-350 (context_only)
- FR-353 (context_only)
- FR-560 (context_only)
- FR-565 (context_only)
- FSpec FR-336 (context_only)
