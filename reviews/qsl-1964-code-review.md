---
id: SR-2724001
title: "code review of PR 2724 clock selection by clock name"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 2724 head bef78a9c8cba9c37d5e63de69f94d452128cee20; fsl-replay/src/spine/clocks.rs, fsl-replay/src/spine/lifecycle.rs, fsl-replay/src/execute.rs, fsl-replay/src/request/package.rs, fsl-package/src/emit.rs, fsl-package/src/emit/clock_selection.rs, fsl-semantics/src/check/temporal_clause.rs, fsl-semantics/src/check/hyper_clause.rs, fsl-command/src/command.rs, fsl-command/src/command/wire.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-326
    type: references
---

## Summary

Ticket: QSL-1964. Code review with the rust-review lane (rust-style loaded) of the diff `origin/main...HEAD`. The per-alias profile rule, the model-derived clocks, the library `clocks` sharing and the replay clocks read from the package were checked against FR-326-AC-4/7 and FR-640-AC-7..9 as merged in #2728.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A hyper clause's derived event-position `sequence_authority` is the model selection identity (`example/config-version`), taken from the first quantifier's alias, while a single-trace model-subject clause uses the subject type's declaration identity text (JCS of the declaration key, pinned by the FR-640-AC-3 test). FR-640 gives one rule ("M is the model's declaration identity text") and never says "first quantifier"; quantifiers may name different models (hyper_clause.rs:1659-1679). Fix: derive M by the same rule as the single-trace path, refuse or ask the spec lane about a hyper clause whose quantifiers name two models, and add a hyper derivation test. | fsl-replay/src/spine/clocks.rs:321-347 |
| FND-002 | medium | Replay loses library-only clocks. `ReplayPackage::clocks` reads only the proving package's own rows; library rows live in the library package, and `DependencyEntry` carries only sources. A program whose library clause uses a clock the program does not use compiles, but its replay recompile refuses `missing-clock-selection`. Fix: get a spec ruling on where library rows come from at replay (file a follow-up ticket), or read them from the dependency packages if the request carries them. | fsl-replay/src/request/package.rs:98-122 |
| FND-003 | low | The FR-326-AC-4 refusal does not name the alias's selection. `clause_profile` filters to temporal selections, so for `v` = `filament.value.complete/v1`, `found` is empty and the message names only the alias. Fix: carry the alias's selected identity in `ProfileSelection` and assert it in the message. | fsl-semantics/src/check/temporal_clause.rs:1222-1241 |

## Verdict

REQUEST CHANGES for FND-001 and FND-002. Q1: no clause path still reads the unit profile. The temporal and hyper checks resolve through `clause_profile`, and protocol awaits, the trace-subject run and compensate templates were already per alias. The library emission remembered by `emit_dependency_with_temporal_selections_and_cancel` is per request, so it does not go stale. The unused rule counts library trace-subject clauses that a member serves. The TC-2183 edit follows FR-099-AC-5, which refuses a use of `g::R`. It does not weaken the test.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | b0e713b59: each hyper trace takes M from the declaration identity text of its own model's parameter type, the same text the single-trace path uses; traces deriving different configurations refuse unsupported_construct/not-yet-implemented (PerTraceClockConfigurations); tests a_hyper_clause_over_one_model_derives_the_single_trace_authority and a_hyper_clause_over_two_models_refuses_two_event_position_clocks |
| FND-002 | deferred | QSL-2274 (spec decision, W4 batch); failing stub a_replay_recompiles_a_library_that_uses_its_own_clock ignored under it |
| FND-003 | fixed | b0e713b59: ProfileSelection carries `selected`, the alias's identities; message names `filament.value.complete/v1`; asserted in a_clause_under_a_value_profile_alias_names_the_alias_and_its_selection |
