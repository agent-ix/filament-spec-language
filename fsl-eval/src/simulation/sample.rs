// SPDX-License-Identifier: AGPL-3.0-or-later
//! Seeded sampling over the same successor relation the exploration engine
//! walks, with the `filament.simulation.sampler/v1`
//! generator (FR-101; FSpec `proposals/filament-v1/definitions/
//! simulation-sampler.md`).

use filament_exact::NodeKey;
use filament_exact::ValueType;
use filament_semantic_value::declaration::TypeEnvironment;
use fsl_foundation::selection::DefinitionRef;

use crate::simulation::choice::{ChoicePoint, Weights, choose};
use crate::simulation::expansion::{ExpansionStop, StateFindings};
use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::{EncodingRefusal, StateKey};
use crate::simulation::not_simulated::{NotSimulated, Root, check_requires_bound};
use crate::simulation::order::{Expanded, OrderedSuccessor, expand, sorted_initial};
use crate::simulation::trace::{SampleProvenance, Step, StopReason, Trace};
use fsl_foundation::IdentityLimits;

/// The one generator `sample_request` runs (FR-101).
const SAMPLER_IDENTITY: &str = "filament.simulation.sampler/v1";

/// A source of draws for sampled exploration.
///
/// A sampler is deterministic in the sense the engine requires: the same
/// sampler state, driven the same way, produces the same sequence of
/// indices.
pub(crate) trait Sampler {
    /// Draw an index in `0..n`. `n` is always at least 1; the caller never
    /// asks a sampler to choose among zero candidates.
    fn next_index(&mut self, n: usize) -> usize;
}

/// The `filament.simulation.sampler/v1` generator without a workload: each step
/// makes one uniform choice over its enabled successors, at choice index 0
/// (FSpec FR-181), which is FR-188's weighted choice over unit weights.
pub(crate) struct PinnedSampler {
    seed: u64,
    trace: u64,
    step: u64,
    /// The last step's uniform weights, kept while the successor count
    /// stays the same, so a step does not recompute `⌊2^256 / n⌋`.
    uniform: Option<Weights>,
    /// How many draw digests this sampler has computed so far (TC-454
    /// step 3b): `n = 1` never computes one, so this is the observable
    /// signal that a call actually drew rather than taking the no-digest
    /// shortcut. Test-only instrumentation, not part of the pinned
    /// generator's behaviour.
    #[cfg(test)]
    digests_computed: u64,
}

impl PinnedSampler {
    pub(crate) fn new(seed: u64, trace: u64) -> Self {
        Self {
            seed,
            trace,
            step: 0,
            uniform: None,
            #[cfg(test)]
            digests_computed: 0,
        }
    }

    /// How many draw digests this sampler has computed so far.
    #[cfg(test)]
    pub(crate) fn digests_computed(&self) -> u64 {
        self.digests_computed
    }
}

impl Sampler for PinnedSampler {
    fn next_index(&mut self, n: usize) -> usize {
        let step = self.step;
        self.step = self.step.wrapping_add(1);
        let weights = match self.uniform.take() {
            Some(weights) if weights.len() == n => weights,
            _ => Weights::uniform(n).expect("next_index is called with n >= 1"),
        };
        let at = ChoicePoint {
            seed: self.seed,
            trace: self.trace,
            step,
            choice: 0,
        };
        let (index, _digests) = choose(at, &weights);
        self.uniform = Some(weights);
        #[cfg(test)]
        {
            self.digests_computed += _digests;
        }
        index
    }
}

/// [`sample`]'s result: the trace, `None` for a system with no initial
/// state, or the first encoding refusal reached.
/// An owned expansion, or an immutable model-memo entry. Sampling only
/// takes the chosen successor; an unchosen adjacency is never deep-copied.
pub(crate) trait SampleExpansionSource<T, S, F> {
    fn view(&self) -> &Expanded<T, S, F>;
    fn take(self, index: usize) -> Option<OrderedSuccessor<T, S>>;
}

/// What the observer saw at a sampled state: its complete ordered successors,
/// or the cause that prevented a complete expansion.
pub enum SampleExpansion<'a, T, S, F> {
    /// The complete successor list and findings for the current state.
    Complete {
        /// Successors in FR-101 canonical order.
        successors: SampleSuccessors<'a, T, S>,
        /// Findings from the complete expansion.
        findings: &'a [F],
    },
    /// The expansion stopped before a complete successor list was available.
    Stopped(ExpansionStop),
}

/// A view of a state's canonical successor list.
pub struct SampleSuccessors<'a, T, S>(&'a [OrderedSuccessor<T, S>]);

impl<'a, T, S> SampleSuccessors<'a, T, S> {
    /// Iterates the full canonical successor list without cloning states.
    pub fn iter(&self) -> impl Iterator<Item = SampleSuccessorRef<'_, T, S>> + '_ {
        self.0.iter().map(|successor| SampleSuccessorRef {
            transition: &successor.transition,
            state: &successor.state,
            key: &successor.key,
        })
    }
}

/// A borrowed view of one sampled successor.
pub struct SampleSuccessorRef<'a, T, S> {
    /// The transition identity of this edge.
    pub transition: &'a T,
    /// The successor state.
    pub state: &'a S,
    /// The complete canonical key of the successor.
    pub key: &'a StateKey,
}

/// One sampled state and the expansion the sampler obtained before choosing
/// its next transition.
pub struct SampleObservation<'a, S, T, F> {
    /// The number of transitions already taken to reach this state.
    pub step: usize,
    /// The current state.
    pub state: &'a S,
    /// The current state's canonical key.
    pub key: &'a StateKey,
    /// The expansion result for the current state.
    pub expansion: SampleExpansion<'a, T, S, F>,
    /// The transition selected to reach this state, absent at step zero.
    pub incoming: Option<&'a Step<T>>,
}

/// Whether an observer allows the sampled walk to continue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObserverDecision {
    /// Continue the ordinary seeded walk.
    Continue,
    /// Return the captured prefix without trace provenance.
    Stop,
}

/// The result of a sampled walk with an incremental observer.
pub enum ObservedSample<T, F> {
    /// The walk ended normally or by its ordinary sampler stop condition.
    Completed(Trace<T, F>),
    /// The observer stopped the walk; this is a prefix, not a completed trace.
    ObserverStopped(SamplePrefix<T, F>),
}

/// A captured observer-stopped walk prefix, without trace provenance.
pub struct SamplePrefix<T, F> {
    /// The canonical key of the initial state.
    pub initial: StateKey,
    /// The transitions taken from the initial state through the current state.
    pub steps: Vec<Step<T>>,
    /// Findings observed at states in this prefix.
    pub findings: Vec<StateFindings<F>>,
}

impl<T, S, F> SampleExpansionSource<T, S, F> for Expanded<T, S, F> {
    fn view(&self) -> &Expanded<T, S, F> {
        self
    }

    fn take(self, index: usize) -> Option<OrderedSuccessor<T, S>> {
        match self {
            Self::Successors { mut successors, .. } => Some(successors.swap_remove(index)),
            Self::Stopped(_) => None,
        }
    }
}

impl<T: Clone, S: Clone, F> SampleExpansionSource<T, S, F> for std::sync::Arc<Expanded<T, S, F>> {
    fn view(&self) -> &Expanded<T, S, F> {
        self
    }

    fn take(self, index: usize) -> Option<OrderedSuccessor<T, S>> {
        match self.as_ref() {
            Expanded::Successors { successors, .. } => successors.get(index).cloned(),
            Expanded::Stopped(_) => None,
        }
    }
}

/// One sampled walk's draw identity and limits.
struct SamplePlan {
    sampler: DefinitionRef,
    seed: u64,
    trace_index: u64,
    max_steps: usize,
    identity: IdentityLimits,
}

/// Draw one sampled run from `system`, starting at trace `trace_index mod m`
/// among `system`'s `m` distinct initial states (no draw selects the
/// start). The run ends at the first state where `end` holds, unexpanded
/// ([`StopReason::Reached`]); at the first state whose expansion stops;
/// after `max_steps` transitions; or at the first state with no successors,
/// checked in that order. Every other state on the trace is expanded, the
/// last included, and its findings recorded at its step index.
///
/// Returns `Ok(None)` when `system.initial()` returns no states.
///
/// # Errors
///
/// [`EncodingRefusal`] when a state or transition identity reached during
/// the run has no RFC 8785 encoding.
fn sample<S: TransitionSystem, E: SampleExpansionSource<S::TransitionId, S::State, S::Finding>>(
    system: &S,
    plan: SamplePlan,
    mut end: impl FnMut(&S::State, &StateKey) -> bool,
    mut expansion: impl FnMut(&S::State, &StateKey) -> Result<E, EncodingRefusal>,
    mut observer: impl FnMut(
        &SampleObservation<'_, S::State, S::TransitionId, S::Finding>,
    ) -> ObserverDecision,
) -> Result<Option<ObservedSample<S::TransitionId, S::Finding>>, EncodingRefusal> {
    let SamplePlan {
        sampler,
        seed,
        trace_index,
        max_steps,
        identity,
    } = plan;
    let mut initial = sorted_initial(system, identity)?;
    if initial.is_empty() {
        return Ok(None);
    }
    let m = initial.len() as u64;
    let start = (trace_index % m) as usize;
    // `start < m == initial.len()`, so `swap_remove` never panics.
    let chosen = initial.swap_remove(start);
    let mut current = chosen.state;
    let mut current_key = chosen.key.clone();
    let initial_key = chosen.key;

    let mut generator = PinnedSampler::new(seed, trace_index);
    let mut steps = Vec::new();
    let mut findings = Vec::new();
    let outcome = loop {
        if end(&current, &current_key) {
            break SampleOutcome::Completed(StopReason::Reached);
        }
        let expanded = expansion(&current, &current_key)?;
        let successors = match expanded.view() {
            Expanded::Successors {
                successors,
                findings: state_findings,
            } => {
                push_findings(&mut findings, &current_key, steps.len(), state_findings);
                let observation = SampleObservation {
                    step: steps.len(),
                    state: &current,
                    key: &current_key,
                    expansion: SampleExpansion::Complete {
                        successors: SampleSuccessors(successors),
                        findings: state_findings,
                    },
                    incoming: steps.last(),
                };
                if observer(&observation) == ObserverDecision::Stop {
                    break SampleOutcome::ObserverStopped;
                }
                successors
            }
            Expanded::Stopped(stop) => {
                let state_findings = system.findings_at_stop(&current);
                if !state_findings.is_empty() {
                    findings.push(StateFindings {
                        state: current_key.clone(),
                        depth: steps.len(),
                        findings: state_findings,
                    });
                }
                let observation = SampleObservation {
                    step: steps.len(),
                    state: &current,
                    key: &current_key,
                    expansion: SampleExpansion::Stopped(*stop),
                    incoming: steps.last(),
                };
                if observer(&observation) == ObserverDecision::Stop {
                    break SampleOutcome::ObserverStopped;
                }
                break SampleOutcome::Completed(StopReason::Stopped(stop.cause));
            }
        };
        if steps.len() >= max_steps {
            break SampleOutcome::Completed(StopReason::StepLimit);
        }
        if successors.is_empty() {
            break SampleOutcome::Completed(StopReason::NoSuccessors);
        }
        let n = successors.len();
        // `next_index(n)` always returns an index `< n` (its own contract),
        // so `swap_remove` never panics: no fallback branch is needed.
        let index = generator.next_index(n);
        // The ordinal is counted before `swap_remove` reorders `successors`.
        let ordinal = successors[..index]
            .iter()
            .filter(|earlier| earlier.transition == successors[index].transition)
            .count();
        let chosen = expanded
            .take(index)
            .expect("the drawn index addresses an expanded successor");
        current = chosen.state;
        current_key = chosen.key.clone();
        steps.push(Step {
            transition: chosen.transition,
            ordinal,
            key: chosen.key,
        });
    };

    Ok(Some(match outcome {
        SampleOutcome::Completed(stopped) => ObservedSample::Completed(Trace {
            initial: initial_key,
            steps,
            findings,
            provenance: Some(SampleProvenance {
                seed,
                trace: trace_index,
                sampler,
                stopped,
            }),
        }),
        SampleOutcome::ObserverStopped => ObservedSample::ObserverStopped(SamplePrefix {
            initial: initial_key,
            steps,
            findings,
        }),
    }))
}

enum SampleOutcome {
    Completed(StopReason),
    ObserverStopped,
}

fn push_findings<F>(
    output: &mut Vec<StateFindings<F>>,
    key: &StateKey,
    depth: usize,
    state_findings: &[F],
) where
    F: Clone,
{
    if !state_findings.is_empty() {
        output.push(StateFindings {
            state: key.clone(),
            depth,
            findings: state_findings.to_vec(),
        });
    }
}

/// Whether `sampler` is the one `filament.simulation.sampler/v1` generator
/// (FR-101), by identity. The only site that compares a sampler's identity
/// string.
#[ix_trace_rs::string_edge]
pub(crate) fn is_pinned_sampler(sampler: &DefinitionRef) -> bool {
    sampler.identity() == SAMPLER_IDENTITY
}

/// Sample `system`, first refusing a `sampler` other than the
/// `filament.simulation.sampler/v1` generator and an unbounded
/// `domains` request, before any `TransitionSystem` method is called or any
/// draw runs (FR-101).
///
/// `sample` and `Sampler` stay `pub(crate)`; this and `explore_request` are
/// the only entries that explore or sample.
///
/// # Errors
///
/// [`NotSimulated::GeneratorMismatch`] when `sampler`'s identity is not the
/// generator's; [`NotSimulated::RequiresBound`] or
/// [`NotSimulated::Extent`] as `explore_request`; [`NotSimulated::EmptyInitial`]
/// when `system.initial()` returns no states; [`NotSimulated::KeyEncoding`]
/// when a state or transition identity reached during the run has no RFC
/// 8785 encoding.
#[allow(
    clippy::too_many_arguments,
    reason = "FR-101 pins this exact signature; the parameters are the request's own fields, not an accretion of unrelated flags"
)]
pub fn sample_request<S: TransitionSystem, R: Root>(
    system: &S,
    domains: &[R],
    types: &TypeEnvironment,
    position_limit: u64,
    sampler: &DefinitionRef,
    seed: u64,
    trace: u64,
    max_steps: usize,
    identity: IdentityLimits,
) -> Result<Trace<S::TransitionId, S::Finding>, NotSimulated> {
    sample_request_until(
        system,
        SampleRequest {
            domains,
            types,
            position_limit,
            sampler,
            seed,
            trace,
            max_steps,
            identity,
        },
        |_| false,
    )
}

/// [`sample_request`]'s inputs.
pub struct SampleRequest<'r, R: Root = (NodeKey, &'r ValueType)> {
    /// The requires-bound roots.
    pub domains: &'r [R],
    /// The type environment the roots are classified under.
    pub types: &'r TypeEnvironment,
    /// The requires-bound pre-check's position budget.
    pub position_limit: u64,
    /// The sampler's `DefinitionRef`.
    pub sampler: &'r DefinitionRef,
    /// The run's seed.
    pub seed: u64,
    /// The trace's index within the run.
    pub trace: u64,
    /// The most steps the trace takes.
    pub max_steps: usize,
    /// The identity limits states and transitions encode under.
    pub identity: IdentityLimits,
}

/// [`sample_request`], ending the trace at the first state, position 0
/// included, where `end` holds, before that state is expanded:
/// [`StopReason::Reached`]. The draws are those of `sample_request` for the
/// same seed and trace index, so the trace is the prefix of the one
/// `sample_request` draws, up to that state (FR-166: a phase 0 walk ends at
/// its first target state).
///
/// # Errors
///
/// As [`sample_request`].
pub fn sample_request_until<S: TransitionSystem, R: Root>(
    system: &S,
    request: SampleRequest<'_, R>,
    mut end: impl FnMut(&S::State) -> bool,
) -> Result<Trace<S::TransitionId, S::Finding>, NotSimulated> {
    let identity = request.identity;
    sample_request_using(
        system,
        request,
        |state, _| end(state),
        |state, _| expand(system, state, identity),
    )
}

/// Draw a seeded walk and let `observer` inspect every visited state after its
/// expansion and before the next successor is selected. Returning `Stop`
/// returns a prefix with no trace provenance. Returning `Continue` at every
/// state produces the same completed trace and draws as [`sample_request`].
///
/// The observer is called on stopped expansions too, with the stop cause, so
/// callers can evaluate the current state while declining to infer closure.
///
/// # Errors
///
/// The same admission and encoding refusals as [`sample_request`].
pub fn sample_request_observed<S: TransitionSystem, R: Root>(
    system: &S,
    request: SampleRequest<'_, R>,
    observer: impl FnMut(
        &SampleObservation<'_, S::State, S::TransitionId, S::Finding>,
    ) -> ObserverDecision,
) -> Result<ObservedSample<S::TransitionId, S::Finding>, NotSimulated> {
    let identity = request.identity;
    sample_request_observed_using(
        system,
        request,
        |state, _| expand(system, state, identity),
        observer,
    )
}

/// The same admitted sampler with caller-supplied canonical expansions.
/// The model walk uses its request-local memo without changing draw order.
pub(crate) fn sample_request_using<
    S: TransitionSystem,
    R: Root,
    E: SampleExpansionSource<S::TransitionId, S::State, S::Finding>,
>(
    system: &S,
    request: SampleRequest<'_, R>,
    end: impl FnMut(&S::State, &StateKey) -> bool,
    expansion: impl FnMut(&S::State, &StateKey) -> Result<E, EncodingRefusal>,
) -> Result<Trace<S::TransitionId, S::Finding>, NotSimulated> {
    if !is_pinned_sampler(request.sampler) {
        return Err(NotSimulated::GeneratorMismatch {
            supplied: request.sampler.clone(),
        });
    }
    check_requires_bound(request.domains, request.types, request.position_limit)?;
    let sampled = sample(
        system,
        SamplePlan {
            sampler: request.sampler.clone(),
            seed: request.seed,
            trace_index: request.trace,
            max_steps: request.max_steps,
            identity: request.identity,
        },
        end,
        expansion,
        |_| ObserverDecision::Continue,
    )?
    .ok_or(NotSimulated::EmptyInitial)?;
    match sampled {
        ObservedSample::Completed(trace) => Ok(trace),
        ObservedSample::ObserverStopped(_) => {
            unreachable!("sample_request_using always continues its observer")
        }
    }
}

/// The same admitted sampler as [`sample_request_observed`] with a caller-
/// supplied canonical expansion source. The model walk uses its memoized
/// expansion without changing draw order.
pub(crate) fn sample_request_observed_using<
    S: TransitionSystem,
    R: Root,
    E: SampleExpansionSource<S::TransitionId, S::State, S::Finding>,
>(
    system: &S,
    request: SampleRequest<'_, R>,
    expansion: impl FnMut(&S::State, &StateKey) -> Result<E, EncodingRefusal>,
    observer: impl FnMut(
        &SampleObservation<'_, S::State, S::TransitionId, S::Finding>,
    ) -> ObserverDecision,
) -> Result<ObservedSample<S::TransitionId, S::Finding>, NotSimulated> {
    if !is_pinned_sampler(request.sampler) {
        return Err(NotSimulated::GeneratorMismatch {
            supplied: request.sampler.clone(),
        });
    }
    check_requires_bound(request.domains, request.types, request.position_limit)?;
    sample(
        system,
        SamplePlan {
            sampler: request.sampler.clone(),
            seed: request.seed,
            trace_index: request.trace,
            max_steps: request.max_steps,
            identity: request.identity,
        },
        |_, _| false,
        expansion,
        observer,
    )?
    .ok_or(NotSimulated::EmptyInitial)
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;
    use crate::simulation::choice::DrawPreimage;
    use crate::simulation::key::plain_digest;

    /// SHA-256 of `bytes`, by `sha2` directly: a path independent of
    /// `quire_canonical`, which the sampler's own digest goes through.
    fn independent_sha256(bytes: &[u8]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        Sha256::digest(bytes).into()
    }

    /// The preimage JSON of a draw, written by hand from FR-181's member list
    /// (sorted keys, decimal strings), independent of the serializer.
    fn written_preimage(seed: u64, trace: u64, step: u64, choice: u32, draw: u64) -> String {
        format!(
            r#"{{"choice":"{choice}","draw":"{draw}","seed":"{seed}","step":"{step}","trace":"{trace}"}}"#
        )
    }

    /// The draw digest's big-endian 256-bit value reduced mod `n`, folded a
    /// byte at a time with no big-integer type.
    fn reduce_big_endian(digest: &[u8; 32], n: usize) -> usize {
        digest
            .iter()
            .fold(0_usize, |rest, byte| (rest * 256 + usize::from(*byte)) % n)
    }

    /// The sampler's first `steps` indices over `n` candidates.
    fn indices(seed: u64, trace: u64, n: usize, steps: usize) -> Vec<usize> {
        let mut sampler = PinnedSampler::new(seed, trace);
        (0..steps).map(|_| sampler.next_index(n)).collect()
    }

    /// FSpec's published TC-210 "Sampler index-sequence vector" section.
    fn tc_210_section() -> Option<String> {
        let bytes = filament_specification::published_file(
            "spec/test-cases/TC-210-finite-simulation-reproducibility.md",
        )?;
        let text = std::str::from_utf8(bytes).expect("TC-210 is UTF-8");
        let start = text
            .find("## Sampler index-sequence vector")
            .expect("TC-210 has the sampler index-sequence section");
        let section = &text[start..];
        let end = section[3..]
            .find("\n## ")
            .map_or(section.len(), |at| at + 3);
        Some(section[..end].to_owned())
    }

    /// TC-454 step 1b: the sampler's step-0 preimage bytes and digest at
    /// seed 424242, trace 0, choice 0 are what the hand-written FR-181
    /// preimage hashes to under an independent SHA-256, and the same seed gives the
    /// same successor sequence.
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_step_0_preimage_and_digest_are_deterministic() {
        let preimage = DrawPreimage::at(
            ChoicePoint {
                seed: 424_242,
                trace: 0,
                step: 0,
                choice: 0,
            },
            0,
        );
        let bytes = quire_canonical::to_vec(&preimage, quire_canonical::Limits::new(u64::MAX))
            .expect("a draw preimage encodes");
        let written = written_preimage(424_242, 0, 0, 0, 0);
        assert_eq!(bytes, written.as_bytes());
        assert_eq!(
            plain_digest(&preimage),
            independent_sha256(written.as_bytes())
        );

        assert_eq!(indices(424_242, 0, 5, 8), indices(424_242, 0, 5, 8));
    }

    /// TC-454 step 1c: the published index sequence for seed 424242,
    /// trace 0, `n = 5` is the sampler's (read from the published FSpec TC-210).
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_published_index_sequence_is_the_samplers() {
        let section = tc_210_section().expect("pinned FSpec publishes TC-210");
        let published: Vec<usize> = section
            .split("index `")
            .skip(1)
            .map(|rest| {
                rest.split('`')
                    .next()
                    .and_then(|digits| digits.parse().ok())
                    .expect("TC-210 index is a decimal")
            })
            .collect();
        assert_eq!(published.len(), 5, "TC-210 publishes five steps");
        assert_eq!(indices(424_242, 0, 5, published.len()), published);
    }

    /// TC-454 step 3b: the sampler computes no digest at all over a chain
    /// where every state has exactly one successor (`n = 1` throughout).
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_one_successor_chain_computes_no_digest() {
        let mut sampler = PinnedSampler::new(424_242, 0);
        for _ in 0..5 {
            assert_eq!(sampler.next_index(1), 0);
        }
        assert_eq!(sampler.digests_computed(), 0);

        // A draw with more than one candidate does compute at least one
        // digest, confirming the counter is wired to the real draw path,
        // not merely never incremented.
        sampler.next_index(5);
        assert!(sampler.digests_computed() > 0);
    }

    /// SR-672 FND-005: the draw is the digest read big-endian, reduced mod
    /// `n`. `n = 3` and `n = 5` cannot tell big-endian from little-endian
    /// (`256 ≡ 1 mod 3` and `mod 5` make `v mod n` invariant under byte
    /// reversal), so this is checked at moduli that can, against an
    /// independent fold of the independently hashed preimage, and the two
    /// readings are required to differ somewhere in the sample so the check
    /// can fail on a byte-order flip.
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_draw_is_the_big_endian_digest_mod_n() {
        let mut readings_differ = false;
        for n in [2_usize, 4, 7, 9, 11] {
            let got = indices(424_242, 0, n, 32);
            for (step, index) in got.iter().enumerate() {
                let preimage = written_preimage(424_242, 0, step as u64, 0, 0);
                let digest = independent_sha256(preimage.as_bytes());
                assert_eq!(*index, reduce_big_endian(&digest, n), "n={n} step={step}");
                let mut reversed = digest;
                reversed.reverse();
                readings_differ |= reduce_big_endian(&reversed, n) != *index;
            }
        }
        assert!(
            readings_differ,
            "byte order must be observable in the sample"
        );
    }

    /// Same seed and trace give the same sequence; a different seed, trace
    /// or step changes the draw digest (domain separation), and every index
    /// is below `n`.
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_sequences_are_deterministic_and_separated() {
        assert_eq!(indices(7, 3, 6, 64), indices(7, 3, 6, 64));
        assert_ne!(indices(7, 3, 6, 64), indices(8, 3, 6, 64));
        assert_ne!(indices(7, 3, 6, 64), indices(7, 4, 6, 64));
        let digests: std::collections::BTreeSet<[u8; 32]> = (0..4_u64)
            .flat_map(|seed| (0..4_u64).map(move |trace| (seed, trace)))
            .flat_map(|(seed, trace)| (0..4_u64).map(move |step| (seed, trace, step)))
            .map(|(seed, trace, step)| {
                plain_digest(&DrawPreimage::at(
                    ChoicePoint {
                        seed,
                        trace,
                        step,
                        choice: 0,
                    },
                    0,
                ))
            })
            .collect();
        assert_eq!(digests.len(), 64, "no two choice points share a draw");
        for n in 1..=12 {
            assert!(indices(1, 1, n, 50).iter().all(|index| *index < n));
        }
    }

    /// Choice behaviour: over many steps every index of `n = 4` is drawn and
    /// none is drawn far from its uniform share.
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_choice_is_uniform_over_the_candidates() {
        let n = 4;
        let draws = 4_000;
        let mut counts = [0_usize; 4];
        for index in indices(99, 0, n, draws) {
            counts[index] += 1;
        }
        let share = draws / n;
        for (index, count) in counts.iter().enumerate() {
            assert!(
                count.abs_diff(share) < share / 10,
                "index {index} drawn {count} times of {draws}"
            );
        }
    }
}
