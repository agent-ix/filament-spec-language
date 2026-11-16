---
id: FR-821
title: "Analyze flow rules through native FSL analysis"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-165
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-628
    type: depends_on
  - target: ix://agent-ix/quire-rs/FR-096
    type: depends_on
---
# FR-821: Analyze flow rules through native FSL analysis

## Description

When the flow-analysis integration receives a valid `quire.flow-report/v1`
document, it SHALL derive an independent native FSL analysis from the report's
rules, vocabularies and facts, and SHALL return the `filament-outcome/1`
`analyze` result defined by FR-621.

The integration SHALL derive its claims from `rules`, `vocabularies` and
`facts`; it SHALL NOT use the report's `denials` to construct claims or to
settle them. It SHALL use the existing source-driven `native-analyze/1` path
(FR-620): FSL compiles the generated source into an in-memory checked package
and analyzes that package. The file written by `fsl compile --output` (FR-810)
is not an input to `fsl analyze`.

## Inputs

- A `quire.flow-report/v1` document as defined by Quire FR-096. The analyzer
  consumes the declared `rules`, their closed `vocabularies` and the `facts`
  those rules read. It does not treat `denials` as model facts.
- The existing `native-analyze/1` request and finite model analysis semantics
  of FR-620, FR-621, FR-165 and FR-628.

## Outputs

- One FR-621 `filament-outcome/1` document with one native analysis item for
  each flow rule, plus an implication item for each direction of every pair
  of rules with the same subject archetype.
- Each flow-rule item retains the original rule name and stable subject
  identity (`archetype`, `id`, and `repo` when present) in its claim or
  evidence. A missing `repo` is not replaced with an inferred repository.

## Behavior

- The integration SHALL preserve rule declaration order and the report's
  closed enum vocabularies. A field omitted from a fact SHALL remain absent;
  it SHALL NOT be filled with a default value.
- The lowering SHALL preserve FR-096 path and predicate meaning, including
  `path`, `any_path`, `any_of`, `present` and `absent`, for every rule whose
  exact meaning can be represented by the native FSL source and analyzer.
  It SHALL preserve each fact's document identity and the resolved edge
  direction and target used by a rule path.
- For a supported rule, the analysis claim SHALL be true exactly when no
  subject fact satisfies the rule's match and no `unless_any` condition
  protects that subject. A proved item therefore means that the independent
  native analysis found no denial; a refuted item means that it found a
  denial and its evidence identifies the same subject and matched facts.
- Where two rules share a subject archetype, the integration SHALL analyze
  both directed implications over the same finite facts: every subject denied
  by the first rule is denied by the second, and every subject denied by the
  second is denied by the first.
  It SHALL report a rule as redundant only when its implication to the other
  rule is proved. A refuted or non-conclusive implication is not redundancy.
- If the native analyzer cannot represent or decide a rule or implication,
  its item SHALL retain an explicit native `unsupported`, `incomplete` or
  `inconclusive` outcome, as applicable. The integration SHALL NOT convert
  such an item into `proved`, omit it, or label it a denial. An unsupported
  item SHALL identify its rule or rule pair and the native reason. Other
  supported items in the same report remain analyzable.
- A syntax, type or malformed-report refusal SHALL retain the existing
  request refusal behavior; it SHALL NOT yield a partial successful outcome.
- Two runs over the same report SHALL preserve rule and item order and give
  equal claim outcomes and evidence. The integration does not promise a
  `quire.differential-report/v2`, a second solver, or a solver-disagreement
  result; `filament-outcome/1` is the analysis result.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-821-AC-1 | Given a supported flow report with a rule that denies connection `CONN-1` because it carries `health` across an untrusted boundary without a protecting control, the native analysis item for that rule is `refuted` and its evidence names `CONN-1`, the matched classification and boundary identities, and the rule name. The report's `denials` array is not used to construct the analyzed claim. | Test |
| FR-821-AC-2 | Given AC-1's facts with an `encrypt-in-transit` control protecting `CONN-1`, the same native claim is `proved`; adding unrelated facts does not change that result. | Test |
| FR-821-AC-3 | Over every supported PLAT-1540 pass and fail fixture, the set of native `refuted` `(rule, subject identity)` results equals the corresponding set in FR-096's `denials`; each `proved` rule contributes no denial. Items whose exact lowering is unsupported are retained with an explicit non-conclusive native result and are excluded from the equality comparison. | Test |
| FR-821-AC-4 | Given the broad `personal-across-untrusted-boundary` rule and a narrower copy restricted to `trust_level: untrusted`, the native implication claim proves that every denial of the narrower rule is also a denial of the broad rule, and the integration reports the narrower rule as redundant. If the implication does not prove, the integration does not report it as redundant. | Test |
| FR-821-AC-5 | Given a valid report where one generated claim reaches an existing native analysis limit, that item is `incomplete` with the native limit cause; another supported item in the same report still settles independently. The limited result is never reported as `proved`. | Test |
| FR-821-AC-6 | Given a valid report with a rule or implication whose exact expression the native analyzer cannot represent, the outcome retains an item for it with native `unsupported` and its reason; no such result is reported as `proved`. A supported rule in the same report still settles independently. | Test |
| FR-821-AC-7 | A malformed report or generated source that fails the existing request, parse, type or compile checks yields the corresponding whole-request refusal and no partial successful outcome. | Test |
| FR-821-AC-8 | Two analyses of one unchanged flow report give equal item order, claim outcomes and evidence. `fsl analyze` consumes its source-driven request and does not read the checked-package file emitted by `fsl compile --output`. | Test |

## Dependencies

- [FR-620](./FR-620-read-a-native-analyze-1-request-and-analyze-its-items.md)
  and [FR-621](./FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md)
  own the native analyze request and result.
- [FR-165](./FR-165-check-state-graph-claims-at-s3.md) and
  [FR-628](./FR-628-request-state-graph-query-items.md) own checked state-graph
  claims and their request items.
- Quire [FR-096](ix://agent-ix/quire-rs/FR-096) owns the `quire.flow-report/v1`
  rules, vocabularies, facts, denials and remediation data. Its report-lowering
  note describes the retired `quire.contract-package/v1` path; this requirement
  uses the current source-driven native FSL analysis path instead.
- **Capability gap:** the former PLAT-1541 AC requiring a Z3/cvc5 disagreement
  to appear in `quire.differential-report/v2` has no equivalent in current
  native FSL analysis. `fsl analyze` emits `filament-outcome/1` and has no
  dual-solver comparison. That capability is outside this requirement and is
  not a shipped acceptance criterion; FR-821-AC-5 instead requires the native
  `incomplete` result when analysis reaches a bound.
