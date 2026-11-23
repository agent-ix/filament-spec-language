// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-327-AC-14 and FR-611 (FSpec FR-092 "Derived forms", "Positions before
//! the origin"): the pre-origin reading of closed past intervals, the
//! origin-bounded reading of unbounded past operators, the derived forms of
//! strong previous, next and weak until, and the S1 refusal of `previous`,
//! `next` and `weak` written as operators.

use fsl_eval::temporal::{TemporalOutcome, TemporalTrace};
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

use crate::temporal_trace::{
    counter, counters, finite, fixture, lasso, outcome, violated, EVENT, INFINITE,
};

/// The closed `Counter` trace with `c.level` 1, 1, 1 under event-position
/// false-extension, `on origin`.
fn closed_ones(body: &str) -> TemporalOutcome {
    outcome(&counter(EVENT, body), finite(3), counters(&[1, 1, 1]))
}

/// The lasso with an empty prefix and loop `c.level` 1, 1, 1 under
/// infinite-trace, `on origin`.
fn lasso_ones(body: &str) -> TemporalOutcome {
    outcome(
        &fixture(INFINITE, "c: Counter", "on origin", body),
        lasso(0, 3),
        counters(&[1, 1, 1]),
    )
}

/// FR-327-AC-14: a closed past interval reads the pre-origin extension (atoms
/// false, constants unchanged, operators evaluated recursively there), under
/// event-position and infinite-trace alike; `eventually[1,1]` moves the read
/// to position 1.
#[trace("TC-1544", "FR-327-AC-14", "FSpec-TC-1041", "FSpec-FR-092-AC-9")]
#[test]
fn a_closed_past_interval_reads_the_pre_origin_extension() {
    for read in [closed_ones as fn(&str) -> TemporalOutcome, lasso_ones] {
        assert_eq!(read("once[1,1] holds(c.level = 1)"), violated(0));
        assert_eq!(
            read("once[1,1] not holds(c.level = 1)"),
            TemporalOutcome::Holds
        );
        assert_eq!(
            read("eventually[1,1] historically[0,2] holds(c.level = 1)"),
            violated(0)
        );
        assert_eq!(
            read("eventually[1,1] historically[0,2] true"),
            TemporalOutcome::Holds
        );
        assert_eq!(
            read("true since[1,1] not holds(c.level = 1)"),
            TemporalOutcome::Holds
        );
    }
}

/// FR-327-AC-14: under infinite-trace the unbounded and lower-bounded past
/// operators range over positions 0 through `i - a` only.
#[trace("TC-1544", "FR-327-AC-14", "FSpec-TC-1041", "FSpec-FR-092-AC-9")]
#[test]
fn an_unbounded_past_operator_ranges_over_the_positions_that_exist() {
    assert_eq!(
        lasso_ones("eventually[1,1] historically holds(c.level = 1)"),
        TemporalOutcome::Holds
    );
    assert_eq!(
        lasso_ones("eventually[1,1] once[2,*] not holds(c.level = 1)"),
        violated(0)
    );
}

/// Every `Counter` trace of length 1 to 4 over the values 0, 1 and 2.
fn traces() -> Vec<Vec<i64>> {
    let mut all = Vec::new();
    for len in 1..=4u32 {
        for code in 0..3i64.pow(len) {
            let mut code = code;
            let mut values = Vec::new();
            for _ in 0..len {
                values.push(code % 3);
                code /= 3;
            }
            all.push(values);
        }
    }
    all
}

/// The outcome an `on each` clause has when `holds_at(i)` is its truth at
/// each activation `i`: true everywhere, or false at the first position.
fn reference(len: usize, holds_at: impl Fn(usize) -> bool) -> TemporalOutcome {
    match (0..len).find(|i| !holds_at(*i)) {
        None => TemporalOutcome::Holds,
        Some(i) => violated(u64::try_from(i).expect("a small position")),
    }
}

/// FR-611-AC-3: the derived forms equal a reference strong previous, next and
/// bounded weak until over every small trace, with false extension past
/// closure and before the origin.
#[trace("TC-1542", "FR-611-AC-3", "FSpec-TC-1041", "FSpec-FR-092-AC-10")]
#[test]
fn the_derived_forms_carry_previous_next_and_weak_until() {
    const PREVIOUS: &str = "holds(c.level = 1) implies once[1,1] holds(c.level = 0)";
    const NEXT: &str = "eventually[1,1] holds(c.level = 1)";
    const WEAK_UNTIL: &str = "(holds(c.level = 0) until[0,2] holds(c.level = 2)) \
                              or always[0,2] holds(c.level = 0)";
    let each = |body: &str| fixture(EVENT, "c: Counter", "on each (q: Counter)", body);
    let (previous, next, weak) = (each(PREVIOUS), each(NEXT), each(WEAK_UNTIL));
    for values in traces() {
        let len = values.len();
        let at = |j: usize| values.get(j).copied();
        let run = |fx| {
            outcome(
                fx,
                TemporalTrace::Finite {
                    len: u64::try_from(len).expect("a small trace"),
                },
                counters(&values),
            )
        };
        assert_eq!(
            run(&previous),
            reference(len, |i| at(i) != Some(1) || (i > 0 && at(i - 1) == Some(0))),
            "{values:?}: previous"
        );
        assert_eq!(
            run(&next),
            reference(len, |i| at(i + 1) == Some(1)),
            "{values:?}: next"
        );
        let weak_until = |i: usize| {
            let p = |j: usize| at(j) == Some(0);
            let q = |j: usize| at(j) == Some(2);
            (0..=2).any(|d| q(i + d) && (0..d).all(|e| p(i + e))) || (0..=2).all(|d| p(i + d))
        };
        assert_eq!(
            run(&weak),
            reference(len, weak_until),
            "{values:?}: weak until"
        );
    }
}

/// The S1 diagnostics of a unit with one temporal clause whose formula is
/// `body`, over a record with a field `next`.
fn parse_diagnostics(body: &str) -> Vec<fsl_cst::CompleteDiagnostic> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile t0 = \"{EVENT}\";\n\
         record Counter {{ level: Int[0, 9]; next: Int[0, 9]; }}\n\
         temporal C using t0 over (c: Counter) clock \"events\" on origin {{ {body} }}\n"
    );
    fsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit")
    .diagnostics()
    .to_vec()
}

/// The one `invalid_syntax` refusal of `body`, its message and the text its
/// region covers.
fn refused_word(body: &str) -> (String, String) {
    let diagnostics = parse_diagnostics(body);
    let [diagnostic] = diagnostics.as_slice() else {
        panic!("{body}: one diagnostic, not {diagnostics:?}");
    };
    assert_eq!(diagnostic.code.as_str(), "invalid_syntax", "{body}");
    let span = diagnostic.byte_span().expect("a located refusal");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile t0 = \"{EVENT}\";\n\
         record Counter {{ level: Int[0, 9]; next: Int[0, 9]; }}\n\
         temporal C using t0 over (c: Counter) clock \"events\" on origin {{ {body} }}\n"
    );
    (
        diagnostic.message.clone(),
        text[span.start..span.end].to_owned(),
    )
}

/// FR-611-AC-1: `previous` and `next` written as operators refuse at the
/// word, naming the derived form.
#[trace("TC-1542", "FR-611-AC-1", "FSpec-TC-1041", "FSpec-FR-092-AC-10")]
#[test]
fn previous_and_next_refuse_naming_their_derived_forms() {
    for (body, word, form) in [
        ("previous holds(c.level = 0)", "previous", "once[1,1]"),
        (
            "always (holds(c.level = 1) implies previous holds(c.level = 0))",
            "previous",
            "once[1,1]",
        ),
        ("next holds(c.level = 1)", "next", "eventually[1,1]"),
    ] {
        let (message, at) = refused_word(body);
        assert_eq!(at, word, "{body}");
        assert!(message.contains(form), "{body}: {message}");
    }
}

/// FR-611-AC-2: `weak until` refuses at `weak`, naming the weak-until form,
/// and `next` stays an ordinary field name.
#[trace("TC-1542", "FR-611-AC-2", "FSpec-TC-1041", "FSpec-FR-092-AC-10")]
#[test]
fn weak_until_refuses_naming_its_derived_form_and_next_stays_an_identifier() {
    let (message, at) = refused_word("holds(c.level = 0) weak until holds(c.level = 2)");
    assert_eq!(at, "weak");
    assert!(message.contains("or always"), "{message}");
    assert_eq!(parse_diagnostics("always[0,1] holds(c.next = 1)"), []);
}

/// QSL-2060: the operator laws on random traces and formulas.
mod operator_laws {
    // SPDX-License-Identifier: AGPL-3.0-or-later
    //! QSL-2060: the temporal operator laws on random finite traces and lassos.
    //!
    //! Formulas come from a small bounded-fragment generator over three atoms
    //! (`holds(c.level = 1)`, `holds(c.level = 2)`, `holds(c.level = 3)`, read by text, not by value);
    //! a trace is a vector of 3-bit valuations. Each law builds two clauses as source
    //! text, takes both through S1, S2, the assembler and the temporal check, and compares
    //! the full `TemporalOutcome` (pending and undefined included) at every represented
    //! position with `evaluate_at`. The profiles covered are event-position false-extension
    //! (closed finite traces) and infinite-trace (finite prefixes and lassos). The
    //! fixed-sample and timestamped finite-window profiles read their positions through
    //! their own letters and are not covered here.
    use filament_exact::Meter;
    use fsl_eval::temporal::{
        evaluate, evaluate_at, AtomReading, CaptureReading, LetterStop, TemporalLetters,
        TemporalOutcome, TemporalPosition, TemporalRefusal, TemporalTrace,
    };
    use fsl_proptest_policy as prop;
    use fsl_semantics::check::{FormulaNode, FormulaNodeId, SCALAR_LIMITS_UNLIMITED};
    use proptest::prelude::*;

    use crate::temporal_trace::{fixture, Fixture, EVENT, INFINITE};

    const OVER: &str = "c: Counter";

    /// An interval `[low, high]`; `high == None` is `[low, *]`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Span {
        low: u8,
        high: Option<u8>,
    }

    impl Span {
        fn text(self) -> String {
            match self.high {
                Some(high) => format!("[{},{}]", self.low, high),
                None => format!("[{},*]", self.low),
            }
        }
    }

    /// A formula of the bounded fragment.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum F {
        Atom(u8),
        True,
        False,
        Not(Box<F>),
        And(Box<F>, Box<F>),
        Or(Box<F>, Box<F>),
        Eventually(Span, Box<F>),
        Always(Span, Box<F>),
        Once(Span, Box<F>),
        Historically(Span, Box<F>),
        Until(Span, Box<F>, Box<F>),
        Release(Span, Box<F>, Box<F>),
        Since(Span, Box<F>, Box<F>),
        Triggered(Span, Box<F>, Box<F>),
    }

    use F::*;

    fn b(f: F) -> Box<F> {
        Box::new(f)
    }

    /// How `until` is spelled when a formula is rendered: the real operator, or the
    /// weak-until stand-in of the negative test.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum UntilSpelling {
        Real,
        Weak,
    }

    impl F {
        fn text(&self, until: UntilSpelling) -> String {
            let t = |f: &F| f.text(until);
            match self {
                Atom(0) => "holds(c.level = 1)".into(),
                Atom(1) => "holds(c.level = 2)".into(),
                Atom(_) => "holds(c.level = 3)".into(),
                True => "true".into(),
                False => "false".into(),
                Not(x) => format!("not ({})", t(x)),
                And(x, y) => format!("(({}) and ({}))", t(x), t(y)),
                Or(x, y) => format!("(({}) or ({}))", t(x), t(y)),
                Eventually(i, x) => format!("(eventually{} ({}))", i.text(), t(x)),
                Always(i, x) => format!("(always{} ({}))", i.text(), t(x)),
                Once(i, x) => format!("(once{} ({}))", i.text(), t(x)),
                Historically(i, x) => format!("(historically{} ({}))", i.text(), t(x)),
                Until(i, x, y) => match until {
                    UntilSpelling::Real => format!("(({}) until{} ({}))", t(x), i.text(), t(y)),
                    UntilSpelling::Weak => format!(
                        "((({}) until{} ({})) or (always{} ({})))",
                        t(x),
                        i.text(),
                        t(y),
                        i.text(),
                        t(x)
                    ),
                },
                Release(i, x, y) => format!("(({}) release{} ({}))", t(x), i.text(), t(y)),
                Since(i, x, y) => format!("(({}) since{} ({}))", t(x), i.text(), t(y)),
                Triggered(i, x, y) => format!("(({}) triggered{} ({}))", t(x), i.text(), t(y)),
            }
        }

        /// Whether the formula is a Boolean combination of atoms.
        fn propositional(&self) -> bool {
            match self {
                Atom(_) | True | False => true,
                Not(x) => x.propositional(),
                And(x, y) | Or(x, y) => x.propositional() && y.propositional(),
                _ => false,
            }
        }

        /// Every atom replaced by `false`.
        fn atoms_false(&self) -> F {
            match self {
                Atom(_) => False,
                Not(x) => Not(b(x.atoms_false())),
                And(x, y) => And(b(x.atoms_false()), b(y.atoms_false())),
                Or(x, y) => Or(b(x.atoms_false()), b(y.atoms_false())),
                other => other.clone(),
            }
        }
    }

    /// What the generated formulas may use: bounded spans everywhere, plus `[l,*]` on infinite traces.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Shape {
        /// Event-position false-extension over a closed finite trace.
        Closed,
        /// Infinite-trace over a finite prefix.
        Prefix,
        /// Infinite-trace over a lasso.
        Lasso,
    }

    impl Shape {
        fn profile(self) -> &'static str {
            match self {
                Self::Closed => EVENT,
                Self::Prefix | Self::Lasso => INFINITE,
            }
        }
        fn open_spans(self) -> bool {
            self != Self::Closed
        }
    }

    fn span(open: bool) -> BoxedStrategy<Span> {
        let bounded = (0u8..=3, 0u8..=3).prop_map(|(low, width)| Span {
            low,
            high: Some(low + width),
        });
        if open {
            prop_oneof![4 => bounded, 1 => (0u8..=3).prop_map(|low| Span { low, high: None })]
                .boxed()
        } else {
            bounded.boxed()
        }
    }

    fn formula(open: bool) -> BoxedStrategy<F> {
        let leaf = prop_oneof![
            4 => (0u8..3).prop_map(Atom),
            1 => Just(True),
            1 => Just(False),
        ];
        leaf.prop_recursive(3, 12, 2, move |inner| {
            let sp = span(open);
            prop_oneof![
                2 => inner.clone().prop_map(|x| Not(b(x))),
                1 => (inner.clone(), inner.clone()).prop_map(|(x, y)| And(b(x), b(y))),
                1 => (inner.clone(), inner.clone()).prop_map(|(x, y)| Or(b(x), b(y))),
                1 => (sp.clone(), inner.clone()).prop_map(|(i, x)| Eventually(i, b(x))),
                1 => (sp.clone(), inner.clone()).prop_map(|(i, x)| Always(i, b(x))),
                1 => (sp.clone(), inner.clone()).prop_map(|(i, x)| Once(i, b(x))),
                1 => (sp.clone(), inner.clone()).prop_map(|(i, x)| Historically(i, b(x))),
                1 => (sp.clone(), inner.clone(), inner.clone()).prop_map(|(i, x, y)| Until(i, b(x), b(y))),
                1 => (sp.clone(), inner.clone(), inner.clone()).prop_map(|(i, x, y)| Release(i, b(x), b(y))),
                1 => (sp.clone(), inner.clone(), inner.clone()).prop_map(|(i, x, y)| Since(i, b(x), b(y))),
                1 => (sp, inner.clone(), inner).prop_map(|(i, x, y)| Triggered(i, b(x), b(y))),
            ]
        })
        .boxed()
    }

    /// A trace and the profile shape it is read under.
    #[derive(Clone, Debug)]
    struct Run {
        shape: Shape,
        valuations: Vec<u8>,
        /// Lasso only: the first position of the loop.
        prefix: usize,
    }

    impl Run {
        fn trace(&self) -> TemporalTrace {
            let len = self.valuations.len() as u64;
            match self.shape {
                Shape::Closed | Shape::Prefix => TemporalTrace::Finite { len },
                Shape::Lasso => TemporalTrace::Lasso {
                    prefix: self.prefix as u64,
                    loop_len: len - self.prefix as u64,
                },
            }
        }
    }

    fn runs() -> BoxedStrategy<Run> {
        (
            prop_oneof![Just(Shape::Closed), Just(Shape::Prefix), Just(Shape::Lasso)],
            proptest::collection::vec(0u8..8, 1..=6),
            0usize..6,
        )
            .prop_map(|(shape, valuations, prefix)| {
                let prefix = prefix % valuations.len();
                Run {
                    shape,
                    valuations,
                    prefix,
                }
            })
            .boxed()
    }

    /// A letter that reads each atom's bit of the position's valuation.
    struct Bits<'a> {
        fixture: &'a Fixture,
        valuations: &'a [u8],
    }

    impl TemporalLetters for Bits<'_> {
        fn atom(
            &mut self,
            atom: FormulaNodeId,
            position: TemporalPosition,
            _meter: &mut Meter,
        ) -> Result<AtomReading, LetterStop> {
            let Some(FormulaNode::Holds { span }) =
                self.fixture.clause.formula().nodes().get(atom.index())
            else {
                panic!("the letter reads only holds atoms");
            };
            let text = &self.fixture.text[span.start..span.end];
            let bit = if text.ends_with("= 1)") {
                0
            } else if text.ends_with("= 2)") {
                1
            } else {
                2
            };
            let valuation = self.valuations[usize::try_from(position.index()).unwrap()];
            Ok(if valuation >> bit & 1 == 1 {
                AtomReading::True
            } else {
                AtomReading::False
            })
        }
        fn activation(
            &mut self,
            _: TemporalPosition,
            _: &mut Meter,
        ) -> Result<AtomReading, LetterStop> {
            Ok(AtomReading::True)
        }
        fn captures(
            &mut self,
            _: TemporalPosition,
            _: &mut Meter,
        ) -> Result<CaptureReading, LetterStop> {
            Ok(CaptureReading::Fixed)
        }
        fn counts_step(&self, _: TemporalPosition) -> bool {
            true
        }
        fn counts_loop_edge(&self) -> bool {
            true
        }
    }

    fn clause(run: &Run, body: &str) -> Fixture {
        fixture(run.shape.profile(), OVER, "on origin", body)
    }

    /// The outcome of `fixture` at `position`, as a comparable string.
    fn at(run: &Run, fixture: &Fixture, position: u64) -> Result<TemporalOutcome, TemporalRefusal> {
        let mut letters = Bits {
            fixture,
            valuations: &run.valuations,
        };
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        evaluate_at(
            &fixture.clause,
            run.trace(),
            TemporalPosition::new(position),
            &mut letters,
            &mut meter,
        )
    }

    fn show(outcome: &Result<TemporalOutcome, TemporalRefusal>) -> String {
        format!("{outcome:?}")
    }

    /// Compare `left` and `right` as clause bodies at every represented position.
    fn agree(run: &Run, left: &str, right: &str) -> Result<(), TestCaseError> {
        let (l, r) = (clause(run, left), clause(run, right));
        for position in 0..run.valuations.len() as u64 {
            let (x, y) = (at(run, &l, position), at(run, &r, position));
            prop_assert_eq!(
                show(&x),
                show(&y),
                "at position {} on {:?} ({:?})\n  left:  {}\n  right: {}",
                position,
                run.valuations,
                run.shape,
                left,
                right
            );
        }
        Ok(())
    }

    fn suite(
        name: &'static str,
        strategy: impl Strategy<Value = (Run, F, F, Span)>,
        law: fn(&Run, &F, &F, Span) -> Result<(), TestCaseError>,
    ) {
        prop::run(name, prop::config(256), &strategy, |(run, x, y, i)| {
            law(&run, &x, &y, i)
        })
        .unwrap();
    }

    /// A run with formulas that match its profile's span rules.
    fn inputs() -> impl Strategy<Value = (Run, F, F, Span)> {
        runs().prop_flat_map(|run| {
            let open = run.shape.open_spans();
            (Just(run), formula(open), formula(open), span(open))
        })
    }

    fn real(f: &F) -> String {
        f.text(UntilSpelling::Real)
    }

    fn always_law(run: &Run, x: &F, _: &F, i: Span) -> Result<(), TestCaseError> {
        agree(
            run,
            &real(&Always(i, b(x.clone()))),
            &real(&Not(b(Eventually(i, b(Not(b(x.clone()))))))),
        )
    }

    /// `a release b` against `not (not a until not b)`, with `until` spelled as `spelling`.
    fn release_law_with(
        run: &Run,
        x: &F,
        y: &F,
        i: Span,
        spelling: UntilSpelling,
    ) -> Result<(), TestCaseError> {
        let right = Not(b(Until(i, b(Not(b(x.clone()))), b(Not(b(y.clone()))))));
        agree(
            run,
            &real(&Release(i, b(x.clone()), b(y.clone()))),
            &right.text(spelling),
        )
    }

    fn since_law(run: &Run, x: &F, y: &F, i: Span) -> Result<(), TestCaseError> {
        agree(
            run,
            &real(&Triggered(i, b(x.clone()), b(y.clone()))),
            &real(&Not(b(Since(
                i,
                b(Not(b(x.clone()))),
                b(Not(b(y.clone()))),
            )))),
        )
    }

    // ---- The explicit-semantics reference -------------------------------------------------
    //
    // A direct reading of the temporal requirement over an explicit trace, written by
    // quantifying over positions and sharing nothing with `fsl-eval/src/temporal/plan.rs`
    // (which implements `always`, `release`, `historically` and `triggered` as Boolean duals of a
    // single scan). Position `i` is a signed integer so the pre-origin extension is reachable.
    //
    // - FR-327 "Bounded-profile evaluation": a position after the closed trace reads every atom
    //   false; a position before 0 reads every atom false, and Boolean, future and past operators
    //   are evaluated there from those values (an offset is never skipped).
    // - FR-329 "Exact evaluation": under a lasso, position `p >= n` reads the observation of
    //   position `prefix + ((p - prefix) mod L)`; `p until[a,*] q` holds when `q` holds at some
    //   `j >= i+a` and `p` at every `k` with `i+a <= k < j`; `p since[a,*] q` holds when `q`
    //   holds at some `j <= i-a` and `p` at every `k` with `j < k <= i-a`. The unbounded and
    //   lower-bounded past operators range over positions 0 through `i - a` only (FR-327).
    // - FSpec FR-091 / FR-092 offsets: `until[a,b]` is `exists j in [i+a, i+b]` with `q` at `j`
    //   and `p` at every `k` in `[i+a, j)`; `release[a,b]` is `forall j in [i+a, i+b]` of `q` at
    //   `j` or `p` at some `k` in `[i+a, j)`; the past operators mirror them with `since[a,b]`
    //   being `exists j in [i-b, i-a]` with `q` at `j` and `p` at every `k` in `(j, i-a]`.
    //
    // It is two-valued, so it covers the closed trace and the lasso exactly. A finite prefix of
    // the infinite-trace profile is three-valued (FR-328): there the reference checks that a
    // settled verdict is confirmed by every sampled extension (a holds or a violation depends
    // on no continuation) and that the evaluator reports pending in no other case.

    /// The valuation bit of atom `bit` at position `p` of `run`, read as FR-327 / FR-329 say.
    fn atom_at(run: &Run, bit: u8, p: i64) -> bool {
        let n = run.valuations.len() as i64;
        let index = if p < 0 {
            return false;
        } else if p < n {
            p
        } else if run.shape == Shape::Lasso {
            let prefix = run.prefix as i64;
            prefix + (p - prefix) % (n - prefix)
        } else {
            return false;
        };
        run.valuations[index as usize] >> bit & 1 == 1
    }

    /// The positions an interval selects from `i`, as an inclusive range `lo..=hi`.
    /// A future `[a,*]` stops a settling margin after the later of `i + a` and the loop start:
    /// a subformula with past operators reads back at most `R` positions (FR-329 "Exact
    /// evaluation"), so from `prefix + R` on its truth repeats every loop; the generated
    /// formulas reach back at most 18 positions plus one loop per unbounded past operator, and
    /// the margin is 24 positions plus four loops.
    fn future_range(run: &Run, i: i64, s: Span) -> (i64, i64) {
        let lo = i + i64::from(s.low);
        let hi = match s.high {
            Some(h) => i + i64::from(h),
            None => {
                let l = (run.valuations.len() - run.prefix) as i64;
                lo.max(run.prefix as i64) + 24 + 4 * l
            }
        };
        (lo, hi)
    }

    /// The positions a past interval selects from `i`: `[i-b, i-a]`, or `[0, i-a]` for `[a,*]`.
    fn past_range(i: i64, s: Span) -> (i64, i64) {
        let hi = i - i64::from(s.low);
        let lo = match s.high {
            Some(h) => i - i64::from(h),
            None => 0,
        };
        (lo, hi)
    }

    /// Whether `f` holds at position `i` of `run`: the definition, quantifier by quantifier.
    fn sat(f: &F, run: &Run, i: i64) -> bool {
        let any = |(lo, hi): (i64, i64), test: &dyn Fn(i64) -> bool| (lo..=hi).any(test);
        let all = |(lo, hi): (i64, i64), test: &dyn Fn(i64) -> bool| (lo..=hi).all(test);
        match f {
            Atom(bit) => atom_at(run, *bit, i),
            True => true,
            False => false,
            Not(x) => !sat(x, run, i),
            And(x, y) => sat(x, run, i) && sat(y, run, i),
            Or(x, y) => sat(x, run, i) || sat(y, run, i),
            Eventually(s, x) => any(future_range(run, i, *s), &|j| sat(x, run, j)),
            Always(s, x) => all(future_range(run, i, *s), &|j| sat(x, run, j)),
            Once(s, x) => any(past_range(i, *s), &|j| sat(x, run, j)),
            Historically(s, x) => all(past_range(i, *s), &|j| sat(x, run, j)),
            Until(s, x, y) => {
                let (lo, hi) = future_range(run, i, *s);
                (lo..=hi).any(|j| sat(y, run, j) && (lo..j).all(|k| sat(x, run, k)))
            }
            Release(s, x, y) => {
                let (lo, hi) = future_range(run, i, *s);
                (lo..=hi).all(|j| sat(y, run, j) || (lo..j).any(|k| sat(x, run, k)))
            }
            Since(s, x, y) => {
                let (lo, hi) = past_range(i, *s);
                (lo..=hi).any(|j| sat(y, run, j) && (j + 1..=hi).all(|k| sat(x, run, k)))
            }
            Triggered(s, x, y) => {
                let (lo, hi) = past_range(i, *s);
                (lo..=hi).all(|j| sat(y, run, j) || (j + 1..=hi).any(|k| sat(x, run, k)))
            }
        }
    }

    /// `Some(truth)` for a settled outcome, `None` otherwise.
    fn settled(outcome: &Result<TemporalOutcome, TemporalRefusal>) -> Option<bool> {
        match outcome {
            Ok(TemporalOutcome::Holds) => Some(true),
            Ok(TemporalOutcome::Violated { .. }) => Some(false),
            _ => None,
        }
    }

    /// A finite prefix's continuation: a loop of one to three further valuations.
    fn extensions() -> impl Strategy<Value = Vec<u8>> {
        proptest::collection::vec(0u8..8, 1..=3)
    }

    /// The implementation against the reference at every represented position.
    fn definition_law(run: &Run, extension: &[u8], f: &F) -> Result<(), TestCaseError> {
        let fixture = clause(run, &real(f));
        for position in 0..run.valuations.len() as u64 {
            let got = at(run, &fixture, position);
            if run.shape == Shape::Prefix {
                // Three-valued (FR-328): every continuation confirms a settled verdict.
                let Some(truth) = settled(&got) else {
                    prop_assert!(
                        matches!(got, Ok(TemporalOutcome::Pending)),
                        "unsettled but not pending: {:?} for {} at {} on {:?}",
                        got,
                        real(f),
                        position,
                        run.valuations
                    );
                    continue;
                };
                let mut valuations = run.valuations.clone();
                valuations.extend_from_slice(extension);
                let lasso = Run {
                    shape: Shape::Lasso,
                    valuations,
                    prefix: run.valuations.len(),
                };
                prop_assert_eq!(
                    truth,
                    sat(f, &lasso, position as i64),
                    "prefix verdict {:?} of {} at {} on {:?} is contradicted by the continuation {:?}",
                    got, real(f), position, run.valuations, extension
                );
            } else {
                prop_assert_eq!(
                    settled(&got),
                    Some(sat(f, run, position as i64)),
                    "{} at {} on {:?} ({:?}, prefix {})",
                    real(f),
                    position,
                    run.valuations,
                    run.shape,
                    run.prefix
                );
            }
        }
        Ok(())
    }

    /// Run `definition_law` on formulas whose top operator is `top`.
    fn against_the_definition(name: &'static str, top: fn(Span, F, F) -> F) {
        let strategy = (inputs(), extensions());
        prop::run(
            name,
            prop::config(256),
            &strategy,
            |((run, x, y, i), extension)| definition_law(&run, &extension, &top(i, x, y)),
        )
        .unwrap();
    }

    /// Provenance: QSL-2060 P1 (FR-327, FR-328, FR-329). `eventually` is the quantifier over its interval.
    #[test]
    fn eventually_matches_its_definition() {
        against_the_definition("eventually_matches_its_definition", |i, x, _| {
            Eventually(i, b(x))
        });
    }

    /// Provenance: QSL-2060 P1. `always` is the universal quantifier over its interval (not the dual).
    #[test]
    fn always_matches_its_definition() {
        against_the_definition("always_matches_its_definition", |i, x, _| Always(i, b(x)));
    }

    /// Provenance: QSL-2060 P2. `until` is a witness of the right operand with the left operand
    /// held from the start of the interval up to it.
    #[test]
    fn until_matches_its_definition() {
        against_the_definition("until_matches_its_definition", |i, x, y| {
            Until(i, b(x), b(y))
        });
    }

    /// Provenance: QSL-2060 P2. `release` is stated directly (the right operand holds up to and
    /// including its first release), not as the dual of `until`.
    #[test]
    fn release_matches_its_definition() {
        against_the_definition("release_matches_its_definition", |i, x, y| {
            Release(i, b(x), b(y))
        });
    }

    /// Provenance: QSL-2060 P3. `since` is a witness of the right operand in the past with the
    /// left operand held after it up to the end of the interval.
    #[test]
    fn since_matches_its_definition() {
        against_the_definition("since_matches_its_definition", |i, x, y| {
            Since(i, b(x), b(y))
        });
    }

    /// Provenance: QSL-2060 P3. `triggered` is stated directly, not as the dual of `since`.
    #[test]
    fn triggered_matches_its_definition() {
        against_the_definition("triggered_matches_its_definition", |i, x, y| {
            Triggered(i, b(x), b(y))
        });
    }

    /// Provenance: QSL-2060 P3. `once` and `historically`, whose pre-origin reading FR-327 fixes.
    #[test]
    fn once_and_historically_match_their_definitions() {
        against_the_definition(
            "once_and_historically_match_their_definitions",
            |i, x, y| Or(b(Once(i, b(x))), b(Historically(i, b(y)))),
        );
    }

    /// Provenance: QSL-2060 P1 to P3. Whole generated formulas, every operator nested.
    #[test]
    fn nested_formulas_match_the_definition() {
        let strategy = (
            runs().prop_flat_map(|run| {
                let open = run.shape.open_spans();
                (Just(run), formula(open))
            }),
            extensions(),
        );
        prop::run(
            "nested_formulas_match_the_definition",
            prop::config(256),
            &strategy,
            |((run, f), extension)| definition_law(&run, &extension, &f),
        )
        .unwrap();
    }

    /// Provenance: QSL-2060 P1 (FR-043, FR-327). Consistency check, not a definition test: `always a`
    /// is `not eventually not a`. `plan.rs` implements `always` as exactly this dual, so the two sides
    /// share their code; `always_matches_its_definition` tests the operator against the reference.
    #[test]
    fn always_is_not_eventually_not() {
        suite("always_is_not_eventually_not", inputs(), always_law);
    }

    /// Provenance: QSL-2060 P2. Consistency check, not a definition test: `a release b` is
    /// `not (not a until not b)`, which is how `plan.rs` implements `release`; see
    /// `release_matches_its_definition` for the test against the reference.
    #[test]
    fn release_is_the_dual_of_until() {
        suite("release_is_the_dual_of_until", inputs(), |r, x, y, i| {
            release_law_with(r, x, y, i, UntilSpelling::Real)
        });
    }

    /// Provenance: QSL-2060 P3. Consistency check, not a definition test: `a triggered b` is
    /// `not (not a since not b)`, which is how `plan.rs` implements `triggered`; see
    /// `triggered_matches_its_definition` for the test against the reference.
    #[test]
    fn since_and_triggered_are_dual() {
        suite("since_and_triggered_are_dual", inputs(), since_law);
    }

    /// Provenance: QSL-2060 P4 (FR-328-AC-3). At position `i >= 1`, `once[1,1] a` is `a` at `i - 1`;
    /// at position 0, atoms read false before the origin, so for a propositional `a` the value is that
    /// of `a` with every atom replaced by `false`.
    #[test]
    fn once_one_one_is_the_previous_position() {
        prop::run(
            "once_one_one_is_the_previous_position",
            prop::config(256),
            &inputs(),
            |(run, x, _, _)| {
                let one = Span {
                    low: 1,
                    high: Some(1),
                };
                let once = clause(&run, &real(&Once(one, b(x.clone()))));
                let plain = clause(&run, &real(&x));
                for position in 1..run.valuations.len() as u64 {
                    let (got, want) = (at(&run, &once, position), at(&run, &plain, position - 1));
                    // `Violated { at }` names the position evaluated; compare the verdict kind and value.
                    prop_assert_eq!(
                        verdict(&got),
                        verdict(&want),
                        "once[1,1] ({}) at {} on {:?}",
                        real(&x),
                        position,
                        run.valuations
                    );
                }
                if x.propositional() {
                    let false_world = clause(&run, &real(&x.atoms_false()));
                    let (got, want) = (at(&run, &once, 0), at(&run, &false_world, 0));
                    prop_assert_eq!(
                        verdict(&got),
                        verdict(&want),
                        "once[1,1] ({}) at 0 on {:?}",
                        real(&x),
                        run.valuations
                    );
                }
                Ok(())
            },
        )
        .unwrap();
    }

    /// The settled value of an outcome, ignoring which position a violation names.
    fn verdict(outcome: &Result<TemporalOutcome, TemporalRefusal>) -> String {
        match outcome {
            Ok(TemporalOutcome::Violated { .. }) => "Violated".into(),
            other => show(other),
        }
    }

    /// Provenance: QSL-2060 P5. `eventually[l,u] a` at `i` is `eventually[0,u-l] a` at `i + l`
    /// whenever position `i + l` is represented.
    #[test]
    fn bounded_intervals_shift() {
        prop::run(
            "bounded_intervals_shift",
            prop::config(256),
            &inputs(),
            |(run, x, _, i)| {
                let Some(high) = i.high else { return Ok(()) };
                let shifted = Span {
                    low: 0,
                    high: Some(high - i.low),
                };
                let wide = clause(&run, &real(&Eventually(i, b(x.clone()))));
                let narrow = clause(&run, &real(&Eventually(shifted, b(x.clone()))));
                for position in 0..run.valuations.len() as u64 {
                    let target = position + u64::from(i.low);
                    if target >= run.valuations.len() as u64 {
                        continue;
                    }
                    let (got, want) = (at(&run, &wide, position), at(&run, &narrow, target));
                    prop_assert_eq!(
                        verdict(&got),
                        verdict(&want),
                        "eventually{} ({}) at {} vs eventually{} at {} on {:?}",
                        i.text(),
                        real(&x),
                        position,
                        shifted.text(),
                        target,
                        run.valuations
                    );
                }
                Ok(())
            },
        )
        .unwrap();
    }

    /// Provenance: QSL-2060 P6. `eventually[l,u] a` is `eventually[l,m] a or eventually[m+1,u] a`
    /// and `always[l,u] a` is the same split with `and`, for `l <= m < u`.
    #[test]
    fn bounded_intervals_split() {
        prop::run(
            "bounded_intervals_split",
            prop::config(256),
            &(inputs(), 0u8..8),
            |((run, x, _, i), cut)| {
                let high = i.high.map_or(i.low + 3, |h| h);
                if high <= i.low {
                    return Ok(());
                }
                let m = i.low + cut % (high - i.low);
                let (head, tail) = (
                    Span {
                        low: i.low,
                        high: Some(m),
                    },
                    Span {
                        low: m + 1,
                        high: i.high,
                    },
                );
                let whole = Span {
                    low: i.low,
                    high: i.high,
                };
                agree(
                    &run,
                    &real(&Eventually(whole, b(x.clone()))),
                    &real(&Or(
                        b(Eventually(head, b(x.clone()))),
                        b(Eventually(tail, b(x.clone()))),
                    )),
                )?;
                agree(
                    &run,
                    &real(&Always(whole, b(x.clone()))),
                    &real(&And(
                        b(Always(head, b(x.clone()))),
                        b(Always(tail, b(x.clone()))),
                    )),
                )
            },
        )
        .unwrap();
    }

    /// Provenance: QSL-2060 negative (FR-329-AC-9). A lasso under a bounded profile is refused
    /// before any atom is read, whatever the formula.
    #[test]
    fn lasso_under_a_bounded_profile_is_refused() {
        prop::run(
            "lasso_under_a_bounded_profile_is_refused",
            prop::config(256),
            &(formula(false), proptest::collection::vec(0u8..8, 1..=6)),
            |(f, valuations)| {
                let fx = fixture(EVENT, OVER, "on origin", &real(&f));
                let mut letters = Bits {
                    fixture: &fx,
                    valuations: &valuations,
                };
                let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
                let trace = TemporalTrace::Lasso {
                    prefix: 0,
                    loop_len: valuations.len() as u64,
                };
                let refused = evaluate(&fx.clause, trace, &mut letters, &mut meter);
                prop_assert!(
                    matches!(
                        refused,
                        Err(TemporalRefusal::LassoUnderBoundedProfile { .. })
                    ),
                    "{}: {:?}",
                    real(&f),
                    refused
                );
                Ok(())
            },
        )
        .unwrap();
    }

    /// Provenance: QSL-2060 negative. With `until` spelled as weak until (the strictness swapped),
    /// the release law fails and shrinks to a small formula on a short trace.
    #[test]
    fn swapped_until_is_caught() {
        let mut config = prop::config(256);
        config.failure_persistence = None;
        let outcome = prop::run(
            "swapped_until_is_caught",
            config,
            &inputs(),
            |(run, x, y, i)| release_law_with(&run, &x, &y, i, UntilSpelling::Weak),
        );
        match outcome {
            Err(proptest::test_runner::TestError::Fail(_, (run, x, y, _))) => {
                assert!(run.valuations.len() <= 3, "trace did not shrink: {run:?}");
                assert!(
                    real(&x).len() + real(&y).len() <= 120,
                    "formula did not shrink: {x:?} {y:?}"
                );
            }
            other => panic!("the swapped until went unnoticed: {other:?}"),
        }
    }
}
