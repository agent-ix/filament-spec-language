// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1268: entry-readable binders keep repeat guards pre-test. A guard that
//! reads a body binder would require post-test semantics, which the current
//! checked-package writer refuses explicitly.

use filament_exact::{Cancel, Meter};
use fsl_analyze::model_check::{check_protocol, ProtocolCheckRequest};
use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::protocol::{MemorySelection, ProtocolSystem, StepKind, Tso};
use fsl_eval::temporal::AtomReading;
use fsl_foundation::MemoryModelName;
use fsl_replay::{ModelCheckItem, ModelCheckLimits, ModelCheckOutcome};
use fsl_semantics::check::FormulaNodeId;
use ix_trace_rs::trace;

use crate::litmus::{attempt, contracts, field, package, protocol, Documents};
use crate::weak_memory::select;

const PRETEST_CLAIM: &str = "temporal NoLoad using t0 over (k: M::Core) clock \"model-steps\" \
    on origin { always holds(k.r1 = 3) }\n";

struct ReadsValue(i64);

impl ModelLetters for ReadsValue {
    crate::units::unguarded!();

    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Ok(if field(state, "cores", &instance.key, "r1") == self.0 {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}

static READS_THREE: ReadsValue = ReadsValue(3);

struct Fixture {
    checked: fsl_package::CheckedPackage,
    documents: Documents,
    memory: Vec<MemorySelection>,
    letters: &'static dyn ModelLetters,
}

impl Fixture {
    /// The pre-test fixture whose guard reads `ready`, the binder of an
    /// event that precedes the repeat (so is readable at its entry) and is
    /// constrained false: a guard that
    /// reads no body binder stays pre-test, and as it is false at entry the
    /// body load never runs (SR-1737002 FND-001).
    fn entry_guard() -> Self {
        let run = format!(
            "parallel Both {{ branch w sequence W {{ {} }} \
             branch r sequence Rr {{ event Ready by c as (ready: Boolean) {{ not ready }}; \
             repeat Poll by c visible (ready) while {{ ready }} sequence R {{ {} }} }} }} \
             join all [w, r];",
            attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
            attempt("Lx", "c", "Core::look", "relaxed", "LooksX, Looked"),
        );
        Self::with_run(&run, PRETEST_CLAIM, &READS_THREE)
    }

    fn with_run(run: &str, claim: &str, letters: &'static dyn ModelLetters) -> Self {
        let body = contracts()
            + "pre LooksX using v on M::Core::look { c.id = 0 }\n"
            + &protocol("SB", run)
            + claim;
        let checked = package(&body);
        Self {
            checked,
            documents: Documents::new(),
            memory: vec![select("SB::Both", MemoryModelName::Tso)],
            letters,
        }
    }

    fn subject(&self) -> fsl_eval::model_subject::ProtocolSubject<'_> {
        self.documents.subject(&self.checked, "SB", &self.memory)
    }

    fn item(&self) -> ModelCheckItem<'_> {
        ModelCheckItem::Clause {
            clause: self.checked.graph().temporal_clauses()[0].clause(),
            letters: self.letters,
        }
    }
}

/// SR-1737002 FND-001: a repeat whose guard reads only a binder readable at
/// its entry is pre-test. Its guard is false on entry, so the body's load
/// never runs and `r1` keeps its initial value on every path.
#[trace("TC-1268", "FR-228-AC-2")]
#[test]
fn a_guard_over_an_entry_binder_is_pre_test_and_skips_the_body() {
    let fixture = Fixture::entry_guard();
    let outcome = check_protocol(
        ProtocolCheckRequest {
            temporal_clock_selections: &[],
            subject: fixture.subject(),
            item: fixture.item(),
            max_depth: None,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
            limits: ModelCheckLimits::default(),
            ignore_memory_constraints: false,
        },
        &Cancel::new(),
    )
    .expect("the checked request runs");
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "a false entry guard must skip the body load: {outcome:?}"
    );
}

/// FR-104-AC-14 / TC-1268: a repeat decision can read its own result-typed
/// body attempt binder, and those reads retain the producing role.
#[trace("TC-1268", "FR-104-AC-14")]
#[test]
fn a_repeat_visible_and_guard_read_the_latest_body_attempt_result() {
    let run = format!(
        "parallel Both {{ branch w sequence W {{ {} }} \
         branch c sequence R {{ repeat Spin by c visible (loaded) max 2 \
         while {{ not loaded }} sequence Poll {{ \
         attempt Load by c on M::Core::look ordering relaxed contracts [LooksX, Looked] \
         as (loaded: Boolean) {{ true }}; }} exhausted sequence SpinDone {{ }} }} }} \
         join all [w, c];",
        attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
    );
    let fixture = Fixture::with_run(&run, PRETEST_CLAIM, &READS_THREE);
    let subject = fixture.subject();
    let checked = subject
        .model
        .simulation
        .input
        .evaluator
        .protocol("SB")
        .expect("the protocol checks");
    let repeat = &checked.slots.repeats[0];
    assert_eq!(repeat.max, Some(2));
    assert!(repeat.post_test, "the guard reads its body attempt result");
    assert!(repeat.decisions_read_body);
    assert_eq!(repeat.visible.len(), 1);
    let binder = checked
        .slots
        .binders
        .iter()
        .position(|binder| binder.name == "loaded")
        .expect("the attempt binder is retained");
    assert_eq!(
        checked.slots.binders[binder].value_type,
        filament_exact::ValueType::Boolean
    );
    assert_eq!(
        checked.slots.readers[binder].readers,
        vec![repeat.node],
        "the repeat owned by role c reads both the visibility and guard binder"
    );

    let model = ModelSystem::new(subject.model.simulation.clone()).expect("the subject admits");
    let system =
        ProtocolSystem::new(model, &subject, Tso::new(4)).expect("the TSO protocol system builds");
    let reached = system
        .reachable(
            std::num::NonZeroUsize::new(1).expect("one is nonzero"),
            10_000,
        )
        .expect("the bounded litmus state graph is finite");
    let paths = protocol_paths(&reached.states);
    assert!(!paths.is_empty());
    assert!(
        paths.iter().all(|path| {
            let loads = path.iter().filter(|step| step.as_str() == "Load").count();
            (1..=2).contains(&loads)
        }),
        "one or two completed Load attempts precede repeat completion: {paths:?}"
    );
}

fn protocol_paths(
    reached: &[fsl_eval::simulation::protocol::ReachedState<Tso>],
) -> std::collections::BTreeSet<Vec<String>> {
    fn walk(
        reached: &[fsl_eval::simulation::protocol::ReachedState<Tso>],
        at: usize,
        prefix: &mut Vec<String>,
        out: &mut std::collections::BTreeSet<Vec<String>>,
    ) {
        if reached[at].steps.is_empty() {
            out.insert(prefix.clone());
            return;
        }
        for (transition, next) in &reached[at].steps {
            let named = match transition.kind() {
                StepKind::Attempt
                | StepKind::Recommend
                | StepKind::Event
                | StepKind::Send
                | StepKind::Receive
                | StepKind::Finish => transition
                    .node()
                    .and_then(|node| node.rsplit('/').next())
                    .map(str::to_owned),
                _ => None,
            };
            if let Some(name) = &named {
                prefix.push(name.clone());
            }
            walk(reached, *next, prefix, out);
            if named.is_some() {
                prefix.pop();
            }
        }
    }
    let mut out = std::collections::BTreeSet::new();
    walk(reached, 0, &mut Vec::new(), &mut out);
    out
}
