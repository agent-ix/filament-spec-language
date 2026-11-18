---
id: FR-940
title: "Analyze a checked package through one sealed read view over source-built and byte-read packages"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-087
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-531
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
---
# FR-940: Analyze a checked package through one sealed read view over source-built and byte-read packages

## Description

When an analyze request selects a `package` (FR-850), the analysis engines
(FR-281) SHALL read the package they analyze only
through `fsl_package::AnalysisView`, a sealed read trait implemented for
exactly two types: `AdmittedPackageRead`, the strict reader's admitted
read-back of an in-process `CheckedPackage`'s own S4 emission, and the
`BarePackage` (FR-531) over a `CheckedPackageV2` the strict reader admitted
from bytes. Both hold an admitted `CheckedPackageV2`; a `CheckedPackage`
holds none (its emission is an optional `OnceLock<Emission>` of bytes), so it
reaches analysis only through its admitted read-back. An analyze request that selects a `package` (FR-850)
analyzes its `BarePackage` through this view. A request that selects a
`program` analyzes the compiled `CheckedPackage`: each fact of FR-531's table
that an engine reads is read from the `AdmittedPackageRead` of its own S4
emission (`read_evaluation_package`) through the same view, and each
evaluator that FR-531's capability inventory lists as taking a
source-checked clause, node or state clause reads that value from the
compiled package's `CheckedGraph`. No value read from bytes becomes a
`CheckedPackage`.

### Decision: the selected package governs the package route, source analysis keeps its checked graph

FR-531 states that the model-check evaluators (`evaluate_holds`,
`evaluate_clause`, `evaluate_value_invariants`, `check_separation`,
`evaluate_frame` and the others of its inventory's source-checked row) take
a source-checked value that the bytes do not carry. Requiring every engine
to read only through the view on both routes would leave the source route
with no way to call them, so an explicit-state, invariant, weak-memory or
frame item over a program would settle `unsupported` for a fact the bytes
lack while its source is in hand. Analysis from source is the primary use of
`analyze` and is never narrowed. The view therefore governs the route that
has only bytes: a `package` selection, where the bytes are the whole input.
The source route keeps the compiled `CheckedGraph` for the evaluators that
need it and reads every byte-table fact through the view, so the facts both
routes share still have one value. The package route reaches the same
evaluators through their package forms ("Package forms of the source-checked
evaluators" below), so FR-850's equality with the source route holds for
every item family. An evaluator reads package bytes only when a package is
selected and the strict reader admitted it; a selection the reader refuses is
refused by FR-850 "Admission" before any engine runs, and never falls back to
source.

### Decision: a typed read view, not an exception to R-10

FR-087 R-10 says checked typestate is built only by S3 and S4, so that every
`CheckedPackage` is a package that passed the checker. Two ways reconcile it
with analysis from package bytes: a narrow named constructor that turns an
admitted v2 package into a `CheckedPackage`, or a read view that the engines
take instead of `CheckedPackage`. The view is chosen.

A `CheckedPackage` holds its S3 `CheckedGraph`: the source arena, the
checked bodies and the source-side names and spans. The v2 bytes do not carry
these (FR-531 "What the bytes do not carry"), so a `CheckedPackage` built from
bytes would have to invent them or leave them empty. Either way it would be a
`CheckedPackage` that never passed S3, which is what R-10 exists to rule out,
and every later reader of a `CheckedPackage` would have to ask which kind it
holds. The view needs no such value. FR-531 already lists every fact
evaluation reads from a package's bytes and requires the in-process route to
read the same bytes column, so one value exists per fact. The view is that
table as a trait. The engines read only those facts, so analysis gives the same
result on both routes because both routes read the same bytes.

## Inputs

- On the source route: the `AdmittedPackageRead` that
  `read_evaluation_package` returns for the `CheckedPackage` S4 built for the
  request's program: its emission's bytes read back by the strict reader.
- On the package route: the `BarePackage` over the `CheckedPackageV2` that the
  strict checked-package reader admitted from the `checked-package.json` of
  the FR-517 handoff directory the request's `package` selection names
  (FR-850 "Admission"), with the request's domain packages bound to its model
  selections and, through `BarePackage::with_supply`, the identity rows
  `ReadIdentityRows::read` admitted from that directory's `identities.json`
  and the equality rows `ReadEqualityRows::read` admitted from its
  `equality.json` when present. These rows are the identity witness the
  reader projection (`BarePackage::world()`, the bare walker and observation
  admission) reads; they come from the handoff files only, never from the
  package bytes (FR-531).

## Outputs

```rust
// fsl_package, behind `std`
pub trait AnalysisView: sealed::Sealed {
    fn package(&self) -> &CheckedPackageV2;
    fn identity(&self) -> &LibraryName;
    fn model_selections(&self) -> Vec<DomainPackageRef>;
    fn dependencies(&self) -> DependencyClosure<'_>;
    // ... one accessor per fact row of FR-531 "What the entry derives from
    // the bytes", each reading `package()`.
}
impl AnalysisView for AdmittedPackageRead<'_> { /* reads `admitted()` */ }
impl AnalysisView for BarePackage<'_> { /* reads the admitted package */ }
```

`sealed::Sealed` is private to `fsl_package`, so no other type implements the
view. The engines take `&dyn AnalysisView` or a type parameter bounded by it,
and `FR-621`'s outcome document is unchanged.

## Behavior

- While a `package` is selected, every analysis engine, the default-item
  rule, the claim-name lookup, the subject binding and counterexample replay SHALL read
  the package only through `AnalysisView`, and no engine SHALL take a
  `CheckedPackage` or a `CheckedGraph`.
- While a `program` is selected, every engine SHALL read each fact of
  FR-531's table through the `AdmittedPackageRead`'s view.
- While a `program` is selected, an evaluator FR-531's capability inventory
  lists as taking a source-checked value SHALL take it from the compiled
  `CheckedPackage`'s `CheckedGraph`.
- While a `program` is selected, FSL SHALL NOT settle an item `unsupported`
  because a fact is absent from FR-531's table or from the package bytes.

### Package forms of the source-checked evaluators

- Each evaluator of FR-531's source-checked inventory row that an analysis
  engine calls SHALL have a package form that reads the clause, node or state
  clause it evaluates from the view: the formula operations, operand edges
  and `holds` atoms from `formula_graph()`, the guard, frame and invariant
  bodies from the admitted checked nodes, and the binders from the checked
  predicate view (FR-515). The package form SHALL return the value the source
  form returns for the package and the source it was compiled from.
- `AnalysisView` SHALL be implemented for `AdmittedPackageRead<'_>` and for
  `BarePackage<'_>` and for no other type; it SHALL NOT be implemented for
  `CheckedPackage`.
- The `AdmittedPackageRead` implementation SHALL read each fact from its
  `admitted()` package, never from its `source()` package's `CheckedGraph`,
  so the two implementations return equal values for a package and the bytes
  it was written to.
- No function SHALL convert a `BarePackage`, a `CheckedPackageV2` or any value
  read from bytes into a `CheckedPackage` or a `CheckedGraph` (FR-087 R-10).
- A fact an engine needs that FR-531's table does not list SHALL be added to
  that table, with the row of the v2 package that carries it, before an engine
  reads it on the package route, so the package route settles every item
  with the verdict, record, counterexample and replay the source route gives
  (FR-850 "Equality with the source route"). No package-route item SHALL
  settle `unsupported` for a fact or an evaluator the source route reads.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-940-AC-1 | For FR-850-AC-1's Counter package, each `AnalysisView` accessor returns equal values over the `AdmittedPackageRead` of the compiled `CheckedPackage` and over the `BarePackage` of the `checked-package.json` of its handoff directory, written by a `native-compile/1` request carrying the `handoff` member (FR-665), with that directory's identity and equality rows attached; an analysis entry called with either value gives the same item records for `Bounded`, `BelowThree` and `AtMostTwo`. | Test (TC-2620) |
| FR-940-AC-2 | `AnalysisView` has no implementation outside `fsl_package` (a `compile_fail` doctest implementing it for a local type fails with `E0277`, and one passing a `CheckedPackage` where `&dyn AnalysisView` is expected fails the same way), and no public function of any FSL crate takes a `BarePackage` or a `CheckedPackageV2` and returns a `CheckedPackage` or a `CheckedGraph` (source scan of every function returning either type). | Inspection (TC-2620) |
| FR-940-AC-3 | `fsl analyze` with a `package` selection no longer refuses `package` as not implemented: FR-850-AC-1's request completes with FR-850's outcome, exit 0. | Test (TC-2620) |
| FR-940-AC-4 | Over source, the Counter request with the items `Bounded`, `AtMostTwo` and the `invariant` state clause `NotAboveThree` (`self.value <= 3`, FR-625-AC-4) settles `proved`, `refuted` with `replay.reproduced` `true`, and `proved` basis `exhaustive` respectively, and the store-buffering litmus claim over `SB` with `memory` `tso` settles `refuted` (FR-629-AC-1); no record of either request is `unsupported` and no diagnostic names a package fact. | Test (TC-2958) |
| FR-940-AC-5 | The `package` request over the handoff directory of AC-4's Counter unit, written by its compile request's `handoff` member, gives, for `NotAboveThree` and `AtMostTwo`, item records equal to the source route's, `AtMostTwo` with the same counterexample, replay target and `reproduced` `true`. | Test (TC-2958) |
| FR-940-AC-6 | For each item family whose evaluator FR-531's inventory lists as taking a source-checked value (the temporal clauses `Bounded` and `AtMostTwo`, the invariant `NotAboveThree`, the deadlock-freedom item and the `SB` litmus claim under `tso`), a `program` request and a `package` request over the handoff directory of the same compiled unit give equal item records, counterexamples and replays, and no package-route record is `unsupported`; a handoff directory whose `checked-package.json` the strict reader refuses, or that holds no `identities.json`, settles no item and compiles no source (FR-850-AC-3, FR-850-AC-6). | Test (TC-2958) |

## Dependencies

- FR-087 R-10: checked typestate is built only by S3 and S4.
- FR-531: the `BarePackage` entry and the table of facts read from bytes.
- FR-850: the analyze request's `package` selection and its admission.
- FR-281: the analysis engines.
