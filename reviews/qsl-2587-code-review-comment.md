<!-- reviewer repo=agent-ix/filament-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2026111920260700000001 method=code-review lang=rust pr=filament-spec-language#3084 model=claude-opus-5-5 run=2f4f380d-d75f-4052-8c31-120ecb8dac64 -->

Verdict: REQUEST CHANGES. The PR merges cleanly with main (31/31 export tests pass on the merge), and the TLA+ and Lean text is plausibly valid, but there are three blocking defects. (1) Population `bound` declarations are silently dropped, while the record still says `preserved`. (2) The Lean receiver-only frame conjunct swallows `WF s'` and the conjuncts after it. (3) Domain-package operation requires/ensures are ignored. Nothing has been checked by SANY, TLC or Lean.

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | high | soundness | Population bounds are silently dropped. The counter with `bound B on Counter::counters [3]` exports exit 0 in both targets with no Cardinality/length conjunct, and `Bounded` is still recorded `preserved` (probe). FR-812 says the bound is part of the population's meaning. The fix is to refuse (exit 30, naming `bound`) until `CheckedGraph::population_bound` is read. Today's per-object subset is truth-equivalent, but the text breaks FR-812-AC-6 and FR-1035-AC-1 while claiming exact fidelity. |
| FND-002 | high | code-bug | The Lean `op_` frame conjunct is not parenthesized. The emitted line `forall k, F_has k s.v -> Not (k = p_self) -> (get k s').f = (get k s).f /\ WF s'` puts `WF s'`, the `s'.w = s.w` conjuncts and later frames under `k /= p_self`, so with a singleton population they are vacuous. The Lean step then admits ill-typed or unframed post-states that TLA+ and FSL exclude. The fix is to wrap each frame line in parentheses. |
| FND-003 | high | soundness | Domain-package operation requires/ensures (kept model-operation clauses, operation_clause.rs) are never read. An operation with a nonempty `pre`/`post` therefore exports without it, and the invariant is still `preserved`. The fix is to lower these clauses or refuse when they are nonempty. |
| FND-004 | medium | soundness | `op_inc` is name-mapped and `fsl:`-commented with the object-type node, whose region is the unit's `model Counter = ...` line (124-164), not the operation. `v_Counter` and the Lean helpers have no name-map entry (the TLA+ map is exactly op_inc, c_Bounded, Th_Bounded), and `VARIABLES` has no comment. FR-814 requires a name map from each generated name to its node. |
| FND-005 | medium | trace | `a_lowered_text_over_the_byte_limit_is_incomplete` is tagged FR-1015-AC-7 but asserts the counter `incomplete` with exit 22 (FR-1015-AC-2 / FSpec FR-817-AC-10). AC-7 is SlowEnough below the floor with exit 21. |
| FND-006 | medium | trace | `an_unlowered_state_construct_is_a_mapper_failure` is tagged FR-1015-AC-5. AC-5 says nothing about a spine invariant failing as a mapper failure with exit 30. |
| FND-007 | medium | coverage | Only the parameterless counter is tested. Parameters (p_ binders, lo..hi domains), Boolean fields, or/implies guards, not/negate, multi-object UNCHANGED, modifies_receiver frames (where FND-002 hides) and refusal attribution have no tests. The counter test claims FR-812-AC-1, whose deposit(a, n: Int[1, 10]) case is not exercised. |
| FND-008 | medium | soundness | Two meter deviations from FR-1015. First, "Writing the text charges one export.work_units unit for each output region" is not implemented (`assembled` charges only output bytes). Second, the lowering charges nodes outside the obligations' dependency closures (every operation's clause nodes and narrow targets), which FR-1015 does not count. |
| FND-009 | low | coverage | The deleted mapper-failure test covered all three floors, an absent `export` member, last_stage S4 and stage `emit`. The replacement checks only the default floor, exit 30, the names and no file. |
| FND-010 | low | other | A refusal always names the first lowerable obligation, not the failing one. `read_models` also refuses on any object type of any selected model, even one no obligation reads, so the attribution can mislead. |
| FND-011 | low | other | The `p_` prefix is not in FR-814's prefix list. `f_` is used for record fields, but FR-814 reserves it for functions. `Prod.fst` is outside FR-816's Lean subset, which allows `.1`/`.2`. |
| FND-012 | low | other | Rust smells: an unused `target` parameter silenced with `let _`, a no-op `binary` closure, and `Show::expr` returning an empty string for an unknown object or field index instead of failing. |

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
- id: FSpec-FR-812-AC-1
  path: filament-specification:spec/functional/output-mapping/
  role: examined
  excerpt: 'A model with a population of `Account` keyed by text, a field `balance: Int[0, 100]` and an
    operation `deposit(a, n: Int[1, 10])` exports `v_Account`, `TypeOK`, `Init`, `op_deposit`, `Next`
    and `Spec` with `WF_vars(Next)` to TLA+, and the matching `State`, `WF`, `Init`, `op_deposit` and
    `Step` to Lean; every record is `preserved`.'
- id: FSpec-FR-812-AC-6
  path: filament-specification:spec/functional/output-mapping/
  role: examined
  excerpt: With `bound B on M::accounts [3]`, the TLA+ `TypeOK` holds the conjunct `Cardinality(DOMAIN
    v_Account) <= 3` and the Lean `WF` holds `v_Account.length <= 3`. Without the `bound`, neither conjunct
    appears.
- id: FSpec-FR-812-AC-7
  path: filament-specification:spec/functional/output-mapping/
  role: examined
  excerpt: 'Exported without `initial`, a state model writes `Init == TypeOK` and `def Init (s : State)
    : Prop := WF s`, and its `constraint` clause appears in neither. ... With `initial` naming a snapshot,
    `Init` states that snapshot''s state.'
- id: FSpec-FR-814-AC-3
  path: filament-specification:spec/functional/output-mapping/
  role: examined
  excerpt: Two exports of one request give byte-identical text that is ASCII only, ends with one LF, has
    no trailing whitespace, and has one `fsl:` comment per definition naming its node and region.
- id: FSpec-FR-814-AC-5
  path: filament-specification:spec/functional/output-mapping/
  role: examined
  excerpt: A TLC error trace naming `v_Account` and `op_deposit` reads back through the name map to the
    FSL object type and operation nodes.
- id: fsl-command/src/command/export/lower.rs
  path: fsl-command/src/command/export/lower.rs
  role: examined
  excerpt: 'FSpec FR-812 to FR-814: the lowering of invariant state clauses over object populations to
    TLA+ and Lean 4 text.'
- id: fsl-command/src/command/export/map.rs
  path: fsl-command/src/command/export/map.rs
  role: examined
  excerpt: lowerable state/state_clause obligations recorded Preserved, then lower::lower; refusal ->
    Unlowered
- id: fsl-inspect/src/render.rs
  path: fsl-inspect/src/render.rs
  role: examined
  excerpt: 'fn export_result_lines(export) -> target:/bytes written:/output: or bytes written: 0 + limit
    reached:'
findings:
- fnd: FND-001
  severity: high
  confidence: high
  method: code-review
  check: soundness
  artifact_id: fsl-command/src/command/export/lower.rs
  related_ids:
  - FSpec-FR-812-AC-6
  - FR-1035-AC-1
  path: fsl-command/src/command/export/lower.rs
  lines: 240-257
  excerpt: for population in items(document, "populations") { ... object.population = Some(id.to_owned());
    }  // no bound read anywhere; TypeOK/WF carry no Cardinality/length conjunct
  finding: Population bounds are silently dropped. The counter with `bound B on Counter::counters [3]`
    exports exit 0 in both targets with no Cardinality/length conjunct, and `Bounded` is still recorded
    `preserved` (probe). FR-812 says the bound is part of the population's meaning. The fix is to refuse
    (exit 30, naming `bound`) until `CheckedGraph::population_bound` is read. Today's per-object subset
    is truth-equivalent, but the text breaks FR-812-AC-6 and FR-1035-AC-1 while claiming exact fidelity.
- fnd: FND-002
  severity: high
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: fsl-command/src/command/export/lower.rs
  related_ids:
  - FSpec-FR-812-AC-1
  path: fsl-command/src/command/export/lower.rs
  lines: 1445-1464
  excerpt: 'let guard = if receiver { "Not (k = p_self) -> " } else { "" };

    lines.push(format!("forall k, {has} k s.{v} -> {guard}({get} k s''.{v}).{f} = ({get} k s.{v}).{f}"));

    ... lines.join(" /\\\n  ")'
  finding: The Lean `op_` frame conjunct is not parenthesized. The emitted line `forall k, F_has k s.v
    -> Not (k = p_self) -> (get k s').f = (get k s).f /\ WF s'` puts `WF s'`, the `s'.w = s.w` conjuncts
    and later frames under `k /= p_self`, so with a singleton population they are vacuous. The Lean step
    then admits ill-typed or unframed post-states that TLA+ and FSL exclude. The fix is to wrap each frame
    line in parentheses.
- fnd: FND-003
  severity: high
  confidence: medium
  method: code-review
  check: soundness
  artifact_id: fsl-command/src/command/export/lower.rs
  related_ids: []
  path: fsl-command/src/command/export/lower.rs
  lines: 195-226
  excerpt: 'operations.push(Operation { name, params, every: set("modifies"), receiver: set("modifies_receiver")
    });  // operation["pre"] / ["post"] never read'
  finding: Domain-package operation requires/ensures (kept model-operation clauses, operation_clause.rs)
    are never read. An operation with a nonempty `pre`/`post` therefore exports without it, and the invariant
    is still `preserved`. The fix is to lower these clauses or refuse when they are nonempty.
- fnd: FND-004
  severity: medium
  confidence: high
  method: code-review
  check: soundness
  artifact_id: FSpec-FR-814-AC-5
  related_ids:
  - fsl-command/src/command/export/lower.rs
  path: fsl-command/src/command/export/lower.rs
  lines: 920-924,1184-1189,1543-1575
  excerpt: A TLC error trace naming `v_Account` and `op_deposit` reads back through the name map to the
    FSL object type and operation nodes.
  finding: '`op_inc` is name-mapped and `fsl:`-commented with the object-type node, whose region is the
    unit''s `model Counter = ...` line (124-164), not the operation. `v_Counter` and the Lean helpers
    have no name-map entry (the TLA+ map is exactly op_inc, c_Bounded, Th_Bounded), and `VARIABLES` has
    no comment. FR-814 requires a name map from each generated name to its node.'
- fnd: FND-005
  severity: medium
  confidence: high
  method: code-review
  check: trace
  artifact_id: FR-1015-AC-7
  related_ids:
  - FR-1015-AC-2
  path: tests/it/export_rulings.rs
  lines: 182-191
  excerpt: '`SlowEnough` under the default floor with `limits` `{"export.output_bytes": 1}` refuses `below-fidelity-floor`,
    exit `21`.'
  finding: '`a_lowered_text_over_the_byte_limit_is_incomplete` is tagged FR-1015-AC-7 but asserts the
    counter `incomplete` with exit 22 (FR-1015-AC-2 / FSpec FR-817-AC-10). AC-7 is SlowEnough below the
    floor with exit 21.'
  test_id: a_lowered_text_over_the_byte_limit_is_incomplete
  claimed_ac: FR-1015-AC-7
  correct_ac: FR-1015-AC-2
- fnd: FND-006
  severity: medium
  confidence: high
  method: code-review
  check: trace
  artifact_id: FR-1015-AC-5
  related_ids: []
  path: tests/it/export_state_lowering.rs
  lines: 183-195
  excerpt: The counter's `Bounded` exports `preserved` with no cause under the default floor, exit `0`.
    `SlowEnough` over `Braking` under `any` gives `target-unrepresentable` causes whose `target` is `tla`,
    whose `construct` holds a `/`, whose `node` is a node id of the package and whose `locus` has `kind`
    `region`.
  finding: '`an_unlowered_state_construct_is_a_mapper_failure` is tagged FR-1015-AC-5. AC-5 says nothing
    about a spine invariant failing as a mapper failure with exit 30.'
  test_id: an_unlowered_state_construct_is_a_mapper_failure
  claimed_ac: FR-1015-AC-5
  correct_ac: none (no FSL AC states it)
- fnd: FND-007
  severity: medium
  confidence: high
  method: code-review
  check: coverage
  artifact_id: FSpec-FR-812-AC-1
  related_ids:
  - fsl-command/src/command/export/lower.rs
  path: tests/it/export_state_lowering.rs
  lines: 41-127
  excerpt: 'A model with a population of `Account` keyed by text, a field `balance: Int[0, 100]` and an
    operation `deposit(a, n: Int[1, 10])` exports `v_Account`, `TypeOK`, `Init`, `op_deposit`, `Next`
    and `Spec` with `WF_vars(Next)` to TLA+, and the matching `State`, `WF`, `Init`, `op_deposit` and
    `Step` to Lean; every record is `preserved`.'
  finding: 'Only the parameterless counter is tested. Parameters (p_ binders, lo..hi domains), Boolean
    fields, or/implies guards, not/negate, multi-object UNCHANGED, modifies_receiver frames (where FND-002
    hides) and refusal attribution have no tests. The counter test claims FR-812-AC-1, whose deposit(a,
    n: Int[1, 10]) case is not exercised.'
- fnd: FND-008
  severity: medium
  confidence: medium
  method: code-review
  check: soundness
  artifact_id: fsl-command/src/command/export/lower.rs
  related_ids:
  - FR-1015-AC-12
  path: fsl-command/src/command/export/map.rs
  lines: 294-308
  excerpt: for &position in &done.read { if charged.insert(position) { if let Err((setting, bound)) =
    meter.charge() { ... } } }
  finding: Two meter deviations from FR-1015. First, "Writing the text charges one export.work_units unit
    for each output region" is not implemented (`assembled` charges only output bytes). Second, the lowering
    charges nodes outside the obligations' dependency closures (every operation's clause nodes and narrow
    targets), which FR-1015 does not count.
- fnd: FND-009
  severity: low
  confidence: high
  method: code-review
  check: coverage
  artifact_id: tests/it/export_surface.rs
  related_ids:
  - FR-1015-AC-5
  path: tests/it/export_surface.rs
  lines: 23-45
  excerpt: for floor in [None, Some("any"), Some("preserved")] { ... assert_eq!(document["last_stage"],
    "S4"); assert_eq!(value["stage"], "emit"); }
  finding: The deleted mapper-failure test covered all three floors, an absent `export` member, last_stage
    S4 and stage `emit`. The replacement checks only the default floor, exit 30, the names and no file.
- fnd: FND-010
  severity: low
  confidence: high
  method: code-review
  check: other
  artifact_id: fsl-command/src/command/export/map.rs
  related_ids:
  - fsl-command/src/command/export/lower.rs
  path: fsl-command/src/command/export/map.rs
  lines: 316-321
  excerpt: 'Err(refusal) => { unlowered.get_or_insert_with(|| Unlowered { obligation: first.clone(), construct:
    refusal.construct, detail: refusal.detail }); }'
  finding: A refusal always names the first lowerable obligation, not the failing one. `read_models` also
    refuses on any object type of any selected model, even one no obligation reads, so the attribution
    can mislead.
- fnd: FND-011
  severity: low
  confidence: high
  method: code-review
  check: other
  artifact_id: fsl-command/src/command/export/lower.rs
  related_ids:
  - FSpec-FR-814-AC-3
  path: fsl-command/src/command/export/lower.rs
  lines: 394,612,1070-1076,1442-1444
  excerpt: E::Param(mangle("p_", name)) ... mangle("f_", &field.name) ... "List.map Prod.fst s'.{v} =
    List.map Prod.fst s.{v}"
  finding: The `p_` prefix is not in FR-814's prefix list. `f_` is used for record fields, but FR-814
    reserves it for functions. `Prod.fst` is outside FR-816's Lean subset, which allows `.1`/`.2`.
- fnd: FND-012
  severity: low
  confidence: high
  method: code-review
  check: other
  artifact_id: fsl-command/src/command/export/lower.rs
  related_ids: []
  path: fsl-command/src/command/export/lower.rs
  lines: 428,582-589,605-610
  excerpt: 'fn int_text(value: &str, target: Target) -> String { if value.starts_with(''-'') { format!("({value})")
    } else { let _ = target; value.to_owned() } }'
  finding: 'Rust smells: an unused `target` parameter silenced with `let _`, a no-op `binary` closure,
    and `Show::expr` returning an empty string for an unknown object or field index instead of failing.'
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
```

+++
