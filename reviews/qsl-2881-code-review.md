---
id: SR-967175026302197768900474181941935084801
title: "code-review (with rust-review lane) of PR 3589: zero-delay cycle without an end component"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/exact_probabilistic/finite_horizon.rs (PR 3589 head 4f5e22b337246d1ebd271225639a6d1c34d0aebc, diff origin/main...HEAD, +116)"
review_set: subset
---

## Summary

Ticket: QSL-2881. The PR adds a comment at the no-component branch of
`collapse_zero_time_components` arguing that `UnboundedError::Singular` is unreachable from
any product the crate builds, and two unit tests tagged TC-632.

The narrow claim holds. `Singular` needs a memoryless policy with a closed class, and every
closed class is an end component of the graph. In a timed bounded-event product (Form::Deadline,
digital.rs:1329, every state carries a Deadline monitor), `has_deadline` adds the full-graph
maximal end components (MECs), so the branch runs only when no MEC exists, which means every
policy is proper. The untimed builder writes `elapsed = 0` on every edge (build.rs:975), so the
zero graph is the full graph. A timed-model subject with a non-timed profile goes to
`digital::build`, and `Form::of` refuses it unless the root has a timed interval (digital.rs:226-266).

The refutation attempt found an adjacent path to the outcome the ticket title names (a zero-delay
loop with no end component settles as an internal fault). It does not go through `Singular`.
It fails in `ZeroTimeSolver`'s recursion when a leaky zero-delay cycle sits outside every end
component and at least one end component exists elsewhere (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A leaky zero-delay cycle outside every end component, with an end component elsewhere in the product, makes `ZeroTimeSolver::ordinary_value` return `FhError::Cycle`, and `run_timed` turns that into `broken()` (runtime_invariant). A user model can build this shape | fsl-analyze/src/exact_probabilistic/finite_horizon.rs:453-455 |
| FND-002 | medium | The new comment says a timed bounded-event product never reaches this branch. It does whenever the Deadline product has no end component, for example TC-2175's guardless, resetless Retx (a leaky `send` self-loop) | fsl-analyze/src/exact_probabilistic/finite_horizon.rs:323-331 |
| FND-003 | low | The Deadline test's shape (a delaying self-loop back to the same Deadline-monitor state) cannot come out of digital::build, and its leaky action is inside the component, not beside it. It does not exercise the real Deadline product path or a leaky cycle beside a component | fsl-analyze/src/exact_probabilistic/finite_horizon.rs:1811-1827 |
| FND-004 | low | Neither oracle discriminates: each test has one action, so Min equals Max, and the reject terminal (state 2) is unreachable, so the asserted 1 is also the trivial upper bound | fsl-analyze/src/exact_probabilistic/finite_horizon.rs:1765-1809 |

## Verdict

FAIL (one high finding). On the narrow `Singular` question, unreachability is shown by code
reading and the argument is sound. Both test values are right when derived independently:

- Test 1: v = 1/2·v + 1/2·1, so v = 1.
- Test 2: the component {0} has one exit, with value (1/2·1)/(1 − 1/2) = 1.

The ticket's acceptance concerns the zero-delay-cycle outcome, and that outcome is still an
internal fault for the FND-001 shape. Fix it in this PR: decide cyclic non-component states by
the policy iteration the no-component branch already uses, over the quotient after the components
collapse. Add a digital fixture that settles the shape.

Side observation, out of diff (QSL-2978, `collapse_zero_time_components` line 311): it is not
reachable. The zero graph's choices are a subset of the full graph's, with the same edges and
terminals, so every zero-time end component is an end component of the full graph. MECs from
`maximal_end_components_from` (unbounded.rs:1950) are disjoint and maximal, so each zero-time MEC
nests in exactly one full-graph MEC. Overlap without nesting cannot occur. The exit is a genuine
internal invariant; `FhError::Cycle` is the wrong label for it (InvalidState fits). QSL-2978 can
close as unreachable or be reworded as a relabel.

rustfmt: clean. `rustfmt --edition 2021 --check` with the pinned 1.98 toolchain (rustfmt 1.9.0)
exits 0 on the file.

## Scope and evidence

- fsl-analyze/src/exact_probabilistic/finite_horizon.rs:323-331 (examined): the new comment at the no-component branch.
- fsl-analyze/src/exact_probabilistic/finite_horizon.rs:1720-1827 (examined): the `leaky_zero_delay_cycle` fixture and the two TC-632 tests.
- finite_horizon.rs:251-505 (context_only): collapse, has_deadline, ZeroTimeSolver.
- analyze/exact.rs:380-460, 930-1175 (context_only): `run_timed` and the untimed caller of decide_bounded_event. Both map `Err(_)` to `broken()`.
- exact_probabilistic/build.rs:110-160 (context_only): a timed subject always routes to digital::build.
- exact_probabilistic/digital.rs:163-315, 1329, 1640-1700, 2771-2915 (context_only): Form::Deadline monitors; zero-delay MECs with exits are kept and leaky cycles are anticipated.
- fsl-semantics/src/check/probabilistic/claim.rs:1565-1845 (context_only): `timed_horizon` is Some exactly when the profile is Timed.
- fsl-analyze/tests/it/digital_clocks.rs:31-96, 757-771, 1375-1388 (context_only): the Retx and TICK fixtures and TC-2175.

FND-001 producible model (traced, not executed): take TC-2175's Retx world with no guard and no
reset (`every("", false)`). `send` is instant, with `random lost ~ {true: 1, false: 9}`, so it
loops back with probability 1/10 and delivers with 9/10. Add a second operation `spin` on
`retried` with:

```
pre Spin using v on M::Call::spin { self.x >= 2 ms }
post Spun using v on M::Call::spin { self.retried = not self.retried }
```

Add the claim `Deadline` over every scheduler. Then:

1. At x = 2 the two `retried` states form a deterministic zero-time end component. Its exits are `send`, so digital.rs:2844 keeps it and `components` is non-empty.
2. At x = 0 and x = 1, `send`'s 1/10 self-loop belongs to no component.
3. `solver.value(x=0)` calls `ordinary_value`, which sets `visiting`. `expectation(send)` then calls `value(x=0)`, which sees `visiting` set and returns `Err(FhError::Cycle)`.
4. `run_timed` (exact.rs:443-460) returns `broken()`.

Model only `spin` in the Every scheduler, or use any guard that places a closed zero-delay loop
away from a leaky one.

Cargo was not run. The titan budget allowed a cargo run only if essential, and the trace is
direct.
