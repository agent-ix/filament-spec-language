---
id: TC-2290
title: "The digital-clock route decides closed timed items and refuses strict ones"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-632
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-243
    type: verifies
---
# TC-2290: The digital-clock route decides closed timed items and refuses strict ones

## Description

Verify the caller-selected digital-clock route: it decides a closed subject
and closed claim on EN-1 over the integer-clock reading with basis
`exhaustive`, and refuses a strict constraint or interval and a claim that
is not digitisable.
Scope: FR-632-AC-4, FR-243-AC-4, FR-243-AC-5, FR-243-AC-7.

### Two-step subject

A dense subject with one clock `x`. `first` (`phase` 0 to 1) resets `x`;
`second` (`phase` 1 to 2) needs `x >= 2 ms`; the time invariants are
`x <= 1 ms` while `phase` is 0 and `x <= 2 ms` while `phase` is 1. On the
behaviour where `first` runs at `t` in `[0, 1]`, the positions are `phase` 0
at 0, `phase` 1 at `t`, `phase` 2 at `t + 2`, then idle-tail positions of
`phase` 2. With `t` strictly between 0 and 1 no position lies in `[1, 2]`,
and only position 0 lies at time 0.

## Test Procedure

1. Analyze `NoLateReply` over the all-closed `Rpc` with `T = 4 ms` and `options.method` `digital-clocks`.
2. Analyze the same item over ADR-026 §9's strict-constraint retry model.
3. Analyze `eventually[0 ms, 2 ms) holds(not c.late)` over the closed `Rpc` with `options.method` `digital-clocks`.
4. Over the two-step subject, analyze each claim of the table below twice: with `options.method` absent and with `digital-clocks`. `P` is `holds(c.phase >= 0)` and `F` is `holds(c.phase < 0)`.
5. Over the two-step subject, analyze `eventually[0 ms, 2 ms] holds(c.phase >= 1)` with a fairness constraint on `second`, with `digital-clocks`.

| Claim | Dense value, by hand | EN-6 | EN-1 |
|-------|----------------------|------|------|
| `eventually[1 ms, 2 ms] P` | false: no position in the window | refuted | declined |
| `P until[1 ms, 2 ms] P` | false: no position in the window | refuted | declined |
| `eventually[0 ms, 0 ms] eventually[1 ms, 2 ms] P` | false: only position 0 is at time 0 | refuted | declined |
| `always[0 ms, 0 ms] eventually[1 ms, 2 ms] P` | false: fails at position 0 | refuted | declined |
| `always (holds(c.phase = 0) implies eventually[1 ms, 2 ms] P)` | false: fails at position 0 | refuted | declined |
| `eventually (holds(c.phase = 0) and eventually[1 ms, 2 ms] P)` | false: only position 0 has `phase` 0 | refuted | declined |
| `eventually[1 ms, 1 ms] P` | false: no position at time 1 | refuted | declined |
| `eventually[2 ms, 3 ms] holds(c.phase = 2)` | true: `t + 2` is in `[2, 3]` | proved | declined (conservative) |
| `eventually[0 ms, 1 ms] holds(c.phase = 2) or eventually[0 ms, 2 ms] holds(c.phase = 1)` | true: `phase` 1 at `t <= 1` | proved | declined (conservative) |
| `eventually[0 ms, 2 ms] holds(c.phase >= 1)` | true: `phase` 1 at `t <= 1` | proved | proved |
| `holds(c.phase < 2) until[0 ms, 2 ms] holds(c.phase = 1)` | true: position 1 at `t <= 1`, `phase` 0 before it | proved | proved |
| `always (holds(c.phase = 1) implies eventually[0 ms, 2 ms] holds(c.phase = 2))` | true: exactly 2 after | proved | proved |
| `always eventually[0 ms, 1 ms] P` | true: each position is its own witness | proved | proved |
| `always[1 ms, 2 ms] holds(c.phase = 0)` | false: `t = 1` puts `phase` 1 at time 1 | refuted | refuted |
| `F release[1 ms, 2 ms] F` | false: `t = 1` puts a position in the window | refuted | refuted |

## Expected Results

- Step 1: `proved`, basis `exhaustive`, backend `en-1`, the same verdict as the zone route.
- Steps 2 and 3: `declined` `invalid-input` naming the strict constraint or interval; no engine runs.
- Step 4: the table's verdicts; each EN-1 decline is `declined` `invalid-input`, `invalid_runtime_input`/`invalid-value`, naming the failing operator, with `backend` `en-1`, and no search runs.
- Step 5: declined the same way, naming a clock constraint of the subject.
- Assertions compare typed members and the exit code, never message text.

Test: `fsl-analyze/tests/it/analyze_reach_rulings.rs`
`digital_clocks_refuse_strict_constraints_and_intervals`.
