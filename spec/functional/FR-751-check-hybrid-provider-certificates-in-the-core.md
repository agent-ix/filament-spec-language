---
id: FR-751
title: "Check hybrid provider certificates in the core in exact interval arithmetic"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-750
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-450
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-652
    type: depends_on
---
# FR-751: Check hybrid provider certificates in the core in exact interval arithmetic

## Description

`fsl-replay` SHALL implement FSpec FR-652's check of the `inductive-enclosure`,
`flowpipe` and `validated-witness` certificates a hybrid provider returns,
in exact rational interval arithmetic over the `CheckedHybridModel`, so an
accepted certificate settles a hybrid proof `certified` and lets a trace
through a non-polynomial flow refute.

## Inputs

The checked hybrid model and claim, the provider's result entry and its
certificate, and the item's work limit.

## Outputs

`Accepted` or `Rejected { rule, at }`, and the settlement FSpec FR-652
"Settlement" gives.

## Behavior

- The checker SHALL evaluate every flow, guard, reset, invariant and claim
  atom by FR-450's interval evaluator, with no floating-point step, and SHALL
  read each over a box by FSpec FR-652 "Interval reading": Kleene's
  three-valued connectives, the definedness reading (an expression not
  definitely defined over a box fails the rule reading it), the one-pass
  contractor `Contract` (in the Picard check only for a contraction-admitted
  mode, the uncontracted tube otherwise, where every conditional in the
  rates must also be strictly decided over the box, as in rules `b` and
  `witness-unique`), the guarded part, the post image contracted by the
  target invariant, the tube and advance, and the boundary face decided from
  the invariant's variable-bound conjuncts. It never probes a point beyond a
  face and never settles an unknown reading by a midpoint, endpoint or
  sample.
- It SHALL read the certificate before any rule runs, by FSpec FR-652
  "Checking order" steps 1 to 3: the FSpec FR-331 schema, the form match by
  basis type (never by `method`), and every structural refusal FSpec FR-652
  "Structural refusals" lists, building the dwell tree of a `flowpipe` from
  each jump step's `from`. A failure is a certificate refusal carrying its
  code (`wrong-value-kind`, `invalid-value`, `missing-member` or
  `unknown-member`) and pointer: FR-750's `read_entry` refuses the entry
  `invalid_result` at that pointer, and FR-098's replay of a request refuses
  it `invalid_runtime_input` at `/replay/certificate/<member>`. A refusal is
  never a rejection and never an internal fault. A fault of the checked
  model, such as a model index the package lacks, settles by FR-750 "Replay
  refusals" and is never a rejection either.
- It SHALL apply FSpec FR-652's rules rule by rule in table order, each over
  every element it applies to in the stated element order, and report the
  first failing rule and the pointer of the box or step it failed at,
  `Rejected { rule, at }`. `rule` is a closed type of the 17 FSpec FR-652
  "Rule names" spellings, and `at` is rooted at `/certificate` on the
  analyze path and on a replay request alike.
- It SHALL charge each evaluation of one expression over one box, the
  unbounded box of a hull included, to the item's work meter; a check the
  meter stops is `Rejected { rule: work-limit, at }`, `at` the pointer the
  running rule would name for the element being checked, and is never
  `Accepted`.
- An accepted certificate SHALL set the proof's certification to
  `Certified`; a rejected one SHALL keep FR-250's certification and record
  `certificate-rejected`.
- The checker SHALL accept a `validated-witness` only when it has a decisive box (FSpec
  FR-652 "Readings and the decisive box"): a reading the interval evaluator
  returns as unknown, as over a box that straddles an atom's boundary,
  never refutes and is never resolved by a midpoint, endpoint or sample.
  An accepted witness SHALL return its decisive reading (its position, its
  instants, the `box` or `B` it reads, and its kind, `Formula` or
  `UndefinedEvaluation{where, cause}`), which FR-450's kind agreement and
  the trace's `end` are compared with.
- An accepted `validated-witness` SHALL let FR-450 record the trace as the
  item's `HybridCounterexample` and settle `refuted` for a trace exact replay
  cannot decide.
- The checker SHALL run only on a certificate the settlement rests on
  (FR-750 "Dispatch"): the certificate of a `proved` record and the witness
  of a trace exact replay returns `NotExact` for. Every other certificate
  is read and recorded unchecked.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-751-AC-1 | FSpec FR-652-AC-1's two `inductive-enclosure` certificates over `Braking` without `t` are accepted, settling `Certified`, and rejected at rule (a) with the `Moving` box pointer, keeping `Uncertified`. | Test (TC-2101) |
| FR-751-AC-2 | A `flowpipe` step violating the Picard condition is rejected at that step's pointer; FSpec FR-652-AC-3's non-polynomial trace with an accepted witness settles `refuted` and is recorded; with either of its witnesses whose boxes only straddle `v = 30` it stays `inconclusive`, the witness `Rejected { rule: witness-violation }` at its last step. | Test (TC-2101) |
| FR-751-AC-3 | A check given a work limit of one box evaluation is `Rejected { rule: work-limit }`, never `Accepted`. | Test (TC-2101) |
| FR-751-AC-4 | The `inductive-enclosure` rules give exactly the outcomes of FSpec FR-652-AC-7 (`Rise`: accepted, `a` and `b` at `/certificate/boxes/Up`) and FSpec FR-652-AC-8 (`c` at `/certificate/boxes/Stopped`, `d` at `/certificate/boxes/Moving` for `h.v < 10` and for the undefined `1 / h.v`, `d` at `/certificate` for the nested `eventually`, and over `Hold` accepted against `always not holds(h.v < 30)` and `d` at `/certificate/boxes/H` against `always not holds(h.v >= 30)`). | Test (TC-2101) |
| FR-751-AC-5 | The `flowpipe` rules give exactly the outcomes of FSpec FR-652-AC-5 and FSpec FR-652-AC-9 to FR-652-AC-14, each with the stated rule and `at`: `initial` for the narrow first box and the missing initial `Stopped` dwell, the contracted Picard inclusion accepted and `picard` and `successor` at the stated steps, `Fork`'s two jump steps accepted and `jump` at `/certificate/steps/4`, `/certificate/steps/6` and `/certificate/steps/10`, the `Tick` self-loop accepted only with `from`, `coverage` at `/certificate/steps/0`, `/certificate/steps/159` and `/certificate/steps/4` with `Drop` accepted, and `claim` at the stated steps for `not`, `implies`, the undefined atom and the `model-steps` reading; the AC-5 jump step with `time` `[0, 20]` and `step` 10 is accepted, a jump step's width never being compared with its `step`. | Test (TC-2101) |
| FR-751-AC-6 | The `validated-witness` rules give exactly the outcomes of FSpec FR-652-AC-15 and FR-652-AC-16: `witness-start` at `/certificate/dwells/0/0`, `witness-picard` at `/certificate/dwells/0/3` and, over `GrowLow`, at `/certificate/dwells/0/7`, `witness-violation` at `/certificate/dwells/0/8` for an `end` that differs from the decisive reading, `Shift` accepted and `witness-unique` at `/certificate/dwells/0/0`, `GrowJump` accepted with its decisive reading at position 1, time 4, and `witness-guard` at `/certificate/dwells/0/1` when the trace's jump time `[5, 5]` does not contain the witness's jump instant 4. | Test (TC-2101) |
| FR-751-AC-7 | Each structural mismatch of FSpec FR-652-AC-17 is a certificate refusal with the stated code and pointer, which `read_entry` refuses `invalid_result` and a replay request refuses `invalid_runtime_input` at `/replay/certificate/...`; none is a rejection, and none settles `failed` `internal_fault`. | Test (TC-2101) |
| FR-751-AC-8 | FSpec FR-652-AC-1's accepted certificate under a work limit of one evaluation is `Rejected { rule: work-limit, at: /certificate/boxes/Moving }`; FSpec FR-652-AC-3's nine-step witness under a work limit that stops its check is `Rejected { rule: work-limit }` at the step being checked, and its item settles as FR-450-AC-13 states. | Test (TC-2101) |

## Dependencies

- FR-450's interval evaluation and replay; FR-750's dispatch; FSpec FR-652.
