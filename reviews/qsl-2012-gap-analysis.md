---
id: SR-2026111911313600000006
title: "QSL-2012 gap-analysis qualified reaches"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2012; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md, spec/functional/FR-102-build-state-clause-forms.md, spec/functional/FR-104-check-state-clauses.md, spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md, spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md, spec/test-cases/TC-456-s2-builds-state-clause-forms.md, spec/test-cases/TC-1046-reaches-over-a-relationship-end-types-boolean-and-refuses-by-name-operand-direction-and-destination.md, spec/test-cases/TC-2158-a-qualified-reaches-edge-names-the-edge.md, spec/test-cases/TC-396-s2-refuses-unrepresented-and-foreign-constructs.md, spec/test-cases/TC-2214-none-and-result-build-and-check-decides-their-position.md, fsl-forms/tests/it/state_model_forms.rs, fsl-forms/tests/it/protocol_clause_forms.rs, fsl-forms/tests/it/value_forms.rs, fsl-semantics/tests/it/relationship_reachability.rs, tests/it/forms_refusal_cause.rs"
review_set: subset
---

## Summary

Reviewed committed head f1ab2290dfbb8e9affc51f180eae6f78c8ad94f7 on qsl-2012-qualified-reaches, diff origin/main...HEAD. Ticket: QSL-2012; PR: filament-spec-language#3039. No active AssuranceProfile declaration was found under spec/. Direct review set: base, integrity and affected failure-domain; no enforced run is required. Artifacts remain in the authorized scratch directory for lead custody; no source edits or commits.

## Verdict

**FAIL** — 1 concrete finding(s); disposition required.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The FR-091-AC-7-tagged refusal test deletes its qualified-reaches case while that AC and TC-396/TC-2214 still require it; trace presence now hides an unverified and contradictory oracle. Reconcile the scoped specification and its fixture descriptions with admission. | fsl-forms/tests/it/value_forms.rs:671 |

## Coverage

Manual changed-scope AC/code/test check explicitly replaces the whole-repository computed matrix by Peter's direct instruction; this is not a whole-repository assurance claim. Plan completion: not assessed

FR-102-AC-3: the forms tests verify the whole qualified spelling, edge span and children; protocol test retains self/result/unqualified behavior. FR-104-AC-17: three owner/qualifier pairs cover direct/base-inherited/derived-inherited fields and relationship ends; separate tests cover unknown model/type/member and exact case, foreign owner, ambiguity and unqualified edges. The mixed absent-member/foreign-owner error path has the code-review defect. FR-365-AC-1/2/3: the same tests exercise resolved field/end identities and typed causes through real check paths. FR-763-AC-4: the parse test asserts exit 0 and no diagnostics; the FR-102 test separately asserts retained spelling/span. There is no FR-763-AC-4 tag on that lower-level span assertion, so automated direct-tag evidence of the span part is limited; existing behavior is nevertheless manually verified. Existing FR-091-AC-7 trace no longer covers its retained qualified-refusal obligation. No stubs, ignored new tests or mocks of the checked behavior were added. Evaluation and lowering algorithms are unchanged and outside this manual gap scope.

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
    },
    {
      "id": "a_qualified_edge_name_builds_a_reaches_form",
      "path": "fsl-forms/tests/it/state_model_forms.rs",
      "role": "examined",
      "excerpt": "fn a_qualified_edge_name_builds_a_reaches_form() {\n    let (text, unit) = build(\n        \"invariant I using v on Config::ConfigVersion at current \\\n         { reaches(self, self, Config::ConfigVersion::parent) }\",\n    );\n    let DeclarationForm::StateClause(form) = unit.forms()[0].form() else {\n        panic!(\"a state clause\");\n    };\n    let ExprNode::Reaches {\n        source,\n        target,\n        edge,\n        edge_span,\n    } = form.body.root_node()\n    else {\n        panic!(\"a reaches form\");\n    };\n    assert_eq!(edge, \"Config::ConfigVersion::parent\");\n    assert_eq!(&text[edge_span.st"
    },
    {
      "id": "self_result_and_reaches_build_with_a_qualified_edge",
      "path": "fsl-forms/tests/it/protocol_clause_forms.rs",
      "role": "examined",
      "excerpt": "fn self_result_and_reaches_build_with_a_qualified_edge() {\n    let (_, unit) = build(\n        \"invariant NoCycle using v on Config::ConfigVersion at current \\\n         { not reaches(self, self, parent) }\",\n    );\n    let form = state_clause(unit.forms()[0].form());\n    assert_eq!(\n        show(form.body.root()),\n        \"Not(Reaches[parent](SelfRef, SelfRef))\"\n    );\n\n    let (_, unit) = build(\n        \"post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate \\\n         { self.versionNumber = pre(self.versionNumber) }\",\n    );\n    let form = state_clause(unit.forms()[0].form());"
    },
    {
      "id": "a_construct_no_variant_represents_refuses_the_unit",
      "path": "fsl-forms/tests/it/value_forms.rs",
      "role": "examined",
      "excerpt": "fn a_construct_no_variant_represents_refuses_the_unit() {\n    let cases = [\n        (\"xs[0]\", Production::Postfix, \"xs[0]\"),\n        (\n            \"size<Int[0, 1]>(c)\",\n            Production::Primary,\n            \"size<Int[0, 1]>(c)\",\n        ),\n    ];\n    for (source, production, construct) in cases {\n        let (text, cause, span) = refusal(&format!(\n            \"function f using v(): Boolean pure {{ {source} }}\"\n        ));\n        assert!(\n            matches!(\n                &cause,\n                FormsCause::UnrepresentedConstruct { production: actual, .. }\n                    if *ac"
    },
    {
      "id": "an_unqualified_edge_still_resolves",
      "path": "fsl-semantics/tests/it/relationship_reachability.rs",
      "role": "examined",
      "excerpt": "fn an_unqualified_edge_still_resolves() {\n    let declarations = invariant(\"Order\", \"reaches(self, self, trail)\");\n    let graph = check(&declarations).expect(\"the field-edge form checks\");\n    let body = graph.state_clause(\"I\").expect(\"I is checked\").body();\n    assert_eq!(body.value_type(), &ValueType::Boolean);\n    assert!(\n        matches!(body.kind(), NodeKind::Reaches { edge, .. }\n            if edge == &FieldRef::new(object(&graph, \"Order\"), \"trail\")),\n        \"a field edge, not an end\"\n    );\n    assert_admitted(\n        &invariant(\"Order\", \"reaches(self, self, next)\"),\n        \"next\","
    },
    {
      "id": "a_qualified_edge_resolves_to_the_declared_edge",
      "path": "fsl-semantics/tests/it/relationship_reachability.rs",
      "role": "examined",
      "excerpt": "fn a_qualified_edge_resolves_to_the_declared_edge() {\n    for (owner, qualifier) in [(\"Order\", \"Order\"), (\"Rush\", \"Order\"), (\"Rush\", \"Rush\")] {\n        let graph = check(&invariant(\n            owner,\n            &format!(\"reaches(self, self, M::{qualifier}::trail)\"),\n        ))\n        .expect(\"the qualified field edge checks\");\n        let body = graph.state_clause(\"I\").expect(\"I is checked\").body();\n        assert_eq!(body.value_type(), &ValueType::Boolean);\n        assert!(matches!(body.kind(), NodeKind::Reaches { edge, .. }\n            if edge == &FieldRef::new(object(&graph, \"Order\"), \"t"
    },
    {
      "id": "a_qualified_edge_naming_an_unknown_edge_refuses_missing_name",
      "path": "fsl-semantics/tests/it/relationship_reachability.rs",
      "role": "examined",
      "excerpt": "fn a_qualified_edge_naming_an_unknown_edge_refuses_missing_name() {\n    for edge in [\n        \"M::Order::noSuchEdge\",\n        \"Missing::Order::trail\",\n        \"M::Missing::trail\",\n        \"M::order::trail\",\n        \"M::Order::Trail\",\n    ] {\n        let refusal = assert_refused(\n            &invariant(\"Order\", &format!(\"reaches(self, self, {edge})\")),\n            Code::MissingDeclaration,\n            \"missing-name\",\n            vec![],\n        );\n        let CheckCause::StateModel(model) = refusal.cause else {\n            panic!(\"a state model refusal\");\n        };\n        assert_eq!(model.cau"
    },
    {
      "id": "a_qualified_edge_of_another_type_refuses",
      "path": "fsl-semantics/tests/it/relationship_reachability.rs",
      "role": "examined",
      "excerpt": "fn a_qualified_edge_of_another_type_refuses() {\n    for edge in [\"M::Person::trail\", \"M::Person::next\", \"M::Rush::trail\"] {\n        assert_refused(\n            &invariant(\"Order\", &format!(\"reaches(self, self, {edge})\")),\n            Code::IllTyped,\n            \"operator-ineligible\",\n            vec![],\n        );\n    }\n}\n\n#[trace(\"TC-1046\", \"FR-365-AC-2\", \"FR-104-AC-17\")]\n#[test]\nfn a_qualified_edge_retains_member_ambiguity() {\n    for edge in [\"M::Order::parent\", \"M::Order::twin\"] {\n        let refusal = assert_refused(\n            &invariant(\"Order\", &format!(\"reaches(self, self, {edge})\"))"
    },
    {
      "id": "a_qualified_edge_retains_member_ambiguity",
      "path": "fsl-semantics/tests/it/relationship_reachability.rs",
      "role": "examined",
      "excerpt": "fn a_qualified_edge_retains_member_ambiguity() {\n    for edge in [\"M::Order::parent\", \"M::Order::twin\"] {\n        let refusal = assert_refused(\n            &invariant(\"Order\", &format!(\"reaches(self, self, {edge})\")),\n            Code::AmbiguousDeclaration,\n            \"ambiguous-name\",\n            vec![],\n        );\n        let CheckCause::StateModel(model) = refusal.cause else {\n            panic!(\"a state model refusal\");\n        };\n        let CheckCause::AmbiguousMember(ambiguous) = model.cause else {\n            panic!(\"an ambiguous member refusal\");\n        };\n        assert_eq!(ambiguo"
    },
    {
      "id": "a_qualified_reaches_edge_builds_without_a_forms_refusal",
      "path": "tests/it/forms_refusal_cause.rs",
      "role": "examined",
      "excerpt": "fn a_qualified_reaches_edge_builds_without_a_forms_refusal() {\n    let edge = \"Config::ConfigVersion::parent\";\n    let (_, document, exit) = parsed(&format!(\n        \"invariant I using v on M::Order at current {{ reaches(self, self, {edge}) }}\"\n    ));\n    assert_eq!(exit, 0, \"{document}\");\n    assert_eq!(document[\"diagnostics\"], serde_json::json!([]));\n}\n\n#[trace(\"TC-2159\", \"FR-763-AC-5\")]\n#[test]\nfn every_construct_message_begins_with_the_construct() {\n    for (declaration, prefix) in [\n        (\n            function(\"null\"),\n            \"`null` is not supported as a value here yet\".to_owned"
    },
    {
      "id": "every_construct_message_begins_with_the_construct",
      "path": "tests/it/forms_refusal_cause.rs",
      "role": "examined",
      "excerpt": "fn every_construct_message_begins_with_the_construct() {\n    for (declaration, prefix) in [\n        (\n            function(\"null\"),\n            \"`null` is not supported as a value here yet\".to_owned(),\n        ),\n        (\n            function(\"xs[0]\"),\n            \"the index selection `xs[0]` is not supported here yet\".to_owned(),\n        ),\n        (\n            function(\"size<Integer>(xs)\"),\n            \"`size<Integer>` is not supported in expressions yet\".to_owned(),\n        ),\n        (\n            \"temporal T using v over (x: M::Order) clock \\\"steps\\\" on origin { always holds(x @ a = x @"
    }
  ],
  "bindings": [
    {
      "test_id": "a_qualified_edge_name_builds_a_reaches_form",
      "ac_id": "FR-102-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "self_result_and_reaches_build_with_a_qualified_edge",
      "ac_id": "FR-102-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "an_unqualified_edge_still_resolves",
      "ac_id": "FR-104-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "a_qualified_edge_resolves_to_the_declared_edge",
      "ac_id": "FR-104-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "a_qualified_edge_naming_an_unknown_edge_refuses_missing_name",
      "ac_id": "FR-104-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "a_qualified_edge_of_another_type_refuses",
      "ac_id": "FR-104-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "a_qualified_edge_retains_member_ambiguity",
      "ac_id": "FR-104-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "a_qualified_reaches_edge_builds_without_a_forms_refusal",
      "ac_id": "FR-763-AC-4",
      "trace": "correct"
    },
    {
      "test_id": "a_construct_no_variant_represents_refuses_the_unit",
      "ac_id": "FR-091-AC-7",
      "trace": "correct"
    }
  ]
}
```
