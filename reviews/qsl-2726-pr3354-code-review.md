---
id: SR-20261121003354001
title: "QSL-2726 package remainder code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2726; frozen PR3354 actual 55-path diff"
review_set: subset
---

## Summary

Reviewed frozen head `178791a0377722d5fd12263511dd5d8a824df52c`, PR filament-spec-language#3354, ticket QSL-2726. Actual merge base is `f89c2e03082342f690be623f6181977c4453d8ed`, also merge eb5ed935b542fc54b20a95c991d5f1838a74cc72 second parent. This actual 55-path PR diff is +5890/-554 after rename detection; the dispatch/handoff incorrectly called its base 660a550b40. No spec or CI files change in this PR.

## Verdict

**CONDITIONAL** — one low Rust error-classification rule violation; no demonstrated production result defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Duplicate-sources classification branches on serde diagnostic prose instead of a typed structural condition. The same prefix match is duplicated in command.rs:3873; route duplicate detection through the structural preflight and preserve other JSON errors unchanged. | fsl-command/src/command/runtime_sources.rs:86 |

## Methods and limits

API/docs: reviewed new source/region/body and model witness APIs and their real cross-crate consumers. Safety/concurrency: input-to-index/slice/refusal paths and shared file/byte/progress/work bounds checked; no new unsafe or asynchronous task lifecycle. Wire: closed five-field source carrier, pre-I/O validation, native-pair matching, original constructor/S0/IO errors, mandatory violated model witness and strict native arm reviewed. Model keys and references come from actual replay; current FR-267 reference exception is intentional. Implementation: printer is a 94% unchanged move with quantity/visibility additions; no duplicated printer, no new stub, CI narrowing or dependency pin change. Tests: real compilation/admission/evaluation/replay seams, nonzero undefined aggregate, shared nodes, authored whitespace, source absence/UTF-8 bounds, progress/work limits and strict malformed model payloads inspected. Rust idiom references loaded for API, implementation, safety, wire and tests; Tokio does not apply.

Unproven concern, not a demonstrated runtime defect: `Context::attribution` computes a Holds-node source_map pointer even if `source_at_location_within` fails on a descendant application. The failure type currently carries a reason without the actual descendant node. Source inspection establishes the mismatch in error provenance but not admission/reachability of a malformed descendant metadata fixture. No admitted mutation or runtime result was available; the historical global-singleton issue has changed and is not re-reported as current. The changed refusal path needs the AC-11 behavioral evidence described in the gap artifact. SourceAttributionInvariant retains an unexpected Body payload as non-wire, without a fake budget denial.

## Coverage

Plan completion: not assessed

Semantic review: ran by explicit dispatch authorization over caller sources, Claim-scoped naming/refusal loci, canonical atom identities, finite-window undefined metadata, decisive walks, and native model production/read/replay. Scope is this PR, not a whole-repository runtime or completion claim.

Commands: quoin 0.36.8 at /home/peter/.npm-global/bin/quoin; quire 0.52.0 (engine 0.62.2) at /home/peter/.npm-global/bin/quire; machine terra. `quoin matrix --repo . --json` exited 1 with `CORE_REFUSED: request exceeds the accepted size`; retained quoin-matrix.stderr/json. Authorized current static fallback `quire matrix --scope . --format json` exited 0; `quire coverage --scope . --json` exited 0. Retained complete JSON and stderr beside this artifact. No old binary, cached matrix, or invented runtime evidence was substituted.

Engine-wide static inventory only: 4886 tagged, 1124 untagged, 81 tagged-by-ignored-test, 105 method-without-symbol, 13 retired. These are repository scanner facts, not newly introduced defects or execution results. Review reports the two scoped untagged criteria below; unrelated repository assurance debt was not attributed to this PR. Coverage census: 13698 Rust candidates, 13128 tagged, 12607 bound. Diagnostic streams contain duplicate fixture/cfg symbol collisions and other repository-wide extraction warnings.

Material extraction limit: FR-283-AC-18/19 and FR-501-AC-31/32 exist in the checked-in spec but are omitted from the computed matrix: they follow a blank line after their acceptance table without a fresh header/separator. Coverage consequently reports their existing source tags as untracked; this is not proof that the criteria do not exist. Their literal requirements, tests and implementation were inspected manually in addition to the engine result. No complete engine-backed coverage or global PASS is claimed. No specification edit or new feedback ticket was made.

Reverse inventory: caller source carrier/preflight/intake; package source-aware call; package sampled/snapshot progress routes; occurrence/body/scoped source resolver; source/package atom rendering; source canonical atom identity; undefined-atom metadata; decisive projection walk; model binding/terminal-key capture; native model writer/reader; replay transition matching. All have governing FR-1039/100/026/940/283/501/931/330/267/128 requirements; no changed unowned behavior or hollow source/test stub identified. The known package aggregate Pending guard and native EffectiveId/UTF-8 exception are explicitly retained, not claimed complete.

Evidence custody: supplied six cohorts total 26 tests but have different heads/features. Only checked-handoff root filter 6/0 was retained on final178791; aggregate default6/0 at4d06, model default5/0 atbd5, model reader default6/0, quantity default2/0 and occurrence helper default1/0 at their reported earlier revisions. Reviewer inspected their scope and actual assertions; reviewer ran no Cargo, build, formatting, clippy, cargo-deny or aggregate gate per dispatch. Root owns final service gate. No full-workspace result is claimed.

## Examined units

```yaml
[
  {
    "id": "FR-1039-AC-8",
    "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
    "role": "examined",
    "excerpt": "A package request admits absent `sources`, `[]`, and an ordered array of exactly the five-field records above. Every malformed carrier and constructor-invalid value in the caller source table refuses with its stated request-stage code, member path and exit 20 before any selected file opens, including when `package` names an unavailable file. Independently replacing `document` with `\"\"` and `formal_revision` with `0` gives outer `invalid-identifier`, stage `request`, exit 20, with `/details/code` respectively `invalid_identifier` and `invalid_source_revision`, and `/details/path` naming the cor"
  },
  {
    "id": "FR-1039-AC-9",
    "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
    "role": "examined",
    "excerpt": "With valid package and model inputs, each explicit source file is read and admitted once in array order before member execution. An unavailable source file gives outer command code `io-error` at stage `file`, exit 20; invalid UTF-8, BOM, NUL and each blank native label preserve FR-001's source/S0 code, cause and offered-source details. These failures produce no member outcome and are distinct from missing-region-source refusals. FR-026's shared intake ceilings apply without an additional source limit."
  },
  {
    "id": "FR-1039-AC-10",
    "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
    "role": "examined",
    "excerpt": "Source and package runs of TC-1295 rows 41 and 42 use the same admitted source bytes and native references: timestamped and progressed inputs, with and without a ledger, render non-empty decision support with the exact whole atom `holds( p )`. Removing or changing its reference, shortening the admitted source below the region, or placing a region endpoint inside a UTF-8 scalar gives `SourceNotSupplied`, `invalid_runtime_input`/`missing-member` naming `sources`, stage `call`, exit 20, source identity and clause, no clause record. Handoff source bytes cannot satisfy the request."
  },
  {
    "id": "FR-1039-AC-11",
    "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
    "role": "examined",
    "excerpt": "A package call refusal and a refused source-native condition with a region locus render the same code, cause, fields and native source span as their source runs, with `file` equal to the caller-supplied authored path even when it differs from the compiler's path. An omitted or wrong native reference, a short source or a non-UTF-8-boundary endpoint gives the caller-source refusal in AC-10 at stage `call`, exit 20, with no rendered member outcome. Different valid formal metadata does not change native region matching or atom text."
  },
  {
    "id": "FR-1039-AC-12",
    "path": "spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md",
    "role": "examined",
    "excerpt": "A package TL lowering with absent `sources`, `[]` and valid caller sources produces the same canonical atom text from the package projections as FR-931 specifies. Caller sources neither change package evaluation nor supply libraries, imports or bindings; no adapter discovers handoff source bytes, compiles caller sources or authenticates caller labels against their bytes."
  },
  {
    "id": "FR-267-AC-30",
    "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
    "role": "examined",
    "excerpt": "FR-330-AC-19's model violations write canonical `native-run-result/2` documents and read back equal typed model-arm counterexamples, preserving initial index, typed binding under the current native reference exception, fairness, transition identities, state keys, loop boundary, interval and report `trace_position`; a later failing activation keeps its nonzero position. The initially terminal case round-trips its actual terminal key. A model `Undefined` document with no counterexample reads back with its reason and position; any carried undefined witness round-trips the model state-key envelope"
  },
  {
    "id": "FR-267-AC-31",
    "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
    "role": "examined",
    "excerpt": "The strict reader refuses AC-30's model document with `initial_state` null, a decimal string, negative or above the `Count` maximum; with null or malformed typed-reference binding; with a document or sample step in either prefix or loop; with a mixed step, unknown member, missing transition member or malformed base64url state key; or with `trace_position` unequal to the report position. It refuses a carried undefined model envelope with `document` or `sample` instead of `state_key`, and AC-13's observed document with a model step, non-null initial state or non-null binding. Removing `counterex"
  },
  {
    "id": "FR-267-AC-32",
    "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
    "role": "examined",
    "excerpt": "AC-30's actually produced model binding and transition references strictly read and round-trip with `universe` a `NodeId`, `object_type` an `EffectiveId` and `identity` UTF-8 text (`a` in the Counter fixture), including references nested in typed arguments. Independently deleting or adding a reference member, giving `universe` the wrong kind, replacing `object_type` with a backend `NodeId`, corrupting its effective identity, or making `identity` non-text refuses naming the offending member in each reference position. Substituting FR-650's run-value wrapper refuses. These native shape assertion"
  },
  {
    "id": "FR-330-AC-12",
    "path": "spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md",
    "role": "examined",
    "excerpt": "`Bounded`'s violation of AC-1 over `Finite` snapshots `c.value` 0, 1, 2 reports a `decisive` with `position` 0, no `quantifier` and no `index`, `element` `false` and the empty `value_path`; a clause `always holds(c.value <= 1)` `on origin` under the infinite-trace profile over the same snapshots reports the violation at position 0 and a `decisive` with `position` 2; a clause `always holds(forall i in self.items : i > 0)` `on origin` over snapshots whose positions 0 and 1 hold `items` 1, 2, 3 and whose position 2 holds `items` 3, 0, 5 reports the violation at position 0 and a `decisive` with `p"
  },
  {
    "id": "FR-330-AC-19",
    "path": "spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md",
    "role": "examined",
    "excerpt": "AC-16's `Model` violation through source and checked-package execution carries equal model-arm counterexamples: non-null initial index `0`, the actual typed `over` reference, checked fairness and the validated transition/post-state sequence with its original loop boundary; `trace_position` equals the report position and `decisive.position` remains 2. A clause whose first failing activation is later than 0 keeps that later `trace_position`. A terminal initial state with no prefix steps carries one terminal-stutter loop step keyed by that actual initial state. A model `Undefined` run preserves i"
  },
  {
    "id": "FR-283-AC-18",
    "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
    "role": "examined",
    "excerpt": "On both source and checked-package routes, TC-1295 rows 45–47 give exact whole caller text and exact expression locus for a unique candidate, identical caller text and the actual coarser claim declaration locus for shared byte-identical candidates, and FR-931's literal canonical name `holds(p.ok)` and that declaration locus for shared differing spellings. The original verdict or refusal, category and support positions remain unchanged, with no attribution refusal for valid differing texts. Missing caller sources and invalid selected spans retain `SourceNotSupplied`; no handoff fallback supplie"
  },
  {
    "id": "FR-283-AC-19",
    "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
    "role": "examined",
    "excerpt": "TC-1295's aggregate controls retain exact caller text for unique and byte-identical regions and render differing shared regions by the canonical aggregate assembly above, including omitted versus explicit `min_count 1` and equivalent reformatted duration bounds. Both routes use the actual declaration locus for shared regions and retain the original evaluator outcome and aggregate counts. Changing a window, filter, threshold, kind or normalized min_count changes the checked aggregate and is not collapsed by label identity; missing source or invalid spans retain `SourceNotSupplied`, never a cano"
  },
  {
    "id": "FR-501-AC-31",
    "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
    "role": "examined",
    "excerpt": "TC-1295 rows 45–47, run over both source and checked-package routes with equal caller sources, preserve the false verdict, `closed-scope` basis and position-1 decision support while rendering a unique atom as exact `holds( p.ok )` with its expression locus, byte-identical shared atoms as that text with the actual claim declaration locus, and differing shared spellings as exact FR-931 canonical `holds(p.ok)` with that declaration locus. Neither shared case chooses a candidate by rank or invents a per-use locus; valid differing text adds no attribution refusal."
  },
  {
    "id": "FR-501-AC-32",
    "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
    "role": "examined",
    "excerpt": "TC-1295's timestamped aggregate controls apply FR-283's unique, identical and differing-text rules to named aggregates and decision-support names on both routes, including normalized duration bounds and default min_count. Differing spellings use canonical aggregate assembly and the actual declaration locus, without attribution refusal or a new outcome row; the original evaluator result, basis, support positions and insufficient-data counts are unchanged."
  },
  {
    "id": "FR-931-AC-11",
    "path": "spec/functional/FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md",
    "role": "examined",
    "excerpt": "Over the `Low` fixture written `on origin { always [0, 3] holds( ( c.versionNumber<=2 ) ) }`, the `atoms` entry of `p0` is exactly `holds(c.versionNumber <= 2)`, the same bytes as for FR-931-AC-1's spelling. Over a variant whose `c` also has Boolean fields `a`, `b`, `p`, `q`, Integer fields `x`, `y` and a Text field `name`, a leaf written `holds(not(c.a=c.b) and ((c.x+1)-(c.y-2))>=0 implies c.p implies c.q)` prints `holds(not (c.a = c.b) and c.x + 1 - (c.y - 2) >= 0 implies c.p implies c.q)`, and a leaf written `holds(c.name = \"a\\u0041\\t\")` prints `holds(c.name = \"aA\\t\")`; and the same request"
  },
  {
    "id": "FR-931-AC-12",
    "path": "spec/functional/FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md",
    "role": "examined",
    "excerpt": "Over the `Low` unit with the ConfigVersion model `example/config-version` selected under the alias `Config`, with a source enumeration `enum Mode { On, Off }` declared by the unit (its `SourceOwner` identity `I`) and a state read `c.mode: Mode`, the leaves `holds(c.mode = Mode::On)` and `holds(size(allInstances<Config::ConfigVersion>(Config::P)) >= 1)` (`P` the model's population of `ConfigVersion`) give the atom texts `holds(c.mode = ix://I/Mode::On)` and `holds(size(allInstances<ix://example/config-version/ConfigVersion>(ix://example/config-version/P)) >= 1)`; the same unit with the alias re"
  },
  {
    "id": "FR-931-AC-18",
    "path": "spec/functional/FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md",
    "role": "examined",
    "excerpt": "Route-neutral lowering input. Each of FR-931-AC-1's two requests, FR-931-AC-2's three `Low` requests and its past-only clause's default, and FR-931-AC-9's requests without `cross_evaluate` (clause `C`, its `mltl.origin-complete-history/v1` request and the two `dimensions` `[\"infinite-trace\"]` clauses), sent over the unit's checked package selected as `package` `{\"file\": P, \"equality\": E}` (FR-1039, `E` its FR-517 handoff's `equality.json`) with the source removed, returns the record the source route returns, byte-equal, with the same exit code (0 or 21). A unit declaring `Low` and a second cla"
  },
  {
    "id": "FR-026-AC-8",
    "path": "spec/functional/FR-026-run-standalone-native-workflow.md",
    "role": "examined",
    "excerpt": "A package request whose dependent files, including every `sources` file, total exactly `dependent_bytes` runs; one additional byte settles incomplete with `resource_exhausted`, exit 22, naming `dependent_bytes` and its configured value. A valid package request selecting 64 files across all roles runs, while adding a 65th through `sources` settles incomplete with `resource_exhausted`, exit 22, naming the selected-file maximum 64. Padding the same request's JSON to exactly `request_bytes` runs; one extra byte settles incomplete with `resource_exhausted`, exit 22, naming `request_bytes`. The exis"
  },
  {
    "id": "Cargo.lock",
    "path": "Cargo.lock",
    "role": "examined",
    "excerpt": "# This file is automatically @generated by Cargo."
  },
  {
    "id": "Cargo.toml",
    "path": "Cargo.toml",
    "role": "examined",
    "excerpt": "[workspace]"
  },
  {
    "id": "fsl-analyze/src/monitor.rs",
    "path": "fsl-analyze/src/monitor.rs",
    "role": "examined",
    "excerpt": "//! FR-283: the `monitor` operation. It checks a supplied trace offline"
  },
  {
    "id": "fsl-analyze/tests/it/main.rs",
    "path": "fsl-analyze/tests/it/main.rs",
    "role": "examined",
    "excerpt": "//! Public API tests for the unreduced graph core and the model checker."
  },
  {
    "id": "fsl-analyze/tests/it/monitor.rs",
    "path": "fsl-analyze/tests/it/monitor.rs",
    "role": "examined",
    "excerpt": "//! FR-283 (TC-765, TC-766): `monitor` over the `Counter` subject. Each trace"
  },
  {
    "id": "fsl-analyze/tests/it/monitor_aggregate_rendering.rs",
    "path": "fsl-analyze/tests/it/monitor_aggregate_rendering.rs",
    "role": "examined",
    "excerpt": "//! FR-283: aggregate names come from actual caller regions or checked components."
  },
  {
    "id": "fsl-analyze/tests/it/monitor_incremental.rs",
    "path": "fsl-analyze/tests/it/monitor_incremental.rs",
    "role": "examined",
    "excerpt": "//! FR-643 (TC-1666, TC-1667): the stepping monitor over a record trace"
  },
  {
    "id": "fsl-command/src/command.rs",
    "path": "fsl-command/src/command.rs",
    "role": "examined",
    "excerpt": "//! Local orchestration for the `1-draft` spine compiler and runner."
  },
  {
    "id": "fsl-command/src/command/output.rs",
    "path": "fsl-command/src/command/output.rs",
    "role": "examined",
    "excerpt": "//! FR-026: construct typed JSON views of native outcomes and actual provenance."
  },
  {
    "id": "fsl-command/src/command/package_run.rs",
    "path": "fsl-command/src/command/package_run.rs",
    "role": "examined",
    "excerpt": "//! FR-100, FR-945, FR-1039: a `native-run/1` request that selects an existing"
  },
  {
    "id": "fsl-command/src/command/runtime_sources.rs",
    "path": "fsl-command/src/command/runtime_sources.rs",
    "role": "examined",
    "excerpt": "//! FR-1039: closed caller source selections, validated before selected-file I/O."
  },
  {
    "id": "fsl-command/src/command/tl.rs",
    "path": "fsl-command/src/command/tl.rs",
    "role": "examined",
    "excerpt": "//! FR-757, FR-931: the `tl_lowering` request member. A native temporal"
  },
  {
    "id": "fsl-command/src/command/tl/package_observed.rs",
    "path": "fsl-command/src/command/tl/package_observed.rs",
    "role": "examined",
    "excerpt": "//! FR-931 \"Cross-evaluation\" over a selected checked package: the native"
  },
  {
    "id": "fsl-command/src/command/wire.rs",
    "path": "fsl-command/src/command/wire.rs",
    "role": "examined",
    "excerpt": "//! FR-026–028, FR-031: closed records; existing constructors own identifier validity."
  },
  {
    "id": "fsl-eval/src/simulation/model/letters.rs",
    "path": "fsl-eval/src/simulation/model/letters.rs",
    "role": "examined",
    "excerpt": "//! FR-125 Instancing, FR-107: the production [`ModelLetters`] of a checked"
  },
  {
    "id": "fsl-eval/src/temporal/finite_window/outcome.rs",
    "path": "fsl-eval/src/temporal/finite_window/outcome.rs",
    "role": "examined",
    "excerpt": "//! What the finite-window evaluator returns (FR-501 \"Outputs\", \"Result"
  },
  {
    "id": "fsl-eval/src/temporal/finite_window/run.rs",
    "path": "fsl-eval/src/temporal/finite_window/run.rs",
    "role": "examined",
    "excerpt": "//! The finite-window evaluator (FR-501): demand each activation's node"
  },
  {
    "id": "fsl-eval/src/temporal/mod.rs",
    "path": "fsl-eval/src/temporal/mod.rs",
    "role": "examined",
    "excerpt": "//! The layer-5 TemporalTrace evaluator (FR-327, FR-328, FR-329, FR-125):"
  },
  {
    "id": "fsl-eval/src/temporal/observed.rs",
    "path": "fsl-eval/src/temporal/observed.rs",
    "role": "examined",
    "excerpt": "//! FR-330 (FR-327, FR-328, FR-329): a temporal clause evaluated over"
  },
  {
    "id": "fsl-eval/src/value/expression/evaluate.rs",
    "path": "fsl-eval/src/value/expression/evaluate.rs",
    "role": "examined",
    "excerpt": "//! The metered evaluator of checked expressions and function calls."
  },
  {
    "id": "fsl-package/src/analysis_view.rs",
    "path": "fsl-package/src/analysis_view.rs",
    "role": "examined",
    "excerpt": "//! FR-940: the one sealed read view the analysis engines read a checked"
  },
  {
    "id": "fsl-package/src/bare_term.rs",
    "path": "fsl-package/src/bare_term.rs",
    "role": "examined",
    "excerpt": "//! FR-940 \"Package forms of the source-checked evaluators\": the `holds`"
  },
  {
    "id": "fsl-package/src/checked_v2.rs",
    "path": "fsl-package/src/checked_v2.rs",
    "role": "examined",
    "excerpt": "//! The I2 reader for admitted checked-package/v2 bytes (FSpec FR-322)."
  },
  {
    "id": "fsl-package/src/lib.rs",
    "path": "fsl-package/src/lib.rs",
    "role": "examined",
    "excerpt": "//! ADR-011 §6.1 layer 4, `package` (ADR-011 §7.3 X-7): the S4"
  },
  {
    "id": "fsl-package/src/occurrence_source.rs",
    "path": "fsl-package/src/occurrence_source.rs",
    "role": "examined",
    "excerpt": "//! FR-940: source attribution of an explicitly selected admitted occurrence."
  },
  {
    "id": "fsl-package/src/reader_protocol_projection.rs",
    "path": "fsl-package/src/reader_protocol_projection.rs",
    "role": "examined",
    "excerpt": "//! Reader-owned protocol expression nodes projected from admitted I2 terms"
  },
  {
    "id": "fsl-replay/src/canonical_atom.rs",
    "path": "fsl-replay/src/canonical_atom.rs",
    "role": "examined",
    "excerpt": "//! FR-931-AC-11/12: canonical atom labels from checked expressions. Source"
  },
  {
    "id": "fsl-replay/src/lib.rs",
    "path": "fsl-replay/src/lib.rs",
    "role": "examined",
    "excerpt": "//! `fsl-replay`: the ADR-011 §6.1 layer **6** crate (ADR-011 §7.3"
  },
  {
    "id": "fsl-replay/src/model_check/counterexample.rs",
    "path": "fsl-replay/src/model_check/counterexample.rs",
    "role": "examined",
    "excerpt": "//! FR-126 \"Counterexamples\" and FR-128 Inputs (ADR-018 CX-1, CX-2, DL-4,"
  },
  {
    "id": "fsl-replay/src/model_check/replay.rs",
    "path": "fsl-replay/src/model_check/replay.rs",
    "role": "examined",
    "excerpt": "//! FR-128 (ADR-018 CX-3, DL-5, UE-5, UE-6): [`replay_model_trace`], the"
  },
  {
    "id": "fsl-replay/src/outcome.rs",
    "path": "fsl-replay/src/outcome.rs",
    "role": "examined",
    "excerpt": "//! FR-286 (ADR-029 CB-4): the one `filament-outcome/1` JSON outcome document"
  },
  {
    "id": "fsl-replay/src/outcome/counterexample.rs",
    "path": "fsl-replay/src/outcome/counterexample.rs",
    "role": "examined",
    "excerpt": "//! The wire spelling of an `analyze` or `prove` item's counterexample"
  },
  {
    "id": "fsl-replay/src/spine.rs",
    "path": "fsl-replay/src/spine.rs",
    "role": "examined",
    "excerpt": "//! FR-027, FR-098 and FR-278: the spine (ADR-011 §5, §7.3 M-6a; ADR-013"
  },
  {
    "id": "fsl-replay/src/spine/call.rs",
    "path": "fsl-replay/src/spine/call.rs",
    "role": "examined",
    "excerpt": "//! FR-100: `fsl_replay::spine::run`, the spine `run` entry (ADR-011 §5)."
  },
  {
    "id": "fsl-replay/src/spine/clause.rs",
    "path": "fsl-replay/src/spine/clause.rs",
    "role": "examined",
    "excerpt": "//! FR-109: `fsl_replay::spine::run_clause`, the spine's clause-run"
  },
  {
    "id": "fsl-replay/src/spine/clause/clause_state/snapshot.rs",
    "path": "fsl-replay/src/spine/clause/clause_state/snapshot.rs",
    "role": "examined",
    "excerpt": "//! FR-820: a carried clause state over snapshot positions (`over` names a"
  },
  {
    "id": "fsl-replay/src/spine/clause/model_trace.rs",
    "path": "fsl-replay/src/spine/clause/model_trace.rs",
    "role": "examined",
    "excerpt": "//! FR-330 `TraceInput::Model`: a clause run over a model-arm behaviour."
  },
  {
    "id": "fsl-replay/src/spine/clause/temporal.rs",
    "path": "fsl-replay/src/spine/clause/temporal.rs",
    "role": "examined",
    "excerpt": "//! FR-330: the `Temporal` selection of [`super::run_clause`]. It resolves one"
  },
  {
    "id": "fsl-replay/src/spine/clause/temporal/package_selection.rs",
    "path": "fsl-replay/src/spine/clause/temporal/package_selection.rs",
    "role": "examined",
    "excerpt": "//! FR-1039, FR-330: a temporal clause a checked package carries, run over a"
  },
  {
    "id": "fsl-replay/src/spine/clause/temporal/package_window.rs",
    "path": "fsl-replay/src/spine/clause/temporal/package_window.rs",
    "role": "examined",
    "excerpt": "//! FR-283/501/504: package window evaluation and caller-source rendering."
  },
  {
    "id": "fsl-replay/src/spine/clause/temporal/source_render.rs",
    "path": "fsl-replay/src/spine/clause/temporal/source_render.rs",
    "role": "examined",
    "excerpt": "//! FR-283: names from all actual atom occurrences, with canonical shared labels."
  },
  {
    "id": "fsl-replay/src/spine/clause/tests/temporal.rs",
    "path": "fsl-replay/src/spine/clause/tests/temporal.rs",
    "role": "examined",
    "excerpt": "//! TC-840, TC-848 (FR-330): `run_clause`'s `Temporal` selection over"
  },
  {
    "id": "fsl-replay/src/spine/clause/tests/temporal/wire.rs",
    "path": "fsl-replay/src/spine/clause/tests/temporal/wire.rs",
    "role": "examined",
    "excerpt": "//! TC-1160 (FR-267 \"Temporal disposition wire\", FR-266-AC-4, FR-285-AC-7):"
  },
  {
    "id": "fsl-replay/src/spine/native_result.rs",
    "path": "fsl-replay/src/spine/native_result.rs",
    "role": "examined",
    "excerpt": "//! FR-267: canonical clause-run result production from the FR-109 report."
  },
  {
    "id": "fsl-replay/src/spine/native_result/temporal.rs",
    "path": "fsl-replay/src/spine/native_result/temporal.rs",
    "role": "examined",
    "excerpt": "//! FR-267 \"Temporal disposition wire\": the closed `disposition.temporal`"
  },
  {
    "id": "fsl-replay/src/spine/native_result/temporal/model_read.rs",
    "path": "fsl-replay/src/spine/native_result/temporal/model_read.rs",
    "role": "examined",
    "excerpt": "//! Structural model-arm members of FR-267's temporal result reader."
  },
  {
    "id": "fsl-replay/src/spine/package_call.rs",
    "path": "fsl-replay/src/spine/package_call.rs",
    "role": "examined",
    "excerpt": "//! FR-945, FR-1039: run a function of a selected checked package from its"
  },
  {
    "id": "fsl-replay/src/witness.rs",
    "path": "fsl-replay/src/witness.rs",
    "role": "examined",
    "excerpt": "//! FR-070 (ADR-013 O-25): the typed counterexample/witness envelope."
  },
  {
    "id": "fsl-replay/src/witness/derivation.rs",
    "path": "fsl-replay/src/witness/derivation.rs",
    "role": "examined",
    "excerpt": "//! FR-265 (ADR-031 SW-1 to SW-4, SW-9): derive a state clause's settlement"
  },
  {
    "id": "fsl-semantics/src/check/lowering/temporal.rs",
    "path": "fsl-semantics/src/check/lowering/temporal.rs",
    "role": "examined",
    "excerpt": "//! FR-337: the `temporal`/`temporal_clause` node each emitted temporal"
  },
  {
    "id": "fsl-semantics/src/check/mod.rs",
    "path": "fsl-semantics/src/check/mod.rs",
    "role": "examined",
    "excerpt": "//! ADR-011 §7.3 M-5 (FR-068): the layer-3 checking half of what was"
  },
  {
    "id": "fsl-semantics/src/check/temporal_clause.rs",
    "path": "fsl-semantics/src/check/temporal_clause.rs",
    "role": "examined",
    "excerpt": "//! FR-326: the `TemporalTrace` family's S3 check of one `temporal` clause"
  },
  {
    "id": "tests/it/main.rs",
    "path": "tests/it/main.rs",
    "role": "examined",
    "excerpt": "//! Single integration-test binary. Every module below was"
  },
  {
    "id": "tests/it/package_model_native.rs",
    "path": "tests/it/package_model_native.rs",
    "role": "examined",
    "excerpt": "//! Source-produced model witnesses use actual simulated states on the native wire."
  },
  {
    "id": "tests/it/request_surface_s1_16.rs",
    "path": "tests/it/request_surface_s1_16.rs",
    "role": "examined",
    "excerpt": "//! Request-surface stubs for model-based testing (FR-752), the planner"
  },
  {
    "id": "rust-style/error-classification",
    "path": "/home/peter/.codex/plugins/cache/agent-ix/dev-tools/0.3.2/skills/rust-style/SKILL.md",
    "role": "examined",
    "excerpt": "- Use `Option` for genuine absence, `Result` for recoverable failures, and a\n  distinct typed condition for each failure a caller must distinguish. Each\n  crate that surfaces errors uses a stable `Copy` code enum and a `thiserror`\n  envelope; never branch on human-readable error text."
  }
]
```

## Reviewed paths

- `Cargo.lock`
- `Cargo.toml`
- `fsl-analyze/src/monitor.rs`
- `fsl-analyze/tests/it/main.rs`
- `fsl-analyze/tests/it/monitor.rs`
- `fsl-analyze/tests/it/monitor_aggregate_rendering.rs`
- `fsl-analyze/tests/it/monitor_incremental.rs`
- `fsl-command/src/command.rs`
- `fsl-command/src/command/output.rs`
- `fsl-command/src/command/package_run.rs`
- `fsl-command/src/command/runtime_sources.rs`
- `fsl-command/src/command/tl.rs`
- `fsl-command/src/command/tl/package_observed.rs`
- `fsl-command/src/command/wire.rs`
- `fsl-eval/src/simulation/model/letters.rs`
- `fsl-eval/src/temporal/finite_window/outcome.rs`
- `fsl-eval/src/temporal/finite_window/run.rs`
- `fsl-eval/src/temporal/mod.rs`
- `fsl-eval/src/temporal/observed.rs`
- `fsl-eval/src/value/expression/evaluate.rs`
- `fsl-package/src/analysis_view.rs`
- `fsl-package/src/bare_term.rs`
- `fsl-package/src/checked_v2.rs`
- `fsl-package/src/lib.rs`
- `fsl-package/src/occurrence_source.rs`
- `fsl-package/src/reader_protocol_projection.rs`
- `fsl-replay/src/canonical_atom.rs`
- `fsl-replay/src/lib.rs`
- `fsl-replay/src/model_check/counterexample.rs`
- `fsl-replay/src/model_check/replay.rs`
- `fsl-replay/src/outcome.rs`
- `fsl-replay/src/outcome/counterexample.rs`
- `fsl-replay/src/spine.rs`
- `fsl-replay/src/spine/call.rs`
- `fsl-replay/src/spine/clause.rs`
- `fsl-replay/src/spine/clause/clause_state/snapshot.rs`
- `fsl-replay/src/spine/clause/model_trace.rs`
- `fsl-replay/src/spine/clause/temporal.rs`
- `fsl-replay/src/spine/clause/temporal/package_selection.rs`
- `fsl-replay/src/spine/clause/temporal/package_window.rs`
- `fsl-replay/src/spine/clause/temporal/source_render.rs`
- `fsl-replay/src/spine/clause/tests/temporal.rs`
- `fsl-replay/src/spine/clause/tests/temporal/wire.rs`
- `fsl-replay/src/spine/native_result.rs`
- `fsl-replay/src/spine/native_result/temporal.rs`
- `fsl-replay/src/spine/native_result/temporal/model_read.rs`
- `fsl-replay/src/spine/package_call.rs`
- `fsl-replay/src/witness.rs`
- `fsl-replay/src/witness/derivation.rs`
- `fsl-semantics/src/check/lowering/temporal.rs`
- `fsl-semantics/src/check/mod.rs`
- `fsl-semantics/src/check/temporal_clause.rs`
- `tests/it/main.rs`
- `tests/it/package_model_native.rs`
- `tests/it/request_surface_s1_16.rs`

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Typed ProvisionsSeed duplicate_sources flag drives the duplicate-source refusal; both diagnostic-prose branches are removed. Malformed duplicate carrier precedence is covered in the passing prefix8 acceptance cohort. |

## Disposition pass 1

**PASS for the reviewed PR scope** — all original findings are fixed; no new findings. The original review verdict and Findings table above remain the historical initial record. Root final service acceptance remains pending.

Reviewed fix head `ff9887a265f23ef27e1535e91f0d4e9504208fb8`, same initial head `178791a0377722d5fd12263511dd5d8a824df52c`, actual base `f89c2e03082342f690be623f6181977c4453d8ed`.

The full 13-path fix diff from original178791 to910b was read, then the exact 910b→ff9887a fixture-only diff. The Population/Reference bridge reads existing admitted types plus selected exact object identities, preserves optional/zero/u64 maximum distinctions, attaches the existing population catalog to the bare walker, and retains the actual EffectiveViews from the same snapshot normalization. Command passes those same views; source population binding is neutrally factored through the shared existing algorithm. No second StateModel, new wire/IR carrier, dependency pin or repeated normalization was introduced. Original committed SR copies matched canonical bytes before this append. No new finding was identified. The descendant metadata error-pointer concern remains UNPROVEN, not a demonstrated runtime defect or an acceptance claim.

No Cargo/build/gate or runtime command was run by the reviewer; the actual focused results were supplied by Root and their retained logs inspected. Root owns the final service gate. No Linear query, read or write was performed during disposition1: hard cooldown through19:45 UTC. Exact disposition comment bodies and keys are queued locally; publication and readback remain pending cooldown clearance.

Acceptance evidence is cohort-specific: root checked-handoff prefix8 at `910b7b1982942124f19b591e1fb3728dc7ea2bf9` passed 8/0 failed/0 ignored (1373 filtered), exit0, `/tmp/qsl-2726-review1-final-prefix8.log`. The exact later diff to `ff9887a265f23ef27e1535e91f0d4e9504208fb8` only removed unused infinite-profile request clocks in the affected legacy fixture; production and prefix8 test code are unchanged. Legacy exact1 at ff9887a passed 1/0 failed/0 ignored (1380 filtered), exit0, `/tmp/qsl-2726-review1-final-lasso2.log`. Its earlier lasso1 setup RED was `Emit ClockSelection UnusedClockSelection(sensor)` before evaluation and is retained, not hidden. Separate bridge unit evidence passed1 at helper3901a43638a98bdbc53a3013e281bedd07839d26, `/tmp/qsl-2726-population-focused-7.log`; this is not an at-final-head result.

Current `quire trace --scope . --id FR-1039-AC-10 --json` and AC-11 counterpart both exited0 at910b, resolved true: respectively one and four structural verifies claims, no citations. These are binding facts, not runtime evidence. Full original matrix/coverage extraction limitations remain recorded above; no global assurance PASS is claimed.

```yaml
{
  "round": 1,
  "head": "ff9887a265f23ef27e1535e91f0d4e9504208fb8",
  "original_head": "178791a0377722d5fd12263511dd5d8a824df52c",
  "artifact_id": "SR-20261121003354001",
  "method": "code-review",
  "initial_comment_id": "9d537fcf-857c-439b-80bb-23a4ac874a01",
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "artifact_id": "rust-style/error-classification",
      "severity": "low",
      "original_finding": "Duplicate-sources classification branches on serde diagnostic prose instead of a typed structural condition. The same prefix match is duplicated in command.rs:3873; route duplicate detection through the structural preflight and preserve other JSON errors unchanged.",
      "excerpt": "- Use `Option` for genuine absence, `Result` for recoverable failures, and a\n  distinct typed condition for each failure a caller must distinguish. Each\n  crate that surfaces errors uses a stable `Copy` code enum and a `thiserror`\n  envelope; never branch on human-readable error text.",
      "reason": "Typed ProvisionsSeed duplicate_sources flag drives the duplicate-source refusal; both diagnostic-prose branches are removed. Malformed duplicate carrier precedence is covered in the passing prefix8 acceptance cohort.",
      "after_excerpt": "        if duplicate_sources {\n            RunCause::ClauseRequest {\n                member: \"/request/sources\".to_owned(),\n                reason: \"repeated member\".to_owned(),\n            }\n        } else {\n            RunCause::Json(error)\n        }\n    })?;\n    decoder.end()?;\n    validate_provisions("
    }
  ],
  "new_findings": []
}
```
