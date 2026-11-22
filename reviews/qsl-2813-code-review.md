---
id: SR-967175026302197768900474181941935084696
title: "Code and Rust review of QSL-2813 stale test expectations"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2813; fsl-semantics/tests/it/model_operations.rs; tests/it/family_outcome_layering.rs; tests/it/first_rule_readme.rs; tests/it/formatter_identity.rs; tests/it/native_boundaries.rs; tests/it/producer_unit_identity.rs; tests/it/spine_deletion_boundaries.rs; tests/it/standard_pin.rs"
review_set: subset
---

## Summary

Reviewed all eight changed test files in PR 3464 against the current requirement text and the accepted native ClockReference decision. The eleven stale expectations are updated without weakening the required byte limits, dependency boundaries, source identity, alias deduplication or exact handoff refusal.

## Verdict

**PASS for source review** — no concrete code or Rust-style defect found. This is not runtime acceptance or merge readiness: the named titan tests have not run, and the evaluator-manifest test requires the product dependency fix in PR 3461 before execution can be green.

## Scope

```yaml
{
  "scope": [
    {
      "id": "FR-003-AC-4",
      "path": "spec/functional/FR-003-format-native-source.md",
      "role": "examined",
      "excerpt": "Output beyond its selected ceiling receives `resource_exhausted`/`input-bytes-exceeded`; its rendered diagnostic calls this a format output-byte limit, reports the configured bound and emitted UTF-8 byte count, names `format.output_bytes`, and does not present the emitted count as input or source bytes."
    },
    {
      "id": "FR-003-AC-5",
      "path": "spec/functional/FR-003-format-native-source.md",
      "role": "examined",
      "excerpt": "Exactly-at-ceiling output succeeds, including the final newline; zero or one-byte-short ceilings refuse without returning a partial string."
    },
    {
      "id": "FR-003-AC-6",
      "path": "spec/functional/FR-003-format-native-source.md",
      "role": "examined",
      "excerpt": "format(source) and format_with_limit(source, 1048576) return identical bytes. Source whose formatted output exceeds 1 MiB refuses under format(source) naming the limit, its value and format_with_limit, and formats in full under format_with_limit with a limit raised to fit it."
    },
    {
      "id": "FR-003-AC-7",
      "path": "spec/functional/FR-003-format-native-source.md",
      "role": "examined",
      "excerpt": "format and format_with_limit take a `fsl_cst::ParsedSource`. `fsl-cst/src/format.rs` depends on layer 1 and layer F only, and formatting complete-V1 source succeeds under AC-1 to AC-3."
    },
    {
      "id": "FR-003-AC-9",
      "path": "spec/functional/FR-003-format-native-source.md",
      "role": "examined",
      "excerpt": "For each complete-V1 fixture unit in the repository's tests that checks, checking its formatted source through the S1 to S4 spine yields a checked package identity equal to the original's, and formatting the formatted source again yields identical bytes."
    },
    {
      "id": "FR-003-AC-12",
      "path": "spec/functional/FR-003-format-native-source.md",
      "role": "examined",
      "excerpt": "Formatting `present (x)`, `forall (r in c : p)`, a parameter `(v : T)`, `Int [0,9]`, `always [0,30] holds (p)`, `convert < T > (e)`, `Option < Int[0,9] >` and `a<b` prints `present(x)`, `forall(r in c: p)`, `(v: T)`, `Int[0, 9]`, `always[0, 30] holds(p)`, `convert<T>(e)`, `Option<Int[0, 9]>` and `a < b`; `over (o: M::O)`, `when (w.l = 0)` and `as (r: Boolean)` keep one space before `(`; removing all whitespace from input and output gives equal bytes."
    },
    {
      "id": "FR-027-AC-9",
      "path": "spec/functional/FR-027-export-compiled-native-package.md",
      "role": "examined",
      "excerpt": "A `1-draft` request selecting a `semantic-ir/2.0.0` domain package document whose program declares `model M` by that document's identity writes exactly the bytes the FR-278 composition returns over the same source and package input. Their `model_root` node holds that package's identity, and FSL's I2 reader, given that package as domain package evidence, reads them back Verified. `M::Nope` refuses at stage `assembly` (`missing_declaration`) at `M::Nope`; a missing document or an identity that names no supplied document refuses at stage `intake` at the `model` declaration."
    },
    {
      "id": "FR-1040-AC-1",
      "path": "spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md",
      "role": "context_only",
      "excerpt": "Over FSpec FR-415-AC-15's `Call` model, `ref(self.x)` and `ref(self.q[0])` (under `size(self.q) >= 1`) in a postcondition write `self.r = ref(self.x)` and in a precondition `self.r = ref(self.q[0])` check as `PlaceReference` nodes typed `Reference<Clock>` and lower to `filament.op.clock_reference.ref` applications with `result_type` the `scalar_type`/`clock_reference` node, and the unit compiles with no `unsupported-feature` refusal."
    },
    {
      "id": "FR-1040-AC-7",
      "path": "spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md",
      "role": "context_only",
      "excerpt": "A pure function `same(a: Reference<Clock>, b: Reference<Clock>): Boolean { a = b }` has a function node whose parameters are typed by the `scalar_type`/`clock_reference` node, and a refinement row or clause calling it lowers its call to that node; with the function's body made ill-typed, the unit refuses once at the function body and not at the call."
    },
    {
      "id": "FR-1040-AC-8",
      "path": "spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md",
      "role": "context_only",
      "excerpt": "Over a domain package `acme/rpc` whose object type `Call` has clock fields `x` and `y` and a field `r` typed `ix://acme/rpc/ClockRef`, a type definition of `kind` `reference` with `target` `ix://filament/native/Clock`, intake gives `r` the type `Reference<Clock>` as a data member, and the postcondition `self.r = ref(self.x)` of an operation `point` whose frame names only `r` checks with no `clock-write` refusal and lowers the write to a `filament.op.clock_reference.ref` application typed by the `scalar_type`/`clock_reference` node, which also types the field; after `point` the state's `r` hold"
    },
    {
      "id": "FR-398-AC-8",
      "path": "spec/functional/FR-398-expose-the-admitted-unit-table-through-the-replay-facade.md",
      "role": "examined",
      "excerpt": "A source family declaring distinct derived dimensions over one base dimension and one root unit per derived dimension yields distinct admitted root keys with the caller's explicit package name. The context's retained nominal facts reconstruct the same FSV unit topology without creating a node key. Equal positive and negative root terms give a dimensionless target. With sufficiently long admitted package names, FSV's exact conversion completes but its `identified_quantity` handoff refuses `Identity(InputBytes)` at the fixed 16 MiB owner limit, yielding neither quantity nor identified unit and l"
    },
    {
      "id": "FR-480-AC-11",
      "path": "spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md",
      "role": "examined",
      "excerpt": "Through a second alias of the FR-360 package, in one unit and in a second unit, a variant case, a record field read, an object type and an operation context resolve to the first alias's declarations, and each variant type is declared once. A subtype's inherited fields and operation resolve through a second alias."
    },
    {
      "id": "FR-493-AC-3",
      "path": "spec/functional/FR-493-retire-the-native-package-runtime-front-end-and-protocol-artifact-surfaces.md",
      "role": "examined",
      "excerpt": "In the later front-end slice, a downstream crate cannot import the root package's `0-draft` parser entry points, arena `syntax`, `linking::native`, native `checking`, `model_source` or `located_json`; every Makefile target, `xtask` lane and example still builds and runs without them. The `composed` modules, `formal_source` and `native_model` remain importable until their own slices."
    },
    {
      "id": "FR-499-AC-5",
      "path": "spec/functional/FR-499-delete-each-retired-family-only-behind-its-named-successor.md",
      "role": "examined",
      "excerpt": "After QSL-868 and QSL-830, TC-1027 steps 2 and 3 pass, the CLI `parse` runs on `fsl_cst::parse` with its flags and exits unchanged, and the interim FR-362 items 8 and 9 wording is gone."
    },
    {
      "id": "FR-760-AC-1",
      "path": "spec/functional/FR-760-consume-the-standard-and-publish-the-facades-through-versioned-cargo-pins.md",
      "role": "examined",
      "excerpt": "`cargo metadata --locked` for the workspace lists `filament-specification-qualification` from `https://github.com/agent-ix/filament-specification` with a `tag` or full-sha `rev` source, and the published positive fixture `proposals/checked-package-v2/fixtures/positive-control-operations.json`, read from that pinned package, carries the `contract_version` FSL writes and reads (`fsl_package::CHECKED_PACKAGE_V2`)."
    },
    {
      "id": "ADR-011/6.1/layer-3",
      "path": "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md",
      "role": "examined",
      "excerpt": "2, F, C, SV, K; FCD crates from `model::intake` only"
    }
  ]
}
```

## Review evidence

- Read the full diff and the relevant AGENTS.md, CLAUDE.md, CONTRIBUTING.md, Cargo.toml lint policy and deny.toml. No workflow, production source, manifest or lockfile is changed by this PR.
- Both formatter tests retain full output equality, exactly-at-boundary success, too-small refusal, UTF-8 byte and final-newline checks. The compact expected spelling follows FR-003-AC-12; the macro scan adds only the standard-library vec macro actually used by format.rs.
- The evaluator fixture extractor reads the real package_of_with_mode format literal and still requires exactly two actual package_of declaration literals. Each fixture still goes through the existing real S1–S4 identity helper.
- The FCD scan still visits every Rust source file, requires at least one FCD use, and rejects every use outside model/intake.rs and model/intake/ child modules. The unchanged evaluator-manifest rule remains dependent on PR 3461.
- README extraction is restricted to the first-rule section and its actual fsl stage=parse fence. The deletion scan keeps its prohibited modules/symbols and uses retained complete/population_fixtures files as positive controls.
- Standard metadata still comes from locked Cargo resolution. The source selector rejects branch sources and non-40-hex revisions; tagged sources retain tag/version equality, revision sources retain numeric semantic-version checks, and published-file checks remain.
- The oversized identity case changes only emit.output_bytes for that compile. It still uses source-admitted root identities and requires a real completed conversion followed by HandoffRefusal::Identity(InputBytes) with bound 16777216, required greater than bound, no retained unit and no extra admission charge. The small-name and direct smaller-limit controls are unchanged.
- The alias fixture requires exactly ClockReference, History::ConfigVersion and History::Sub; it additionally checks the native filament/native / Clock effective identity. All three inherited clauses must still check. The canonical insertion at fsl-semantics/src/check/assemble.rs:3812–3869 agrees with the accepted decision and FR-1040.
- Source review found no new duplication, vendoring, stubs, ignored tests, conditional assertions that silently skip required behavior, unsafe code or production API changes.
- quire matrix --scope /home/peter/dev/worktrees/qsl-2813-stale-scans --format json exited 0. All twelve directly examined acceptance rows are tagged. Repository-wide symbol collisions, unmarked trace ids and section-matches-nothing warnings were emitted; this review does not claim repository-wide traceability or runtime assurance from that command.
- git diff --check origin/main...HEAD exited 0. The review artifact is validated at its explicit scratchpad path.
- No applicable AssuranceProfile is declared for this change under the repository spec tree.
- No build, test, compiled repository scanner or titan job was run or queued.

## Methods and limits

Applied code-review with the rust-review lane, rust-style API/implementation/testing rules, exact requirement-to-assertion comparison and computed trace reconciliation. A full quoin gap-analysis repository audit is not applicable to this test-only diff and was not performed under the caller's explicit scope restriction; production reverse-gap and stub audit claims are therefore not made. Spec review is not applicable because no spec artifact changes.

The runtime model identifier is unavailable in the harness; it is not guessed. Runtime gates remain pending titan release and the dependency on PR 3461.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Fix-round source disposition

The one fix round adds a three-file product correction after the existing root layering test progressed beyond its stale FCD scan and exposed a direct layer-4 dependency on layer 2. This disposition examines only fsl-package/Cargo.toml, fsl-package/src/protocol_view.rs and fsl-package/src/checked_views/hyper.rs. Original test assertions, their expected dependency tables and the original Findings section are unchanged.

**Source clear** — no new finding in the bounded fix. AccessOrdering and QuantifierKind are existing fsl_semantics::check re-exports of the same fsl_forms enums; this change does not copy types, alter nominal identity, add an adapter, or change serialization, parsing, match arms or runtime values. The direct forms dependency moves to the dev table; remaining source references are confined to cfg(test) modules or test-only module trees.

The lead reports the pre-fix twelve-case titan cohort as eleven passed and one failed, plus a workspace check exit 0. Those are historical pre-fix results: the root layering case and six direct protocol/memory/hyper consumers have been queued by the lead for the product correction. This reviewer has not run local builds or tests and does not claim those checks have passed at the fix head.

### Additional examined scope

```yaml
{
  "scope": [
    {
      "id": "ADR-011",
      "path": "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md",
      "role": "examined",
      "excerpt": "| 4 | `package` | S4, I2 byte reader (verification calls `library`) | 3, F, C, SV, K; `filament-contract-model` for the v2 wire contract: the typed structs the emitter builds and the IR reader the I2 reader delegates to (X-7) |"
    },
    {
      "id": "FR-519-AC-5",
      "path": "spec/functional/FR-519-publish-the-protocol-read-surface-the-filament-protocol-link-layer-imports.md",
      "role": "examined",
      "excerpt": "For each record the map marks read by an FSL view function, the view function over **R** returns the node, argument or lock member the map names, equal to the value the checked clause holds in memory."
    },
    {
      "id": "FR-940-AC-16",
      "path": "spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md",
      "role": "examined",
      "excerpt": "Over AC-4's Counter unit with the hyper clauses `Equal` (`forall trace a of M forall trace b of M { always holds(c.value @ a <= 3 and c.value @ b <= 3) }`, universal, copy-swap symmetric), `Partner` (forall-exists equality), `CanReachThree` (single existential) and a projection-aligned clause with an `align skip` list and a `match` block holding one universal and one existential conjunct, `hyper_clause_form` of each clause's `claim_name` occurrence returns equal records over the `AdmittedPackageRead` and over the `BarePackage` of its handoff directory"
    }
  ]
}
```

### Checks

- The layer-4 dependency row permits layers 3, F, C, SV and K plus the v2 wire owner, and the manifest now contains no normal or build fsl-forms edge.
- Attempt/Fence public carriers and the five accepted ordering words use the existing layer-3 AccessOrdering alias. Unknown words retain the same typed malformed refusal.
- Package hyper reconstruction uses the existing layer-3 QuantifierKind alias and still accepts exactly the same forall wire spelling; pending/refusal paths and atom/formula logic are unchanged.
- Existing cfg(test) source fixtures keep their direct forms dependency through the dev table. No new test-support feature forwarding or production wrapper is introduced.
- git diff --check for the bounded fix exits 0; no test/scanner/allow-list changes occur in the fix commit.
- The new product gap artifact records bounded static trace reconciliation separately. Native matrix/resource limitations are disclosed there; no engine-backed assurance or fresh runtime result is claimed.

## Dispositions

The original source review contained no real findings, only its required placeholder. There are therefore no FND outcomes to append. This round adds no finding. Runtime acceptance remains pending the lead's confined titan results.

## Confined dependency-expectation disposition

**Source clear, runtime conditional.** This pass examines only the latest delta in tests/it/family_outcome_layering.rs: exactly filament-observation, sha2 and unicode-normalization are added to the evaluator's sorted exact normal-dependency list, with the corresponding architecture comment updated. No production source, manifest, feature, lockfile, package-layer restriction or other test assertion changes in this delta.

The three additions have the arch ruling recorded by the lead. ADR-011 section 6.1 distinguishes external third-party crates from first-party layer edges and expressly retains per-crate exact dependency tables as stricter gates. Its X-8 FR-861 sentence expressly admits filament-observation as observation-authority owner, with its dependencies below layer 5. The PR body quotes both full sentences. The two third-party dependencies are listed individually rather than accepted through a blanket exemption.

The assert_eq on the complete sorted table is retained. An extra normal dependency still fails that equality. The build-dependency emptiness assertion and later workspace-crate edge checks are unchanged; fsl-forms remains absent from the evaluator's normal table and the package's prohibited direct forms edge remains removed by the already-reviewed product fix. The loop checks workspace members separately, so the named external observation owner is not accidentally treated as a new workspace layer exception.

The lead reports the preceding measured tree as sixteen of seventeen named cases passing and workspace check exit 0. Those are prior-tree results; the current delta awaits the queued compilation and single formerly failing exact case. No local build, test, scanner or matrix sweep was run by this reviewer. No new finding or unresolved source defect is identified; the original Findings section and all prior evidence remain unchanged.

### Additional examined scope for this disposition

```yaml
{
  "scope": [
    {
      "id": "ADR-011",
      "path": "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md",
      "role": "examined",
      "excerpt": "The column is the exhaustive allow-list of first-party layer edges; an external third-party crate is not a layer edge and this column does not govern it (for example, `filament-driver` uses `serde_json` to read FR-331 manifest bytes as data, ADR-013 T-7), except that a cell that restricts its external crates with \"only\" or \"without `std`\" (SV, WG) is exhaustive for that crate; a per-crate exact dependency table in a layering test is a stricter gate that governs where it exists."
    },
    {
      "id": "ADR-011/X-8",
      "path": "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md",
      "role": "examined",
      "excerpt": "FR-861 adds `filament-observation`, the observation-authority owner, as a git dependency on the repository `agent-ix/filament-observation` at `branch = \"main\"`, resolved through the workspace patch that keeps one `fsl-foundation` in the graph; its own dependencies (`fsl-foundation`, `filament-exact`, `quire-canonical`, serialization crates) are all below layer 5."
    }
  ]
}
```

### Disposition-pass outcome

The existing artifacts contain no real FND rows to close, and this bounded pass adds none. The source disposition is clear; runtime acceptance remains conditional until the lead receives the current-head titan result.
