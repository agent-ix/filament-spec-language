---
id: SR-2026111911313600000009
title: "QSL-2012 spec-review/failure-domain qualified reaches"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/filament-spec-language; QSL-2012; spec/functional/FR-104-check-state-clauses.md, spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md, spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md, spec/functional/FR-102-build-state-clause-forms.md"
review_set: subset
---

## Summary

Reviewed committed head f1ab2290dfbb8e9affc51f180eae6f78c8ad94f7 on qsl-2012-qualified-reaches, diff origin/main...HEAD. Ticket: QSL-2012; PR: filament-spec-language#3039. No active AssuranceProfile declaration was found under spec/. Direct review set: base, integrity and affected failure-domain; no enforced run is required. Artifacts remain in the authorized scratch directory for lead custody; no source edits or commits.

## Verdict

**PASS** — No additional findings in this scoped method.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Affected failure-semantics review: typed missing-name retains the entire authored spelling; known incompatible owners are operator-ineligible; member ambiguity retains candidates. Source-unit object-type namespace and effective identities prevent last-segment fallback and cross-type aliasing. No new extension callback, mutation or graph traversal is introduced. Existing conformance and iterative reaches evaluation own cycle/termination behavior; those algorithms were context only and were not requalified here. No additional specification failure-domain finding beyond the code-review precedence defect and integrity lookup-rule mismatch. This PASS applies only to this lens and does not override those findings.

## Validation limits

Named validation is pending in /private/tmp/qsl-2012-focused.log (coder process moved from slot wait to compilation during review). No duplicate cargo process was started. The old-behavior negative log /private/tmp/qsl-2012-baseline.log proves the new forms test fails on baseline 6d48fb879. Changed rustfmt and spec validation were reported passed; this review independently checks scoped specs/artifacts and diff whitespace. /private/tmp/qsl-2012-clippy.log records 589 existing fsl-semantics lib-test diagnostics, mostly result_large_err: lint is not clean. Owner restriction overrides full make ci, workspace tests/clippy/matrix and repeated gates. No gate or workflow dispatch was run. No dependency or CI files changed; cargo-deny was not rerun. Exact runtime model identifier is unavailable; marker records model=unknown.

## Examined units

The following JSON is YAML 1.2-compatible and includes every scoped unit, including clean units. Excerpts are verbatim prefixes of at most 600 characters.

```yaml
{
  "scope": [
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
    }
  ],
  "bindings": []
}
```
