<!-- reviewer repo=agent-ix/filament-spec-language visibility=public quoin=0.29.4 module=spec-artifacts-process@v0.29.2 id=SR-2724002 method=gap-analysis lang=rust pr=filament-spec-language#2724 model=claude-opus-5-5 run=062A720A-199D-48DE-8FC5-971C75313C69 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | coverage | The FR-640-AC-8 byte-identical recompile has no running test; TC-2182 covers only the conversion of rows to clocks |
| FND-002 | medium | test-intent | The FR-640-AC-1 test splits the three profiles into three units; its module doc saying FR-326 requires this is stale |
| FND-003 | low | test-intent | The TC-2183 missing case does not assert that the library clause is the one at fault |

Verdict: CONDITIONAL. All 11 ACs are tagged. The five run_wire stubs are genuinely blocked by QSL-2254 and QSL-2136, through the timed Ack clause in their fixture.

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-640-AC-1, path: "spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md", role: examined, excerpt: "Through command::compile, a unit with three clauses over a record trace subject, one under each of event-position, fixed-sample and timestamped-event, and a clocks member configuring each, exits 0; the package holds three temporal_bindings rows whose clock_name and clock equal the members."}
  - {id: FR-640-AC-7, path: "spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md", role: examined, excerpt: "Through command::compile with the ConfigVersion model and no clocks member, a unit with one fixed-sample clause and one timestamped-event clause over Config::ConfigVersion on clock model-steps exits 0, with the derived rows."}
  - {id: FR-640-AC-8, path: "spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md", role: examined, excerpt: "The clocks object rebuilt from the temporal_bindings rows of the package FR-640-AC-1 compiles compiles the same program to byte-identical package bytes."}
  - {id: FR-640-AC-9, path: "spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md", role: examined, excerpt: "A request compiling a program and one library unit whose clause writes clock plant-ms, with one clocks member plant-ms, binds both clauses to that member; without the member, the library clause refuses missing-clock-selection naming plant-ms."}
  - {id: FR-326-AC-7, path: "spec/functional/FR-326-admit-temporal-operators-by-the-unit-s-temporal-profile.md", role: examined, excerpt: "A unit selecting ev and inf, with one clause using ev and one using inf, checks, each clause under its own alias's profile."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-640-AC-8
    related_ids: []
    path: fsl-replay/src/request/package/tests.rs
    lines: "19"
    excerpt: |-
      The clocks object rebuilt from the temporal_bindings rows of the package FR-640-AC-1 compiles compiles the same program to byte-identical package bytes.
    finding: "There is no running end-to-end test, and the stub is ignored only because of the timed clause in its fixture. Fix: over the clock_selection.rs units, compile, rebuild clocks from the rows, recompile, and compare the bytes."
  - fnd: FND-002
    severity: medium
    confidence: high
    method: gap-analysis
    check: test-intent
    artifact_id: FR-640-AC-1
    related_ids: [FR-326-AC-7]
    path: tests/it/clock_selection.rs
    lines: "5-7,133-176"
    excerpt: |-
      Through command::compile, a unit with three clauses over a record trace subject, one under each of event-position, fixed-sample and timestamped-event, and a clocks member configuring each, exits 0.
    finding: "The test splits the clauses into three single-profile units, and its doc comment that FR-326 forces this is stale. Fix: use one unit with aliases e/f/w (the run_wire fixture without Ack)."
  - fnd: FND-003
    severity: low
    confidence: high
    method: gap-analysis
    check: test-intent
    artifact_id: FR-640-AC-9
    related_ids: []
    path: tests/it/clock_derivation.rs
    lines: "128-135"
    excerpt: |-
      without the member, the library clause refuses missing-clock-selection naming plant-ms.
    finding: "The program clause also uses plant-ms. Fix: assert that details.declaration is Late."
bindings:
  - {test_id: model_subject_clauses_derive_their_clock_configuration, ac_id: FR-640-AC-7, trace: correct}
  - {test_id: library_units_share_the_request_clocks, ac_id: FR-640-AC-9, trace: correct}
  - {test_id: a_member_only_a_library_clause_reads_is_not_unused, ac_id: FR-640-AC-9, trace: correct}
  - {test_id: the_rows_give_one_member_per_trace_subject_clock, ac_id: FR-640-AC-8, trace: correct}
```

+++
