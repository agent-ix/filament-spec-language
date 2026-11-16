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

The analysis engines (FR-281) SHALL read the package they analyze only
through `fsl_package::AnalysisView`, a sealed read trait implemented for
exactly two types: the in-process `CheckedPackage` that S4 builds from source,
and the `BarePackage` (FR-531) over a `CheckedPackageV2` the strict reader
admitted from bytes. An analyze request that selects a `package` (FR-850)
analyzes its `BarePackage` through this view; a request that selects a
`program` analyzes the compiled `CheckedPackage` through the same view. No
value read from bytes becomes a `CheckedPackage`.

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

- On the source route: the `CheckedPackage` S4 built for the request's
  program, with its `Emission` (the v2 package value S4 emitted for it).
- On the package route: the `BarePackage` over the `CheckedPackageV2` that the
  strict checked-package reader admitted from the request's `package` file
  (FR-850 "Admission"), with the request's domain packages bound to its model
  selections.

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
impl AnalysisView for CheckedPackage { /* reads its own S4 emission */ }
impl AnalysisView for BarePackage<'_> { /* reads the admitted package */ }
```

`sealed::Sealed` is private to `fsl_package`, so no other type implements the
view. The engines take `&dyn AnalysisView` or a type parameter bounded by it,
and `FR-621`'s outcome document is unchanged.

## Behavior

- Every analysis engine, the default-item rule, the claim-name lookup, the
  subject binding and counterexample replay SHALL read the package only
  through `AnalysisView`. No engine SHALL take a `CheckedPackage` or a
  `CheckedGraph` as its package input.
- `AnalysisView` SHALL be implemented for `CheckedPackage` and for
  `BarePackage<'_>` and for no other type.
- The `CheckedPackage` implementation SHALL read each fact from the v2 package
  value of its own S4 emission, never from its `CheckedGraph`, so the two
  implementations return equal values for a package and the bytes it was
  written to.
- No function SHALL convert a `BarePackage`, a `CheckedPackageV2` or any value
  read from bytes into a `CheckedPackage` or a `CheckedGraph` (FR-087 R-10).
- A fact an engine needs that FR-531's table does not list SHALL be added to
  that table, with the row of the v2 package that carries it, before an engine
  reads it; until it is, an item whose engine needs it settles `unsupported`
  naming the fact on both routes, never on one route only.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-940-AC-1 | For FR-850-AC-1's Counter package, each `AnalysisView` accessor returns equal values over the compiled `CheckedPackage` and over the `BarePackage` of its written bytes; an analysis entry called with either value gives the same item records for `Bounded`, `BelowThree` and `AtMostTwo`. | Test (TC-2620) |
| FR-940-AC-2 | `AnalysisView` has no implementation outside `fsl_package` (a `compile_fail` doctest implementing it for a local type fails with `E0277`), and no public function of any FSL crate takes a `BarePackage` or a `CheckedPackageV2` and returns a `CheckedPackage` or a `CheckedGraph` (source scan of every function returning either type). | Inspection (TC-2620) |
| FR-940-AC-3 | `fsl analyze` with a `package` selection no longer refuses `package` as not implemented: FR-850-AC-1's request completes with FR-850's outcome, exit 0. | Test (TC-2620) |

## Dependencies

- FR-087 R-10: checked typestate is built only by S3 and S4.
- FR-531: the `BarePackage` entry and the table of facts read from bytes.
- FR-850: the analyze request's `package` selection and its admission.
- FR-281: the analysis engines.
