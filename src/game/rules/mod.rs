//! Game rules, read from `rules.toml` `[game.rules.*]` / `[game.judges.*]`.
//!
//! - **patterns** (one file each): recognise one kind of node (a standing
//!   behaviour, a trigger or an action), offer the code-level variants that fit,
//!   and emit Rust against Droid's `Ctx` API for the chosen one
//! - **judges** (`judges.rs`): score choices across the whole program
//!   (helper reuse, hoisting reuse)
//!
//! Numbers come from the sentence or from `[game.tuning]`; pattern code has
//! none of its own.

use super::codegen::Gen;
use super::parser::{Action, Body, GameProgram, Mode, Standing, Target, Trigger};
use crate::config::JudgeConfig;
use std::collections::BTreeSet;

pub mod animate;
pub mod at_bottom;
pub mod back_to_start;
pub mod bite;
pub mod bounce;
pub mod chase;
pub mod come_back;
pub mod drawn;
pub mod destroy;
pub mod enemy_shoot;
pub mod every;
pub mod every_frame;
pub mod falls_below;
pub mod fly;
pub mod get_away;
pub mod hang;
pub mod hit;
pub mod hooked;
pub mod hurt;
pub mod jig;
pub mod judges;
pub mod jump;
pub mod key_press;
pub mod landed;
pub mod lands_on;
pub mod lives;
pub mod lose;
pub mod march;
pub mod meter;
pub mod move_player;
pub mod music;
pub mod next_wave;
pub mod none_left;
pub mod patrol;
pub mod play_anim;
pub mod player_shoot;
pub mod random_shot;
pub mod reaches;
pub mod rise;
pub mod restart;
pub mod score;
pub mod show;
pub mod sound;
pub mod squash;
pub mod start;
pub mod swim;
pub mod touch;
pub mod wander;
pub mod win;

/// What a pattern looks at.
#[derive(Clone, Copy, Debug)]
pub enum Node<'p> {
    Standing(&'p Standing),
    Trigger(&'p Trigger),
    Action { act: &'p Action, trigger: &'p Trigger },
}

/// Where the emitted lines go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    /// `init`
    Init,
    /// `update`, before the "still playing?" check (key triggers).
    Keys,
    /// `update`, while playing.
    Update,
    /// `on_collision`, before the touches ("lands on" excludes them).
    Land,
    /// `on_collision`.
    Collide,
    /// `update`, after every other behaviour this frame (clamps and drawing that must see
    /// where things ended up).
    Late,
    /// Returned to the caller (actions inside a trigger).
    Inline,
}

/// Emitted code: lines relative to the function body, and where they go.
pub struct Emitted {
    pub section: Section,
    pub lines: Vec<String>,
}

impl Emitted {
    pub fn inline(lines: Vec<String>) -> Emitted {
        Emitted { section: Section::Inline, lines }
    }
    pub fn to(section: Section, lines: Vec<String>) -> Emitted {
        Emitted { section, lines }
    }
}

/// The function an action ends up in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Init,
    Update,
    Collision,
}

/// What an action can refer to where it runs.
#[derive(Clone, Debug, PartialEq)]
pub struct At {
    pub place: Place,
    /// `&str` expression for "it".
    pub it: Option<String>,
    /// (tag, `&str` expression) for "the shot" / "the alien".
    pub named: Vec<(String, String)>,
    /// The action runs inside a loop over entities.
    pub in_loop: bool,
    /// This is the last action of its trigger (a `return` after it is reachable-safe).
    pub last: bool,
}

impl At {
    pub fn plain(place: Place) -> At {
        At { place, it: None, named: Vec::new(), in_loop: false, last: false }
    }
    /// `&str` expression for a target that names one thing.
    pub fn one(&self, t: &Target) -> String {
        match t {
            Target::It => self.it.clone().expect("parser checked \"it\""),
            Target::Named(tag) => {
                self.named.iter().find(|(n, _)| n == tag).map(|(_, e)| e.clone()).expect("parser checked the name")
            }
            Target::Player => "\"player\"".into(),
            Target::Both | Target::All(_) => unreachable!("not a single target"),
        }
    }
}

/// Where a trigger's actions run and what "it" is there.
pub fn site(t: &Trigger) -> At {
    match t {
        Trigger::Start => At::plain(Place::Init),
        Trigger::Touch { thing, .. } | Trigger::LandsOn(thing) => At {
            place: Place::Collision,
            it: Some("b".into()),
            named: vec![(thing.clone(), "b".into())],
            in_loop: false,
            last: false,
        },
        Trigger::Hit { a, b, .. } => {
            let mut named = vec![(a.clone(), "a".to_string())];
            let it = match b {
                super::parser::Party::Thing(tb) => {
                    named.push((tb.clone(), "b".into()));
                    "b"
                }
                super::parser::Party::Player => "a",
            };
            At { place: Place::Collision, it: Some(it.into()), named, in_loop: false, last: false }
        }
        Trigger::Landed { kind, .. } | Trigger::AtBottom { kind, .. } => At {
            place: Place::Update,
            it: Some("&n".into()),
            named: vec![(kind.clone(), "&n".into())],
            in_loop: true,
            last: false,
        },
        Trigger::ThingBelow(thing, _) => At {
            place: Place::Update,
            it: Some("&n".into()),
            named: vec![(thing.clone(), "&n".into())],
            in_loop: true,
            last: false,
        },
        _ => At::plain(Place::Update),
    }
}

/// Program-wide facts patterns need (computed before the search).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Facts {
    /// The key of a "When the player presses K, restart the game." sentence.
    pub restart_key: Option<String>,
    /// How the player moves, if a sentence says.
    pub move_mode: Option<Mode>,
    /// Someone shoots "forward": movement must remember where it went.
    pub needs_aim: bool,
    /// Kinds that come back ("start the next wave of aliens").
    pub respawned: BTreeSet<String>,
    /// Any "loses a life".
    pub hurts: bool,
    /// Something gets hooked ("bites", "each hooked ...", "a hooked X reaches"): hooked
    /// things stop their own movement.
    pub hooks: bool,
    /// The lowest point a hook sinks to ("sinking ... down to D").
    pub hook_depth: Option<f32>,
    /// Something "gets away": it is shy for a while and doesn't bite.
    pub shy: bool,
    /// Kinds with an "Animate each X ... while it is hooked." sentence: their other
    /// animations skip the hooked ones.
    pub hooked_anim: BTreeSet<String>,
}

impl Facts {
    pub fn of(p: &GameProgram) -> Facts {
        let mut f = Facts::default();
        for s in &p.sentences {
            match &s.body {
                Body::Standing(Standing::Move { mode, .. }) => {
                    f.move_mode.get_or_insert(*mode);
                }
                Body::Standing(Standing::Shoot { aim: super::parser::Aim::Forward, .. }) => f.needs_aim = true,
                Body::Standing(Standing::Hooked { .. }) => f.hooks = true,
                Body::Standing(Standing::Animate { who: super::parser::Who::Each(k), hooked: true, .. }) => {
                    f.hooks = true;
                    f.hooked_anim.insert(k.clone());
                }
                Body::Standing(Standing::Hang { sink: Some((_, depth)), .. }) => {
                    f.hook_depth.get_or_insert(*depth);
                }
                Body::Standing(_) => {}
                Body::Event { trigger, actions } => {
                    if matches!(trigger, Trigger::Landed { .. } | Trigger::AtBottom { .. }) {
                        f.hooks = true;
                    }
                    for a in actions {
                        match &a.act {
                            Action::Restart => {
                                if let Trigger::Press(k) = trigger {
                                    f.restart_key.get_or_insert(k.clone());
                                }
                            }
                            Action::NextWave(k) => {
                                f.respawned.insert(k.clone());
                            }
                            Action::LoseLife { .. } => f.hurts = true,
                            Action::Bite(_) => f.hooks = true,
                            Action::GetAway(_) => f.shy = true,
                            _ => {}
                        }
                    }
                }
            }
        }
        f
    }
}

/// Helper-able code: win, lose, hurt. Index into `Stats` arrays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Helper {
    Win,
    Lose,
    Hurt,
}

/// What one choice contributes to the program-wide counters.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Uses {
    /// (which helper-able block, written as a helper call?)
    pub helper: Option<(Helper, bool)>,
    /// Reads the player position in a loop: hoisted once per frame?
    pub hoist: Option<bool>,
}

/// Counters the judges read. `[helper sites, inline sites]` per block, and
/// how many sites of each kind the search has not decided yet (so "a helper
/// used once" is only charged when no later site can reuse it).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    pub win: [u32; 2],
    pub lose: [u32; 2],
    pub hurt: [u32; 2],
    pub hoisted: u32,
    pub per_loop: u32,
    /// Undecided sites: win, lose, hurt.
    pub pending: [u32; 3],
    /// Undecided loops that read the player position.
    pub pending_hoist: u32,
}

fn helper_index(h: Helper) -> usize {
    match h {
        Helper::Win => 0,
        Helper::Lose => 1,
        Helper::Hurt => 2,
    }
}

impl Stats {
    /// Count a site the search will decide later.
    pub fn expect(&mut self, u: &Uses) {
        if let Some((h, _)) = u.helper {
            self.pending[helper_index(h)] += 1;
        }
        if u.hoist.is_some() {
            self.pending_hoist += 1;
        }
    }
    pub fn add(&mut self, u: &Uses) {
        if let Some((h, helper)) = u.helper {
            let slot = match h {
                Helper::Win => &mut self.win,
                Helper::Lose => &mut self.lose,
                Helper::Hurt => &mut self.hurt,
            };
            slot[usize::from(!helper)] += 1;
            let p = &mut self.pending[helper_index(h)];
            *p = p.saturating_sub(1);
        }
        match u.hoist {
            Some(true) => self.hoisted += 1,
            Some(false) => self.per_loop += 1,
            None => {}
        }
        if u.hoist.is_some() {
            self.pending_hoist = self.pending_hoist.saturating_sub(1);
        }
    }
    /// Sites of `h` still to come (hurt blocks carry a lose).
    pub fn pending_of(&self, h: Helper) -> u32 {
        match h {
            Helper::Lose => self.pending[1] + self.pending[2],
            other => self.pending[helper_index(other)],
        }
    }
    /// Losing the last life loses the game: that "lose" is written the way
    /// the program's own lose sites are (the majority, helper on a tie), or
    /// inline when there are none.
    pub fn embedded_lose_helper(&self) -> bool {
        self.lose[0] + self.lose[1] > 0 && self.lose[0] >= self.lose[1]
    }
    /// Lose sites including the ones inside hurt blocks.
    pub fn lose_sites(&self) -> [u32; 2] {
        let embedded = u32::from(self.hurt[0] > 0) + self.hurt[1];
        let mut c = self.lose;
        if embedded > 0 {
            c[usize::from(!self.embedded_lose_helper())] += embedded;
        }
        c
    }
}

pub trait GamePattern: Send + Sync {
    /// Key in `rules.toml` (`[game.rules.<name>]`).
    fn name(&self) -> &'static str;
    /// Every variant (validated against rules.toml both ways).
    fn all_variants(&self) -> &'static [&'static str];
    /// Does this pattern handle the node?
    fn matches(&self, n: &Node) -> bool;
    /// Variants that fit here.
    fn variants(&self, _n: &Node, _f: &Facts) -> Vec<&'static str> {
        self.all_variants().to_vec()
    }
    /// What the choice adds to the judges' counters.
    fn uses(&self, _variant: &str, _n: &Node) -> Uses {
        Uses::default()
    }
    /// Emit Rust. Standing behaviours place their lines; actions return them
    /// inline; triggers wrap `body` (their actions' lines) and place the block.
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, at: &At, body: Vec<String>) -> Emitted;
}

pub fn registry() -> Vec<&'static dyn GamePattern> {
    vec![
        // standing behaviours
        &move_player::MovePlayer,
        &jump::Jump,
        &player_shoot::PlayerShoot,
        &chase::Chase,
        &patrol::Patrol,
        &swim::Swim,
        &fly::Fly,
        &wander::Wander,
        &march::March,
        &enemy_shoot::EnemyShoot,
        &animate::Animate,
        &hang::Hang,
        &rise::Rise,
        &hooked::Hooked,
        &jig::Jig,
        &drawn::Drawn,
        // triggers
        &start::Start,
        &every_frame::EveryFrame,
        &every::Every,
        &touch::Touch,
        &hit::Hit,
        &lands_on::LandsOn,
        &none_left::NoneLeft,
        &reaches::Reaches,
        &falls_below::FallsBelow,
        &landed::Landed,
        &at_bottom::AtBottom,
        &key_press::KeyPress,
        // actions
        &destroy::Destroy,
        &score::Score,
        &lives::Lives,
        &hurt::Hurt,
        &win::Win,
        &lose::Lose,
        &sound::Sound,
        &music::Music,
        &show::Show,
        &bounce::Bounce,
        &back_to_start::BackToStart,
        &squash::Squash,
        &play_anim::PlayAnim,
        &restart::Restart,
        &next_wave::NextWave,
        &random_shot::RandomShot,
        &bite::Bite,
        &get_away::GetAway,
        &come_back::ComeBack,
        &meter::Meter,
    ]
}

pub fn pattern(name: &str) -> &'static dyn GamePattern {
    registry().into_iter().find(|p| p.name() == name).unwrap_or_else(|| panic!("unknown game pattern {name}"))
}

/// The search's decision for one node.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub pattern: &'static str,
    pub variant: &'static str,
    pub score: f32,
    pub uses: Uses,
}

impl Choice {
    pub fn label(&self) -> String {
        format!("{}/{}", self.pattern, self.variant)
    }
}

pub trait Judge: Send + Sync {
    /// Key in `rules.toml` (`[game.judges.<name>]`).
    fn name(&self) -> &'static str;
    /// Score the program so far.
    fn global(&self, stats: &Stats, cfg: &JudgeConfig) -> Option<(f32, String)>;
}

pub fn judges() -> Vec<&'static dyn Judge> {
    vec![&judges::HelperReuse, &judges::HoistReuse]
}

/// Score of the judges on `stats`, with notes for --explain.
pub fn judge_score(stats: &Stats, cfg: &crate::config::Config) -> (f32, Vec<String>) {
    let mut total = 0.0;
    let mut notes = Vec::new();
    for j in judges() {
        if let Some((s, note)) = j.global(stats, cfg.game_judge(j.name())) {
            total += s;
            notes.push(note);
        }
    }
    (total, notes)
}

// ---- small emit helpers shared by the patterns -------------------------------

/// An f32 literal (`2.0`, `-12.5`).
pub fn f(x: f32) -> String {
    format!("{x:?}")
}

/// A Rust string literal.
pub fn s(x: &str) -> String {
    format!("{x:?}")
}

/// Indent lines by `levels` x 4 spaces.
pub fn ind(lines: Vec<String>, levels: usize) -> Vec<String> {
    let pad = "    ".repeat(levels);
    lines.into_iter().map(|l| if l.is_empty() { l } else { format!("{pad}{l}") }).collect()
}

/// `head {` body `}`
pub fn block(head: impl Into<String>, body: Vec<String>) -> Vec<String> {
    let mut v = vec![format!("{} {{", head.into())];
    v.extend(ind(body, 1));
    v.push("}".into());
    v
}

/// Lines from a `&str` block (each line one entry).
pub fn lines(src: &str) -> Vec<String> {
    src.lines().map(|l| l.to_string()).collect()
}

/// The game's map of catches: hooked thing -> the hook it bit.
pub fn hooked_map(g: &mut Gen) -> String {
    g.field("hooked", "BTreeMap<String, String>", "BTreeMap::new()")
}

/// When each catch bit (game clock): hooked thing -> time.
pub fn hooked_at_map(g: &mut Gen) -> String {
    g.field("hooked_at", "BTreeMap<String, f32>", "BTreeMap::new()")
}

/// Things that got away, and until when they won't bite.
pub fn shy_map(g: &mut Gen) -> String {
    g.field("shy", "BTreeMap<String, f32>", "BTreeMap::new()")
}

/// When each thing was last jigged (game clock): jigged thing -> time.
pub fn jigged_map(g: &mut Gen) -> String {
    g.field("jigged", "BTreeMap<String, f32>", "BTreeMap::new()")
}

/// An expression: is the thing named by `n` (a `&str` or `&String` expression) jigged now, a
/// twitch less than `[game.tuning] jig_secs` ago?
pub fn is_jigged(g: &mut Gen, n: &str) -> String {
    let map = jigged_map(g);
    let clock = g.clock();
    format!("{map}.get({n}).is_some_and(|t| {clock} - *t < {})", f(g.t.jig_secs))
}

/// Lines that bind `top` to the position of the first `anchor` and run `body`
/// only when there is one.
pub fn with_anchor(anchor: &str, body: Vec<String>) -> Vec<String> {
    block(format!("if let Some(top) = cx.find_tag({}).first().map(|h| cx.pos(h))", s(anchor)), body)
}

/// A vector literal in the plane a mode moves in (x, y as given).
pub fn plane_vec(mode: Mode, x: &str, y: &str) -> String {
    match mode {
        Mode::Ground => format!("vec3({x}, 0.0, {y})"),
        _ => format!("vec2({x}, {y})"),
    }
}

/// Test helpers shared by the pattern files.
#[cfg(test)]
pub mod test_util {
    use super::*;
    use crate::game::parser::{parse, Sentence};

    pub fn sentence(s: &str) -> Sentence {
        parse(s, false).unwrap_or_else(|d| panic!("{d:?}")).sentences.remove(0)
    }

    /// Does `p` match a sentence's standing behaviour, trigger or any action?
    /// (`src` may hold several sentences, for sentences that need another one.)
    pub fn matches(p: &dyn GamePattern, src: &str) -> bool {
        parse(src, false).unwrap_or_else(|d| panic!("{d:?}")).sentences.iter().any(|s| match &s.body {
            Body::Standing(st) => p.matches(&Node::Standing(st)),
            Body::Event { trigger, actions } => {
                p.matches(&Node::Trigger(trigger))
                    || actions.iter().any(|a| p.matches(&Node::Action { act: &a.act, trigger }))
            }
        })
    }

    /// Compile a whole program with the built-in rules (for pattern tests that look at code).
    pub fn code(src: &str) -> String {
        let cfg = crate::config::Config::builtin();
        crate::game::compile(src, &cfg, &crate::game::GameOptions::default())
            .unwrap_or_else(|d| panic!("{d:?}"))
            .code
    }
}
