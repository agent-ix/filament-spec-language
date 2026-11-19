<!-- reviewer-dispositions repo=agent-ix/filament-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2026111911313600000002 round=1 model=claude-opus-5-5 run=92e745d0-947b-40c7-b975-b187fca814d0 -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | deferred | QSL-2547: no fixture combines a parameterised operation + workload + claim (abstraction fixture hits QSL-2444); random-parameter emission/admission is covered by other TC-2062 tests and QSL-2547's staged-vote-probability examples combine all four. Acceptable; add the AC-4 random-parameter case to QSL-2547 scope. |

New findings (disposition pass 1):

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-004 | low | coverage | Source-reference half still unasserted: the package test checks the label span via the unit-text slice but not region's source reference; the fsl-command twin checks only locus is_object and `reward` (feature word), not `cost` or the locus source/span members. |

+++ [reviewer data]

```yaml
head: 9588a1a78f391de7c609d8e5b0cca221398b30b9
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: 'let cases: [(String, Vec<u8>, ProbabilisticFeature, &str); 4] = [ ... Reward "cost",
    Delay "step", RandomParameter "promote(t)", Workload "Both" ];

    ...

    let label = source.get(region.start() as usize..region.end() as usize) ...;

    assert!(label.contains(word), ...);

    assert!(message.contains(word), "{message}");'
- fnd: FND-002
  outcome: fixed
  after_excerpt: 'Follow-ups exist: QSL-2546 (Claim identity covers workload and reward; ignore now cites
    it) and QSL-2547 (Vendor staged-vote-probability and staged-availability examples; probabilistic_examples.rs).'
- fnd: FND-003
  outcome: deferred
  reason: 'QSL-2547: no fixture combines a parameterised operation + workload + claim (abstraction fixture
    hits QSL-2444); random-parameter emission/admission is covered by other TC-2062 tests and QSL-2547''s
    staged-vote-probability examples combine all four. Acceptable; add the AC-4 random-parameter case
    to QSL-2547 scope.'
new_findings:
- fnd: FND-004
  severity: low
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-185-AC-12
  related_ids:
  - probabilistic_refusals_have_locus
  - a_package_stage_refusal_of_a_reward_carries_its_locus
  path: spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md
  lines: '308'
  excerpt: An S4 refusal concerning a random parameter, a workload, a reward or a delay carries that declaration's
    source reference and label span in its locus member (never `null`), and its message names the declaration
    in words and holds no debug rendering of a span or source reference.
  finding: 'Source-reference half still unasserted: the package test checks the label span via the unit-text
    slice but not region''s source reference; the fsl-command twin checks only locus is_object and `reward`
    (feature word), not `cost` or the locus source/span members.'
```

+++
