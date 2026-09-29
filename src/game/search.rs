//! Beam search over code-level choices, one unit at a time.
//!
//! A unit is a standing behaviour, a trigger or an action. Each offers
//! candidates (pattern x variant) scored `weight * (axes . profile)`, where
//! the profile is `[game.profile]` shifted by the sentence's modifiers.
//! Judges (helper reuse, hoisting reuse) score the whole program so far,
//! which is why this is a beam and not a greedy pick. Ties break on the
//! candidate indices taken, so the result is deterministic.

use super::parser::{Body, GameProgram, Sentence};
use super::rules::{self, judge_score, Choice, Facts, Node, Stats};
use crate::config::{Axes, Config};
use crate::parser::Diag;

/// One node of the program, in emission order.
pub struct Unit<'p> {
    pub sentence: &'p Sentence,
    pub node: Node<'p>,
    /// Words shown in --explain.
    pub words: String,
}

/// Every unit, in order: per sentence, its standing behaviour, or its trigger then its actions.
pub fn units(p: &GameProgram) -> Vec<Unit<'_>> {
    let mut v = Vec::new();
    for s in &p.sentences {
        match &s.body {
            Body::Standing(st) => v.push(Unit { sentence: s, node: Node::Standing(st), words: s.text.clone() }),
            Body::Event { trigger, actions } => {
                let head = s.text.split(',').next().unwrap_or(&s.text).to_string();
                v.push(Unit { sentence: s, node: Node::Trigger(trigger), words: head });
                for a in actions {
                    v.push(Unit { sentence: s, node: Node::Action { act: &a.act, trigger }, words: a.words.clone() });
                }
            }
        }
    }
    v
}

/// The sentence's profile: the game default plus its modifiers.
pub fn profile(s: &Sentence, cfg: &Config) -> Axes {
    let mut p = cfg.game.profile;
    for m in &s.mods {
        p = p.add(&cfg.modifiers[m.as_str()]);
    }
    p
}

/// Every candidate for a unit, in registry x variant order.
pub fn candidates(u: &Unit, facts: &Facts, prof: &Axes, cfg: &Config) -> Result<Vec<Choice>, Diag> {
    let mut out = Vec::new();
    for p in rules::registry() {
        if !p.matches(&u.node) {
            continue;
        }
        for v in p.variants(&u.node, facts) {
            out.push(Choice { pattern: p.name(), variant: v, score: cfg.game_variant_score(p.name(), v, prof), uses: p.uses(v, &u.node) });
        }
    }
    if out.is_empty() {
        return Err(Diag::new(format!("no game pattern handles \"{}\" (in \"{}\")", u.words, u.sentence.text)));
    }
    Ok(out)
}

#[derive(Clone, Debug)]
struct State {
    choices: Vec<Choice>,
    local: f32,
    stats: Stats,
    key: Vec<u16>,
}

impl State {
    fn score(&self, cfg: &Config) -> f32 {
        self.local + judge_score(&self.stats, cfg).0
    }
}

pub struct Outcome {
    /// One choice per unit, in `units()` order.
    pub choices: Vec<Choice>,
    pub stats: Stats,
    pub score: f32,
    pub judge_notes: Vec<String>,
    pub runner_up: Option<f32>,
    pub explored: usize,
}

pub fn search(p: &GameProgram, cfg: &Config) -> Result<Outcome, Vec<Diag>> {
    let facts = Facts::of(p);
    let us = units(p);
    let mut all = Vec::new();
    let mut diags = Vec::new();
    for u in &us {
        let prof = profile(u.sentence, cfg);
        match candidates(u, &facts, &prof, cfg) {
            Ok(c) => all.push(c),
            Err(d) => diags.push(d),
        }
    }
    if !diags.is_empty() {
        return Err(diags);
    }
    // Every unit's candidates share their kind of site, so the first one says what is to come.
    let mut start = Stats::default();
    for cands in &all {
        start.expect(&cands[0].uses);
    }
    let mut beam = vec![State { choices: Vec::new(), local: 0.0, stats: start, key: Vec::new() }];
    let mut explored = 0;
    for cands in &all {
        let mut next = Vec::with_capacity(beam.len() * cands.len());
        for st in &beam {
            for (i, c) in cands.iter().enumerate() {
                explored += 1;
                let mut n = st.clone();
                n.local += c.score;
                n.stats.add(&c.uses);
                n.choices.push(c.clone());
                n.key.push(i as u16);
                next.push(n);
            }
        }
        let mut scored: Vec<(f32, State)> = next.into_iter().map(|s| (s.score(cfg), s)).collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.key.cmp(&b.1.key)));
        scored.truncate(cfg.search.beam.max(1));
        beam = scored.into_iter().map(|(_, s)| s).collect();
    }
    let best = beam[0].clone();
    let (_, judge_notes) = judge_score(&best.stats, cfg);
    Ok(Outcome {
        score: best.score(cfg),
        runner_up: beam.get(1).map(|s| s.score(cfg)),
        choices: best.choices,
        stats: best.stats,
        judge_notes,
        explored,
    })
}
