# QSL-2092: protocol and timed replay boundaries

Ten new behavior tests exercise real replay paths under current FR-227,
FR-217 and FR-237. Three existing integration parents register child modules:
`weak_memory`, `protocol_system` and `zone_replay`.

- Store-buffering counterexamples are produced by the actual protocol checker.
  TSO replay succeeds with default, exact-one and larger buffers; RA replay
  succeeds with default/two messages and refuses one message, which is already
  occupied by the initial write. Removing each carried memory component names
  that exact transition and the typed content mismatch.
- Actual `Fill` deadlocks bind every blocked location and reason. Dropping each
  blocked thread or changing its node/cause cannot reproduce the deadlock.
  Changing the initial snapshot invalidates the first recorded post-state.
  Delays, including zero, refuse on every transition and endpoint of an untimed
  protocol.
- Actual EN-6 counterexamples require every transition's delay and reject a
  formula witness's extra final delay. Exact rational delays just below and
  above the deadline distinguish the guard from the time invariant. A legal
  fractional trace that replies before timeout does not reproduce a claimed
  late-reply violation.

The timed fixture checks the actual clock-bearing source declarations and uses
the repository's explicit test-support admission for unwritten timed members.
These tests exercise the timed replay core; they do not claim independent
package-route timed read-back or task-automaton lowering. The legal fractional
trace is executed through the real timed system to capture its transition keys,
then replayed against the package's actual clause.

No production, dependency or configuration change is included. No evidence
producer or successful proof is replaced by a test double. The final gate also
corrected two old Undefined fixtures to call an admitted Kernel remainder
function, preserving their actual Undefined and exhaustion assertions. The
gate reached one existing strict-package failure, tracked by QSL-2717; its
assertion remains strict and its failed result is excluded from accepted
coverage. The broader historical QSL-2092 families remain unclaimed;
previously delivered or pending QSL-2082 and QSL-2088 cases were not duplicated
for trace tags.
