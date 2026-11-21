---
id: "SR-17952890000006"
title: "QSL-2743 spec-review-failure-domain review"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/filament-spec-language; QSL-2743/QSL-2718; FR092/531/533 enum carrier and returns; terminal target admission"
review_set: subset
---

## Summary

The scoped carrier describes and tests its relevant identity and failure boundaries without inventing fallback semantics.

Ticket: QSL-2743; secondary QSL-2718. Frozen head 999023232ed624fe22b98739e4cac161883e6b2f, source base 19c53934c2e710e8d17de59a862108b4f5dd66e0. One existing worktree; no code changes, Cargo commands, checkouts, pushes or extra worktrees. Quoin 0.36.7, Quire 0.52.0 (engine 0.62.2), spec-artifacts-process@v0.29.2. No active AssuranceProfile document applies; matches were quoted review prose rather than type: AssuranceProfile frontmatter.

## Verdict

PASS. No new defect found in this analysis.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Examined unknown/missing/repeated channel refusal, original declaration/member/variant identity, selected document mismatch, duplicate/missing members, canonical rank/case order, zero/exact/one-short shared native budgets, unsupported unrelated declarations, payload-bearing/Clock exclusion, aliases, and source/Bare return equivalence. Traversal remains within finite admitted graph/row sets and existing stage budgets; no extension callbacks, external IO or side effects are introduced. Native equality locality and dimension scaling remain explicit external blockers rather than silent coercions. Reached/full native ownership behavior stays with Contract IR; no FSL private decision reconstruction or ID rekeying is present.

## Examined scope

```yaml
{
  "scope": [
    {
      "id": "FR-092-AC-26",
      "path": "spec/functional/FR-092-key-type-parameter-and-declared-nodes.md",
      "role": "examined",
      "excerpt": "Emitting a unit that selects a domain model with a variant type of cases `Beta` and `Alpha` gives one `SelectedModelEnumShapeRow` for it. Its shape is unordered with the variants in case-name byte order `Alpha`, `Beta`, each case's variant is the shape's variant at that rank, the declaration id equals the model enum declaration row's id and each case's node id and original identity equal the matching member row's. A shape with a variant reordered, removed or added is not equal to the row's shape. A declaration whose case set differs from its shape, or a case whose variant is not at its rank, i"
    },
    {
      "id": "FR-531-AC-8",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "The handoff directory of **R** holds required `identities.json` with `format` `filament.checked-handoff.identities/5`; `ReadIdentityRows::read` over it and the package gives a slot call that evaluates; the file equals, byte for byte, the encoding above of the in-process unit's rows, and two writes of equal requests give equal files; omitting the file from a `BareSupply` makes the slot call refuse `MissingIdentityWitness` while the temporal call is unaffected."
    },
    {
      "id": "FR-531-AC-12",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "`ReadIdentityRows::read` over a file that is not JSON, one with an unknown or repeated member, one with another `format`, one with a row whose `structural_node` the package does not hold and one whose `form` does not match its identity returns `Malformed`, `Malformed`, `Format`, `Unjoined` and `Unjoined` respectively, naming the member, the format string or the first row, and returns no rows."
    },
    {
      "id": "FR-531-AC-29",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "Materializing an authentic emission's equality witness with a shared native budget preserves every application, fact and source-unit row, including original identities and order. The source admission and handoff paths use the same budgeted row builder. Exact measured work and storage allowances return the complete witness; one unit less in either resource refuses without allocating the denied material or a diagnostic, retains the first unpaid resource, amount and original locus, and returns no partial witness. Two individually affordable materializations that exceed one shared allowance refuse"
    },
    {
      "id": "FR-531-AC-30",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "Source-enum materialization and FSL-to-IR model-enum-shape and selected-collection conversion preserve original structural/declaration/member identities, canonical case order and complete Exact types on one shared native budget. Exact measured bounds admit; work or storage one short and zero storage return allocation-free compact denials at the first unpaid locus with prior consumption intact. A later conversion cannot reset a preceding adapter's charges or replace its first denial. A selected source map preserves its original optional key node and complete Exact map value type through actual "
    },
    {
      "id": "FR-531-AC-31",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "An authentic emission with source records, record members and unions produces complete identity rows equal to the original producer claims after native joining and FSL materialization. The measured work and storage include the FSL producer input copies, the single native join and each later FSL declaration, field, record/member and union copy. Exact allowances admit; one unit below either measured allowance refuses at its first unpaid step with no partial identity rows, the original configured bound and the admitted prefix intact. Denied external row copies retain AdapterRow(i) at their origin"
    },
    {
      "id": "FR-531-AC-33",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "Allocation observation encloses only the adapter/conversion call, with fixture preparation and assertions outside it. A successful owned materialization performs observed allocations; a denial before its first allocation performs none, and a later denial allocates neither the denied material nor an owned diagnostic. The controls include fresh owner-produced lazy numeric payloads, values `2^127` and `-(2^127 + 1)`, and larger heap-backed values without prior warming or formatting. These values remain exact on successful copies; insufficient work/storage retains the first real row locus and orig"
    },
    {
      "id": "FR-531-AC-42",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "A selected model with `Tier` cases `Gold`, `Standard` and an unused payload-free variant with non-identifier cases in noncanonical authored order writes all three required model-enum channels in identities/5. Reading the file preserves every original declaration, member and shape; full runtime projection retains both enums, and the bare world resolves every case to its authenticated canonical rank and member identity. Calling enum-valued functions, including a nested typed parameter round trip, from source and the bytes gives equal variants, ranks and semantic charge logs; renaming the model a"
    },
    {
      "id": "FR-531-AC-43",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "Omitting or repeating any required enum channel, an unknown channel, or identities/4 refuses without a partial read. Foreign selected-document claims, missing or repeated members, reversed shape variants, and reordered canonical case rows each fail the native witness join."
    },
    {
      "id": "FR-531-AC-44",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "For original model-enum rows, one cumulative native budget covers producer materialization, native identity join and full runtime projection. The exact measured work and storage ceilings succeed with the same consumed totals; zero and one-less work or storage ceilings refuse with the original first-unpaid resource and no partial enum view."
    },
    {
      "id": "FR-092",
      "path": "spec/functional/FR-092-key-type-parameter-and-declared-nodes.md",
      "role": "examined",
      "excerpt": "For every payload-free model-owned enum of a selected domain model, including\nan unused declaration, lowering SHALL retain the real declaration and case\nnodes. The `Emission` SHALL carry one\n`SelectedModelEnumShapeRow` beside the model enum declaration and member\nrows. The row holds the declaration's emitted structural id and key, the\nselected document and the declaration's original model identity, the checked\n`EnumShape` of the variant type (unordered, with the cases' variants in\nascending byte order of the case names), and one entry per case in that\norder: the case label, the case's emitted "
    },
    {
      "id": "FR-531",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "These three model-enum channels carry only payload-free selected model variant\ntypes, including unused declarations and every real case node. A case with a\nClock payload is not payload-free. The writer retains the checked shape and\nvariant identities, never derives them from case keys, and orders cases by the\nshape's canonical rank, independently of authored source positions or model\naliases. Contract IR's `SelectedExactIdentities::with_admitted_model_enum_rows`\njoins all three channels against this package's selected model documents under\nthe invocation's cumulative native work and storage bu"
    },
    {
      "id": "FR-533",
      "path": "spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md",
      "role": "examined",
      "excerpt": "Payload-free selected model enum declarations and all their members SHALL be\navailable through the three required identities/5 channels of FR-531. The native\npackage-local join authenticates their complete canonical shapes before use,\nincluding selected declarations no slot reads. Enum literals SHALL retain the\noriginal variant and its canonical rank; source and bare enum-returning calls SHALL return the\nsame value and semantic charge sequence. Enum equality and inequality remain\nunmet until the native equality reader admits held selected-model declaration\nfacts independently of their original"
    }
  ]
}
```

