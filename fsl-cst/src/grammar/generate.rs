// SPDX-License-Identifier: AGPL-3.0-or-later
//! Grammar-derived source inputs for downstream property tests.
//!
//! This module is available only with `test-support`. It derives complete
//! units directly from the parser's rule table; no parallel grammar is kept.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::{complete_v1, Grammar, Rule, Terminal};
use crate::Production;

const INFINITE: usize = usize::MAX / 4;

/// One bounded derivation of a complete unit containing a requested production.
#[derive(Clone, Debug)]
pub struct Derivation {
    /// Production whose rule was expanded inside this unit.
    pub target: Production,
    /// Source derived from the real complete-V1 grammar.
    pub source: String,
}

// `productions!` defines both the fieldless enum and `all()` in the same
// order, so production ordinals index these private dense work arrays.
fn cost(rule: &Rule, costs: &[usize]) -> usize {
    match rule {
        Rule::Terminal(_) => 1,
        Rule::Production(p) => costs[*p as usize],
        Rule::Sequence(rules) => rules
            .iter()
            .map(|rule| cost(rule, costs))
            .fold(0usize, |a, b| a.saturating_add(b).min(INFINITE)),
        Rule::Choice(rules) => rules
            .iter()
            .map(|rule| cost(rule, costs))
            .min()
            .unwrap_or(INFINITE),
        Rule::Optional(_) => 0,
        Rule::Repeat { rule, minimum, .. } => {
            cost(rule, costs).saturating_mul(*minimum).min(INFINITE)
        }
    }
}

fn targeted_cost(rule: &Rule, normal: &[usize], targeted: &[usize]) -> usize {
    match rule {
        Rule::Terminal(_) => INFINITE,
        Rule::Production(p) => targeted[*p as usize],
        Rule::Sequence(rules) => rules
            .iter()
            .enumerate()
            .map(|(selected, _)| {
                rules
                    .iter()
                    .enumerate()
                    .map(|(index, rule)| {
                        if index == selected {
                            targeted_cost(rule, normal, targeted)
                        } else {
                            cost(rule, normal)
                        }
                    })
                    .fold(0usize, |a, b| a.saturating_add(b).min(INFINITE))
            })
            .min()
            .unwrap_or(INFINITE),
        Rule::Choice(rules) => rules
            .iter()
            .map(|rule| targeted_cost(rule, normal, targeted))
            .min()
            .unwrap_or(INFINITE),
        Rule::Optional(rule) => targeted_cost(rule, normal, targeted),
        Rule::Repeat { rule, minimum, .. } => targeted_cost(rule, normal, targeted)
            .saturating_add(cost(rule, normal).saturating_mul(minimum.saturating_sub(1)))
            .min(INFINITE),
    }
}

struct Builder<'a> {
    grammar: &'a Grammar,
    normal: &'a [usize],
    targeted: Vec<usize>,
    target: Production,
    replacement: Option<(Production, Box<Builder<'a>>)>,
}

impl Builder<'_> {
    fn render(&self, rule: &Rule, seek: bool, timed: bool, tokens: &mut Vec<String>) {
        match rule {
            Rule::Terminal(terminal) => {
                let token = match terminal {
                    Terminal::Exact(word)
                    | Terminal::Literal(word)
                    | Terminal::ContextualWord(word) => (*word).to_owned(),
                    Terminal::TextValue(value) => format!("{value:?}"),
                    Terminal::EmptyCallEnd { .. } => ")".to_owned(),
                    Terminal::Text => "\"x\"".to_owned(),
                    Terminal::UnsignedInteger | Terminal::Bound => "1".to_owned(),
                    Terminal::DecimalLiteral => "1.0".to_owned(),
                    Terminal::ClockType { word, .. } => (*word).to_owned(),
                    Terminal::Identifier
                    | Terminal::CallableName(_)
                    | Terminal::MemberName
                    | Terminal::TypeIdentifier { .. } => "sample".to_owned(),
                    Terminal::ExtensionKeyword(word)
                    | Terminal::ExtensionClause(word)
                    | Terminal::ExtensionSeparator(word) => word.to_string(),
                    Terminal::End | Terminal::Peek(_) => return,
                };
                tokens.push(token);
            }
            Rule::Production(production) => {
                let start = tokens.len();
                if let Some((context, replacement)) = &self.replacement {
                    if production == context {
                        replacement.render(&replacement.grammar[production], true, timed, tokens);
                        return;
                    }
                }
                self.render(
                    &self.grammar[production],
                    seek && *production != self.target,
                    timed || *production == Production::TimedInterval,
                    tokens,
                );
                if matches!(
                    production,
                    Production::Hex32
                        | Production::Hex64
                        | Production::ReferenceLiteral
                        | Production::StepLabel
                ) {
                    let hex = tokens.drain(start..).collect::<String>();
                    tokens.push(hex);
                }
            }
            Rule::Sequence(rules) => {
                let selected = seek
                    .then(|| {
                        rules
                            .iter()
                            .enumerate()
                            .min_by_key(|(index, _)| {
                                rules
                                    .iter()
                                    .enumerate()
                                    .map(|(other, rule)| {
                                        if *index == other {
                                            targeted_cost(rule, self.normal, &self.targeted)
                                        } else {
                                            cost(rule, self.normal)
                                        }
                                    })
                                    .fold(0usize, |a, b| a.saturating_add(b).min(INFINITE))
                            })
                            .map(|(index, _)| index)
                    })
                    .flatten();
                for (index, rule) in rules.iter().enumerate() {
                    self.render(rule, selected == Some(index), timed, tokens);
                }
            }
            Rule::Choice(rules) => {
                // A timed upper endpoint must be a quantity or `*`; this
                // shared choice also contains the untimed integer form.
                if timed {
                    if let Some(rule) = rules
                        .iter()
                        .find(|rule| matches!(rule, Rule::Production(Production::QuantityLiteral)))
                    {
                        self.render(rule, seek, timed, tokens);
                        return;
                    }
                }
                if let Some(rule) = rules.iter().min_by_key(|rule| {
                    if seek {
                        targeted_cost(rule, self.normal, &self.targeted)
                    } else {
                        cost(rule, self.normal)
                    }
                }) {
                    self.render(rule, seek, timed, tokens);
                }
            }
            Rule::Optional(rule) => {
                if seek {
                    self.render(rule, true, timed, tokens);
                }
            }
            Rule::Repeat { rule, minimum, .. } => {
                for index in 0..(*minimum).max(usize::from(seek)) {
                    self.render(rule, seek && index == 0, timed, tokens);
                }
            }
        }
    }
}

fn context_edges(
    rule: &Rule,
    normal: &[usize],
    context: usize,
    output: &mut Vec<(Production, usize)>,
) {
    match rule {
        Rule::Terminal(_) => {}
        Rule::Production(production) => output.push((*production, context)),
        Rule::Sequence(rules) => {
            for (selected, rule) in rules.iter().enumerate() {
                let siblings = rules
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != selected)
                    .map(|(_, rule)| cost(rule, normal))
                    .fold(context, |a, b| a.saturating_add(b).min(INFINITE));
                context_edges(rule, normal, siblings, output);
            }
        }
        Rule::Choice(rules) => {
            for rule in rules {
                context_edges(rule, normal, context, output);
            }
        }
        Rule::Optional(rule) => context_edges(rule, normal, context, output),
        Rule::Repeat { rule, minimum, .. } => {
            let copies = cost(rule, normal).saturating_mul(minimum.saturating_sub(1));
            context_edges(
                rule,
                normal,
                context.saturating_add(copies).min(INFINITE),
                output,
            );
        }
    }
}

type Contexts = Vec<Vec<(Production, usize)>>;

fn contexts(grammar: &Grammar, normal: &[usize]) -> Contexts {
    let mut graph = vec![Vec::new(); Production::all().len()];
    for (parent, rule) in grammar {
        let mut edges = Vec::new();
        context_edges(rule, normal, 1, &mut edges);
        for (child, weight) in edges {
            graph[child as usize].push((*parent, weight));
        }
    }
    graph
}

fn target_costs(
    contexts: &[Vec<(Production, usize)>],
    normal: &[usize],
    target: Production,
) -> Vec<usize> {
    let mut targeted = vec![INFINITE; Production::all().len()];
    targeted[target as usize] = normal[target as usize];
    let mut frontier = BinaryHeap::from([Reverse((normal[target as usize], target))]);
    while let Some(Reverse((distance, child))) = frontier.pop() {
        if targeted[child as usize] != distance {
            continue;
        }
        for &(parent, weight) in &contexts[child as usize] {
            let candidate = distance.saturating_add(weight).min(INFINITE);
            if candidate < targeted[parent as usize] {
                targeted[parent as usize] = candidate;
                frontier.push(Reverse((candidate, parent)));
            }
        }
    }
    targeted
}

/// Derive one bounded complete-unit context per grammar-table production.
///
/// The finite least-cost calculation bounds recursion without filtering parser
/// results. The caller must check admission and actual CST coverage: ambiguous
/// grammar alternatives can derive identical source, so a derivation alone is
/// deliberately not reported as parser coverage.
pub fn covering_derivations() -> Vec<Derivation> {
    let grammar = complete_v1();
    let mut normal = vec![INFINITE; Production::all().len()];
    for _ in 0..grammar.len() {
        let previous = normal.clone();
        for (production, rule) in &grammar {
            normal[*production as usize] = cost(rule, &previous).saturating_add(1).min(INFINITE);
        }
        if normal == previous {
            break;
        }
    }
    let contexts = contexts(&grammar, &normal);
    let mut output = Vec::new();
    for target in grammar.keys().copied() {
        let targeted = target_costs(&contexts, &normal, target);
        if targeted[Production::CompleteUnit as usize] >= INFINITE {
            continue;
        }
        let builder = Builder {
            grammar: &grammar,
            normal: &normal,
            targeted,
            target,
            replacement: None,
        };
        // `a.step.op` is also an ordinary member path. In a `holds`
        // condition that earlier grammar branch wins; MatchBlock is the
        // grammar's unambiguous hyper-expression context.
        let builder = if target == Production::StepLabel {
            Builder {
                grammar: &grammar,
                normal: &normal,
                targeted: target_costs(&contexts, &normal, Production::MatchBlock),
                target: Production::MatchBlock,
                replacement: Some((Production::MatchBlock, Box::new(builder))),
            }
        } else {
            builder
        };
        let mut tokens = Vec::new();
        builder.render(
            &Rule::Production(Production::CompleteUnit),
            true,
            false,
            &mut tokens,
        );
        output.push(Derivation {
            target,
            source: tokens.join(" "),
        });
    }
    output
}
