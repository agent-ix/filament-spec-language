<!-- reviewer repo=agent-ix/filament-spec-language visibility=public quoin=0.29.4 module=spec-artifacts-process@v0.29.2 id=SR-2724001 method=code-review lang=rust pr=filament-spec-language#2724 model=claude-opus-5-5 run=062A720A-199D-48DE-8FC5-971C75313C69 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | soundness | The hyper clause's event-position sequence_authority comes from the first quantifier's model selection identity, which differs from the single-trace rule (declaration identity text). "First quantifier" is not in FR-640, and the case is untested |
| FND-002 | medium | soundness | The replay recompile reads only the program package's rows, so a clock that only a library uses is lost and the replay refuses missing-clock-selection |
| FND-003 | low | code-bug | The FR-326-AC-4 refusal names the alias but not its non-temporal selection |

Verdict: REQUEST CHANGES. Q1 (no clause path reads the unit profile), the library sharing, the unused rule and the TC-2183 change (it follows FR-099-AC-5) were all checked and are clean.

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-326-AC-7, path: "spec/functional/FR-326-admit-temporal-operators-by-the-unit-s-temporal-profile.md", role: examined, excerpt: "A unit selecting the event-position profile as ev and the infinite-trace profile as inf, with one clause using ev and one using inf, checks, each clause under its own alias's profile, with no unknown_profile refusal."}
  - {id: FR-326-AC-4, path: "spec/functional/FR-326-admit-temporal-operators-by-the-unit-s-temporal-profile.md", role: examined, excerpt: "A temporal clause whose using alias selects filament.value.complete/v1, a profile with no temporal facet, refuses unknown_profile/unsupported-selection at the clause's span, naming the alias and its selection."}
  - {id: FR-640-AC-7, path: "spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md", role: examined, excerpt: "Through command::compile with the ConfigVersion model and no clocks member, a unit with one fixed-sample clause and one timestamped-event clause over Config::ConfigVersion on clock model-steps exits 0, and their temporal_bindings rows hold the derived fixed_sample and timestamped_event configurations."}
  - {id: FR-640-AC-8, path: "spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md", role: examined, excerpt: "The clocks object rebuilt from the temporal_bindings rows of the package FR-640-AC-1 compiles compiles the same program to byte-identical package bytes; this is the selection list the replay recompile builds from the proving package. ReplayRequestWire has no clocks member."}
  - {id: FR-640-AC-9, path: "spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md", role: examined, excerpt: "A request compiling a program and one library unit whose clause writes clock plant-ms, with one clocks member plant-ms, binds both clauses to that member and refuses no member as unused-clock-selection; without the member, the library clause refuses missing-clock-selection naming plant-ms."}
  - {id: FR-099-AC-5, path: "spec/functional/FR-099-compile-against-supplied-libraries.md", role: context_only, excerpt: "... a use g::R, each refuses ill_typed/operator-ineligible at the use."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: medium
    method: code-review
    check: soundness
    artifact_id: FR-640
    related_ids: [FR-640-AC-3]
    path: fsl-replay/src/spine/clocks.rs
    lines: "321-347"
    excerpt: |-
      A hyper clause over a model's behaviours is a clock-bearing declaration under the same rule: its binding comes from the model and the command derives its configuration by the table above. M is the model's declaration identity text.
    finding: "The hyper path takes M from the first quantifier's model selection identity (example/config-version). The single-trace path uses the subject type's declaration identity text, the JCS key pinned by the AC-3 test. FR-640 says neither 'first quantifier' nor how to handle quantifiers over different models, and no test covers it. Fix: use one rule for M, refuse or raise a spec question for a hyper clause over two models, and add a test."
  - fnd: FND-002
    severity: medium
    confidence: medium
    method: code-review
    check: soundness
    artifact_id: FR-640-AC-8
    related_ids: [FR-640-AC-9, FR-099]
    path: fsl-replay/src/request/package.rs
    lines: "98-122"
    excerpt: |-
      The clocks object rebuilt from the temporal_bindings rows of the package FR-640-AC-1 compiles compiles the same program to byte-identical package bytes; this is the selection list the replay recompile builds from the proving package, so a replay recompiles under exactly the configuration the proof ran under.
    finding: "Library rows are in the library package, and DependencyEntry carries only sources. A clock that only a library clause uses compiles, but its replay recompile refuses missing-clock-selection. Fix: get a spec ruling and file a follow-up ticket, or read the rows from the dependency packages."
  - fnd: FND-003
    severity: low
    confidence: medium
    method: code-review
    check: code-bug
    artifact_id: FR-326-AC-4
    related_ids: []
    path: fsl-semantics/src/check/temporal_clause.rs
    lines: "1222-1241"
    excerpt: |-
      A temporal clause whose using alias selects filament.value.complete/v1, a profile with no temporal facet, refuses unknown_profile/unsupported-selection at the clause's span, naming the alias and its selection.
    finding: "clause_profile filters to temporal selections, so found is empty and the message omits filament.value.complete/v1. Fix: carry the alias's selected identity and assert it."
bindings:
  - {test_id: the_unit_selects_exactly_one_temporal_profile, ac_id: FR-326-AC-4, trace: correct}
  - {test_id: clauses_of_one_unit_use_their_own_temporal_profiles, ac_id: FR-326-AC-7, trace: correct}
```

+++
