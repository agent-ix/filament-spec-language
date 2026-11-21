---
id: SR-20261121043350
title: "spec-review of PR 3257 FR-760 rev-pin edits"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; spec/functional/FR-760-consume-the-standard-and-publish-the-facades-through-versioned-cargo-pins.md; PR 3257 head 704c4431b951df98a17a0e0160f90a99a8497574"
review_set: subset
---

## Summary

The three edits ("a release (tag), a full-sha rev or a registry version"; "without a tag or full-sha rev"; the AC-3 row "names a tag or a full 40-hex-character rev") state exactly what pin_check.rs does and match the 2026-11-20 arch ruling. TC-2141 backs the new AC-3 text. `quire validate --scope .` on FR-760 exits 0. The PR does not carry the ruling into the rest of FR-760, which still requires a tag for the standard.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-760-AC-1 still says the standard is listed "with a `tag` source", and Behavior still says it is taken "with `tag = \"vX.Y.Z\"`". Cargo.toml:8 pins it by `rev`. The AC contradicts the code and the ruling, and TC-2140 (tests/it/standard_pin.rs:90, `the_standard_is_a_tag_pinned_dependency...`) never checks the source kind, so nothing catches it. | spec/functional/FR-760-consume-the-standard-and-publish-the-facades-through-versioned-cargo-pins.md:56-57,102 |

## Scope and evidence

Examined: the FR-760 Description (line 25, "a released version, named in Cargo", which also reads as tag-only), Behavior "The standard as a pinned dependency" (lines 52-64), Behavior "Pins on every Filament dependency" (lines 66-75), FR-760-AC-1 and FR-760-AC-3. Context only: AC-2, AC-4 and AC-5, which this change does not affect, and the facade-release text at lines 81-82, which is about FSL's own tags for consumers and stays correct.

## Verdict

Mergeable once FND-001 is fixed: AC-1 and line 57 accept a tag or full-sha rev source, and line 25 says "a pinned version" rather than "a released version". Alternatively, drop the source-kind clause from AC-1, since TC-2140 never tests it.
