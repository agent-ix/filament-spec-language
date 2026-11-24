---
id: TC-3084
title: "Clock-place references and presence reads check and lower in population forms"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1040
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: verifies
---
# TC-3084: Clock-place references and presence reads check and lower in population forms

## Description

Verify that `ref(p)` and `present(p)` over clock places check at S3 and lower
at S4 to FSpec FR-829's place-reference application and FSpec FR-322's
presence read, per instance under a population binder and through an object
reference, and that the remaining wrong forms refuse with typed causes.
Scope: FR-1040-AC-1 to FR-1040-AC-12.
Step 7 also verifies FR-230-AC-43's exact encoding vector and
FR-230-AC-49's tagged nested-collection indices.
Step 8 verifies FR-230-AC-71's kernel value form on both routes.

## Test Procedure

Fixture: FSpec FR-415-AC-15's `Call` model under `time dense unit ms`, with
the fields `v: Option<Clock>`, `q: Sequence<Option<Clock>>[0, 2]`,
`ts: Sequence<Timer>[0, 2]` (`Timer { x: Clock; n: Int }`), `m: Map<Text, Timer>[0, 2]`, `peer: Reference<Call>` and
`n: Int[0, 9]` added, an operation
`pick(i: Int[0, 1], k: Text)` of `Call` whose precondition holds the forms of
FR-1040-AC-5 and the `Map` form of FR-1040-AC-6, a population `calls`, and a
refinement whose subject admits `calls = {c1, c2}`; the other forms sit in
refinement rows.

1. Compile the `ref` forms of FR-1040-AC-1 and FR-1040-AC-2 and read the
   checked and emitted nodes.
2. Compile and evaluate the population forms of FR-1040-AC-3 and
   FR-1040-AC-4.
3. Compile the presence variants of FR-1040-AC-5.
4. Compile each negative form of FR-1040-AC-6.
5. Compile FR-1040-AC-7's function and its caller, then the ill-typed
   variant.
6. Admit FR-1040-AC-8's `acme/rpc` package with `Call.r` typed by the `reference` type over `Clock`, compile `point`, run it and read the emitted field type, the write and the state keys; retype `r` as `ix://filament/native/Clock`; compile FR-1040-AC-8's model unit and read its document.

7. Through both the source and bare checked routes, build the declared-place
   table and evaluate actual `ref` terms for: a record `W { c: Clock }`
   exposed through a type alias; a nested tuple with paths `(0)` and
   `(1, 0)`; union cases `A` and `B` with clock payloads; and a sequence
   clock slot at indices 0 and 1. Use corresponding authenticated
   declarations and compare source raw `pN` with selected `tuple/N` shapes.
   Repeat with two authenticated populations holding the same owner local
   key. Import one authenticated declaration through two aliases as a
   duplicate-view control. Supply two distinct declaration origins whose
   normalized full path and type/model scope coincide as a negative table
   construction fixture for each route, retaining both origins. For the
   independent-document control, use admitted qualified or selected-document
   forms with equal short names but different authenticated owner scopes;
   require distinct identities. For union expansions use distinct case/path
   prefixes as the distinct-identity control, and supply distinct origins
   with deliberately equal canonical scope and full path to the negative
   table fixture. For inheritance, use `Base { c: Clock }`, `Child extends
   Base {}` and `time Base dense unit ms`: access the same owner instance's
   inherited `c` through the original `Base` template. For authenticated
   source authority `A` and identity `I`, require the exact components
   `S = ["source","A","I","ix://I/Base"]`,
   `T = [["field",["source","A","I","ix://I/Base"],"ix://I/Base/c",["clock"]]]`
   and `J = []` on both routes, retaining the same actual `P` and `K` for
   the same owner. As earlier-refusal
   controls, redeclare source field `c` on `Child` (FR-569:
   `ambiguous_declaration`/`ambiguous-name` at `Child.c`), and
   select the legal package field redefinition `Child/d -> Base/c` with
   both fields typed `Clock` under `time M` (two candidate object model
   nodes, FR-230-AC-38: `ambiguous_declaration`/`ambiguous-name` at `M`
   in the `time M` declaration). Neither control supplies a redefined
   canonical table. Keep the distinct-origin/equal-scope/equal-path negative
   table fixture separate. Separately compile two source units each declaring a
   literal record `W`: require FR-480-AC-2's earlier
   `ambiguous_declaration`/`ambiguous-name` at the repeated declaration,
   naming both declarations and their units, not entry into canonical-table
   construction. Exercise an ambiguous short-name ownership lookup on
   each route: require `ambiguous_declaration`/`ambiguous-name` at that
   lookup, naming both competing origins and producing no place table.
   Bind these cases to FR-1040-AC-9.
   Also encode the exact delimiter-bearing vector of FR-230-AC-43 and
   require its literal text, not a comparison of two copies of one encoder.
   Assert the typed path and tagged-index differences of FR-230-AC-49,
   including two nested collections. TC-464 step 10 owns malformed-text
   and re-added-kind raw admission, including the noncanonical decimal
   `pathPosition` spelling `"01"` of FR-230-AC-45. Bind the literal vector
   control to `FR-230-AC-43` and the nested-collection controls to
   `FR-230-AC-49`, additionally to the `FR-1040-AC-9` cases above.

8. Through both the source and bare checked routes, admit TC-464 steps 7
   and 8's `Call` snapshot and evaluate `ref(self.x)`; read the kernel
   value of `r`, `maybe`, `refs` and the `ref` result, compare each text
   with the snapshot `key` it was admitted from, and re-emit each value as a
   snapshot reference. Then admit, one at a time, `r` with a `key` that has
   one uppercase hex letter, one with prefix `place/v1`, one whose `J` is
   `[["slot","0","0"]]` over a path with no collection step, one whose
   `S` names another authenticated declaration scope, one whose `P` names
   another population declaration, and the stored `x` identity after `x` is
   removed. Bind these cases to `FR-230-AC-71`.

## Expected Results

- Steps 1, 2 and 5: each form checks and lowers to the nodes stated, with no
  `unsupported-feature` or `not-yet-implemented` refusal, and evaluates as
  stated.
- Steps 3 and 4: each form refuses with the code, cause and locus stated, or
  checks where the criterion says it does.
- Step 6: `r` is a `Reference<Clock>` data member typed by the `clock_reference` node, the write checks and lowers with no `clock-write`, the post-state holds the place id of `self.x` and clock-only changes keep one state key; the clock-typed `r` refuses `clock-write`; the model unit's document holds `ix://acme/rpc/Reference_Call_r`.

- Step 7: the routes give equal full `place/v2` references for each same
  declared place; different tuple paths, union cases, sequence indices and
  populations give distinct references. Independent admitted documents with
  different authenticated owner scopes also give distinct references despite
  equal short names. Duplicate import views of the same authenticated
  declaration and canonical normalized declared path succeed as one view. Each deliberately
  equal-scope/equal-path distinct-origin table fixture refuses
  `conflicting_declaration`/`duplicate-declaration`, names both declaration
  origins and produces no partial table or checked package. Ambiguous owner
  resolution refuses `ambiguous_declaration`/`ambiguous-name` at the
  ownership lookup, naming both competing origins and producing no table,
  without choosing a candidate. The inherited `Child.c`
  control uses the original `Base.c` template, retaining exactly the `S`,
  `T` and empty `J` specified above on both routes; it does not mint a
  `Child` template. The source
  field redeclaration and selected-package redefinition controls refuse
  earlier as `ambiguous_declaration`/`ambiguous-name` at `Child.c`
  (FR-569) and at `M` in `time M` (FR-230-AC-38), respectively, and are not
  canonical collision evidence. These particular fixtures do not prohibit
  other redefinitions already admitted by the language: any admitted
  redefining member retains its own original declared scope and path.
  The two-literal-`W`
  source control refuses earlier `ambiguous_declaration`/`ambiguous-name`,
  naming both declarations with their units (FR-480-AC-2), and is not
  canonical collision evidence. Raw observation round trips and the actual
  removed-`x`, retained-`y` safety regression are TC-464 steps 7 and 8.
  The concrete text equals FR-230-AC-43's literal on both routes. Nested
  slot selection changes only the corresponding `J` index; every outer
  entry, declared type scope and typed path is unchanged (FR-230-AC-49).
- Step 8: each value is a filament-exact clock-place reference typed
  `ValueType::ClockReference` with the population universe and the exact
  `place/v2` text, never an `ObjectReference`; its text equals the snapshot
  `key` byte for byte and re-emission gives the same `key`. The three malformed identities and the two foreign-scope
  identities refuse `invalid_runtime_input`/`invalid-value` at `r`; the
  removed-`x` identity refuses
  `dangling_reference`/`absent-target-in-complete-population`. Both routes
  agree.

Tests: `fsl-semantics/src/check/lowering/tests.rs`
`ref_of_a_clock_place_with_an_id_lowers`, `clock_place_presence_read_lowers`,
`a_call_to_a_function_without_a_key_yet_lowers`, and
`fsl-semantics/tests/it/refinement_populations.rs` for the population forms.

Implementation status: the existing tests listed above do not establish
step 7 or FR-1040-AC-9. Those STEP2 acceptance obligations remain unmet;
the same applies to TC-464's raw admission controls until implementation
and acceptance proof.

FR-1040-AC-10 to FR-1040-AC-12: evaluate `ref` values across states and universes, supply sets and maps in two entry orders with a repeated place id, and round-trip references through the state key and wire form; equality, canonical order, duplicate and dangling refusals and the round trip are as the criteria state.
