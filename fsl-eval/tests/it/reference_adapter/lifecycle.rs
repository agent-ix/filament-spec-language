// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-262-AC-3, AC-5, AC-8, AC-10 and AC-13 over the std reference adapter:
//! registry sealing and first-wins binding, the order of argument and call
//! charges, a charging body that saves a local across a child request, a
//! prepared entry against the synchronous reference, cancellation at kernel
//! and call charges, and the checked body root location of an own and an
//! imported callable.
//!
//! This module is a child of `reference_adapter` so it reuses that file's
//! harness.

use super::*;
use filament_eval::ExecutionBackendChoice;
use filament_exact::{Charge, IeeeFlag};
use filament_semantic_value::loss::{LocatedLoss, ValueLoss};
use fsl_eval::execution::{
    CallLimits, ExecuteLimits, ExecuteRequest, ExecuteSelection, ExecutionBackend, Interpreter,
    PrepareLimits, PreparedEntries, SuppliedBackends,
};
use fsl_forms::BinaryOperator;
use std::sync::OnceLock;

fn int_signature() -> Signature {
    Signature::new(vec![ValueType::Integer], ValueType::Integer)
}

/// `<name>(a: Integer): Integer { a }`.
fn identity_function(name: &str) -> FunctionDeclaration {
    FunctionDeclaration::new(
        name,
        vec![("a".to_owned(), form(&ValueType::Integer))],
        form(&ValueType::Integer),
        None,
        Expression::name("a".to_owned()),
    )
}

/// A host body that returns `value` and counts how often its factory begins.
struct Counted {
    value: i64,
    begun: Arc<AtomicUsize>,
}

impl BodyFactory for Counted {
    fn begin(&self) -> Box<dyn Body> {
        self.begun.fetch_add(1, Ordering::SeqCst);
        Box::new(ConstantBody(self.value))
    }
}

fn counted(value: i64, begun: &Arc<AtomicUsize>) -> Counted {
    Counted {
        value,
        begun: Arc::clone(begun),
    }
}

fn begun(counter: &Arc<AtomicUsize>) -> usize {
    counter.load(Ordering::SeqCst)
}

/// FR-262-AC-3: in an open registry the first factory for an admitted identity
/// is accepted and a second factory, or the first again, is a duplicate that
/// leaves the binding alone. A failed Begin leaves registration open, a
/// successful one seals it, after which the seal is reported before a
/// duplicate and for an unbound identity alike. The session and a later
/// session keep the first binding, and ending a session does not reopen it.
/// Registration invokes no factory.
#[trace("FR-262-AC-3")]
#[test]
fn registration_is_first_wins_and_a_successful_begin_seals_it() {
    let package = package(vec![
        identity_function("host"),
        identity_function("extra"),
        identity_function("late"),
    ]);
    let objects = ObjectEnvironment::default();
    let (host, extra, late) = (
        identity(&package, "host"),
        identity(&package, "extra"),
        identity(&package, "late"),
    );
    let (a, b, c) = (
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicUsize::new(0)),
    );
    let mut registry = Registry::new(Closure::new(&package));
    let signature = int_signature();

    assert_eq!(
        registry.register(host.clone(), &signature, counted(1, &a)),
        Admission::Accepted
    );
    assert_eq!(
        registry.register(host.clone(), &signature, counted(2, &b)),
        Admission::DuplicateBinding
    );
    assert_eq!(
        registry.register(host.clone(), &signature, counted(1, &a)),
        Admission::DuplicateBinding
    );
    assert_eq!(
        (begun(&a), begun(&b)),
        (0, 0),
        "registration invokes no factory"
    );

    // A failed Begin (an arity refusal) leaves registration open.
    let mut refused_meter = Meter::new(UNLIMITED);
    let refused = registry.begin_call(
        host.clone(),
        Vec::new(),
        SessionLimits::default(),
        &objects,
        &mut refused_meter,
    );
    assert!(matches!(refused, Err(CallFailure::Input(_))));
    assert!(!registry.is_sealed());
    assert_eq!(refused_meter.admission_count(), 0);
    assert_eq!(
        registry.register(extra.clone(), &signature, counted(3, &c)),
        Admission::Accepted,
        "the failed Begin left registration open"
    );

    // A successful Begin seals before any factory runs or charge is made.
    let mut meter = Meter::new(UNLIMITED);
    let session = registry
        .begin_call(
            host.clone(),
            vec![int(7)],
            SessionLimits::default(),
            &objects,
            &mut meter,
        )
        .expect("admitted call");
    assert!(registry.is_sealed());
    assert_eq!(begun(&a), 0, "sealing invokes no factory");
    assert_eq!(
        registry.register(host.clone(), &signature, counted(2, &b)),
        Admission::RegistrySealed,
        "the seal is reported before the duplicate"
    );
    assert_eq!(
        registry.register(late, &signature, counted(2, &b)),
        Admission::RegistrySealed,
        "a different admitted identity is sealed out too"
    );
    let first = session.run().expect("application");
    assert_eq!(completed_int(&first), Some(1), "the first binding runs");
    assert_eq!(work(&meter), 1);
    assert!(registry.is_sealed(), "a session's end does not reopen it");
    assert_eq!(
        registry.register(host.clone(), &signature, counted(2, &b)),
        Admission::RegistrySealed
    );

    let mut again = Meter::new(UNLIMITED);
    let later = registry
        .begin_call(
            host,
            vec![int(7)],
            SessionLimits::default(),
            &objects,
            &mut again,
        )
        .expect("admitted call")
        .run()
        .expect("application");
    assert_eq!(completed_int(&later), Some(1), "a later session keeps A");
    assert_eq!((begun(&a), begun(&b)), (2, 0));
}

/// `one(x: Int[0, 9]): Int[0, 9] { x }` and
/// `add(a: Int[0, 9], b: Int[0, 9]): Integer { a + b }`.
fn one_and_add() -> Vec<FunctionDeclaration> {
    vec![
        FunctionDeclaration::new(
            "one",
            vec![("x".to_owned(), form(&range(0, 9)))],
            form(&range(0, 9)),
            None,
            Expression::name("x".to_owned()),
        ),
        FunctionDeclaration::new(
            "add",
            vec![
                ("a".to_owned(), form(&range(0, 9))),
                ("b".to_owned(), form(&range(0, 9))),
            ],
            form(&ValueType::Integer),
            None,
            Expression::binary(
                BinaryOperator::Add,
                Expression::name("a".to_owned()),
                Expression::name("b".to_owned()),
            ),
        ),
    ]
}

/// The package over `functions` and the checked expression `source`.
fn with_expression(
    functions: Vec<FunctionDeclaration>,
    source: &Expression,
) -> (CheckedPackage, CheckedExpression) {
    let graph = declarations(functions)
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .unwrap();
    let expression = graph
        .check_expression(
            Vec::new(),
            source,
            Some(&ValueType::Integer),
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .unwrap();
    (CheckedPackage::link(graph), expression)
}

fn run_expression(
    package: &CheckedPackage,
    expression: &CheckedExpression,
    work_units: u64,
    stepped: bool,
) -> (Result<Evaluation, CallFailure>, Vec<ChargePoint>, u64) {
    let objects = ObjectEnvironment::default();
    let mut registry = Registry::new(Closure::new(package));
    let mut meter = Meter::new(ScalarLimits {
        work_units,
        ..UNLIMITED
    });
    let mut session = registry
        .begin_expression(
            expression,
            Vec::new(),
            SessionLimits::default(),
            &objects,
            &mut meter,
        )
        .expect("admitted expression");
    let result = if stepped {
        loop {
            match session.step() {
                Progress::Runnable(next) => session = next,
                Progress::Terminal(result) => break result,
            }
        }
    } else {
        session.run()
    };
    let points = meter.admitted_charges().iter().map(|c| c.point).collect();
    (result, points, work(&meter))
}

/// FR-262-AC-5: in `add(one(1), one(2))` the two argument applications are
/// charged left to right before `add`'s own call charge and body work, which
/// the located denial at each fuel limit shows. Stepping adds no charge, a
/// standalone literal charges no `function.call`, and a rejected root input
/// causes no charge and no body invocation.
#[trace("FR-262-AC-5")]
#[test]
fn argument_applications_charge_left_to_right_before_the_call_and_its_body() {
    let source = Expression::call(
        "add".to_owned(),
        vec![
            Expression::call("one".to_owned(), vec![Expression::integer(1_i64)]),
            Expression::call("one".to_owned(), vec![Expression::integer(2_i64)]),
        ],
    );
    let (package, expression) = with_expression(one_and_add(), &source);

    let (done, points, spent) = run_expression(&package, &expression, u64::MAX, false);
    assert_eq!(completed_int(&done.unwrap()), Some(3));
    assert_eq!(
        &points[..3],
        [ChargePoint::FunctionCall; 3],
        "two argument calls then add's own call: {points:?}"
    );
    assert!(
        points[3..]
            .iter()
            .all(|point| *point != ChargePoint::FunctionCall),
        "the body's work follows the three calls: {points:?}"
    );
    assert!(points.len() > 3, "add's body charges work: {points:?}");

    let (stepped, stepped_points, stepped_spent) =
        run_expression(&package, &expression, u64::MAX, true);
    assert_eq!(completed_int(&stepped.unwrap()), Some(3));
    assert_eq!(
        (stepped_points, stepped_spent),
        (points.clone(), spent),
        "steps add no charge"
    );

    // Each fuel limit below the first N denies the (N+1)th charge, so the
    // denial's location names which application is charged at that position.
    let denied_at = |limit: u64| {
        let (result, _, spent) = run_expression(&package, &expression, limit, false);
        let denied = result.expect("a denied charge is an evaluation");
        let FamilyOutcome::Evaluated(Outcome::Incomplete(record)) = &denied.outcome else {
            panic!("limit {limit}: expected an incomplete outcome, got {denied:?}");
        };
        assert_eq!(spent, limit, "earlier spend kept, denied charge unrecorded");
        (record.charge_point, denied.location.expect("located"))
    };
    let (point0, first) = denied_at(0);
    let (point1, second) = denied_at(1);
    let (point2, outer) = denied_at(2);
    assert_eq!(
        [point0, point1, point2],
        [ChargePoint::FunctionCall; 3],
        "the first three charges are calls"
    );
    assert_eq!(first.path(), [0], "the left argument is charged first");
    assert_eq!(second.path(), [1], "the right argument is charged second");
    assert!(
        outer.path().is_empty(),
        "add's own call is charged third, at the application"
    );
    let (point3, body) = denied_at(3);
    assert_ne!(point3, ChargePoint::FunctionCall, "then the body's work");
    assert_ne!(body, outer, "the body is located in add, not the call site");

    // A standalone literal charges no `function.call`, even with no fuel.
    let (literal_package, literal) = with_expression(one_and_add(), &Expression::integer(7_i64));
    let (result, literal_points, spent) = run_expression(&literal_package, &literal, 0, false);
    assert_eq!(completed_int(&result.unwrap()), Some(7));
    assert!(literal_points.is_empty(), "{literal_points:?}");
    assert_eq!(spent, 0);

    // A rejected root input charges nothing and invokes no body.
    let host_package = package_of_host();
    let host = identity(&host_package, "host");
    let invoked = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::new(Closure::new(&host_package));
    assert_eq!(
        registry.register(host.clone(), &int_signature(), counted(9, &invoked)),
        Admission::Accepted
    );
    let objects = ObjectEnvironment::default();
    for arguments in [vec![], vec![int(1), int(2)], vec![Value::Boolean(true)]] {
        let mut meter = Meter::new(UNLIMITED);
        let refused = registry.begin_call(
            host.clone(),
            arguments,
            SessionLimits::default(),
            &objects,
            &mut meter,
        );
        assert!(matches!(refused, Err(CallFailure::Input(_))));
        assert_eq!(meter.admission_count(), 0);
        assert_eq!(work(&meter), 0);
    }
    assert_eq!(begun(&invoked), 0, "no body was invoked");
}

fn package_of_host() -> CheckedPackage {
    package(vec![identity_function("host")])
}

/// A body that charges its own kernel work, saves its argument as a local,
/// requests a child, charges again and returns the child's value plus the
/// local, keeping the child's losses.
struct ChargingRelay {
    target: NodeKey,
    resumed: Arc<AtomicUsize>,
    begun: Arc<AtomicUsize>,
    /// The poll count of the meter's handle when the body started.
    polls: Arc<AtomicUsize>,
    start_poll: Arc<AtomicUsize>,
}

impl BodyFactory for ChargingRelay {
    fn begin(&self) -> Box<dyn Body> {
        self.begun.fetch_add(1, Ordering::SeqCst);
        Box::new(ChargingBody {
            target: self.target.clone(),
            resumed: Arc::clone(&self.resumed),
            polls: Arc::clone(&self.polls),
            start_poll: Arc::clone(&self.start_poll),
            saved: 0,
        })
    }
}

struct ChargingBody {
    target: NodeKey,
    resumed: Arc<AtomicUsize>,
    polls: Arc<AtomicUsize>,
    start_poll: Arc<AtomicUsize>,
    saved: i64,
}

impl ChargingBody {
    /// One unit of the body's own kernel work; a denial ends the body with the
    /// located incomplete evaluation, as the `Body` contract says.
    fn work(meter: &mut Meter) -> Option<Transition> {
        let denied = meter
            .charge(Charge::new(ChargePoint::IntegerArithmeticArithmetic))
            .err()?;
        Some(Transition::Done(Evaluation {
            outcome: FamilyOutcome::Evaluated(Outcome::Incomplete(denied)),
            location: Some(call_site()),
            losses: Vec::new(),
            frame_depth: 0,
            peak_live_occurrences: 0,
        }))
    }
}

impl Body for ChargingBody {
    fn resume(
        &mut self,
        input: Resume,
        _context: &mut filament_eval::LiveContext,
        meter: &mut Meter,
    ) -> Result<Transition, BodyFailure> {
        self.resumed.fetch_add(1, Ordering::SeqCst);
        if matches!(input, Resume::Start(_)) {
            self.start_poll
                .store(self.polls.load(Ordering::SeqCst), Ordering::SeqCst);
        }
        if let Some(done) = Self::work(meter) {
            return Ok(done);
        }
        match input {
            Resume::Start(arguments) => {
                self.saved = match arguments.first() {
                    Some(Value::Integer(n)) => i64::try_from(n.to_u64().unwrap()).unwrap(),
                    other => panic!("an Integer argument, not {other:?}"),
                };
                Ok(Transition::Call(filament_eval::ChildRequest {
                    callee: self.target.clone(),
                    arguments,
                    site: call_site(),
                }))
            }
            Resume::Child(child) => {
                let value = match &child.outcome {
                    FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(n))) => {
                        i64::try_from(n.to_u64().unwrap()).unwrap()
                    }
                    other => panic!("a completed child, not {other:?}"),
                };
                let mut done = Evaluation::completed(int(value + self.saved));
                done.losses = child.losses;
                Ok(Transition::Done(done))
            }
            Resume::Again => panic!("the charging body never yields"),
        }
    }
}

/// The Inexact loss a lossy child reports.
fn inexact_loss() -> LocatedLoss {
    LocatedLoss {
        location: call_site(),
        loss: ValueLoss::IeeeFlags([IeeeFlag::Inexact].into_iter().collect()),
    }
}

/// A child host body that returns `1` with one loss record.
struct Lossy;

impl BodyFactory for Lossy {
    fn begin(&self) -> Box<dyn Body> {
        Box::new(LossyBody)
    }
}

struct LossyBody;

impl Body for LossyBody {
    fn resume(
        &mut self,
        _input: Resume,
        _context: &mut filament_eval::LiveContext,
        _meter: &mut Meter,
    ) -> Result<Transition, BodyFailure> {
        let mut done = Evaluation::completed(int(1));
        done.losses = vec![inexact_loss()];
        Ok(Transition::Done(done))
    }
}

struct Relay {
    resumed: Arc<AtomicUsize>,
    begun: Arc<AtomicUsize>,
    start_poll: Arc<AtomicUsize>,
}

fn register_relay(
    registry: &mut Registry<Closure<'_>>,
    host: NodeKey,
    target: NodeKey,
    polls: &Arc<AtomicUsize>,
) -> Relay {
    let relay = Relay {
        resumed: Arc::new(AtomicUsize::new(0)),
        begun: Arc::new(AtomicUsize::new(0)),
        start_poll: Arc::new(AtomicUsize::new(usize::MAX)),
    };
    assert_eq!(
        registry.register(
            host,
            &int_signature(),
            ChargingRelay {
                target,
                resumed: Arc::clone(&relay.resumed),
                begun: Arc::clone(&relay.begun),
                polls: Arc::clone(polls),
                start_poll: Arc::clone(&relay.start_poll),
            },
        ),
        Admission::Accepted
    );
    relay
}

fn drive(session: filament_eval::Session<'_, Closure<'_>>) -> Result<Evaluation, CallFailure> {
    let mut session = session;
    loop {
        match session.step() {
            Progress::Runnable(next) => session = next,
            Progress::Terminal(result) => break result,
        }
    }
}

/// FR-262-AC-8: a body charges work, saves a local, requests a child and
/// continues with the child's value. Each body transition and the child occur
/// exactly once across steps, the body's charges and the child's call are made
/// in that order, and the child's loss record survives the hand-off.
#[trace("FR-262-AC-8")]
#[test]
fn a_charging_body_runs_each_transition_once_and_keeps_the_childs_losses() {
    let package = package(vec![identity_function("host"), identity_function("lossy")]);
    let objects = ObjectEnvironment::default();
    let (host, lossy) = (identity(&package, "host"), identity(&package, "lossy"));
    let polls = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::new(Closure::new(&package));
    let relay = register_relay(&mut registry, host.clone(), lossy.clone(), &polls);
    let child_begun = Arc::new(AtomicUsize::new(0));
    struct CountedLossy(Arc<AtomicUsize>);
    impl BodyFactory for CountedLossy {
        fn begin(&self) -> Box<dyn Body> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Lossy.begin()
        }
    }
    assert_eq!(
        registry.register(
            lossy,
            &int_signature(),
            CountedLossy(Arc::clone(&child_begun))
        ),
        Admission::Accepted
    );
    let mut meter = Meter::new(UNLIMITED);
    let result = drive(
        registry
            .begin_call(
                host,
                vec![int(5)],
                SessionLimits::default(),
                &objects,
                &mut meter,
            )
            .unwrap(),
    )
    .expect("application");
    assert_eq!(
        completed_int(&result),
        Some(1 + 5),
        "the child's value plus the saved local"
    );
    assert_eq!(result.losses, vec![inexact_loss()], "the loss survives");
    assert_eq!(
        (begun(&relay.begun), begun(&relay.resumed)),
        (1, 2),
        "one start and one child delivery"
    );
    assert_eq!(begun(&child_begun), 1, "the child ran once");
    let points: Vec<_> = meter.admitted_charges().iter().map(|c| c.point).collect();
    assert_eq!(
        points,
        [
            ChargePoint::FunctionCall,
            ChargePoint::IntegerArithmeticArithmetic,
            ChargePoint::FunctionCall,
            ChargePoint::IntegerArithmeticArithmetic,
        ]
    );
}

/// `f(): Float32[nearest-even]` over two NaN constants, whose sum records an
/// invalid-operation loss.
fn loss_package() -> CheckedPackage {
    let text = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function f using v(): Float32[nearest-even] pure \
         { float32(bits: 0x7fc00012) + float32(bits: 0xff800021) }\n";
    let parsed = fsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler admits the floating type");
    declarations.ieee_profile = Some(
        fsl_semantics::value::admit_ieee_profile(
            &[fsl_semantics::value::DefinitionRole::IeeeProfile.reference()],
            &[],
        )
        .unwrap(),
    );
    CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .expect("the package checks"),
    )
}

/// FR-262-AC-8: prepared-entry execution gives the synchronous reference's
/// evaluation, including a kernel loss record, and an entry whose backend is
/// not supplied is a pre-application disposition: `execute` returns
/// `Unsupported` before any application.
#[trace("FR-262-AC-8")]
#[test]
fn a_prepared_entry_matches_the_synchronous_reference_and_an_unsupported_one_stops_before_it() {
    use filament_eval::{UnsupportedCause, UnsupportedEntry};
    use filament_semantic_value::supplied_stop::{SuppliedLimits, SuppliedRun};

    let package = loss_package();
    let name = QualifiedName::unqualified("f").unwrap();
    let objects = ObjectEnvironment::default();
    let cancel = Cancel::new();
    let run = || SuppliedRun::new(SuppliedLimits::default(), Cancel::new()).unwrap();

    let reference = package
        .call_supplied(
            &name,
            Vec::new(),
            &objects,
            &mut Meter::new(UNLIMITED),
            &mut run(),
        )
        .expect("the synchronous reference runs");
    assert!(
        !reference.losses.is_empty(),
        "the reference records a kernel loss: {reference:?}"
    );

    let entries = [fsl_eval::execution::EntrySelection::function(name.clone())];
    let prepared = Interpreter
        .prepare(&package, &entries, &PrepareLimits::default(), &cancel)
        .expect("prepares");
    assert!(prepared.unsupported().is_empty());
    let called = prepared
        .call(
            &name,
            &[],
            &objects,
            &CallLimits {
                accounting: UNLIMITED,
                session: SessionLimits::default(),
            },
            &mut run(),
            &cancel,
        )
        .expect("the prepared entry runs");
    assert_eq!(format!("{called:?}"), format!("{reference:?}"));
    assert_eq!(called.losses, reference.losses);

    let limits = ExecuteLimits {
        accounting: UNLIMITED,
        prepare: PrepareLimits::default(),
        session: SessionLimits::default(),
    };
    let request = |backend| ExecuteRequest {
        selection: ExecuteSelection::Function {
            name: name.clone(),
            arguments: Vec::new(),
            environment: ObjectEnvironment::default(),
        },
        backend,
    };
    let executed = fsl_eval::execution::execute(
        &package,
        &request(ExecutionBackendChoice::Interpreter),
        &SuppliedBackends::default(),
        &limits,
        &mut run(),
        &cancel,
    )
    .expect("the interpreter executes");
    assert_eq!(format!("{executed:?}"), format!("{reference:?}"));
    let refused = fsl_eval::execution::execute(
        &package,
        &request(ExecutionBackendChoice::Jit),
        &SuppliedBackends::default(),
        &limits,
        &mut run(),
        &cancel,
    );
    assert_eq!(
        refused.expect_err("an unsupplied backend is not run"),
        CallFailure::Unsupported(UnsupportedEntry {
            backend: ExecutionBackendChoice::Jit,
            cause: UnsupportedCause::NotSupplied,
        })
    );
}

/// A handle that counts every poll and cancels with `cause` at poll number
/// `at`, once the cell holds it.
fn observing(polls: &Arc<AtomicUsize>, at: Option<usize>, cause: CancelCause) -> Cancel {
    let cell: Arc<OnceLock<Cancel>> = Arc::new(OnceLock::new());
    let cancel = Cancel::observing({
        let (cell, polls) = (Arc::clone(&cell), Arc::clone(polls));
        move || {
            if polls.fetch_add(1, Ordering::SeqCst) == at.unwrap_or(usize::MAX) {
                if let Some(handle) = cell.get() {
                    handle.cancel(cause);
                }
            }
        }
    });
    assert!(cell.set(cancel.clone()).is_ok());
    cancel
}

struct Run {
    result: Result<Evaluation, CallFailure>,
    cancel: Cancel,
    consumed: u64,
    start_poll: usize,
}

/// The charging relay over `keep`, with a handle that trips with `cause` at
/// handle poll `trip` (never when `None`).
fn relay_run(trip: Option<usize>, cause: CancelCause) -> Run {
    let package = package(vec![identity_function("host"), keep()]);
    let objects = ObjectEnvironment::default();
    let (host, target) = (identity(&package, "host"), identity(&package, "keep"));
    let polls = Arc::new(AtomicUsize::new(0));
    let cancel = observing(&polls, trip, cause);
    let mut registry = Registry::new(Closure::new(&package));
    let relay = register_relay(&mut registry, host.clone(), target, &polls);
    let mut meter = Meter::new(UNLIMITED).with_cancel(cancel.clone());
    let session = registry
        .begin_call(
            host,
            vec![int(0)],
            SessionLimits::default(),
            &objects,
            &mut meter,
        )
        .expect("admitted call");
    let result = drive(session);
    Run {
        result,
        cancel,
        consumed: work(&meter),
        start_poll: relay.start_poll.load(Ordering::SeqCst),
    }
}

fn assert_incomplete_at(run: &Run, point: ChargePoint) {
    let denied = run
        .result
        .as_ref()
        .expect("a denied charge is an evaluation, not CallFailure::Cancelled");
    let FamilyOutcome::Evaluated(Outcome::Incomplete(record)) = &denied.outcome else {
        panic!("expected an incomplete outcome, got {denied:?}");
    };
    assert_eq!(record.limit_kind, LimitKind::WorkUnits);
    assert_eq!(record.charge_point, point);
    assert_eq!(record.limit, record.consumed, "limit equals consumed");
    assert!(denied.location.is_some(), "the result is located");
}

/// FR-262-AC-10: `Begin` takes no `Cancel` argument. A meter handle cancelled
/// with `Deadline` at the first kernel charge a registered body makes ends the
/// session with that charge's located incomplete result, not
/// `CallFailure::Cancelled`, and `tripped()` reports `Deadline`. Cancelled at a
/// `function.call` charge, the result has the same form at that charge.
#[trace("FR-262-AC-10")]
#[test]
fn a_handle_cancelled_at_a_kernel_or_call_charge_is_a_located_incomplete_result() {
    let baseline = relay_run(None, CancelCause::Deadline);
    assert_eq!(completed_int(baseline.result.as_ref().unwrap()), Some(0));
    assert_eq!(baseline.consumed, 4);
    let start = baseline.start_poll;
    assert_ne!(start, usize::MAX, "the body started");

    // The first kernel charge the body makes: poll `start`.
    let at_work = relay_run(Some(start), CancelCause::Deadline);
    assert_incomplete_at(&at_work, ChargePoint::IntegerArithmeticArithmetic);
    assert_eq!(at_work.cancel.tripped(), Some(CancelCause::Deadline));
    assert_eq!(at_work.consumed, 1, "only the root call was charged");

    // The child's `function.call` charge follows the body's work.
    let at_call = relay_run(Some(start + 1), CancelCause::Deadline);
    assert_incomplete_at(&at_call, ChargePoint::FunctionCall);
    assert_eq!(at_call.cancel.tripped(), Some(CancelCause::Deadline));
    assert_eq!(at_call.consumed, 2);

    // The root's own `function.call` charge precedes the body's start.
    let at_root = relay_run(Some(start - 1), CancelCause::Deadline);
    assert_incomplete_at(&at_root, ChargePoint::FunctionCall);
    assert_eq!(at_root.consumed, 0);
}

/// A body that yields once with no charge, then requests `target`.
struct Yielding {
    target: NodeKey,
}

impl BodyFactory for Yielding {
    fn begin(&self) -> Box<dyn Body> {
        Box::new(YieldingBody {
            target: self.target.clone(),
            arguments: Vec::new(),
        })
    }
}

struct YieldingBody {
    target: NodeKey,
    arguments: Vec<Value>,
}

impl Body for YieldingBody {
    fn resume(
        &mut self,
        input: Resume,
        _context: &mut filament_eval::LiveContext,
        _meter: &mut Meter,
    ) -> Result<Transition, BodyFailure> {
        match input {
            Resume::Start(arguments) => {
                self.arguments = arguments;
                Ok(Transition::Continue)
            }
            Resume::Again => Ok(Transition::Call(filament_eval::ChildRequest {
                callee: self.target.clone(),
                arguments: self.arguments.clone(),
                site: call_site(),
            })),
            Resume::Child(child) => Ok(Transition::Done(*child)),
        }
    }
}

/// FR-262-AC-10, third clause: when the caller cancels between two steps and
/// the next transition makes no charge, that transition runs and the session
/// terminates at the first charge after it.
#[trace("FR-262-AC-10")]
#[test]
fn a_chargeless_transition_runs_after_a_cancellation_and_the_next_charge_ends_the_session() {
    let package = package(vec![identity_function("host"), keep()]);
    let objects = ObjectEnvironment::default();
    let (host, target) = (identity(&package, "host"), identity(&package, "keep"));
    let mut registry = Registry::new(Closure::new(&package));
    assert_eq!(
        registry.register(host.clone(), &int_signature(), Yielding { target }),
        Admission::Accepted
    );
    let cancel = Cancel::new();
    let mut meter = Meter::new(UNLIMITED).with_cancel(cancel.clone());
    let session = registry
        .begin_call(
            host,
            vec![int(2)],
            SessionLimits::default(),
            &objects,
            &mut meter,
        )
        .unwrap();
    // Enter the host (its call charge), then its first transition (a yield).
    let Progress::Runnable(session) = session.step() else {
        panic!("entering the host is not terminal");
    };
    let Progress::Runnable(session) = session.step() else {
        panic!("the yield is not terminal");
    };
    cancel.cancel(CancelCause::Deadline);
    // The next transition saves the child request and charges nothing: it runs.
    let Progress::Runnable(session) = session.step() else {
        panic!("a chargeless transition runs after the cancellation");
    };
    // The child's call charge is the first charge after the cancellation.
    let Progress::Terminal(Ok(denied)) = session.step() else {
        panic!("expected the incomplete result at the child's call charge");
    };
    assert!(matches!(
        &denied.outcome,
        FamilyOutcome::Evaluated(Outcome::Incomplete(record))
            if record.charge_point == ChargePoint::FunctionCall
                && record.limit_kind == LimitKind::WorkUnits
                && record.limit == record.consumed
    ));
    assert_eq!(cancel.tripped(), Some(CancelCause::Deadline));
    assert_eq!(work(&meter), 1);
}

/// FR-262-AC-13: `Begin` of an own callable and of an imported dependency
/// callable takes each one's checked body root location from the graph that
/// declares it. After a successful Begin, a handle cancelled before the first
/// `function.call` charge, and a work limit of zero, each return an incomplete
/// evaluation at exactly that location, with no admitted charge.
#[trace("FR-262-AC-13")]
#[test]
fn a_denied_root_call_is_located_at_the_declaring_graphs_body_root() {
    let dependency = Arc::new(located_dependency());
    let g = identity(&dependency, "g");
    let graph = declarations(vec![f()])
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .unwrap();
    let root = CheckedPackage::link_with(
        graph,
        vec![Import {
            identity: LibraryName::new("lib.g").unwrap(),
            package: Arc::clone(&dependency),
        }],
        fsl_package::EmitLimits::fixture(),
    )
    .expect("the dependency links");
    let f_key = identity(&root, "f");
    let own_root = root
        .graph()
        .function_by_identity(f_key.clone())
        .expect("f is declared")
        .body
        .location()
        .clone();
    let imported_root = dependency
        .graph()
        .function_by_identity(g.clone())
        .expect("g is declared")
        .body
        .location()
        .clone();
    let objects = ObjectEnvironment::default();

    for (callable, arguments, expected) in [
        (f_key, vec![int(3)], own_root),
        (g, Vec::new(), imported_root),
    ] {
        // Cancelled after Begin, before the first call charge.
        let mut registry = Registry::new(Closure::new(&root));
        let cancel = Cancel::new();
        let mut meter = Meter::new(UNLIMITED).with_cancel(cancel.clone());
        let session = registry
            .begin_call(
                callable.clone(),
                arguments.clone(),
                SessionLimits::default(),
                &objects,
                &mut meter,
            )
            .expect("admitted root");
        cancel.cancel(CancelCause::Requested);
        let result = session.run().expect("a denied charge is an evaluation");
        assert_eq!(result.location, Some(expected.clone()));
        assert!(matches!(
            &result.outcome,
            FamilyOutcome::Evaluated(Outcome::Incomplete(record))
                if record.charge_point == ChargePoint::FunctionCall
        ));
        assert_eq!(cancel.tripped(), Some(CancelCause::Requested));
        assert_eq!((work(&meter), meter.admission_count()), (0, 0));

        // A work-limit denial at that charge has the same location.
        let mut registry = Registry::new(Closure::new(&root));
        let mut meter = Meter::new(ScalarLimits {
            work_units: 0,
            ..UNLIMITED
        });
        let result = registry
            .begin_call(
                callable,
                arguments,
                SessionLimits::default(),
                &objects,
                &mut meter,
            )
            .expect("admitted root")
            .run()
            .expect("a denied charge is an evaluation");
        assert_eq!(result.location, Some(expected));
        assert!(matches!(
            &result.outcome,
            FamilyOutcome::Evaluated(Outcome::Incomplete(record))
                if record.charge_point == ChargePoint::FunctionCall
                    && record.limit_kind == LimitKind::WorkUnits
        ));
        assert_eq!(meter.admission_count(), 0);
    }
    assert_ne!(
        root.graph()
            .function_by_identity(identity(&root, "f"))
            .unwrap()
            .body
            .location(),
        dependency
            .graph()
            .function_by_identity(identity(&dependency, "g"))
            .unwrap()
            .body
            .location(),
        "the two roots are distinguished by their declaring graph"
    );
}
