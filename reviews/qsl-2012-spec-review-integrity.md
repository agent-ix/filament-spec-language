---
id: SR-2026111911313600000008
title: "QSL-2012 spec-review/integrity qualified reaches"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; QSL-2012; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md, spec/functional/FR-102-build-state-clause-forms.md, spec/functional/FR-104-check-state-clauses.md, spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md, spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md, spec/test-cases/TC-456-s2-builds-state-clause-forms.md, spec/test-cases/TC-1046-reaches-over-a-relationship-end-types-boolean-and-refuses-by-name-operand-direction-and-destination.md, spec/test-cases/TC-2158-a-qualified-reaches-edge-names-the-edge.md, spec/test-cases/TC-396-s2-refuses-unrepresented-and-foreign-constructs.md, spec/test-cases/TC-2214-none-and-result-build-and-check-decides-their-position.md"
review_set: subset
---

## Summary

Reviewed committed head f1ab2290dfbb8e9affc51f180eae6f78c8ad94f7 on qsl-2012-qualified-reaches, diff origin/main...HEAD. Ticket: QSL-2012; PR: filament-spec-language#3039. No active AssuranceProfile declaration was found under spec/. Direct review set: base, integrity and affected failure-domain; no enforced run is required. Artifacts remain in the authorized scratch directory for lead custody; no source edits or commits.

## Verdict

**CONDITIONAL** — 2 concrete finding(s); disposition required.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-365 still defines every reaches candidate lookup on operand type S, while FR-104 requires qualified lookup in the qualifier's effective view; restrict the old rule to unqualified names and state the qualified path explicitly. | spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md:73 |
| FND-002 | medium | Removing the qualified-reaches-edge table row leaves QualifiedReachesEdge in ConstructKind, contrary to FR-763's explicit no-variant-without-a-row rule; reconcile the retained legacy vocabulary contract without restoring the S2 refusal. | spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md:92 |

## Coverage

Scoped traceability: US-003 -> FR-102/FR-104 -> TC-456/TC-1046; US-012 -> FR-365 -> TC-1046; FR-763 -> TC-2158. FR-091-AC-7's direct contradiction is reported in the base artifact; this method records the additional affected resolution/vocabulary consistency defects. No new CLI/API/pagination/concurrency/auth dependency is introduced. The qualifier's namespace and exact-case identity are defined, but the old candidate-owner paragraph and no-orphan-vocabulary assertion were not reconciled. No unrelated legacy clauses were classified as new defects.

## Validation limits

Named validation is pending in /private/tmp/qsl-2012-focused.log (coder process moved from slot wait to compilation during review). No duplicate cargo process was started. The old-behavior negative log /private/tmp/qsl-2012-baseline.log proves the new forms test fails on baseline 6d48fb879. Changed rustfmt and spec validation were reported passed; this review independently checks scoped specs/artifacts and diff whitespace. /private/tmp/qsl-2012-clippy.log records 589 existing fsl-semantics lib-test diagnostics, mostly result_large_err: lint is not clean. Owner restriction overrides full make ci, workspace tests/clippy/matrix and repeated gates. No gate or workflow dispatch was run. No dependency or CI files changed; cargo-deny was not rerun. Exact runtime model identifier is unavailable; marker records model=unknown.

## Examined units

The following JSON is YAML 1.2-compatible and includes every scoped unit, including clean units. Excerpts are verbatim prefixes of at most 600 characters.

```yaml
{
  "scope": [
    {
      "id": "FR-091-AC-7",
      "path": "spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md",
      "role": "examined",
      "excerpt": "S2 refuses the whole unit with cause `UnrepresentedConstruct`, naming the construct's CST production and span and returning no form, for a function body that holds exactly one of: `xs[0]`, `reaches(a, b, M::R)` (naming the edge `M::R`). For FR-230-AC-21's model, whose reset `self.ts[1].x = 0 ms`, reset `self.m[k].x = 0 ms` and replacement `self.m[k] = Timer { x: 0 ms, n: 1 }` index a clock place expression, S2 refuses none of the three with `UnrepresentedConstruct` and builds the place expressions FR-230 checks; the same unit with the function body `xs[0]` added refuses with `UnrepresentedCons"
    },
    {
      "id": "FR-102-AC-3",
      "path": "spec/functional/FR-102-build-state-clause-forms.md",
      "role": "examined",
      "excerpt": "`not reaches(self, self, parent)` builds `Not(Reaches { SelfRef, SelfRef, edge: parent })`, and `reaches(self, self, Config::ConfigVersion::parent)` builds `Reaches` retaining the entire edge name and span; and `self.versionNumber = pre(self.versionNumber)` builds an equality of two `Field` reads of `SelfRef`, the right one under `Pre`. `result` builds `Expression::Result`. None of these forms refuses."
    },
    {
      "id": "FR-104-AC-17",
      "path": "spec/functional/FR-104-check-state-clauses.md",
      "role": "examined",
      "excerpt": "A qualified field edge `M::Order::trail` resolves to the declared `trail` field and `M::Order::next` to the declared relationship end, including a subtype receiver. An unknown qualifier or member refuses `missing_declaration`/`missing-name` retaining the full edge name. A qualifier to which the source does not conform refuses `ill_typed`/`operator-ineligible`, without selecting a same-spelled member of the source. Unqualified edges retain their existing resolution."
    },
    {
      "id": "FR-763-AC-4",
      "path": "spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md",
      "role": "examined",
      "excerpt": "S2 over a state clause `reaches(self, self, Config::ConfigVersion::parent)` builds the form without an unsupported-construct diagnostic; the complete edge name and span are retained for S3."
    },
    {
      "id": "FR-763-AC-5",
      "path": "spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md",
      "role": "examined",
      "excerpt": "For every `ConstructKind` variant that source reaches (every row but `declaration`), the refusal's message begins with the row's sentence filled from `text`; a construct whose source text is 200 bytes long carries a `text` of at most 123 bytes that ends with `...`. No refusal of either cause has an empty `text`."
    },
    {
      "id": "FR-365-AC-1",
      "path": "spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md",
      "role": "examined",
      "excerpt": "`reaches(a, b, name)` over a relationship end whose destination is `[0,1]` or `[1,1]` of the near end's own type, or a finite multi-valued end of that type (unique, `Set`-shaped, and non-unique, `Bag`-shaped), checks as `Boolean` in an invariant, a precondition and a postcondition; a receiver of a subtype of the near end's type, an inverse end under `bidirectional` and under `undirected`, and a subtype operand beside a base operand each check; the field-edge form `reaches(self, self, trail)` still checks and lowers as before."
    },
    {
      "id": "FR-365-AC-2",
      "path": "spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md",
      "role": "examined",
      "excerpt": "A name matching a field and a relationship end, or two ends, refuses `ambiguous_declaration`/`ambiguous-name`; a name matching neither refuses `missing_declaration`/`missing-name`."
    },
    {
      "id": "FR-365-AC-3",
      "path": "spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md",
      "role": "examined",
      "excerpt": "An operand of a type that does not conform to the near end's type, a forward traversal against `target-to-source`, an inverse traversal against `source-to-target`, a destination end of another type (single-valued or multi-valued) and `reaches` in a function body each refuse `ill_typed`/`operator-ineligible`; an unbounded, an ordered and a multiplicity-less destination end of type `E` each check as `Boolean` and none refuses `unsupported_construct`; operands are checked ahead of direction and direction ahead of destination type, so a defect earlier in that order is the one reported."
    },
    {
      "id": "FR-365-AC-4",
      "path": "spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md",
      "role": "examined",
      "excerpt": "A reachability over `E` whose type is a member type of two populations refuses `ambiguous_declaration`/`ambiguous-name` at the form, naming both populations; with one covering population the requirements record adds that domain once and none when it is the context population."
    },
    {
      "id": "FR-091/unrepresented",
      "path": "spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md",
      "role": "examined",
      "excerpt": "A construct that no family's variant represents refuses the whole unit with\ncause `UnrepresentedConstruct`, naming the construct's CST production and\nspan, and carrying the construct payload and message of\n[FR-763](FR-763-name-the-construct-in-every-forms-stage-refusal.md). These constructs are indexing `e[i]`\noutside a clock place expression. A qualified `reaches(..)` edge builds with\nits full name and span (FR-102). `null` outside a record field value is not unrepresented: S2 builds it as the `Null` expression form, and check refuses it `ill_typed`/`type-mismatch` at `null` wherever it stand"
    },
    {
      "id": "FR-102/mapping",
      "path": "spec/functional/FR-102-build-state-clause-forms.md",
      "role": "examined",
      "excerpt": "- S1 parses `reaches`'s edge as a `QualifiedName`. S2 SHALL retain every\n  segment, including a qualifier such as `M::T` in `M::T::parent`, and the\n  complete edge span. S3 resolves the owner and member (FR-104, FR-365).\n- S2 SHALL NOT resolve the `using` alias, the model alias, the context type\n  or the operation. It keeps their spellings and spans for S3.\n- S2 SHALL build a state clause body of any depth S1 admits, exactly as it\n  builds a function body (FR-257).\n- The new `Expression` variants SHALL each have an owning family: `SelfRef`\n  and `Result` belong to `ProtocolClause`, and `Reache"
    },
    {
      "id": "FR-104/qualified-edge",
      "path": "spec/functional/FR-104-check-state-clauses.md",
      "role": "examined",
      "excerpt": "- For a qualified edge `M::T::member`, S3 SHALL resolve `M::T` through the\n  source unit's existing object-type namespace and look up `member` in that\n  type's effective view. The first operand SHALL conform to that qualifier;\n  otherwise it refuses `ill_typed`/`operator-ineligible`. An unknown qualifier\n  or member SHALL refuse `missing_declaration`/`missing-name`, retaining the\n  full authored edge name. It SHALL NOT fall back to an unqualified member\n  lookup. An unqualified edge continues to use the first operand's type.\n\n### Definedness facts\n\n- The checker SHALL run the static definednes"
    },
    {
      "id": "FR-365/resolution",
      "path": "spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md",
      "role": "examined",
      "excerpt": "`name` is resolved on the static type `S` of `a` by FR-363's rule of the near\nend: the relationship's `sourceEnd.role` names the forward end for receivers\nconforming to the source end's type, and the `targetEnd.role`, when declared,\nnames the inverse end for receivers conforming to the target end's type. The\ncandidates are the field `name` of `S`'s effective view and every relationship\nend navigable from `S` under that role. The form resolves to the one\ncandidate; otherwise:\n\n- no candidate: `missing_declaration`/`missing-name`, located at the form, the\n  refusal a name that is no field alread"
    },
    {
      "id": "FR-763/construct-vocabulary",
      "path": "spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md",
      "role": "examined",
      "excerpt": "- A construct that S2 later learns to build leaves this table with it; a new\n  site that refuses with either cause adds a row. Every row's kind is a\n  `ConstructKind` variant, and `ConstructKind` has no variant without a row.\n\n### Gate classification\n\n- FR-341 classifies these refusals by their cause, `not-yet-implemented`,\n  under its rule 5 (`unsupported`, an FSL implementation gap), not as a\n  profile prohibition.\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n| --- | --- | --- |\n| FR-763-AC-1 | The `parse` outcome document of a unit holding `synthesis Pick using v grammar Digit"
    },
    {
      "id": "TC-456",
      "path": "spec/test-cases/TC-456-s2-builds-state-clause-forms.md",
      "role": "examined",
      "excerpt": "## Test Procedure\n\nEvery unit starts with the header\n`language \"ix:native\" edition \"1-draft\";`, one `profile v = ...;` and\n`model Config = \"example/config-version\";`,\nFSpec's shared-grammar `model` production.\nParse each through S1 and build through `fsl_forms::build_unit`.\n\n1. Declare `invariant ParentOrder using v on Config::ConfigVersion at current\n   { present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }`.\n2. Declare `pre P using v on Config::ConfigVersion::attemptUpdate { true }`,\n   then the same with `post`.\n3. Declare `invariant NoCycle using v on"
    },
    {
      "id": "TC-1046",
      "path": "spec/test-cases/TC-1046-reaches-over-a-relationship-end-types-boolean-and-refuses-by-name-operand-direction-and-destination.md",
      "role": "examined",
      "excerpt": "## Test Procedure\n\nCompile one unit per vector over fixture **R** with model alias `M` and profile\nalias `V`, writing the expression in the clause shown. `self` is the context\nobject and `target` is the parameter of `M::Order::reroute(target:\nReference<M::Order>)` in a `pre` or `post` clause. Compare the checked node's\nstatic type or the refusal's code, cause, family and location with the vector.\n\nFixture **R** (domain package `test/orders`) declares object types `Order`,\n`Rush` (supertype `Order`) and `Person`; `Order` declares the field `trail` typed\n`Option<Reference<Order>>` and the field"
    },
    {
      "id": "TC-2158",
      "path": "spec/test-cases/TC-2158-a-qualified-reaches-edge-names-the-edge.md",
      "role": "examined",
      "excerpt": "## Test Procedure\n\nRun the `parse` operation over a state clause `reaches(self, self, Config::ConfigVersion::parent)`. Test: `tests/it/forms_refusal_cause.rs` `a_qualified_reaches_edge_builds_without_a_forms_refusal`.\n\nTag the tests `#[trace(\"TC-2158\", \"FR-763-AC-4\")]`.\n\n## Expected Results\n\n- Successful parse (exit 0), no diagnostic. The form retains\n  `Config::ConfigVersion::parent` and its complete span (FR-102-AC-3)."
    },
    {
      "id": "TC-396",
      "path": "spec/test-cases/TC-396-s2-refuses-unrepresented-and-foreign-constructs.md",
      "role": "examined",
      "excerpt": "## Test Procedure\n\nEvery fixture unit below starts with the complete-V1 header\n(`language \"ix:native\" edition \"1-draft\";`) and one profile selection whose\nalias is `v`.\n\n1. For each body below, build an admissible unit whose one function body is\n   that construct, and run S2: `xs[0]`; `none`; `reaches(a, b, M::R)`.\n2. Run S2 on `a div b`, `a rem b`, `a mod b` and\n   `if true then 1 else a mod b`, each under a state-only declaration\n   without `filament.value.complete/v1`; pass the resulting forms to check.\n3. For each body `B` of `deref(x)`, `allInstances<M::T>(x)` and `pre(x)`,\n   build an ad"
    },
    {
      "id": "TC-2214",
      "path": "spec/test-cases/TC-2214-none-and-result-build-and-check-decides-their-position.md",
      "role": "examined",
      "excerpt": "## Test Procedure\n\n1. Check the four functions of FR-091-AC-76 and `function a using v():\n   Option<Integer> pure { absent }`. Test:\n   `fsl-semantics/tests/it/option_values.rs`\n   `none_and_result_build_and_check_decides_their_position`.\n2. Build the function bodies `xs[0]`, `null` and `reaches(a, b, M::R)`.\n   Test: `fsl-forms/tests/it/value_forms.rs`\n   `a_construct_no_variant_represents_refuses_the_unit`.\n\n## Expected Results\n\n- Step 1: `o` and `a` check; `n` refuses `ill_typed`/`type-mismatch`; the `let`\n  refuses `ill_typed`/`ambiguous-literal`; `r` refuses\n  `wrong_snapshot`/`wrong-anch"
    }
  ],
  "bindings": []
}
```
