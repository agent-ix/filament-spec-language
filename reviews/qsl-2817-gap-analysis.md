---
id: SR-967175026302197768900474181941935084706
title: "Gap analysis of QSL-2817 package expectations"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2817; fsl-eval tests/invariants, fsl-command handoff test, fsl-conformance cases/front_end_rows, fsl-inspect render golden"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

The repository-driven matrix and scoped reverse-gap inspection were applied to PR 3471. The assigned delta has owning requirements and non-vacuous assertions; repository-wide baseline diagnostics and missing runtime evidence prevent a repository-wide PASS claim.

## Verdict

**CONDITIONAL** — no new scoped reverse gap, stub or trace defect found. The full-repository evidence matrix was refused for size, the static fallback contains baseline gaps, and the explicitly pending calibration and named titan tests have not been discharged.

## Scope

The full rows were read; long excerpts retain their first 600 characters verbatim. No plan was supplied.

```yaml
{
  "scope": [
    {
      "id": "FR-056-AC-59",
      "path": "spec/functional/FR-056-admit-domain-package-model-declarations.md",
      "role": "examined",
      "excerpt": "Over a Float64 field `a`, each of `self.a = 0`, `self.a = self.versionNumber` (`Int[0, 1000]`), `self.a < self.count` (`Integer`), `self.a = self.f32` (a Float32 field), `self.a + 1 = self.a` and `self.a = convert<Float64[nearest-even]>(self.a)` refuses `ill_typed`/`type-mismatch` at the operator, naming both operand types, and the unit yields no checked graph."
    },
    {
      "id": "FR-090-AC-15",
      "path": "spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md",
      "role": "examined",
      "excerpt": "Each production evaluator producer in the table other than `record-present-value-missing`, exercised at the detecting site with its stated malformed state, returns the table's `S6a` fault rather than a manufactured kernel CheckedInvariant."
    },
    {
      "id": "FR-091-AC-19",
      "path": "spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md",
      "role": "examined",
      "excerpt": "The assembler admits a parameter typed `Float64[nearest-even]`, `Float32[toward-zero]` or `Float64`. Resolving each gives a `ValueType::Float` of the written width and rounding mode `nearest-even`, `toward-zero` and `exact` (the bare spelling). The evaluator rounds a `Float64[mode]` `+` by that mode. It refuses a parameter typed `Reference<M::T>`, in a unit with no admitted domain package, with an unresolved-type-name error naming `M::T`, and returns no `PackageDeclarations`."
    },
    {
      "id": "FR-104-AC-13",
      "path": "spec/functional/FR-104-check-state-clauses.md",
      "role": "examined",
      "excerpt": "With `type P = Decimal[-99, 99; 2, 2; nearest-even];` and `type R = Decimal[0, 1000; 1, 1; exact];`, `function c using v(a: P, b: P): Boolean pure { a + b = b + a }` checks with each sum typed `P`, as do the bodies `a * b = b * a`, `a - b = a - b` and `a + decimal(1, 2) = b`, the last with the literal typed `P`. With `b: R`, `a + b = b + a` refuses `ill_typed`/`type-mismatch` at the first `a + b`. `let s = decimal(1, 2) + decimal(2, 2) in s = a` refuses `ill_typed`/`ambiguous-literal` at the sum. Evaluating `c` with `a = decimal(1, 2)`, `b = decimal(2, 2)` returns `true`, and with `a = decimal"
    },
    {
      "id": "FR-104-AC-16",
      "path": "spec/functional/FR-104-check-state-clauses.md",
      "role": "examined",
      "excerpt": "The six FSpec TC-185 D25 expressions are FSL source (QSL-1394), each built by S2 as `DecimalValue` operands wherever an operand may stand (FR-091-AC-57). With `type P = Decimal[0, 1000; 2, 2; exact];`, `type R = Decimal[0, 1000; 1, 1; exact];` and parameters `p: P`, `r: R`: `function e1 using v(): P pure { let v = decimal(15, 1) + decimal(225, 2) in v }` refuses `ill_typed`/`ambiguous-literal` at the sum, because a `let` binding supplies no expected type; `function e2 using v(): Boolean pure { decimal(15, 1) = decimal(150, 2) }` refuses `ill_typed`/`ambiguous-literal` at its first literal `dec"
    },
    {
      "id": "FR-185-AC-7",
      "path": "spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md",
      "role": "examined",
      "excerpt": "`Service` (with `d` typed `Millis = Int[1, 12]`) checks at S3 and S4 emits it with its random parameters, reward and workload as FR-740 nodes; the package reader admits it and gives back the checked record."
    },
    {
      "id": "FR-185-AC-12",
      "path": "spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md",
      "role": "examined",
      "excerpt": "An S4 refusal concerning a random parameter, a workload, a reward or a delay carries that declaration's source reference and label span in its locus member (never `null`), and its message names the declaration in words and holds no debug rendering of a span or source reference."
    },
    {
      "id": "FR-286-AC-26",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "The bytes of every golden document, the `analyze` documents `fsl-analyze` assembles and the `monitor` and `execute` documents `fsl-command` writes included, equal their own RFC 8785 re-encoding through `quire-canonical`. FR-286-AC-2's diagnostic is written with its members in the order `cause`, `code`, `details`, `hint`, `locus`, `message`, and an `unsupported_construct`/`declaration-form` diagnostic carrying `construct`, `declaration`, `profile` and `text` in the order `cause`, `code`, `construct`, `declaration`, `details`, `hint`, `locus`, `message`, `profile`, `text`. The canonical check re"
    },
    {
      "id": "FR-564-AC-2",
      "path": "spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md",
      "role": "examined",
      "excerpt": "Each of `package`'s five applicable cells returns its kind: success exit 0; unsupported `unsupported_construct` exit 21; timeout `Cancelled(Deadline)`, cancellation `Cancelled(Requested)` and incomplete `LimitExceeded` naming the `package` limit field, each exit 22."
    },
    {
      "id": "FR-564-AC-7",
      "path": "spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md",
      "role": "examined",
      "excerpt": "Every outcome of AC-1 to AC-5 serializes to an FR-286 document whose `operation` names its verb and whose `category` is its kind's, and every AC-6 item serializes to an FR-286 `items` entry whose `category` is its kind's; serializing each twice gives equal bytes, and each exit code is the one FR-285's function returns for that category."
    },
    {
      "id": "FR-565-AC-2",
      "path": "spec/functional/FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md",
      "role": "examined",
      "excerpt": "For every `Case`, `run_case` returns `Ok` with a typed document (or, for `prove`, an item) whose category and exit code equal the case's expected category and exit code, two `run_case` calls give equal `OutcomeDocument` values, and every authored source identity in the document equals `SourceIdentity::new(case.parameters.source_authority, case.parameters.source_identity)`, independently of the input's physical file name. A case with an input the test removes returns `Err(CaseError)` whose `stage` is `CaseStage::Read`, the `Copy` stage value, and no `CaseOutcome`."
    },
    {
      "id": "FR-850-AC-15",
      "path": "spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md",
      "role": "examined",
      "excerpt": "For AC-12's handoff, compute `B` as the bytes of exactly the root, manifest, unique dependency package/witness files and selected model/runtime files read. At `dependent_bytes = B` intake completes; at `B - 1` it settles `incomplete`, `resource_exhausted`, exit `22`, naming `dependent_bytes` and its configured bound. A handoff whose manifest expands the selected-file total from 64 to 65 stops at the same phase naming the shared file limit before opening file 65. A diamond charges its shared triple once. For strict-read/native-join work and storage, an allowance equal to the completed closure p"
    },
    {
      "id": "FR-093/Explicit numeric narrowing",
      "path": "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md",
      "role": "examined",
      "excerpt": "An unproved narrowing SHALL refuse `undefined_expression`/`unproved-range` at\nthe conversion."
    }
  ]
}
```

## Coverage

Plan completion: not assessed

- Reconciliation: `quoin matrix --repo /home/peter/dev/worktrees/qsl-2817-package-tests --json` on terra, resolved executable `/home/peter/.npm-global/bin/quoin` version 0.36.8, exited 1 with `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`. No evidence store result was accepted.
- The documented static fallback `quire matrix --scope /home/peter/dev/worktrees/qsl-2817-package-tests --format json` and `quire coverage --scope /home/peter/dev/worktrees/qsl-2817-package-tests --json` exited 0 with quire 0.52.0 / engine 0.62.2. No run evidence was read.
- The static matrix marks all twelve scoped ACs tagged: FR-056-AC-59, FR-090-AC-15, FR-091-AC-19, FR-104-AC-13, FR-104-AC-16, FR-185-AC-7, FR-185-AC-12, FR-286-AC-26, FR-564-AC-2, FR-564-AC-7, FR-565-AC-2 and FR-850-AC-15. FR-093's explicit narrowing statement supplies the conversion rule.
- The engine coverage totals report backed 5101 of 6220 reference rows, 6241 criteria and 15 retired rows. Its Rust census reports 14440 candidates, 13865 tagged and 13267 bound symbols. Coverage output includes 23490 diagnostics and 6208 untracked-symbol records across the repository. Those baseline conditions are disclosed and are not described as resolved by this PR or mistaken for runtime evidence.
- Reverse-gap inspection covered the changed lifecycle fixture selection and both new fixture inputs plus the changed assertions. The fixture selection is owned by FR-564-AC-2 and FR-565-AC-2; Service readback by FR-185-AC-7; error causes, owner preservation and canonical member order by the scoped criteria. No new API, CLI option or independent evaluator semantics is introduced.
- No new scoped source stub, unconditional skip, ignored test, mock replacement, or assertion-free test was found. The original twelve selected test function names remain selectable (11 assigned plus the affected float arithmetic case). There is no coverage-percentage claim.
- The existing FR-564 input table still names historical Service, while its current AC-2 and the direct red-main assignment require a live unsupported package cell. This pre-existing table/Service inconsistency is context, not a new claim that Service remains unsupported.
- Optional standalone semantic gap review was not requested and did not run; requirement/test/producer consistency was checked as part of the code/Rust review. No build or test was executed.
- Detailed static outputs are retained in this scratchpad as matrix.json, coverage.json and scoped-matrix.json; no hand-written Test Matrix was created.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No new scoped gaps (placeholder) | - |
