---
id: SR-2026112007511801
title: "spec-review of completion spec batch 3 (filament-spec-language PR 3158)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR 3158 head f224903a6bf0083e2ea013e0178fcfb91dd6a20b; spec/functional/{FR-095,FR-100,FR-1039,FR-124,FR-267,FR-621,FR-665,FR-850,FR-940,FR-990}; spec/test-cases/{TC-1748,TC-2955,TC-2958,TC-3019,TC-3081,TC-422,TC-519}; context FR-663, FR-255, FR-281, FR-127; code on origin/main 38dbd6b7"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1039
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-124
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-095
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-665
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-663
    type: references
---

## Summary

Ticket: QSL-2664 (with QSL-2559, QSL-2576, QSL-2235, QSL-2675). One pass of quoin:spec-review over `git diff origin/main...HEAD`, reviewer-only, no builds. Each batch rule was checked against its ruling and against the code on filament-spec-language origin/main 38dbd6b7.

Checked clean: FR-940-AC-21 and the FR-621 last-stage row. `AnalyzeRefusal::Package(..).document()` writes `last_stage` `S4`, the reader's raw cause (`null` when it has none), `details.path` and the code's category, as `analyze_bound` reads the package back before the binding is admitted. FR-124 and FR-124-AC-11/AC-12: all five worked cases in AC-12 were checked by hand against the classification rule and hold; AC-11's corrected sentence matches FSpec FR-361-AC-17 and FR-368-AC-22. `ModelSystem::terminal_class` reads `self.terminals()` (every package terminal), so the code change the ruling requires is real. FR-095-AC-7 and FR-267 `input`: the wire shape follows the serde `kind`-tagged form of the other kinds. FR-850 and the FR-1039 file-form rules agree on `missing-file`/`refused-file`/`invalid-request`. FR-990-AC-9 agrees with FSpec FR-059-AC-6 (main's `recording.rs:543` gives `open-history` for an open closure, so the code changes, as ruled). `quire validate` exits 0 on every changed file (EARS warnings only).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-1039-AC-4 and FR-1039-AC-5 run packages that hold equality applications from `{file}` alone (the counter's `self.value = pre(self.value) + 1`, `Even`'s `n * 2 - n = n`), which the new FR-1039 rule now refuses with `invalid_package`/`missing-file`. | spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md:263-264 |
| FND-002 | high | FR-940 and FR-940-AC-22 code a replay read above its byte ceiling as `resource_exhausted`. FR-255 and the in-tree mapping of the same `ReplayPackage::read` limit (`plugin_replay.rs:389`) give `stage_limit_exceeded`, naming `replay.input_bytes`. | spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md:691,727 |
| FND-003 | high | FR-663 "Refusal locus" still lists a closed set of four locus forms and "the five tagged forms", and gives `null` for an unreadable file. It contradicts FR-267's sixth `input` kind and FR-665-AC-5's `input` locus for a missing handoff member file. | spec/functional/FR-663-report-the-parsed-unit-and-the-locus-of-every-refusal.md:62-87 |
| FND-004 | medium | FR-940 says every other reader refusal is `invalid_package`, but `of_import_view` keeps `V2ReadRefusal::code()`: `unknown_wire` (UnsupportedVersion), `invalid_source_map` (SourceMap) and the library code (Structural). | spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md:682 |
| FND-005 | medium | FR-940-AC-22 calls for a test-only replay ceiling, but `replay.input_bytes` is a real FR-255 setting, raised from the library and the CLI. Analyze's replay reads hardcode `ReplayLimits::default().input_bytes` (`clause_item.rs:455,600`, `state_graph_item.rs:1472`), and FR-940 does not say the run's setting applies. | spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md:727 |
| FND-006 | medium | FR-940-AC-22 says "the same holds" for `ReachThree`, a `possible` claim proved through witness-evidence replay. FR-127's V-6 row covers only `Violated`, FR-170 and FR-628 define no `replay_refused` settlement, and the AC states no expected label. | spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md:727 |
| FND-007 | medium | The FR-940 replay-refusal rule covers only a replay that emits the package. The package-route clause replay reads the selected bytes and maps a refusal to the fault `analyze`/`package-bytes` (`clause_item.rs:871`), which is left unstated. | spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md:687-700 |
| FND-008 | medium | FR-267-AC-28 collides with the FR-267-AC-28 added by open PR #3098 (qsl-2590-false-guard-activation, updated 2026-11-19). | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:656 |
| FND-009 | medium | The foreign-witness case of FR-1039-AC-2 ("the `equality.json` of another unit's handoff") gives `Unjoined` only if that unit's equality rows are non-empty. FR-517 writes empty lists for a unit with no equality application, and those rows join any package. | spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md:261 |
| FND-010 | low | FR-665's "Today `fsl_command::command::inspect_handoff` writes ..." paragraph is a build log, not a statement of system behavior. | spec/functional/FR-665-write-and-verify-a-checked-package-handoff-through-the-command.md:131-134 |
| FND-011 | low | In FR-665-AC-5, the `handoff-reader` pointer "(or `null` when the reader names none)" over an unspecified changed byte cannot fail, and the `absent` case states no code. | spec/functional/FR-665-write-and-verify-a-checked-package-handoff-through-the-command.md:144 |

## Recommendations

- FND-001: in FR-1039-AC-4, replace `{"file": P, "source_package": ...}` with `{"file": P, "equality": E, "source_package": ...}`, where `E` is the `equality.json` of the counter's FR-517 handoff. In FR-1039-AC-5, select `Even` as `{"file": P, "equality": E}` the same way and keep RR-12 on `{"file": P}`, since it has no equality application. Re-check FR-1039-AC-6's `Low` unit and FR-945's ACs for any `=`/`==`/`!=` application. Optionally add to AC-5: "`Even` from `{file}` alone refuses `invalid_package`/`missing-file` at `/request/package/equality`".
- FND-002: in FR-940, replace "(`resource_exhausted` for bytes above the replay read's byte ceiling or an allocation it cannot make, `invalid_package` otherwise)" with "(`stage_limit_exceeded`/`input-bytes-exceeded` naming `replay.input_bytes` (FR-255) for bytes above the replay read's byte ceiling, `resource_exhausted` for an allocation it cannot make, `invalid_package` otherwise)". In FR-940-AC-22 and TC-2958 step 6, replace "carrying `resource_exhausted`" with "carrying `stage_limit_exceeded`/`input-bytes-exceeded` naming `replay.input_bytes`".
- FND-003: in FR-663, add the bullet "- an input locus (FR-095 `Locus::Input`) for a refusal located in a caller-named input that is not an FR-001 artifact or a JSON input document, such as a member file of a handoff directory (FR-665);". Change the `null` bullet to "`null` when the refusal has no position, such as an unreadable request or input document file, whose path stays in the cause;". Replace "one of the five tagged forms ... (`region`, `located`, `occurrence`, `artifact`, `document`)" with "one of the six tagged forms ... (`region`, `located`, `occurrence`, `artifact`, `document`, `input`)".
- FND-004: replace "every other reader refusal is `invalid_package`" with "every other refusal of the Contract IR reader is `invalid_package`, and a refusal FSL's v2 read view raises itself keeps its own code (`unknown_wire` for an unsupported version, `invalid_source_map`, a structural import refusal's library code)".
- FND-005: add to the FR-940 replay bullet "the replay read is bounded by the run's `replay.input_bytes` (FR-255)". In AC-22, replace "run with the replay's `ReplayPackage` byte ceiling set below the length of the package's emission (a test-only ceiling; ...)" with "run with `replay.input_bytes` set below the length of the package's emission (the up-front read-back is bounded by `i2.input_bytes` and admits the package)".
- FND-006: either drop `ReachThree` from AC-22, or state it: "`ReachThree`, whose witness replay reads the package the same way, settles `inconclusive`, cause `replay_refused` carrying the same code, never `proved`". Add to FR-940's bullet: "this holds for the evidence replay of a proved state-graph item as for a counterexample replay".
- FND-007: replace "that emits the package and reads the emitted bytes as a `ReplayPackage`" with "that reads the run's package bytes as a `ReplayPackage` (the emitted bytes on the source route, the selected bytes on the package route)", and add that a refusal of that read is never the fault `analyze`/`package-bytes`.
- FND-008: renumber this batch's AC to FR-267-AC-29 (and TC-3081's scope "FR-267-AC-25 to FR-267-AC-29"), or coordinate with #3098 so that one of the two renumbers before either merges.
- FND-009: replace "naming the `equality.json` of another unit's handoff" with "naming the `equality.json` of the handoff of another unit with an equality application (non-empty rows, none of which `Pick`'s package joins)".
- FND-010: delete the "Today `fsl_command::command::inspect_handoff` ... each row above replaces it." paragraph.
- FND-011: name the changed byte (for example "a byte inside `/semantic_graph/nodes/0/body`"), state the pointer the reader gives, and add "code `handoff-read`" to the `inspect --handoff absent` case.

## Verdict

Blocking: FND-001, FND-002 and FND-003 are high. Each new rule otherwise has an AC that can fail on exact codes, causes, exits and pointers. The FSL and FSpec texts agree on the six locus kinds and on the file-form `equality` selection. No other AC id collides with origin/main or open PRs, and nothing previously specified was dropped. Main's code also has these gaps, outside this diff: `fsl_foundation::diagnostic::Locus::Artifact` and `OutcomeLocus::Artifact` carry only `pointer`, and `RefusalLocusRead` reads `{domain, digest, pointer}` with no `document` kind. The FR-001 `{authority, identity}` artifact wire is therefore a ruled code change.
