---
id: SR-202611210022442313144108558
title: "Repository gap baseline and composition producer scoped proof"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2739; repository matrix and coverage census, combined producer diff"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

Ticket: QSL-2739. Independent combined-diff review of agent-ix/filament-spec-language PR3393, base e4578fa3ca51b22d60f863dd3525043f1f7ced5c through frozen head 13f242003f7964e0735c19807b2c316671473988. Exactly one existing worktree was inspected. No source/spec edits, builds, runtime tests, fetch/rebase, commits or tracker operations were performed. Root retains final qualification and merge custody.

No applicable AssuranceProfile was found by searching for type: AssuranceProfile under spec/. Direct review selected code-review with Rust lane, gap-analysis and spec-review/base. The sole spec edit corrects descriptive current behavior; no normative requirement, AC, domain object, edge or structural change calls for an extra analysis. Optional whole-repository semantic fan-out was not run.

Publication is QUEUED PRIVATELY: NOT POSTED; no tracker readback yet; has-review status was not read or changed. Root must publish each method once, deduplicate and perform actual byte/YAML readback. QSL-2739, QSL-2271, QSL-2726 and QSL-2128 remain In Progress; FR-1044-AC-10 is unmet.

Repository matrix and coverage remain FAIL. The bounded producer/pin change introduces no scoped gap found; preexisting whole-repository assurance cleanup is excluded from this engineering phase by the dispatching owner's explicit override.

## Verdict

FAIL for whole-repository assurance, with scoped producer/pin proof accepted. Three representative baseline findings below preserve specific primary units; the complete census and exact engine diagnostics remain in the private machine output. These are existing obligations, not newly introduced producer defects or a request to expand this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-1044-AC-10 remains untagged and its Package parity test remains ignored; Source/Package item equality is not implemented. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:417 |
| FND-002 | high | FR-530-AC-10 has no engine-bound test; repository-wide computed coverage retains 1142 untagged criteria. | spec/functional/FR-530-carry-and-check-liveness-and-fairness-premises-in-native-composition.md:188 |
| FND-003 | high | FR-1012-AC-3 is bound only to an ignored test; repository-wide computed coverage retains 66 ignored-only criteria. | spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred: QSL-2739 | Owner explicitly retains unmet AC-10 for the package-consumer lane and requires all four related tickets to remain In Progress after this producer/pin PR. |
| FND-002 | deferred: existing repository assurance campaign | Owner explicitly excludes unrelated whole-repository assurance cleanup from this engineering phase; this criterion and its tags are unchanged by the PR. |
| FND-003 | deferred: existing repository assurance campaign | Owner explicitly excludes unrelated whole-repository assurance cleanup from this engineering phase; this criterion and ignored test are unchanged by the PR. |
| FND-001 | deferred | Root explicitly retains full FR-1044-AC-10 Source/Package parity on existing QSL-2739, still unmet; all four related tickets remain In Progress. This producer/pin phase does not complete the package-consumer lane. |
| FND-002 | accepted-no-change | Root explicitly accepts the unchanged repository untagged-coverage baseline under the ARCH scope override. No criterion or tag for this baseline was changed in this PR; unrelated whole-repository coverage cleanup does not hold the scoped producer/pin merge. Gap verdict remains FAIL and global reverse coverage remains unestablished. |
| FND-003 | accepted-no-change | Root explicitly accepts the unchanged repository ignored-only coverage baseline under the ARCH scope override. No criterion or ignored test for this baseline was changed in this PR; unrelated whole-repository coverage cleanup does not hold the scoped producer/pin merge. Gap verdict remains FAIL and global reverse coverage remains unestablished. |

Round 1 records the root's explicit baseline dispositions and normalizes their latest outcome enums. The earlier rows remain as history. No semantic fix round or source change was performed. The findings and repository FAIL verdict are unchanged.

## Coverage

Plan completion: not assessed

Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2) — no run evidence read. Installed quoin 0.36.7 at /Users/peter/.nvm/versions/node/v24.15.0/bin/quoin; quire at /Users/peter/.nvm/versions/node/v24.15.0/bin/quire. quoin matrix --repo . --json exited 1, with exact stderr: Error: native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size. This reproduces the accepted matrix-size baseline once; the explicitly approved Quire static fallback completed exit 0 without retry or old binaries. /private/tmp/qsl-2739-ir-feature-pin-review/quoin-matrix.stderr preserves the diagnostic.

Computed matrix inventory: tagged 4925; untagged 1142; tagged-by-ignored-test 66; method-without-symbol 106; retired 15. No qualified-record status was emitted. FR-530-AC-1 and FR-093-AC-19 are tagged, including the new source-emission tests. FR-1044-AC-10 is untagged, not ignored-only: its bare prose references are unmarked-trace-id and bind no source symbol. The equality test is separately present with its explicit QSL-2739 ignore reason. Do not infer parity from the three passing current-boundary tests.

quire coverage --scope . --json completed exit 0. Engine reports 13905 evidence-symbol candidates, 13329 tagged, 12789 bound; 45832 production symbols examined, 0 carrying declared implements markers. This marker count is not a claim that every unmarked symbol has no owning requirement. Coverage population totals separately report 5003 backed of 6183 rows, 6204 criteria, 15 retired. Matrix and coverage populations are reported separately; no invented reconciliation total. Coverage includes 5976 untracked-symbol records, 6774 unmatched-tag records and 22996 diagnostics. Raw quire-matrix.json/quire-matrix.stderr and coverage.json/coverage.stderr preserve full detail. Foreign FSpec-prefixed tags in the new tests are intentional under AGENTS.md and cannot bind to locally minted FSL criteria; their unresolved local diagnostics are not fabricated upstream coverage or stale-ID findings.

Reverse inspection inventoried the eight changed behavior seams: source owner capture, whole declaration region, declared structural identity/QName, canonical term/member-span pairing, member-region mapping, emitted occurrence rows, composition claim-name rows and genuine model-root admission. They are owned by FR-530-AC-1, FSpec FR-322/491 and FR-093-AC-19/FR-154; no unowned changed behavior, source stub, test stub or coverage inflation found in those eight seams. Existing CompositionTerm and name-resolution helpers were inspected for reuse. The repository-wide production marker census is not a complete semantic mapping of 45832 production symbols; global reverse coverage remains unestablished and does not receive PASS. No done-plan claim was examined.

Optional whole-repository semantic review: skipped; bounded code/test alignment is recorded in the code/Rust review. Existing test logs are root-supplied runtime evidence, not Quoin evidence-store qualification. The strict reader controls exercise authentic source, full owner-local foreign references, six canonical arrays, sparse arrays, map-row reversal and negative precedence. No scoped reader weakening or stub was found.

## Examined units

```yaml
{
  "scope": [
    {
      "id": "FR-530-AC-1",
      "path": "spec/functional/FR-530-carry-and-check-liveness-and-fairness-premises-in-native-composition.md",
      "role": "examined",
      "excerpt": "Compiling, emitting and reading back a `composition-decl` keeps the component subjects, the rule and the `environment`, `relies`, `liveness_dependencies` and `fairness` lists separate, with each slot's source span its member; a Boolean success flag in place of the slots is refused `invalid_runtime_input`/`wrong-value-kind`."
    },
    {
      "id": "FR-1044-current-package-behavior",
      "path": "spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md",
      "role": "examined",
      "excerpt": "The package carries a composition's\n  declaration name row, so an item naming it is settled the same way naming\n  the `declaration` family."
    },
    {
      "id": "FR-1044-AC-10",
      "path": "spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md",
      "role": "examined",
      "excerpt": "Over `test/system-default`, a request with no `items` writes `Local` at index 0, `Sys` at 1, the subject's derived deadlock-freedom item at 2 (FR-620-AC-5), then the premise items `Bounded` at 3 and `Capped` at 4, and exits 21; over the unit's `package` handoff (FR-850) it writes the same items, records and exit code."
    },
    {
      "id": "FR-093-AC-19",
      "path": "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md",
      "role": "examined",
      "excerpt": "Every IR checked-package/v2 node kind (`node_tag`, `semantic_form`) is classified exactly once: as a family FSL's lowering writes, or as a kind FSL's lowering never writes, with the reason. For each family FSL writes, a package holding a node of it is admitted by IR's checked-package/v2 reader, through FSL's I2 read, as emitted."
    },
    {
      "id": "FSpec-FR-322-composition-declaration-names",
      "path": "filament-specification@f6b0480/spec/objects/interfaces/FR-322-checked-package-artifact.md",
      "role": "examined",
      "excerpt": "A\n`correspondence`/`component_composition` node SHALL carry its declared\ncomposition name in `declaration.qualified_name` and exactly one\n`claim_names` row, keyed to its `declaration` occurrence at ordinal 0."
    },
    {
      "id": "FSpec-FR-491-composition-member-occurrences",
      "path": "filament-specification@f6b0480/spec/functional/analysis/FR-491-spell-and-check-a-component-composition-declaration.md",
      "role": "examined",
      "excerpt": "The composition node SHALL carry one `generated` occurrence per body\nmember on its own node. The member sequence is the concatenation of the\nbody arrays `components`, `guarantees`, `relies`,\n`liveness_dependencies`, `supplies` and `environment`, in that order,\npreserving each array's canonical order."
    },
    {
      "id": "FSpec-FR-491-authored-regions",
      "path": "filament-specification@f6b0480/spec/functional/analysis/FR-491-spell-and-check-a-component-composition-declaration.md",
      "role": "examined",
      "excerpt": "The package writer SHALL capture each member's exact authored region\nbefore canonical body ordering. The corresponding `generated` entry's\n`regions` SHALL contain exactly that one region, under the declaration's\nsource reference and strictly within its declaration region."
    },
    {
      "id": "IR-source-owner-requirement",
      "path": "filament-contract-ir@96554ea/crates/filament-contract-model/src/checked_package/v2/owner.rs",
      "role": "context_only",
      "excerpt": "if declaration.is_some() {\n        return OwnerRequirement::Source;\n    }"
    },
    {
      "id": "FSpec-FR-154-artifact-id",
      "path": "filament-specification@f6b0480/spec/functional/type-model/FR-154-admit-domain-package-model.md",
      "role": "examined",
      "excerpt": "A type definition's or population node's artifact id does not match `^[A-Za-z][A-Za-z0-9_-]*$`"
    },
    {
      "id": "FSpec-FR-831-optional-carrier",
      "path": "filament-specification@f6b0480/spec/objects/interfaces/FR-831-carry-the-authored-correlation-clauses-of-protocol-controls.md",
      "role": "context_only",
      "excerpt": "Where a checked package carries protocol controls with `related` records, a\n  producer MAY publish the member."
    },
    {
      "id": "FR-530-AC-10",
      "path": "spec/functional/FR-530-carry-and-check-liveness-and-fairness-premises-in-native-composition.md",
      "role": "examined",
      "excerpt": "A `component-guarantee` slot key carries `assumptions`, the strictly ascending peer slot indices from the declaration (empty under every rule but `assume-guarantee`), and a premise result for the same guarantee checked under other assumptions or none is refused `invalid_runtime_input`/`invalid-value` at `/slots/<i>/result`."
    },
    {
      "id": "FR-1012-AC-3",
      "path": "spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md",
      "role": "examined",
      "excerpt": "A decimal per-step cost objective is written exactly in FSpec FR-181's typed `decimal` form (`{\"type\": \"decimal\", \"coefficient\": \"30\", \"scale\": \"2\"}` for `0.30`) and certified with a `potential` certificate."
    }
  ]
}
```

