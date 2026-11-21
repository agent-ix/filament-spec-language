---
id: SR-20261121003328002
title: "base review of native temporal result rendering"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; QSL-2726; FR-267, FR-283, FR-501, TC-1160, TC-1295"
review_set: subset
---

## Summary

Ticket: QSL-2726; PR #3328. Frozen head a036c0a638df57212c335e8007e91275a1d1b64f; merge base 7303f8d88b6b0534c65d1ef2a9f6dbff33abd7ef. Five-file specification-only diff. Read-only source corroboration; no Cargo, runtime test, aggregate gate, pin change, or author-branch edit. quoin 0.36.8, quire 0.52.0 (engine 0.62.2), spec-artifacts-process v0.36.0. Explicit ISO validation: 5/5 grammar-clean, zero grammar findings; git diff --check exits 0. New runtime trace coverage remains pending implementation and is not claimed by this readiness review.

No applicable AssuranceProfile frontmatter exists in the local spec tree; direct review is authorized by the dispatch and repository guidance. Selection: base plus applicable failure-domain, EARS-conformance and scope-boundary lenses. No production diff means no gap-analysis; no Rust/Cargo diff means no Rust review. Integrity/dependency/evidence/risk-complexity/object-review/criterion-strength lenses are not separately run: artifact structure, real dependency contracts, declared verification method and object inventory are unchanged; only reference links to existing FRs are added. Required native boundary checks are included in scope-boundary. No Jev assessment is claimed.

## Verdict

CONDITIONAL — specification readiness for this method only. Overall PR is not mergeable while the base and failure-domain findings remain unresolved.

## Method

Reviewed all changed FR behavior, ACs and TC procedure/expected-result rows. The unique, identical-shared and differing-shared branches have distinct independent name/locus oracles; source/reference/span refusals retain their original class; semantic node identity is independent of label equality; the temporary native encoding remains closed and excludes FR-650 and backend schema forms. Missing/extra/wrong-kind/native-vs-backend and nested-reference controls are testable. One exact producer expectation is wrong (FND-001). Aggregate-domain incompleteness is recorded by the failure-domain lens. Existing Verification cell conventions are unchanged; runtime trace tags will be assessed in the implementation PR, not falsely reported complete here.

Source evidence: fsl-semantics/src/model/intake.rs:2963-2974 refuses a type identity not ix://{package}/<artifact id> and preserves that exact identity in DeclarationKey.node. fsl-semantics/src/model/key.rs:159-163 copies the original package/node into EffectiveId unchanged; projection.rs:858-880 serializes it unchanged. The actual existing model fixture fsl-eval/tests/it/model_simulation.rs:46-64 uses package test/counters and ident("Counter") = ix://test/counters/Counter, not Counter. A direct low-level constructor can manufacture the bare key but does not establish an actually admitted model report.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The exact produced Counter reference fixture uses original.node="Counter", but admitted model type identities are qualified ix://<package>/<artifact id> and effective_id_of preserves that declaration key. Ground the fixture and literal expectation in an actually admitted model/source unit; do not change the producer to satisfy this invented key. Correct the matching FR-267 example too. | spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md:84-87 |

## Examined scope

Synthetic line/part suffixes locate verbatim fragments; they are not new specification ids.

```yaml
{
  "clean": false,
  "scope": [
    {
      "id": "FR-267-line-362-part-1",
      "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
      "role": "examined",
      "excerpt": "  is one of: `missing-valuation` with `position`, `atom` (the atom name under\n  FR-283's caller-text/canonical rendering rule) and, for an instance-bound atom of an evidence call (FR-524),"
    },
    {
      "id": "FR-267-line-412-part-1",
      "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
      "role": "examined",
      "excerpt": "  and `TransitionIdentity` definitions specify the model members, subject\n  only to the current native reference exception below: non-null"
    },
    {
      "id": "FR-267-line-431-part-1",
      "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
      "role": "examined",
      "excerpt": "- **Current native temporal model reference encoding.** At model\n  `over_binding`, transition `receiver`, and references in transition\n  `arguments` (including recursively nested typed values), the writer and\n  strict reader SHALL use the existing native producer's closed object\n  `{\"type\":\"reference\",\"universe\":U,\"object_type\":E,\"identity\":S}`.\n  `U` is the structural checked-package `NodeId` `{package, path, ordinal}`\n  of the universe root. `E` is the native `EffectiveId`: its required\n  `original` is the declaration key `{package, node}` and its required\n  `owner_effect"
    },
    {
      "id": "FR-267-line-431-part-2",
      "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
      "role": "examined",
      "excerpt": "ive_type` is null or another such effective identity, under\n  the existing effective-identity grammar. `S` is the object's UTF-8\n  identity text, not hexadecimal identity octets. For example, a produced\n  `Counter` binding with object identity `a` has\n  `\"object_type\":{\"owner_effective_type\":null,\"original\":{\"package\":\"counter\",\"node\":\"Counter\"}}`\n  and `\"identity\":\"a\"`; `universe` remains the actual produced node id.\n  The same native shape SHALL be used coherently across model value\n  projection, outcome counterexamples and task-product transition wires.\n  This exception "
    },
    {
      "id": "FR-267-line-431-part-3",
      "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
      "role": "examined",
      "excerpt": "changes no other model-arm member or typed-value rule.\n  The strict native reader SHALL validate this shape alone, without\n  accepting both native and backend reference encodings. FR-650's native\n  call/run value `{reference:{universe,type,object_identity}}` is a\n  different grammar and does not authorize this temporal encoding.\n- **Remaining QSL-2726 canonical reference conversion.** The required end\n  state is FSpec FR-181's backend `TypedValue.reference`: `universe` and\n  `object_type` are declaration `NodeId`s and `identity` is lowercase\n  even-length hexadecimal identi"
    },
    {
      "id": "FR-267-line-431-part-4",
      "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
      "role": "examined",
      "excerpt": "ty octets. Authoritative package-bound\n  conversion and replay restoration remain a separate change after the\n  current model-arm reader/writer change, covering all existing reference\n  producers: model projection, outcome counterexamples and task-product\n  receivers and arguments, including nested references. Conversion SHALL\n  resolve the actual admitted declaration identities and identity bytes;\n  guessed node ids or merely hex-encoding the present identity text do not\n  establish that mapping. Until that conversion lands, the current native\n  model reference SHALL NOT b"
    },
    {
      "id": "FR-267-line-431-part-5",
      "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
      "role": "examined",
      "excerpt": "e claimed to conform to the backend schema's\n  `TypedValue.reference`. The native model-arm change SHALL remain\n  internally consistent with the current producers in the meantime; the\n  exception grants no general relaxation of the model contract."
    },
    {
      "id": "FR-267-line-733-part-1",
      "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
      "role": "examined",
      "excerpt": "| FR-267-AC-30 | FR-330-AC-19's model violations write canonical `native-run-result/2` documents and read back equal typed model-arm counterexamples, preserving initial index, typed binding under the current native reference exception, fairness, transition identities, state keys, loop boundary, interval and report `trace_position`; a later failing activation keeps its nonzero position. The initially terminal case round-trips its actual terminal key. A model `Undefined` document with no counterexample reads back with its reason and position; any carried undefined witness rou"
    },
    {
      "id": "FR-267-line-733-part-2",
      "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
      "role": "examined",
      "excerpt": "nd-trips the model state-key envelope and actual truncated prefix. AC-13's snapshot and FR-330-AC-18's sample documents retain their observed-arm forms. | Test (TC-1160) |"
    },
    {
      "id": "FR-267-line-735-part-1",
      "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
      "role": "examined",
      "excerpt": "| FR-267-AC-32 | AC-30's actually produced model binding and transition references strictly read and round-trip with `universe` a `NodeId`, `object_type` an `EffectiveId` and `identity` UTF-8 text (`a` in the Counter fixture), including references nested in typed arguments. Independently deleting or adding a reference member, giving `universe` the wrong kind, replacing `object_type` with a backend `NodeId`, corrupting its effective identity, or making `identity` non-text refuses naming the offending member in each reference position. Substituting FR-650's run-value wrapper "
    },
    {
      "id": "FR-267-line-735-part-2",
      "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
      "role": "examined",
      "excerpt": "refuses. These native shape assertions establish no backend-schema reference conformance and do not replace semantic replay authentication. | Test (TC-1160) |"
    },
    {
      "id": "FR-283-line-28-part-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "  - target: ix://agent-ix/filament-spec-language/FR-931\n    type: references"
    },
    {
      "id": "FR-283-line-230-part-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "name follows the atom rendering rule in Behavior: whole caller source text\nwhen one region or byte-identical candidate texts identify it, and FR-931's\ncanonical printed form when a shared node has differing candidate texts.\nA `holds` atom includes the whole `holds(...)`. An `InsufficientData` record's atom is its aggregate, such as"
    },
    {
      "id": "FR-283-line-236-part-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "`ConflictingValuation`) names, under that same rendering rule."
    },
    {
      "id": "FR-283-line-476-part-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "  `Indeterminate` reason's `atom`), the `monitor` operation SHALL apply the\n  following same rule on both source and checked-package routes and expose the name as\n  `MonitorRecord::atom()`. Candidate `Expression` regions are those of the\n  formula node within the selected claim declaration's region and source,\n  selected using the explicit `Claim` occurrence. The `monitor` operation SHALL\n  retain all such candidates. Declaration containment does not select\n  a first use.\n  - When exactly one valid candidate region exists, the `monitor` operation SHALL\n    return the exact "
    },
    {
      "id": "FR-283-line-476-part-2",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "caller text of the whole atom and that exact\n    region as its locus.\n  - When several candidate regions have byte-identical whole caller texts,\n    the `monitor` operation SHALL return that text and the selected claim\n    declaration's actual region as its coarser locus. An arbitrary candidate\n    is not the selected locus.\n  - When several candidate regions have differing caller texts, the\n    `monitor` operation SHALL return the checked node's canonical printed\n    form under FR-931's existing atom printer, shared with TL lowering,\n    and the selected claim declaration'"
    },
    {
      "id": "FR-283-line-476-part-3",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "s actual region as its locus.\n    The `monitor` operation SHALL retain the original evaluator outcome or\n    refusal unchanged. This valid case introduces no attribution refusal\n    or new disposition row.\n  Text SHALL come from caller `sources` matched by native source reference.\n  The report layer owns text retrieval and canonical printing; an\n  occurrence-source helper supplies admitted regions and occurrence identity,\n  never text. Source rank, formula-use pairing by order, label-string\n  deduplication and synthetic loci SHALL NOT select an occurrence. Atom\n  identity r"
    },
    {
      "id": "FR-283-line-476-part-4",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "emains the checked node's identity even when printed labels\n  are equal. The unique-region case SHALL NOT be universally canonicalized."
    },
    {
      "id": "FR-283-line-512-part-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "  These checks apply to every candidate text region the rule reads and the\n  actual selected locus region, including a coarser declaration locus.\n  Missing references, wrong references, short bytes and split UTF-8\n  endpoints retain this original `SourceNotSupplied` behavior; canonical\n  printing SHALL NOT supply missing caller text. Neither route discovers\n  source in a handoff or recompiles source to render a package report.\n\nExact per-use loci for repeated shared nodes remain QSL-2726 work requiring\nadmitted use-to-occurrence facts in a later dependency update. Declarati"
    },
    {
      "id": "FR-283-line-512-part-2",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "on\ncontainment and the existing `Claim` occurrence provide no such edge fact;\nthe coarser declaration locus above does not assert per-use attribution.\n"
    },
    {
      "id": "FR-283-line-584-part-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "| FR-283-AC-18 | On both source and checked-package routes, TC-1295 rows 45–47 give exact whole caller text and exact expression locus for a unique candidate, identical caller text and the actual coarser claim declaration locus for shared byte-identical candidates, and FR-931's literal canonical name `holds(p.ok)` and that declaration locus for shared differing spellings. The original verdict or refusal, category and support positions remain unchanged, with no attribution refusal for valid differing texts. Missing caller sources and invalid selected spans retain `SourceNotS"
    },
    {
      "id": "FR-283-line-584-part-2",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "upplied`; no handoff fallback supplies them. | Test |\n"
    },
    {
      "id": "FR-283-line-594-part-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "- FR-931: canonical atom printing for shared nodes with differing caller text."
    },
    {
      "id": "FR-501-line-6-part-1",
      "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
      "role": "examined",
      "excerpt": "  - target: ix://agent-ix/filament-spec-language/FR-931\n    type: references"
    },
    {
      "id": "FR-501-line-169-part-1",
      "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
      "role": "examined",
      "excerpt": "In each decision-support `(position, atom)` pair, both the source and\nchecked-package routes SHALL apply FR-283's atom rendering rule. The explicit\n`Claim` occurrence and its declaration region select all candidate\n`Expression` regions of the formula node in that declaration and source:\n\n- One valid candidate SHALL give the exact whole caller text and that exact\n  region as locus.\n- Several candidates with byte-identical whole caller texts SHALL give that\n  text and the selected claim declaration's actual region as a coarser locus,\n  never an arbitrary candidate's region.\n-"
    },
    {
      "id": "FR-501-line-169-part-2",
      "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
      "role": "examined",
      "excerpt": " Several candidates with differing caller texts SHALL give the checked\n  node's canonical printed form from FR-931's existing TL-lowering atom\n  printer and the selected claim declaration's actual region as locus.\n\nThe report layer SHALL obtain caller text from `sources` by native source\nreference and reuse that canonical printer; occurrence-source helpers supply\nregions and identity, never text. Both routes SHALL preserve FR-283's\n`SourceNotSupplied` refusal for a missing or wrong reference, short bytes or\nsplit UTF-8 span at each text region read or actual selected locus."
    },
    {
      "id": "FR-501-line-169-part-3",
      "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
      "role": "examined",
      "excerpt": " A handoff\nprovides no fallback text and rendering compiles no source. The differing-text\ncase SHALL retain the original verdict or refusal, basis and support positions\nwithout an attribution refusal or new row. Source rank, order-based use\npairing, label-string deduplication and synthetic loci SHALL NOT select a use.\nFR-931's TL atom IDs still follow first formula occurrence and checked atom\nidentity, not label equality; unique caller text remains exact. Exact per-use\nloci for a repeated shared node remain QSL-2726 work requiring admitted\nuse-to-occurrence facts in a later"
    },
    {
      "id": "FR-501-line-169-part-4",
      "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
      "role": "examined",
      "excerpt": " dependency update."
    },
    {
      "id": "FR-501-line-500-part-1",
      "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
      "role": "examined",
      "excerpt": "| FR-501-AC-31 | TC-1295 rows 45–47, run over both source and checked-package routes with equal caller sources, preserve the false verdict, `closed-scope` basis and position-1 decision support while rendering a unique atom as exact `holds( p.ok )` with its expression locus, byte-identical shared atoms as that text with the actual claim declaration locus, and differing shared spellings as exact FR-931 canonical `holds(p.ok)` with that declaration locus. Neither shared case chooses a candidate by rank or invents a per-use locus; valid differing text adds no attribution refusa"
    },
    {
      "id": "FR-501-line-500-part-2",
      "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
      "role": "examined",
      "excerpt": "l. | Test |\n"
    },
    {
      "id": "TC-1160-line-30-part-1",
      "path": "spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md",
      "role": "examined",
      "excerpt": "FR-327-AC-6, FR-330-AC-7, FR-267-AC-30, FR-267-AC-31 and FR-267-AC-32."
    },
    {
      "id": "TC-1160-line-79-part-1",
      "path": "spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md",
      "role": "examined",
      "excerpt": "11. Retain the actual producer's model references in step 9's reports. Assert\n    the explicit native shape at `over_binding`, transition receivers and\n    reference arguments, including references inside existing recursive\n    typed-value containers: exactly `type`, `universe`, `object_type` and\n    `identity`, with `type` `reference`, the actual universe `NodeId`, native\n    `EffectiveId` object type and identity UTF-8 text. The Counter object `a`\n    has `object_type` exactly\n    `{\"owner_effective_type\":null,\"original\":{\"package\":\"counter\",\"node\":\"Counter\"}}`\n    and `i"
    },
    {
      "id": "TC-1160-line-79-part-2",
      "path": "spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md",
      "role": "examined",
      "excerpt": "dentity` exactly `\"a\"`. Read and re-encode the produced documents.\n    Independently apply the missing/extra member, wrong-kind universe,\n    backend-node object type, malformed effective identity, non-text identity\n    and FR-650 wrapper mutations of FR-267-AC-32 at each reference position.\n    Preserve all other model-arm members in each mutation. Use independent\n    literal expectations for these fields, never a call to the producer as\n    the expected-value oracle."
    },
    {
      "id": "TC-1160-line-141-part-1",
      "path": "spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md",
      "role": "examined",
      "excerpt": "- Step 11: the actual native reference form strictly reads and round-trips\n  equal typed payloads and canonical bytes; each malformed reference refuses\n  at its offending member. These are native reader assertions. The current\n  native reference is not a backend `TypedValue.reference` conformance\n  fixture and SHALL NOT be submitted to the backend schema and reported as\n  passing it. QSL-2726's remaining package-bound canonical reference conversion\n  and replay restoration cover projection, outcome counterexamples and\n  task-product wires separately. Step 9's model invarian"
    },
    {
      "id": "TC-1160-line-141-part-2",
      "path": "spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md",
      "role": "examined",
      "excerpt": "ts and TC-523's\n  semantic replay authentication remain required."
    },
    {
      "id": "TC-1295-line-6-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "  - target: ix://agent-ix/filament-spec-language/FR-283\n    type: verifies"
    },
    {
      "id": "TC-1295-line-19-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "Verify FR-501-AC-1 to AC-26, FR-501-AC-31, FR-283-AC-18 and\nFR-330-AC-9 (the finite-window part): the"
    },
    {
      "id": "TC-1295-line-31-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "that changes unique authored atom text, invents a per-use locus for a shared\nnode, refuses valid differing spellings, or obtains text from a source the\ncaller did not supply."
    },
    {
      "id": "TC-1295-line-66-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "For rows 45–47, use the existing valid source fixture\n`tests/corpus/temporal/timestamped-window.native`: `record Pulse { ok: Boolean; }`,\nclause `StampedPulse`, binder `p: Pulse`, clock `sensor`, with its existing\ntimestamped-event profile and `timestamp_unit` `ms` selection. Replace only\nthe clause body as stated in the row. Supply samples stamped 0, 2, 5 whose\n`ok` values are true, false, true, `W` 5, `C(5)`, scope `Closed`. The body\nis `always[0,5]` over the stated atom or conjunction. Run each through the\nsource and emitted-package routes with equal caller sources, then"
    },
    {
      "id": "TC-1295-line-66-part-2",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": " through\ntheir command routes with row 43's explicit package source selection.\nCheck that the shared-node fixtures actually retain multiple `Expression`\nregions for one checked atom, selected by the explicit `Claim` occurrence\nand declaration containment; do not silently replace that condition with\ntwo distinct checked atoms. Assert exact regions against the admitted\nsource map, independently of the rendering helper. `holds(p.ok)` is the\nliteral canonical field-read expectation under FR-931's printer (the same\nfield-printing rule exercised by `holds(c.versionNumber <= 2)` i"
    },
    {
      "id": "TC-1295-line-66-part-3",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "n\n`tests/it/request_surface_s1_16.rs`); the expected name SHALL be this literal,\nnot a call to the implementation's printer.\n"
    },
    {
      "id": "TC-1295-line-131-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "| 45 | UniqueRegionExactText: `always[0,5] holds( p.ok )` | one `Expression` candidate for the atom in `StampedPulse`; the stated samples and progress | both routes name the atom exactly `holds( p.ok )` and use that exact expression region as locus; verdict `Completed(false)`, basis `closed-scope`, support exactly {(position 1, that checked atom)} |\n| 46 | SharedIdenticalText: `always[0,5] (holds( p.ok ) and holds( p.ok ))` | one shared atom with both whole caller texts byte-identical; the stated samples and progress | both routes name it exactly `holds( p.ok )` and use `St"
    },
    {
      "id": "TC-1295-line-131-part-2",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "ampedPulse`'s actual claim declaration region as the coarser locus, never either expression region; original false verdict, basis and support positions retained |\n| 47 | SharedDifferingText: `always[0,5] (holds( p.ok ) and holds(p.ok))` | one shared checked atom with differing caller text bytes; the stated samples and progress | both routes name it exactly `holds(p.ok)` by FR-931 and use `StampedPulse`'s actual claim declaration region as the coarser locus; original false verdict, `closed-scope` basis and position-1 support retained; no attribution refusal or additional res"
    },
    {
      "id": "TC-1295-line-131-part-3",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "ult row |"
    },
    {
      "id": "TC-1295-line-150-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "- Rows 45–47 assert both name and locus and equal source/package results.\n  Reversing candidate enumeration changes neither. Distinct checked atoms\n  remain distinct if their printed labels coincide; no test uses label\n  deduplication or source rank as identity. Apply rows 42 and 44's missing,\n  wrong-reference, short-byte and split-UTF-8 controls to the candidate text\n  regions and actual selected declaration locus; canonical printing gives\n  no caller-source fallback. A refused condition retains its original\n  refusal and category under the same attribution rule. These as"
    },
    {
      "id": "TC-1295-line-150-part-2",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "sertions\n  establish no exact per-use attribution for repeated nodes: that remains\n  QSL-2726's later admitted use-to-occurrence work."
    },
    {
      "id": "FR-931",
      "path": "spec/functional/FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md",
      "role": "context_only",
      "excerpt": "The atom text is `holds(` + `T(e)` + `)`, one line, with no line break,"
    },
    {
      "id": "FR-283",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "context_only",
      "excerpt": "An `InsufficientData` record's atom is its aggregate, such as\n`count over past[0, 9] min_count 2 >= 1`."
    }
  ],
  "findings": [
    {
      "fnd": "FND-001",
      "severity": "medium",
      "confidence": "high",
      "method": "spec-review/base",
      "check": "test-intent",
      "artifact_id": "TC-1160",
      "related_ids": [
        "FR-267",
        "FR-267-AC-32"
      ],
      "path": "spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md",
      "lines": "84-87",
      "excerpt": "    `EffectiveId` object type and identity UTF-8 text. The Counter object `a`\n    has `object_type` exactly\n    `{\"owner_effective_type\":null,\"original\":{\"package\":\"counter\",\"node\":\"Counter\"}}`\n    and `identity` exactly `\"a\"`. Read and re-encode the produced documents.",
      "finding": "The exact produced Counter reference fixture uses original.node=\"Counter\", but admitted model type identities are qualified ix://<package>/<artifact id> and effective_id_of preserves that declaration key. Ground the fixture and literal expectation in an actually admitted model/source unit; do not change the producer to satisfy this invented key. Correct the matching FR-267 example too."
    }
  ],
  "bindings": []
}
```

## Disposition pass 1

Assessed exact head 18de79b725817420d0f3dbb2f0dcbb59f352c649, fix range a036c0a638df57212c335e8007e91275a1d1b64f..18de79b725817420d0f3dbb2f0dcbb59f352c649. Current verdict: PASS, no unresolved findings and no new findings. This supersedes only the original verdict; the original finding and scope text above remain unchanged.

Reviewed the complete five-spec-file fix, not only the original finding locations. FR-283 now owns temporal aggregate assembly and restricts FR-931's complete atom printer to holds leaves. The six checked kinds have heads; value, predicate, filter and threshold reuse T; quantile and normalized duration magnitudes reuse the exact rational spelling; position bounds/min_count use I; comparison covers the five AggregateComparison variants. Duration unit identities and exact selected-clock conversion agree with FR-194 and FR-337 and the actual package_window/emit_with_clock fixture. Omitted min_count is the checked value 1. TC-1295 rows48-52 give independent literal names, true declaration loci, all kinds/optional components, shared-node assertions, default/equivalent-unit controls, candidate reversal, source/span refusal controls, and semantic mutations. Count0 under the false filter and count1 before min_count2 agree with window admission semantics, including fraction's admitted-position count. These are prescribed tests, not claims that the package aggregate evaluator or component printers have run or exist. Missing implementation remains ordinary implementation work. The Counter key now matches the actual admitted test/counters fixture and preserved EffectiveId mapping. No new defect found.

Evidence read: fsl-forms/src/syntax.rs:4060-4133 and temporal_clause.rs:525-646 (six kinds, five comparisons, optional filter); fsl-package/src/emit/tests/temporal_aggregate.rs sampled, package_window, package_unit and emit_with_clock plus exact-normalization/identity tests; FR-194 S3 default/count/family/unit rules; FR-337 Aggregate formula emission; FR-931 Names/Literals and canonical.rs:1240-1278 (currently specified quantity printer is still implementation work); fsl-eval/src/temporal/window.rs:512-547,793 (false fraction predicates still contribute admitted count); model_simulation.rs:46-64 (actual original key). No source/runtime changes or Cargo. ISO validation returns 5/5 docs grammar-clean and zero grammar findings; diff-check0; author worktree clean at the exact head.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Corrected TC-1160 and FR-267 to the actual admitted test/counters declaration key ix://test/counters/Counter; literal expectations remain independent of the producer. |

## Fix scope examined

```yaml
{
  "scope": [
    {
      "id": "FR-267-line-442-part-1",
      "path": "spec/functional/FR-267-write-run-results-as-native-run-result-2.md",
      "role": "examined",
      "excerpt": "  `Counter` binding with object identity `a` in the admitted `test/counters`\n  fixture (`fsl-eval/tests/it/model_simulation.rs`) has\n  `\"object_type\":{\"owner_effective_type\":null,\"original\":{\"package\":\"test/counters\",\"node\":\"ix://test/counters/Counter\"}}`"
    },
    {
      "id": "FR-283-line-231-part-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "when one region or byte-identical candidate texts identify it, and the\ncanonical printed form below when a shared node has differing candidate texts."
    },
    {
      "id": "FR-283-line-492-part-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "    form under FR-931's existing atom printer for `holds` leaves, or under\n    \"Canonical window-aggregate labels\" below for aggregate leaves,"
    },
    {
      "id": "FR-283-line-499-part-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "  The report layer owns text retrieval and canonical printing, reusing\n  FR-931's expression, literal and identity printers; an"
    },
    {
      "id": "FR-283-line-564-part-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "### Canonical window-aggregate labels\n\nWhen the differing-text branch names a window aggregate, the report layer SHALL\nassemble its canonical label from the checked aggregate and its checked\nexpressions under the following rule, on both source and checked-package\nroutes. This defines aggregate label assembly; FR-931's TL lowering still\nhas no aggregate proposition and gains none from this rule.\n\nThe label has one of these heads, chosen by the checked aggregate kind:\n\n| Kind | Head |\n| --- | --- |\n| count | `count` |\n| sum, min, max | the kind word followed by ` of ` and `T("
    },
    {
      "id": "FR-283-line-564-part-2",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "value)` |\n| fraction | `fraction holds(` + `T(predicate)` + `)` |\n| quantile | `quantile ` + `R(q)` + ` of ` + `T(value)` |\n\nHere `T` is FR-931's checked-expression printer with the selected clause's\nactual binder names and declaration identities, without the outer `holds`\nwrapper. `R` is FR-931's exact rational literal spelling\n`rational(n, d)` in lowest terms with positive denominator. An existing\nfilter appends ` where holds(` + `T(filter)` + `)`; an absent filter appends\nnothing. Every head then appends, in this exact order:\n\n` over past[` + `B(lower)` + `, ` + `B(upper"
    },
    {
      "id": "FR-283-line-564-part-3",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": ")` + `] min_count ` + `I(min_count)` + ` ` + `comparison` + ` ` + `T(threshold)`.\n\n`I` is FR-931's decimal integer literal printer. `comparison` is exactly\n`<`, `<=`, `=`, `>=` or `>`, as selected by the checked comparison. The\n`past[...]` punctuation and spacing follow FR-003. A position bound `B`\nprints with `I`. A duration bound `B` prints its normalized exact magnitude\nwith `R`, one space and its selected clock unit's declaration key under\nFR-931's identity rule. Both duration bounds use the same selected unit\nafter FR-194/FR-337's exact conversion; authored unit aliase"
    },
    {
      "id": "FR-283-line-564-part-4",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "s and the\noriginal choice of equivalent time unit do not enter the label. The label\nalways includes the checked `min_count`, including `1` when omitted in\nsource under FR-194. The threshold retains its checked conversion and\nFR-931 literal spelling. No source reformatting or token-preserving source\nformatter selects a canonical spelling.\n\nThe report layer SHALL print all admitted aggregate kinds, including\nquantity-bearing components, under this rule. Missing implementation of a\nspecified component printer is implementation work, not an attribution\nrefusal or remaining rend"
    },
    {
      "id": "FR-283-line-564-part-5",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "ering scope. The unique and byte-identical cases\nstill retain exact caller text. All three cases retain the original\nevaluator result, refusal, category, basis, support positions and aggregate\n`count`/`min_count`; the shared cases use the actual claim declaration locus.\n"
    },
    {
      "id": "FR-283-line-634-part-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "| FR-283-AC-19 | TC-1295's aggregate controls retain exact caller text for unique and byte-identical regions and render differing shared regions by the canonical aggregate assembly above, including omitted versus explicit `min_count 1` and equivalent reformatted duration bounds. Both routes use the actual declaration locus for shared regions and retain the original evaluator outcome and aggregate counts. Changing a window, filter, threshold, kind or normalized min_count changes the checked aggregate and is not collapsed by label identity; missing source or invalid spans ret"
    },
    {
      "id": "FR-283-line-634-part-2",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "ain `SourceNotSupplied`, never a canonical fallback. | Test |"
    },
    {
      "id": "FR-283-line-644-part-1",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "- FR-931: canonical `holds` printing and the expression, literal and identity\n  component printers used by canonical window-aggregate label assembly."
    },
    {
      "id": "FR-501-line-180-part-1",
      "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
      "role": "examined",
      "excerpt": "  node's canonical printed form: FR-931's existing TL-lowering atom printer\n  for `holds` leaves, and FR-283's canonical window-aggregate assembly for\n  aggregate leaves. Both use the selected claim declaration's actual region\n  as locus."
    },
    {
      "id": "FR-501-line-186-part-1",
      "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
      "role": "examined",
      "excerpt": "reference and reuse those canonical component printers; occurrence-source helpers supply"
    },
    {
      "id": "FR-501-line-503-part-1",
      "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
      "role": "examined",
      "excerpt": "| FR-501-AC-32 | TC-1295's timestamped aggregate controls apply FR-283's unique, identical and differing-text rules to named aggregates and decision-support names on both routes, including normalized duration bounds and default min_count. Differing spellings use canonical aggregate assembly and the actual declaration locus, without attribution refusal or a new outcome row; the original evaluator result, basis, support positions and insufficient-data counts are unchanged. | Test |"
    },
    {
      "id": "TC-1160-line-84-part-1",
      "path": "spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md",
      "role": "examined",
      "excerpt": "    `EffectiveId` object type and identity UTF-8 text. Use the admitted\n    `test/counters` model from `fsl-eval/tests/it/model_simulation.rs` for this\n    reference control. Its Counter object `a` has `object_type` exactly\n    `{\"owner_effective_type\":null,\"original\":{\"package\":\"test/counters\",\"node\":\"ix://test/counters/Counter\"}}`"
    },
    {
      "id": "TC-1295-line-19-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "Verify FR-501-AC-1 to AC-26, FR-501-AC-31, FR-501-AC-32,\nFR-283-AC-18, FR-283-AC-19 and"
    },
    {
      "id": "TC-1295-line-86-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "For rows 48–50 and 52, use the existing `sampled` fixture in\n`fsl-package/src/emit/tests/temporal_aggregate.rs`: `Sample` has\n`v: Int[0, 9]` and `w: Int[0, 9]`, clause `Window` binds `e: Sample` under\nthe event-position profile. Put the stated aggregate or conjunction under\n`always[0,0]`, with one sample `{v: 0, w: 1}`. For row 51 use that file's\n`package_window` fixture under the timestamped profile, importing the\nexisting `example/config-version` model as `Config`, selecting clock\n`events` in `ix://example/config-version/unit/ms`. Supply one Sample at\nstamp 0, `W` 0 and `"
    },
    {
      "id": "TC-1295-line-86-part-2",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "C(0)`. Put the stated conjunction under `always[0,0]`.\nThis uses the fixture's actual `Config::s`/`Config::ms` normalization,\nnot a guessed unit identity. Exercise source and checked-package rendering\nwith their actual evaluated records and equal caller sources. The\nrecord-subject source run is an additional monitor control for\n`InsufficientData`. Verify the shared checked node and all candidate regions\nas in rows 45–47; an equal-body fixture that never creates multiple\ncandidates does not satisfy these controls.\n\nAll expected aggregate labels below are literal independent "
    },
    {
      "id": "TC-1295-line-86-part-3",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "oracles for\nFR-283's assembly, not output captured from a printer. These acceptance\nprocedures prescribe the rendering behavior; this specification change\nclaims no package aggregate evaluator or printer implementation coverage.\n"
    },
    {
      "id": "TC-1295-line-157-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "| 48 | UniqueAggregateText: `count over past[0,9] min_count 2 >= 1` | one aggregate region; the stated Sample | name exactly `count over past[0,9] min_count 2 >= 1` and that exact expression locus; monitor retains `InsufficientData`, position 0, count 1, min_count 2 |\n| 49 | SharedIdenticalAggregateText: two copies of row 48's atom joined by `and` | byte-identical caller text at two regions for one shared checked aggregate | name exactly row 48's caller text; actual `Window` declaration locus, never either expression; original insufficient-data result and counts unchanged |"
    },
    {
      "id": "TC-1295-line-157-part-2",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "\n| 50 | SharedDefaultAggregateText: `count where holds(e.v > 1) over past[0,9] >= 1` and `count where holds( e.v>1 ) over past[ 0 , 9 ] min_count 1 >= 1`, joined by `and` | differing text but one shared checked aggregate under FR-194's default min_count rule | canonical name exactly `count where holds(e.v > 1) over past[0, 9] min_count 1 >= 1`, actual `Window` declaration locus; monitor retains `InsufficientData`, position 0, count 0, min_count 1; neither authored spelling is selected by rank |\n| 51 | SharedDurationAggregateText: `count where holds(e.v > 1) over past[0 Conf"
    },
    {
      "id": "TC-1295-line-157-part-3",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "ig::s, 3 Config::s] >= 1` and `count where holds( e.v>1 ) over past[ 0 Config::ms , 3000 Config::ms ] min_count 1 >= 1`, joined by `and` | equivalent duration bounds normalized to selected ms, with differing source bytes and omitted versus explicit min_count | canonical name exactly `count where holds(e.v > 1) over past[rational(0, 1) ix://example/config-version/unit/ms, rational(3000, 1) ix://example/config-version/unit/ms] min_count 1 >= 1`; actual `Window` declaration locus; original result, counts, basis and support positions unchanged; no attribution refusal |\n| 52 | A"
    },
    {
      "id": "TC-1295-line-157-part-4",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "ggregateKindsAndComponents: for each literal label in the following table, use two source spellings differing only in whitespace (and `0.5` versus `rational(1, 2)` for quantile) | same shared-node/candidate checks; one stated Sample; min_count 2 | exact literal label from the following table, actual declaration locus, original insufficient-data count 1 and min_count 2; quantity components follow row 51's literal rule rather than a no-printer refusal |\n\nRow 52's independent labels cover every aggregate kind and optional parts:\n\n| Kind | Exact canonical label |\n| --- | --- |\n"
    },
    {
      "id": "TC-1295-line-157-part-5",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "| count | `count over past[0, 9] min_count 2 >= 1` |\n| sum | `sum of e.v over past[0, 9] min_count 2 <= 15` |\n| min | `min of e.v over past[0, 9] min_count 2 >= 0` |\n| max | `max of e.v over past[0, 9] min_count 2 <= 9` |\n| fraction | `fraction holds(e.v > 1) over past[0, 9] min_count 2 >= rational(1, 2)` |\n| quantile with filter | `quantile rational(1, 2) of e.v where holds(e.w = 1) over past[0, 9] min_count 2 <= 9` |"
    },
    {
      "id": "TC-1295-line-199-part-1",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "- Rows 48–52 also assert exact names and true loci, retaining all evaluator\n  fields rather than converting an aggregate outcome to a different row.\n  Reverse candidate enumeration and move whitespace between the two\n  candidates: the differing-text canonical result stays equal. Repeat the\n  missing/wrong source, short-byte and split-UTF-8 controls at each candidate\n  and the selected declaration region; each retains `SourceNotSupplied`.\n  Independently change the normalized min_count, one bound, kind, filter or\n  threshold: each semantically different aggregate stays a dis"
    },
    {
      "id": "TC-1295-line-199-part-2",
      "path": "spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md",
      "role": "examined",
      "excerpt": "tinct checked\n  node, even if another printed label coincides. Reformatting source,\n  omitting the default min_count or converting equivalent duration units\n  does not authorize source rank, label deduplication, a synthetic locus or\n  an attribution refusal. FR-931 TL lowering remains unchanged and does not\n  gain an aggregate proposition from these rendering tests."
    }
  ]
}
```
