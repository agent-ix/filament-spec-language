<!-- reviewer repo=agent-ix/filament-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2026111920260700000002 method=gap-analysis lang=rust pr=filament-spec-language#3084 model=claude-opus-5-5 run=2f4f380d-d75f-4052-8c31-120ecb8dac64 -->

Verdict: coverage is partial. This was a manual AC-to-test check; `quoin matrix` was not run because of thermal limits. Two trace tags are wrong (FR-1015-AC-7, FR-1015-AC-5), one FR-865-AC-7 clause is untested, and the FR-813-AC-5 tag overstates coverage.

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | trace | The test is tagged FR-1015-AC-7 but asserts the counter `incomplete` with exit 22 (FR-1015-AC-2). |
| FND-002 | medium | trace | The spine mapper-failure test is tagged FR-1015-AC-5, which does not state that behaviour. |
| FND-003 | low | coverage | FR-865-AC-7's no-output-file clause (`bytes written: 0`, `output: none`) has no test. |
| FND-004 | low | trace | The counter Lean test claims FR-813-AC-5, which is about the temporal AC-1/AC-2 Lean files, but it exports a state invariant. |

+++ [reviewer data]

```yaml
clean: false
scope:
- id: FR-1015-AC-5
  path: spec/functional/
  role: examined
  excerpt: The counter's `Bounded` exports `preserved` with no cause under the default floor, exit `0`.
    `SlowEnough` over `Braking` under `any` gives `target-unrepresentable` causes whose `target` is `tla`,
    whose `construct` holds a `/`, whose `node` is a node id of the package and whose `locus` has `kind`
    `region`.
- id: FR-1015-AC-6
  path: spec/functional/
  role: examined
  excerpt: '`obligations` `[]` refuses `invalid-value` at `/request/export/obligations`, exit `20`. `["CanInc",
    "Bounded", "Nope"]` gives three records in that order, with `CanInc` and `Nope` `refused` with `not-an-obligation`,
    and exit `21`.'
- id: FR-1015-AC-7
  path: spec/functional/
  role: examined
  excerpt: '`SlowEnough` under the default floor with `limits` `{"export.output_bytes": 1}` refuses `below-fidelity-floor`,
    exit `21`.'
- id: FR-865-AC-7
  path: spec/functional/
  role: examined
  excerpt: '`render_outcome` with `RenderForm::Text` over FR-286-AC-22''s counter export document written
    to `m.tla` writes exactly `export: success (exit 0) at S4`, `target: tla filament.output.tla2/v1`,
    `bytes written: <n>` ..., and `output: m.tla`, and no other line; over the same export with no output
    file named its last two lines are `bytes written: 0` and `output: none`; over the export under `export.output_bytes`
    1 it writes `export: incomplete (exit 22) at S4`, `bytes written: 0` and a line naming `export.output_bytes`.'
- id: FR-1035-AC-1
  path: spec/functional/
  role: examined
  excerpt: The spine unit with `bound HistoryBound on Config::config_history [8]` and no reachability
    exports to TLA+ with `Cardinality(DOMAIN v_` and `) <= 8` in the `b_HistoryBound` definition that
    `TypeOK` conjoins. The same unit without the `bound` has no `Cardinality` in its text.
- id: FSpec-FR-812-AC-7
  path: filament-specification:spec/functional/output-mapping/
  role: examined
  excerpt: 'Exported without `initial`, a state model writes `Init == TypeOK` and `def Init (s : State)
    : Prop := WF s`, and its `constraint` clause appears in neither. ... With `initial` naming a snapshot,
    `Init` states that snapshot''s state.'
- id: FSpec-FR-813-AC-5
  path: filament-specification:spec/functional/output-mapping/
  role: examined
  excerpt: The Lean file of AC-1 and AC-2 contains no `theorem`, no proof term and no `sorry`.
- id: FSpec-FR-814-AC-3
  path: filament-specification:spec/functional/output-mapping/
  role: examined
  excerpt: Two exports of one request give byte-identical text that is ASCII only, ends with one LF, has
    no trailing whitespace, and has one `fsl:` comment per definition naming its node and region.
findings:
- fnd: FND-001
  severity: medium
  confidence: high
  method: gap-analysis
  check: trace
  artifact_id: FR-1015-AC-7
  related_ids:
  - FR-1015-AC-2
  path: tests/it/export_rulings.rs
  lines: 182-191
  excerpt: '`SlowEnough` under the default floor with `limits` `{"export.output_bytes": 1}` refuses `below-fidelity-floor`,
    exit `21`.'
  finding: The test is tagged FR-1015-AC-7 but asserts the counter `incomplete` with exit 22 (FR-1015-AC-2).
  test_id: a_lowered_text_over_the_byte_limit_is_incomplete
  claimed_ac: FR-1015-AC-7
  correct_ac: FR-1015-AC-2
- fnd: FND-002
  severity: medium
  confidence: high
  method: gap-analysis
  check: trace
  artifact_id: FR-1015-AC-5
  related_ids: []
  path: tests/it/export_state_lowering.rs
  lines: 183-195
  excerpt: The counter's `Bounded` exports `preserved` with no cause under the default floor, exit `0`.
    `SlowEnough` over `Braking` under `any` gives `target-unrepresentable` causes whose `target` is `tla`,
    whose `construct` holds a `/`, whose `node` is a node id of the package and whose `locus` has `kind`
    `region`.
  finding: The spine mapper-failure test is tagged FR-1015-AC-5, which does not state that behaviour.
  test_id: an_unlowered_state_construct_is_a_mapper_failure
  claimed_ac: FR-1015-AC-5
  correct_ac: none (no FSL AC states it)
- fnd: FND-003
  severity: low
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-865-AC-7
  related_ids: []
  path: fsl-inspect/src/render.rs
  lines: 539-545
  excerpt: '`render_outcome` with `RenderForm::Text` over FR-286-AC-22''s counter export document written
    to `m.tla` writes exactly `export: success (exit 0) at S4`, `target: tla filament.output.tla2/v1`,
    `bytes written: <n>` ..., and `output: m.tla`, and no other line; over the same export with no output
    file named its last two lines are `bytes written: 0` and `output: none`; over the export under `export.output_bytes`
    1 it writes `export: incomplete (exit 22) at S4`, `bytes written: 0` and a line naming `export.output_bytes`.'
  finding: 'FR-865-AC-7''s no-output-file clause (`bytes written: 0`, `output: none`) has no test.'
- fnd: FND-004
  severity: low
  confidence: medium
  method: gap-analysis
  check: trace
  artifact_id: FSpec-FR-813-AC-5
  related_ids: []
  path: tests/it/export_state_lowering.rs
  lines: 89-94
  excerpt: The Lean file of AC-1 and AC-2 contains no `theorem`, no proof term and no `sorry`.
  finding: The counter Lean test claims FR-813-AC-5, which is about the temporal AC-1/AC-2 Lean files,
    but it exports a state invariant.
  test_id: the_counter_invariant_lowers_to_lean
  claimed_ac: FSpec-FR-813-AC-5
  correct_ac: FSpec-FR-814-AC-3 / FR-1017 (no exact AC)
bindings:
- test_id: the_counter_invariant_lowers_to_tla
  ac_id: FR-1015-AC-5
  trace: correct
- test_id: the_counter_invariant_lowers_to_tla
  ac_id: FSpec-FR-812-AC-7
  trace: correct
- test_id: the_counter_invariant_lowers_to_tla
  ac_id: FSpec-FR-814-AC-3
  trace: correct
- test_id: equal_requests_give_equal_text
  ac_id: FSpec-FR-814-AC-3
  trace: correct
- test_id: init_without_a_snapshot_is_the_well_typedness
  ac_id: FSpec-FR-812-AC-7
  trace: correct
- test_id: an_incomplete_export_renders_its_limit
  ac_id: FR-865-AC-7
  trace: correct
- test_id: an_export_outcome_renders_as_four_lines
  ac_id: FR-865-AC-7
  trace: correct
- test_id: an_invariant_among_refused_names_is_preserved
  ac_id: FR-1015-AC-6
  trace: correct
- test_id: only_what_the_target_cannot_state_is_unrepresented
  ac_id: FR-1015-AC-5
  trace: correct
- test_id: a_lowered_text_over_the_byte_limit_is_incomplete
  ac_id: FR-1015-AC-7
  trace: wrong
- test_id: an_unlowered_state_construct_is_a_mapper_failure
  ac_id: FR-1015-AC-5
  trace: wrong
- test_id: the_counter_invariant_lowers_to_lean
  ac_id: FSpec-FR-813-AC-5
  trace: wrong
```

+++
