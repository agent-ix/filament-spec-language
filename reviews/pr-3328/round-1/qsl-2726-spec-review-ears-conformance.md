---
id: SR-20261121003328004
title: "ears-conformance review of native temporal result rendering"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; QSL-2726; FR-267, FR-283, FR-501, TC-1160, TC-1295"
review_set: subset
---

## Summary

Ticket: QSL-2726; PR #3328. Frozen head a036c0a638df57212c335e8007e91275a1d1b64f; merge base 7303f8d88b6b0534c65d1ef2a9f6dbff33abd7ef. Five-file specification-only diff. Read-only source corroboration; no Cargo, runtime test, aggregate gate, pin change, or author-branch edit. quoin 0.36.8, quire 0.52.0 (engine 0.62.2), spec-artifacts-process v0.36.0. Explicit ISO validation: 5/5 grammar-clean, zero grammar findings; git diff --check exits 0. New runtime trace coverage remains pending implementation and is not claimed by this readiness review.

No applicable AssuranceProfile frontmatter exists in the local spec tree; direct review is authorized by the dispatch and repository guidance. Selection: base plus applicable failure-domain, EARS-conformance and scope-boundary lenses. No production diff means no gap-analysis; no Rust/Cargo diff means no Rust review. Integrity/dependency/evidence/risk-complexity/object-review/criterion-strength lenses are not separately run: artifact structure, real dependency contracts, declared verification method and object inventory are unchanged; only reference links to existing FRs are added. Required native boundary checks are included in scope-boundary. No Jev assessment is claimed.

## Verdict

PASS — specification readiness for this method only. Overall PR is not mergeable while the base and failure-domain findings remain unresolved.

## Method

The installed deterministic grammar check over the three FRs and two TCs reports zero grammar findings (5/5 docs clean). Semantically, triggers distinguish one candidate, several equal texts and several differing texts; responses prescribe exact text and actual regions, not vague support or performance promises. Caller-source failures have explicit refusal behavior. The native exception names the writer and strict reader and limits the accepted shape. No additional EARS defect was found. The substantive missing aggregate response is owned by failure-domain, independent of grammar.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

Synthetic line/part suffixes locate verbatim fragments; they are not new specification ids.

```yaml
{
  "clean": true,
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
  "findings": [],
  "bindings": []
}
```
