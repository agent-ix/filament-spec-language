---
id: SR-2026111610291801
title: "Integrity review of the FSL wave 6 spec readiness batch, language side"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #2747 branch fsl-spec-w6 at 0ed71f1385f075422c2d149b4ba0f2fd4e3dc188, git diff origin/main...HEAD (FR-027, FR-093, FR-243, FR-266, FR-267, FR-287, FR-330, FR-489, FR-495, FR-504, FR-621, FR-625, FR-628, FR-632, FR-634, FR-635, FR-641, new FR-810, TC-1588/1592/1594/1595 and new TC-2290..TC-2299)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-243
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-632
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-634
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-810
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: reviews
---

## Summary

Ticket: QSL-2296 (wave 6 batch: QSL-2296, QSL-2299, QSL-2293, QSL-2300, QSL-2303, QSL-2304). One combined review with filament-specification#663. Checked against FR-239, FR-286, FR-489, FR-620-AC-8, FR-640, FR-665, FR-767, FSpec FR-166/FR-391/FR-393 (well-definedness-unchecked, typed canonical instance key), FSpec FR-331 (result entry `backend`, accounting `settled_by`), FSpec FR-561 (no discovery, no search path), and FR-093/FR-495 as merged in #2745.

Ids: FR-810 and TC-2290..TC-2299 are new and none exists on origin/main (cd359a8ba). `quire validate --scope . 'spec/**/*.md'` exits 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The digital-clock refusal says `declined` `invalid-input` and "no engine runs" / "reaches no engine". `declined` is an FSpec FR-331 result entry, which carries `backend`, and FR-621-AC-10 now requires `backend` exactly when `settled_by` is `result`. The spec does not say which backend the record names or whether `settled_by` is `result`. Fix: say EN-1's digital-clock route declines before any search, so the record has `settled_by` `result` and `backend` `en-1`, and change "no engine runs" to "no search runs" in FR-632-AC-4 and FR-243-AC-4. | spec/functional/FR-632-request-timed-zone-items.md:104; spec/functional/FR-243-digitize-a-closed-timed-subject-for-explicit-state-checking.md:78 |
| FND-002 | medium | `options.method` now picks the engine, but FR-635 says `options` are the provider's option names, checked against its manifest's published options. Both EN-1 and EN-6 are `temporal-satisfaction` candidates. The spec does not say whether EN-1's manifest publishes `method` `digital-clocks` (so the options check would select it) or whether `method` is a routing member read before negotiation and never sent as a provider option. Fix: say which one in FR-243 and FR-632. | spec/functional/FR-243-digitize-a-closed-timed-subject-for-explicit-state-checking.md:52-56; spec/functional/FR-635-route-analyze-items-to-external-providers-through-the-registry.md:38 |
| FND-003 | medium | Process providers reach only `analyze_with_providers`, the `native-analyze/1` entry (`analyze` and `prove`). FR-287 and FSpec FR-561 also accept `--provider-config` on `monitor`, `replay` and `generate`, and say each configured plugin is registered before negotiation, but no entry of those verbs takes the started providers. So provider selection is specified for only part of the feature. Fix: give each provider-routing command entry the same `providers` parameter (FR-644 monitor, replay, generate), or state one rule that covers every such entry. | spec/functional/FR-635-route-analyze-items-to-external-providers-through-the-registry.md:61-67; spec/functional/FR-287-reach-fsl-through-the-driver-cli.md:105-117 |
| FND-004 | medium | "Every explored state lies on exactly one reported trace" is false whenever the discovery tree branches: an initial or internal state lies on every trace through its subtree. A test written from this sentence would fail on correct output. Fix: "every explored state lies on at least one reported trace, and each leaf ends exactly one". | spec/functional/FR-634-request-a-simulation-item.md:79-81 |
| FND-005 | medium | FR-810 says `output` writes the package file "and `handoff` writes the directory around it". That reads as if the output file is inside the handoff directory. Both writers refuse an existing path, and no write order is given, so an `output` path inside the `handoff` directory refuses or succeeds depending on which writes first. Fix: state that the two are independent writes, refuse an `output` path inside the `handoff` directory naming both members, and drop "around it". | spec/functional/FR-810-write-the-checked-package-to-a-file-from-a-compile-request.md:40-42 |
| FND-006 | medium | FR-810-AC-3 puts the library override and the driver's `fsl compile --output` in one criterion verified "Test (TC-2297)", but the driver binary and its tests are in filament-driver (FR-767-AC-7 is the precedent: "Driver test ... the code ticket adds the test in the filament-driver repository"). No test in this repo can bind the driver sentence. Fix: move the driver sentence into its own AC with Driver-test verification naming the repository and QSL-2299. | spec/functional/FR-810-write-the-checked-package-to-a-file-from-a-compile-request.md:95 |
| FND-007 | low | FR-287 still says "native library artifacts remain in-process command products", which FR-810's `output` and FR-665's `handoff` now contradict (the command writes both to disk). Fix: add "except the files FR-665 and FR-810 write". | spec/functional/FR-287-reach-fsl-through-the-driver-cli.md:156-157 |
| FND-008 | low | The control-node rule applies the "hull of the lowered types of the leaves" to every `if`, `let` and `case`, but a hull is defined only for integer intervals. For a Boolean, text or record control node the rule has no stated meaning. Fix: limit the hull rule to an integer-typed control node, and say every other control node's lowered type is its checked type. | spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md:227-231; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:56-59 |

## Verdict

REQUEST CHANGES, for FND-001 to FND-006. Nothing was dropped. The narrowings in the diff are each grounded in a merged rule:

- `digital-clocks` now also needs closed claim intervals. Digitisation keeps the dense-time verdict only then, and the zone route still decides those items.
- `witness_samples` bounds walks, not witnesses, which is FSpec FR-166's rule.
- `max_clocks` 0 is now a request refusal, like every other FR-625 limit of 0.
- The `possible (value = 4)` target is replaced, because FR-104 refuses it at check.

Item 1.9 is consistent with FR-489: the production snapshot stays linked-only (new FR-489 bullet), and FR-635 defines process-provider start-up, the `hello` conversion and the registration-failure rule, with no discovery, matching FSpec FR-561. The gap is FND-003 (the verbs other than analyze and prove).

Item 1.5 is consistent with FR-239: EN-6 still never digitises, FR-243's route is a function of the item, and FR-243 reads `options.method`. FND-001 and FND-002 remain open.

Item 2 (FR-810) is coherent with FR-286 and FR-287. FR-286 says `artifacts` is always `[]`, and the never-overwrite rule matches FR-665. FND-005 and FND-006 remain open.

Item 3 (EN-7) follows FR-620-AC-8, and FR-489 is amended to match. The knock-on to live tests is in the code-review SR.

Items 5 to 7 agree with FR-093/FR-495 as merged (#2745), FSpec FR-331's two artifacts and `settled_by`, and FR-267 with FSpec FR-352.

FR-504-AC-1 against FR-266 is settled cleanly through `ClauseRunReport::finite_window`, which exists in `fsl-replay/src/spine/clause.rs`.

## Dispositions

Round 1, verified at b3ee227de56d7909ddfd92346cf289c31aa1aac4. After the fixes, `quire validate --scope . 'spec/**/*.md'` exits 0, with no warning on any changed file.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-243 and FR-632 now say EN-1's digital-clock route declines before any search, with `settled_by` `result` and `backend` `en-1`, and "no search runs". FR-632-AC-4 and FR-243-AC-4 say the same, and the TC-2290 stub asserts both members. |
| FND-002 | fixed | Lead ruling: `options.method` is a provider option negotiated against the manifests. EN-1 publishes `digital-clocks` and EN-6 publishes `zones` (its default), as FR-243 and FR-632 now state. FR-243-AC-4 adds that an absent `method` negotiates to EN-6. |
| FND-003 | fixed | FR-635's new section "One rule for every provider-routing entry" covers analyze, prove, monitor, replay and generate. FR-287, FR-644 (`command::monitor`) and FR-280 (the prove, replay and generate entries) refer to it. |
| FND-004 | fixed | FR-634 now reads "lies on at least one reported trace and each leaf ends exactly one ... no more traces than states". |
| FND-005 | fixed | FR-810 now makes `output` and `handoff` independent writes, drops "around it", and adds an If/then refusal for an `output` inside the `handoff` directory, naming both members before any write. FR-810-AC-5 and its stub cover the refusal. |
| FND-006 | fixed | The driver sentence is now FR-810-AC-4, verified "Driver test (TC-2297)" in filament-driver under QSL-2299. TC-2297 step 4 covers it. |
| FND-007 | fixed | FR-287 now reads "remain in-process command products, except the files the command writes to disk on request: FR-665's handoff directory and FR-810's package file". |
| FND-008 | fixed | FR-495 and FR-093 now apply the hull rule only to an integer-typed control node (checked type `Integer` or an `Int[..]`). Every other control node's lowered type is its checked type. |

Round 1 verdict: APPROVE. Every finding is fixed.
