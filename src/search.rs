//! Beam search over implementation choices, clause by clause.
//!
//! Each clause offers candidates (pattern variant x error strategy x retry
//! style). A candidate's local score is `weight * (axes . profile)`, where
//! the profile comes from `rules.toml` and the modifiers in the sentence.
//! Global judges (Result signature, consistent error style, helper reuse)
//! see the whole program so far, which is why this is a beam and not a
//! greedy pick. Ties break on candidate order, so the result is
//! deterministic.

use crate::config::{Axes, Config};
use crate::lexicon::{Modifier, Verb};
use crate::model::Ty;
use crate::parser::{Clause, Diag, Program};
use crate::rules::{self, error_strategy, retry, Choice, Env, Stats, Strategy, Variant};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// One option for one clause.
#[derive(Clone, Debug)]
pub struct Cand {
    pub choice: Choice,
    pub variant: Variant,
    pub delta: Stats,
}

/// Profile for a clause: the default plus its modifiers (and those of the
/// loops it sits in).
pub fn profile(c: &Clause, inherited: &[Modifier], cfg: &Config) -> Axes {
    let mut p = cfg.profile;
    let mut mods: Vec<Modifier> = inherited.to_vec();
    mods.extend(c.mods.iter().copied());
    mods.sort();
    mods.dedup();
    for m in mods {
        if let Some(key) = m.profile_key() {
            p = p.add(&cfg.modifiers[key]);
        }
    }
    p
}

/// Every candidate for `c` in `env`, or a diagnostic saying why none fit.
pub fn candidates(c: &Clause, env: &Env, prof: &Axes, cfg: &Config) -> Result<Vec<Cand>, Diag> {
    let matching: Vec<_> = rules::registry().into_iter().filter(|p| p.matches(c)).collect();
    if matching.is_empty() {
        return Err(Diag::new(format!("no pattern handles \"{}\"", c.words))
            .hint("v0 knows: read/write files, env vars, parse numbers, retry, for each, workers, sum, count, sort, remove duplicates, skip blank lines, trim, print, log, wait, create/list directories"));
    }
    let mut out = Vec::new();
    let mut why: Option<String> = None;
    for p in matching {
        let variants = p.variants(c, env);
        if variants.is_empty() {
            why.get_or_insert_with(|| {
                format!("\"{}\" doesn't fit here: the current value is {}", c.words, env.ty.describe())
            });
        }
        for v in variants {
            let base = cfg.variant_score(p.name(), v.name, prof);
            let standalone = c.verb == Verb::Retry;
            if !v.fallible && !standalone {
                if !c.on_error.is_empty() {
                    why = Some(format!("\"{}\" can't fail, so \"if it fails\" has nothing to handle", c.words));
                    continue;
                }
                if c.retry.is_some() {
                    why = Some(format!("\"{}\" can't fail, so there is nothing to retry", c.words));
                    continue;
                }
                out.push(make(p.name(), v, None, None, base, prof, cfg));
                continue;
            }
            if standalone {
                let spec = c.retry.as_ref().expect("parser attaches the spec");
                for style in retry::valid(spec, true) {
                    let s = base + cfg.retry.styles[style.key()].dot(prof);
                    out.push(make(p.name(), v.clone(), None, Some(style), s, prof, cfg));
                }
                continue;
            }
            let ok = if v.produces { v.output } else { Ty::Unit };
            let strategies = match error_strategy::valid(c, env, ok) {
                Ok(s) => s,
                Err(e) => {
                    why = Some(e);
                    continue;
                }
            };
            let styles: Vec<Option<retry::RetryStyle>> = match &c.retry {
                Some(spec) => retry::valid(spec, false).into_iter().map(Some).collect(),
                None => vec![None],
            };
            for st in strategies {
                for style in &styles {
                    let mut s = base + cfg.strategies[st.key()].dot(prof);
                    if let Some(style) = style {
                        s += cfg.retry.styles[style.key()].dot(prof);
                    }
                    out.push(make(p.name(), v.clone(), Some(st), *style, s, prof, cfg));
                }
            }
        }
    }
    if out.is_empty() {
        return Err(Diag::new(why.unwrap_or_else(|| format!("nothing fits \"{}\"", c.words))));
    }
    // Handlers that `return Err` need a Result signature too.
    for cand in &mut out {
        if cand.choice.strategy == Some(Strategy::Handle) && c.on_error.iter().any(|h| h.verb == Verb::Stop) {
            cand.delta.propagate += 1;
        }
    }
    Ok(out)
}

fn make(
    pattern: &'static str,
    variant: Variant,
    strategy: Option<Strategy>,
    style: Option<retry::RetryStyle>,
    score: f32,
    prof: &Axes,
    cfg: &Config,
) -> Cand {
    let mut choice = Choice {
        pattern,
        variant: variant.name,
        strategy,
        retry: style,
        score,
        notes: Vec::new(),
    };
    for j in rules::judges() {
        if let Some((s, note)) = j.local(&choice, prof, cfg.judge(j.name())) {
            choice.score += s;
            choice.notes.push(note);
        }
    }
    let delta = Stats {
        propagate: u32::from(strategy == Some(Strategy::Propagate)),
        expect: u32::from(strategy == Some(Strategy::Expect)),
        helper_uses: u32::from(style.is_some_and(|s| s.is_helper()) && pattern != "retry_helper"),
        brevity_sum: prof.brevity,
        clauses: 1,
    };
    Cand { choice, variant, delta }
}

pub fn global_score(stats: &Stats, cfg: &Config) -> (f32, Vec<String>) {
    let mut total = 0.0;
    let mut notes = Vec::new();
    for j in rules::judges() {
        if let Some((s, note)) = j.global(stats, cfg.judge(j.name())) {
            total += s;
            notes.push(note);
        }
    }
    (total, notes)
}

#[derive(Clone, Debug)]
struct Frame {
    env: Env,
    mods: Vec<Modifier>,
    produced: bool,
}

#[derive(Clone, Debug)]
struct State {
    choices: BTreeMap<usize, Choice>,
    /// Environment and profile each clause was decided in (for --explain).
    seen: BTreeMap<usize, (Env, Axes)>,
    local: f32,
    stats: Stats,
    frames: Vec<Frame>,
    /// Candidate indices taken, for deterministic tie-breaks.
    key: Vec<u16>,
}

impl State {
    fn score(&self, cfg: &Config) -> f32 {
        self.local + global_score(&self.stats, cfg).0
    }
    fn top(&mut self) -> &mut Frame {
        self.frames.last_mut().expect("root frame")
    }
}

enum Step<'a> {
    Clause(&'a Clause),
    Open(&'a Clause),
    Close,
}

fn flatten<'a>(cs: &'a [Clause], out: &mut Vec<Step<'a>>) {
    for c in cs {
        if c.is_loop() {
            out.push(Step::Open(c));
            flatten(&c.body, out);
            out.push(Step::Close);
        } else {
            out.push(Step::Clause(c));
        }
    }
}

pub struct Outcome {
    pub choices: BTreeMap<usize, Choice>,
    pub seen: BTreeMap<usize, (Env, Axes)>,
    pub score: f32,
    pub global_notes: Vec<String>,
    pub runner_up: Option<f32>,
    pub explored: usize,
}

fn apply(st: &State, c: &Clause, idx: usize, cand: &Cand, prof: Axes) -> State {
    let mut next = st.clone();
    let env_before = next.top().env.clone();
    next.seen.insert(c.id, (env_before.clone(), prof));
    next.choices.insert(c.id, cand.choice.clone());
    next.local += cand.choice.score;
    next.stats.propagate += cand.delta.propagate;
    next.stats.expect += cand.delta.expect;
    next.stats.helper_uses += cand.delta.helper_uses;
    next.stats.brevity_sum += cand.delta.brevity_sum;
    next.stats.clauses += cand.delta.clauses;
    next.key.push(idx as u16);
    if c.is_loop() {
        let inner_mods: Vec<Modifier> = {
            let top = next.top();
            let mut m = top.mods.clone();
            m.extend(c.mods.iter().copied().filter(|m| m.profile_key().is_some()));
            m
        };
        let in_worker = env_before.in_worker || cand.choice.pattern == "parallel";
        next.frames.push(Frame {
            env: Env { ty: cand.variant.output, sorted: false, in_loop: true, in_worker },
            mods: inner_mods,
            produced: false,
        });
    } else {
        let top = next.top();
        top.env.ty = cand.variant.output;
        top.env.sorted = cand.variant.sorted || (!cand.variant.produces && top.env.sorted);
        if cand.variant.produces {
            top.produced = true;
        }
    }
    next
}

pub fn search(prog: &Program, cfg: &Config) -> Result<Outcome, Vec<Diag>> {
    let mut steps = Vec::new();
    flatten(&prog.clauses, &mut steps);

    let root = Frame { env: Env::start(), mods: Vec::new(), produced: false };
    let mut beam = vec![State {
        choices: BTreeMap::new(),
        seen: BTreeMap::new(),
        local: 0.0,
        stats: Stats::default(),
        frames: vec![root],
        key: Vec::new(),
    }];
    let mut explored = 0;

    for step in &steps {
        let mut next: Vec<State> = Vec::new();
        let mut first_err: Option<Diag> = None;
        for st in &beam {
            match step {
                Step::Clause(c) | Step::Open(c) => {
                    let mut st = st.clone();
                    let (env, mods) = {
                        let top = st.top();
                        (top.env.clone(), top.mods.clone())
                    };
                    let prof = profile(c, &mods, cfg);
                    match candidates(c, &env, &prof, cfg) {
                        Ok(cands) => {
                            explored += cands.len();
                            for (i, cand) in cands.iter().enumerate() {
                                next.push(apply(&st, c, i, cand, prof));
                            }
                        }
                        Err(d) => {
                            first_err.get_or_insert(d);
                        }
                    }
                }
                Step::Close => {
                    let mut st = st.clone();
                    let inner = st.frames.pop().expect("loop frame");
                    let top = st.top();
                    top.env.sorted = false;
                    if inner.produced {
                        match inner.env.ty {
                            Ty::One(s) => {
                                top.env.ty = Ty::Many(s);
                                top.produced = true;
                            }
                            other => {
                                first_err.get_or_insert(Diag::new(format!(
                                    "a loop body must end with one value per item, not {}",
                                    other.describe()
                                )));
                                continue;
                            }
                        }
                    }
                    next.push(st);
                }
            }
        }
        if next.is_empty() {
            return Err(vec![first_err.unwrap_or_else(|| Diag::new("no implementation fits"))]);
        }
        sort_beam(&mut next, cfg);
        next.truncate(cfg.search.beam.max(1));
        beam = next;
    }

    let best = beam[0].clone();
    let (_, global_notes) = global_score(&best.stats, cfg);
    Ok(Outcome {
        score: best.score(cfg),
        runner_up: beam.get(1).map(|s| s.score(cfg)),
        choices: best.choices,
        seen: best.seen,
        global_notes,
        explored,
    })
}

fn sort_beam(states: &mut [State], cfg: &Config) {
    let mut scored: Vec<(f32, usize)> = states.iter().enumerate().map(|(i, s)| (s.score(cfg), i)).collect();
    scored.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then_with(|| states[a.1].key.cmp(&states[b.1].key))
    });
    let order: Vec<State> = scored.iter().map(|(_, i)| states[*i].clone()).collect();
    for (slot, s) in states.iter_mut().zip(order) {
        *slot = s;
    }
}

/// Stable ordering helper used by explain.
pub fn by_score(a: &Cand, b: &Cand) -> Ordering {
    b.choice.score.total_cmp(&a.choice.score)
}
