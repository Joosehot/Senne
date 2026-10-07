//! Game sentences -> a typed program.
//!
//! A sentence is either a **standing behaviour** (no trigger; runs every
//! frame) or **trigger, actions**. The trigger ends at the first comma.
//! Actions split on `,`, `and` and `then`; a piece without a verb joins the
//! action before it ("..., safe for 1.5 seconds"). Every head form is matched
//! strictly; the options after it may come in any order, each at most once,
//! and a word nothing consumed is an error. The parser never guesses.

use super::lexicon::{self, Tok, Token, ANIMS, KEYS, MODIFIERS, SOUNDS, WAVES};
use crate::parser::Diag;

/// How the player or an enemy moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    /// x only (platformers): "sideways".
    Sideways,
    /// Both axes of the 2D XY plane.
    Plane,
    /// 3D, the XZ plane: "across the ground".
    Ground,
}

/// Which way a projectile flies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aim {
    Up,
    Down,
    /// Where the player last moved.
    Forward,
    AtPlayer,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Who {
    Player,
    Each(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Standing {
    Move { mode: Mode, speed: f32, face: bool, smooth: bool, within: Option<f32> },
    Jump { key: String, speed: f32, air: Option<f32>, held: bool, sound: Option<String> },
    Shoot { aim: Aim, ground: bool, key: String, speed: f32, every: Option<f32>, sprite: String, sound: Option<String> },
    Chase { kind: String, speed: f32, mode: Mode, within: Option<f32>, speedup: Option<f32> },
    /// "patrols / swims N units to each side" (from home) or "within N units" (`edges`: turn at x = +-N).
    Patrol { kind: String, range: f32, speed: f32, edges: bool },
    Fly { kind: String, height: f32, freq: f32 },
    Wander { kind: String, speed: f32, turn: f32, ground: bool },
    March { kind: String, speed: f32, within: f32, step: f32, max_mult: Option<f32>, wave_pct: Option<f32> },
    EnemyShoot { kind: String, aim: Aim, speed: f32, every: f32, within: Option<f32>, sprite: String, sound: Option<String> },
    /// `walking`: "while it walks" (the player); `hooked`: "while it is hooked" (a kind: only
    /// the ones on a hook; a sentence for the same kind without it then skips them).
    Animate { who: Who, frames: Vec<String>, fps: f32, walking: bool, hooked: bool },
    /// "The hook hangs on a line from the hole[, sinking at N units per second down to D]."
    Hang { kind: String, anchor: String, sink: Option<(f32, f32)> },
    /// "The hook rises while the player shouts [or holds space], at [up to] N units per second [as loud as the shout]."
    Rise { kind: String, speed: f32, voice: bool, key: Option<String>, loud: bool },
    /// "Each hooked perch follows its hook[, fighting][, pulling it down at N units per second[ in bursts]]."
    /// `fight`: it darts from side to side and jerks the hook; `bursts`: it pulls in surges with rests.
    Hooked { kind: String, pull: Option<f32>, fight: bool, bursts: bool },
    /// "Each perch swims [in a school | in a shoal] within N units at N units per second" plus
    /// how: a gait, "turning smoothly", "in a wave N units high", "keeping its depth within N
    /// units", "nosing toward the hook within N units", "lunging at the hook within N units at
    /// N units per second". Without any of these the sentence is a [`Standing::Patrol`].
    Swim(Swim),
}

/// How a swimmer moves along: its speed over time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gait {
    /// The same speed all the time.
    Steady,
    /// "darting": a quick burst, a glide that slows down, a short pause; it may turn at a burst.
    Dart,
    /// "drifting": calm, the speed swells and eases a little.
    Drift,
    /// "lurking": hovers almost still, the fins drifting it slowly; now and then it turns.
    Lurk,
}

/// The parts of a swim sentence.
#[derive(Clone, Debug, PartialEq)]
pub struct Swim {
    pub kind: String,
    /// Turns at x = +-within.
    pub within: f32,
    pub speed: f32,
    /// "in a school" / "in a shoal": keeps together with the others of its kind, loosely spaced,
    /// and turns with the school.
    pub school: bool,
    pub gait: Gait,
    /// "turning smoothly": slows down before it turns.
    pub smooth: bool,
    /// "in a wave N units high": rises and sinks this much while it swims.
    pub wave: Option<f32>,
    /// "keeping its depth within N units": drifts within this of the depth it started at.
    pub band: Option<f32>,
    /// "nosing toward the hook within N units": (thing, distance).
    pub nose: Option<(String, f32)>,
    /// "lunging at the hook within N units at N units per second": (thing, distance, speed).
    pub lunge: Option<(String, f32, f32)>,
}

/// An extra condition on a collision trigger: "and no coins are left", "and no fish is hooked".
#[derive(Clone, Debug, PartialEq)]
pub enum Cond {
    NoneLeft(String),
    NoneHooked(String),
}

/// One side of a collision.
#[derive(Clone, Debug, PartialEq)]
pub enum Party {
    Player,
    Thing(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Trigger {
    Start,
    EveryFrame,
    Every(f32),
    /// "the player touches a coin [and no keys are left]"
    Touch { thing: String, none_left: Option<String> },
    /// "a shot hits an alien", "a bomb hits the player", "a hook touches a fish and no fish is hooked"
    Hit { a: String, b: Party, cond: Option<Cond> },
    /// "a hooked perch reaches the hole": the hook it bit is at the hole's height.
    Landed { kind: String, place: String },
    /// "a hooked perch stays at the bottom for 3 seconds": its hook has been at its lowest that long.
    AtBottom { kind: String, secs: f32 },
    LandsOn(String),
    NoneLeft(String),
    Score(i64),
    Time(f32),
    PlayerBelow(f32),
    ThingBelow(String, f32),
    Press(String),
}

impl Trigger {
    /// The thing "it" refers to, if any.
    pub fn it(&self) -> Option<&str> {
        match self {
            Trigger::Touch { thing, .. }
            | Trigger::LandsOn(thing)
            | Trigger::ThingBelow(thing, _)
            | Trigger::Landed { kind: thing, .. }
            | Trigger::AtBottom { kind: thing, .. } => Some(thing),
            Trigger::Hit { a, b, .. } => match b {
                Party::Thing(t) => Some(t),
                Party::Player => Some(a),
            },
            _ => None,
        }
    }
    /// Things this trigger names (for "the shot" / "the alien").
    pub fn named(&self) -> Vec<&str> {
        match self {
            Trigger::Hit { a, b, .. } => {
                let mut v = vec![a.as_str()];
                if let Party::Thing(t) = b {
                    v.push(t);
                }
                v
            }
            other => other.it().into_iter().collect(),
        }
    }
    pub fn is_collision(&self) -> bool {
        matches!(self, Trigger::Touch { .. } | Trigger::Hit { .. } | Trigger::LandsOn(_))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    It,
    Both,
    /// "the shot": the one in the trigger.
    Named(String),
    /// "the bombs": every one.
    All(String),
    Player,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Place {
    TopLeft,
    TopRight,
    Top,
    Middle,
    Bottom,
}

/// A piece of shown text.
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Text(String),
    Score,
    Lives,
    Wave,
    Time,
    Countdown(f32),
    Left(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Destroy(Target),
    AddScore(i64),
    Lives(i64),
    LoseLife { safe: Option<f32> },
    Win(Option<String>),
    Lose(Option<String>),
    Sound(String),
    Music { notes: String, bpm: Option<f32>, wave: Option<String> },
    /// `live`: shown at the start with placeholders, so it is kept up to date.
    /// `secs`: "for N seconds", a message that goes away again.
    Show { parts: Vec<Part>, place: Place, color: Option<[f32; 3]>, live: bool, secs: Option<f32> },
    Bounce(f32),
    BackToStart,
    Squash(Target),
    Anim { anim: String, target: Target },
    Restart,
    NextWave(String),
    RandomShot { kind: String, aim: Aim, speed: f32, sprite: String, sound: Option<String> },
    /// "it bites" / "the fish bites": it is hooked on the trigger's other thing.
    Bite(Target),
    /// "it gets away": a hooked thing is let go and won't bite again for a while.
    GetAway(Target),
    /// "a new perch comes back / comes in from the side after N seconds": its scene entity again.
    ComeBack { kind: String, after: f32, side: bool },
    /// "show the shout meter at the bottom": the microphone's loudness as a bar.
    Meter { place: Place },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionItem {
    pub act: Action,
    /// The words that asked for it.
    pub words: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Standing(Standing),
    Event { trigger: Trigger, actions: Vec<ActionItem> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sentence {
    pub id: usize,
    /// The sentence as written (whitespace collapsed).
    pub text: String,
    /// safely / quickly / simply
    pub mods: Vec<String>,
    pub body: Body,
}

#[derive(Clone, Debug)]
pub struct GameProgram {
    pub tokens: Vec<Token>,
    pub sentences: Vec<Sentence>,
}

// ---------------------------------------------------------------------------

type R<T> = Result<T, Diag>;

struct Cur<'a> {
    t: &'a [Tok],
    i: usize,
    sentence: &'a str,
}

impl<'a> Cur<'a> {
    fn new(t: &'a [Tok], sentence: &'a str) -> Cur<'a> {
        Cur { t, i: 0, sentence }
    }
    fn done(&self) -> bool {
        self.i >= self.t.len()
    }
    fn word_at(&self, k: usize) -> Option<&'a str> {
        match self.t.get(self.i + k) {
            Some(Tok::Word(w)) => Some(w.as_str()),
            _ => None,
        }
    }
    fn is(&self, w: &str) -> bool {
        self.word_at(0) == Some(w)
    }
    fn eat(&mut self, w: &str) -> bool {
        debug_assert!(lexicon::known(w), "parser uses a word the lexicon lacks: {w}");
        if self.is(w) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    /// All the words of `phrase`, or none.
    fn eat_all(&mut self, phrase: &str) -> bool {
        let save = self.i;
        for w in phrase.split(' ') {
            if !self.eat(w) {
                self.i = save;
                return false;
            }
        }
        true
    }
    fn num(&mut self) -> Option<f32> {
        if let Some(Tok::Num(n, _)) = self.t.get(self.i) {
            self.i += 1;
            Some(*n)
        } else {
            None
        }
    }
    fn quote(&mut self) -> Option<String> {
        if let Some(Tok::Quote(q)) = self.t.get(self.i) {
            self.i += 1;
            Some(q.clone())
        } else {
            None
        }
    }
    fn article(&mut self) -> bool {
        self.eat("a") || self.eat("an") || self.eat("the")
    }
    /// A thing word or a quoted custom tag: (tag, plural).
    fn thing(&mut self) -> Option<(String, bool)> {
        match self.t.get(self.i) {
            Some(Tok::Word(w)) => {
                let (tag, plural) = lexicon::thing(w)?;
                self.i += 1;
                Some((tag.to_string(), plural))
            }
            Some(Tok::Quote(q)) if is_tag(q) => {
                let q = q.clone();
                self.i += 1;
                Some((q, false))
            }
            _ => None,
        }
    }
    fn key(&mut self) -> Option<String> {
        let w = self.word_at(0)?;
        if KEYS.contains(&w) || (w.len() == 1 && w.chars().all(|c| c.is_ascii_lowercase())) {
            self.i += 1;
            Some(w.to_string())
        } else {
            None
        }
    }
    /// Connective between options: "," "and" "then".
    fn glue(&mut self) -> bool {
        if matches!(self.t.get(self.i), Some(Tok::Comma)) {
            self.i += 1;
            return true;
        }
        self.eat("and") || self.eat("then")
    }
    fn rest(&self) -> String {
        self.t[self.i.min(self.t.len())..].iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" ")
    }
    fn err(&self, msg: impl Into<String>) -> Diag {
        Diag::new(format!("{} (in \"{}\")", msg.into(), self.sentence))
    }
    fn leftover(&self) -> Diag {
        self.err(format!("don't understand \"{}\"", self.rest()))
            .hint("each option may appear once; check the sentence forms in the game language")
    }
    /// Run `f`; on None, rewind.
    fn attempt<T>(&mut self, f: impl FnOnce(&mut Cur<'a>) -> Option<T>) -> Option<T> {
        let save = self.i;
        let r = f(self);
        if r.is_none() {
            self.i = save;
        }
        r
    }
}

fn is_tag(q: &str) -> bool {
    !q.is_empty() && q.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn once<T>(slot: &mut Option<T>, v: T, what: &str, c: &Cur) -> R<()> {
    if slot.is_some() {
        return Err(c.err(format!("{what} is given twice")));
    }
    *slot = Some(v);
    Ok(())
}

fn flag(slot: &mut bool, what: &str, c: &Cur) -> R<()> {
    if *slot {
        return Err(c.err(format!("\"{what}\" is given twice")));
    }
    *slot = true;
    Ok(())
}

fn need<T>(v: Option<T>, what: &str, c: &Cur) -> R<T> {
    v.ok_or_else(|| c.err(format!("missing {what}")))
}

fn positive(v: f32, what: &str, c: &Cur) -> R<f32> {
    if v > 0.0 {
        Ok(v)
    } else {
        Err(c.err(format!("{what} must be more than 0 (got {v})")))
    }
}

fn integer(v: f32, what: &str, c: &Cur) -> R<i64> {
    if v.fract() == 0.0 && v.abs() < 1e9 {
        Ok(v as i64)
    } else {
        Err(c.err(format!("{what} must be a whole number (got {v})")))
    }
}

// ---- shared options -------------------------------------------------------

/// "at N units per second"
fn opt_speed(c: &mut Cur) -> Option<f32> {
    c.attempt(|c| {
        if !c.eat("at") {
            return None;
        }
        let n = c.num()?;
        if !(c.eat("units") || c.eat("unit")) {
            return None;
        }
        c.eat_all("per second").then_some(n)
    })
}

/// "every N seconds"
fn opt_every(c: &mut Cur) -> Option<f32> {
    c.attempt(|c| {
        if !c.eat("every") {
            return None;
        }
        let n = c.num()?;
        (c.eat("seconds") || c.eat("second")).then_some(n)
    })
}

/// "within N units"
fn opt_within(c: &mut Cur) -> Option<f32> {
    c.attempt(|c| {
        if !c.eat("within") {
            return None;
        }
        let n = c.num()?;
        (c.eat("units") || c.eat("unit")).then_some(n)
    })
}

/// `using "shot"`
fn opt_using(c: &mut Cur) -> Option<String> {
    c.attempt(|c| {
        if !c.eat("using") {
            return None;
        }
        c.quote()
    })
}

/// "with the jump sound"
fn opt_sound(c: &mut Cur) -> Option<String> {
    c.attempt(|c| {
        if !c.eat_all("with the") {
            return None;
        }
        let w = c.word_at(0)?;
        if !SOUNDS.contains(&w) {
            return None;
        }
        c.i += 1;
        c.eat("sound").then(|| w.to_string())
    })
}

/// "with space"
fn opt_key(c: &mut Cur) -> Option<String> {
    c.attempt(|c| {
        if !c.eat("with") {
            return None;
        }
        c.key()
    })
}

fn opt_ground(c: &mut Cur) -> bool {
    c.eat_all("across the ground")
}

fn opt_aim(c: &mut Cur, forward: bool) -> Option<Aim> {
    if c.eat("up") {
        Some(Aim::Up)
    } else if c.eat("down") {
        Some(Aim::Down)
    } else if forward && c.eat("forward") {
        Some(Aim::Forward)
    } else if c.eat_all("at the player") {
        Some(Aim::AtPlayer)
    } else {
        None
    }
}

// ---- standing behaviours --------------------------------------------------

fn standing(c: &mut Cur) -> R<Option<Standing>> {
    if c.eat_all("move the player") {
        return move_player(c).map(Some);
    }
    if c.eat_all("the player jumps") {
        return jump(c).map(Some);
    }
    if c.eat_all("the player shoots") {
        return player_shoot(c).map(Some);
    }
    if c.eat("animate") {
        return animate(c).map(Some);
    }
    if c.is("each") {
        c.eat("each");
        if c.eat("hooked") {
            let Some((kind, _)) = c.thing() else {
                return Err(c.err("\"each hooked\" needs a thing (each hooked perch ...)"));
            };
            return hooked(c, kind).map(Some);
        }
        let Some((kind, _)) = c.thing() else {
            return Err(c.err("\"each\" needs a thing (each slime, each alien ...)"));
        };
        return each(c, kind).map(Some);
    }
    // "The hook hangs ...": a thing named with "the" behaves like "each".
    if let Some(kind) = c.attempt(|c| {
        if !c.eat("the") {
            return None;
        }
        c.thing().map(|(k, _)| k)
    }) {
        return each(c, kind).map(Some);
    }
    Ok(None)
}

fn hooked(c: &mut Cur, kind: String) -> R<Standing> {
    if !(c.eat("follows") && (c.eat_all("its hook") || c.eat_all("the hook"))) {
        return Err(c.err(format!("what does a hooked {kind} do?")).hint("\"follows its hook, pulling it down at N units per second\""));
    }
    let (mut pull, mut fight, mut bursts) = (None, false, false);
    while !c.done() {
        if c.glue() {
            continue;
        }
        if let Some(v) = c.attempt(|c| {
            if !c.eat_all("pulling it down") {
                return None;
            }
            opt_speed(c)
        }) {
            once(&mut pull, positive(v, "the pull", c)?, "\"pulling it down\"", c)?;
        } else if c.eat("fighting") {
            flag(&mut fight, "fighting", c)?;
        } else if c.eat_all("in bursts") {
            flag(&mut bursts, "in bursts", c)?;
        } else {
            return Err(c.leftover());
        }
    }
    if bursts && pull.is_none() {
        return Err(c.err("\"in bursts\" needs a pull").hint("\"pulling it down at 3 units per second in bursts\""));
    }
    Ok(Standing::Hooked { kind, pull, fight, bursts })
}

fn hang(c: &mut Cur, kind: String) -> R<Standing> {
    let anchor = c.attempt(|c| {
        if !c.eat_all("on a line from") {
            return None;
        }
        c.article();
        c.thing().map(|(t, _)| t)
    });
    let Some(anchor) = anchor else {
        return Err(c.err("hangs from what?").hint("\"hangs on a line from the hole\""));
    };
    if anchor == kind {
        return Err(c.err(format!("a {kind} can't hang from itself")));
    }
    let mut sink = None;
    while !c.done() {
        if c.glue() {
            continue;
        }
        if let Some(v) = c.attempt(|c| {
            if !c.eat("sinking") {
                return None;
            }
            let speed = opt_speed(c)?;
            if !c.eat_all("down to") {
                return None;
            }
            Some((speed, c.num()?))
        }) {
            once(&mut sink, (positive(v.0, "the sinking speed", c)?, v.1), "\"sinking\"", c)?;
        } else {
            return Err(c.leftover());
        }
    }
    Ok(Standing::Hang { kind, anchor, sink })
}

fn rise(c: &mut Cur, kind: String) -> R<Standing> {
    let (mut voice, mut key, mut speed, mut loud, mut up_to, mut when) = (false, None, None, false, false, false);
    while !c.done() {
        if c.glue() {
            continue;
        }
        if c.eat_all("while the player") {
            flag(&mut when, "while the player ...", c)?;
            loop {
                if c.eat("shouts") {
                    flag(&mut voice, "shouts", c)?;
                } else if c.eat("holds") {
                    let k = need(c.key(), "a key (\"holds space\")", c)?;
                    once(&mut key, k, "the key", c)?;
                } else {
                    return Err(c.err("while the player does what?").hint("\"shouts\" or \"holds space\""));
                }
                if !c.eat("or") {
                    break;
                }
            }
        } else if let Some((v, up)) = c.attempt(|c| {
            if !c.eat("at") {
                return None;
            }
            let up = c.eat_all("up to");
            let n = c.num()?;
            if !(c.eat("units") || c.eat("unit")) {
                return None;
            }
            c.eat_all("per second").then_some((n, up))
        }) {
            once(&mut speed, positive(v, "the speed", c)?, "a speed", c)?;
            up_to = up;
        } else if c.eat_all("as loud as the shout") {
            flag(&mut loud, "as loud as the shout", c)?;
        } else {
            return Err(c.leftover());
        }
    }
    if !when {
        return Err(c.err("rises when?").hint("\"while the player shouts\" or \"while the player holds space\""));
    }
    if loud && !voice {
        return Err(c.err("\"as loud as the shout\" needs \"while the player shouts\""));
    }
    if up_to && !loud {
        return Err(c.err("\"up to\" needs \"as loud as the shout\"").hint("or say \"at N units per second\""));
    }
    let speed = need(speed, "a speed (\"at up to N units per second\")", c)?;
    Ok(Standing::Rise { kind, speed, voice, key, loud })
}

fn move_player(c: &mut Cur) -> R<Standing> {
    let (mut sideways, mut ground, mut arrows, mut face, mut smooth) = (false, false, false, false, false);
    let (mut speed, mut within) = (None, None);
    while !c.done() {
        if c.glue() {
            continue;
        }
        if c.eat("sideways") {
            flag(&mut sideways, "sideways", c)?;
        } else if opt_ground(c) {
            flag(&mut ground, "across the ground", c)?;
        } else if c.eat_all("with the arrow keys") || c.eat_all("with the arrows") {
            flag(&mut arrows, "with the arrow keys", c)?;
        } else if c.eat_all("face where it moves") {
            flag(&mut face, "face where it moves", c)?;
        } else if c.eat("smoothly") {
            flag(&mut smooth, "smoothly", c)?;
        } else if let Some(v) = opt_speed(c) {
            once(&mut speed, v, "a speed", c)?;
        } else if let Some(v) = opt_within(c) {
            once(&mut within, v, "\"within\"", c)?;
        } else {
            return Err(c.leftover());
        }
    }
    if !arrows {
        return Err(c.err("missing the controls").hint("add \"with the arrow keys\""));
    }
    if sideways && ground {
        return Err(c.err("\"sideways\" and \"across the ground\" contradict each other"));
    }
    let speed = positive(need(speed, "a speed (\"at N units per second\")", c)?, "the speed", c)?;
    let mode = if sideways {
        Mode::Sideways
    } else if ground {
        Mode::Ground
    } else {
        Mode::Plane
    };
    let within = within.map(|w| positive(w, "\"within\"", c)).transpose()?;
    Ok(Standing::Move { mode, speed, face, smooth, within })
}

fn jump(c: &mut Cur) -> R<Standing> {
    let (mut key, mut speed, mut air, mut sound) = (None, None, None, None);
    let mut held = false;
    while !c.done() {
        if c.glue() {
            continue;
        }
        if let Some(k) = opt_key(c) {
            once(&mut key, k, "the key", c)?;
        } else if let Some(v) = opt_speed(c) {
            once(&mut speed, v, "a speed", c)?;
        } else if c.eat("twice") {
            let v = need(opt_speed(c), "the second jump's speed (\"twice at N units per second\")", c)?;
            once(&mut air, positive(v, "the second jump's speed", c)?, "\"twice\"", c)?;
        } else if c.eat_all("higher while held") {
            flag(&mut held, "higher while held", c)?;
        } else if let Some(s) = opt_sound(c) {
            once(&mut sound, s, "a sound", c)?;
        } else {
            return Err(c.leftover());
        }
    }
    let key = need(key, "the key (\"with space\")", c)?;
    let speed = positive(need(speed, "a speed (\"at N units per second\")", c)?, "the speed", c)?;
    Ok(Standing::Jump { key, speed, air, held, sound })
}

fn player_shoot(c: &mut Cur) -> R<Standing> {
    let (mut aim, mut key, mut speed, mut every, mut sprite, mut sound) = (None, None, None, None, None, None);
    let mut ground = false;
    while !c.done() {
        if c.glue() {
            continue;
        }
        if let Some(a) = opt_aim(c, true) {
            if a == Aim::AtPlayer {
                return Err(c.err("the player can't shoot at itself").hint("use up, down or forward"));
            }
            once(&mut aim, a, "a direction", c)?;
        } else if opt_ground(c) {
            flag(&mut ground, "across the ground", c)?;
        } else if let Some(k) = opt_key(c) {
            once(&mut key, k, "the key", c)?;
        } else if let Some(v) = opt_speed(c) {
            once(&mut speed, v, "a speed", c)?;
        } else if let Some(v) = opt_every(c) {
            once(&mut every, positive(v, "the time between shots", c)?, "\"every\"", c)?;
        } else if let Some(s) = opt_using(c) {
            once(&mut sprite, s, "\"using\"", c)?;
        } else if let Some(s) = opt_sound(c) {
            once(&mut sound, s, "a sound", c)?;
        } else {
            return Err(c.leftover());
        }
    }
    let aim = need(aim, "a direction (up, down or forward)", c)?;
    if ground && aim != Aim::Forward {
        return Err(c.err("\"across the ground\" only goes with \"forward\""));
    }
    let key = need(key, "the key (\"with space\")", c)?;
    let speed = positive(need(speed, "a speed (\"at N units per second\")", c)?, "the speed", c)?;
    Ok(Standing::Shoot { aim, ground, key, speed, every, sprite: sprite.unwrap_or_else(|| "shot".into()), sound })
}

fn frames(c: &mut Cur) -> Vec<String> {
    let mut v = Vec::new();
    while let Some(q) = c.quote() {
        v.push(q);
        let save = c.i;
        if !(c.glue() && matches!(c.t.get(c.i), Some(Tok::Quote(_)))) {
            c.i = save;
            break;
        }
    }
    v
}

fn animate(c: &mut Cur) -> R<Standing> {
    let who = if c.eat_all("the player") {
        Who::Player
    } else if c.eat("each") {
        let Some((kind, _)) = c.thing() else {
            return Err(c.err("\"animate each\" needs a thing"));
        };
        Who::Each(kind)
    } else {
        return Err(c.err("animate what?").hint("\"animate the player ...\" or \"animate each slime ...\""));
    };
    if !c.eat("with") {
        return Err(c.err("missing the frames").hint("with \"frame_1\" and \"frame_2\""));
    }
    let frames = frames(c);
    if frames.len() < 2 {
        return Err(c.err("an animation needs at least two quoted frames"));
    }
    let (mut fps, mut walking, mut hooked) = (None, false, false);
    while !c.done() {
        if c.glue() {
            continue;
        }
        if let Some(v) = c.attempt(|c| {
            if !c.eat("at") {
                return None;
            }
            let n = c.num()?;
            c.eat_all("frames per second").then_some(n)
        }) {
            once(&mut fps, v, "a frame rate", c)?;
        } else if c.eat_all("while it walks") {
            flag(&mut walking, "while it walks", c)?;
        } else if c.eat_all("while it is hooked") {
            flag(&mut hooked, "while it is hooked", c)?;
        } else {
            return Err(c.leftover());
        }
    }
    let fps = positive(need(fps, "a frame rate (\"at N frames per second\")", c)?, "the frame rate", c)?;
    if walking && who != Who::Player {
        return Err(c.err("\"while it walks\" is for the player"));
    }
    if hooked && who == Who::Player {
        return Err(c.err("\"while it is hooked\" is for things on a hook (animate each perch ...)"));
    }
    if hooked && walking {
        return Err(c.err("\"while it walks\" and \"while it is hooked\" are two conditions; give one"));
    }
    Ok(Standing::Animate { who, frames, fps, walking, hooked })
}

/// "Each perch swims ...": a patrol, or with any way of swimming a [`Swim`].
fn swims(c: &mut Cur, kind: String) -> R<Standing> {
    let save = c.i;
    c.eat("sideways");
    let side = c.attempt(|c| {
        let n = c.num()?;
        if !(c.eat("units") || c.eat("unit")) {
            return None;
        }
        c.eat_all("to each side").then_some(n)
    });
    let (mut speed, mut within) = (None, None);
    let (mut school, mut gait, mut smooth) = (false, None, false);
    let (mut wave, mut band, mut nose, mut lunge) = (None, None, None, None);
    while !c.done() {
        if c.glue() {
            continue;
        }
        if let Some(v) = opt_speed(c) {
            once(&mut speed, v, "a speed", c)?;
        } else if let Some(v) = opt_within(c) {
            once(&mut within, v, "\"within\"", c)?;
        } else if c.eat_all("in a school") || c.eat_all("in a shoal") {
            flag(&mut school, "in a school", c)?;
        } else if c.eat("darting") {
            once(&mut gait, Gait::Dart, "how it swims (darting, drifting or lurking)", c)?;
        } else if c.eat("drifting") {
            once(&mut gait, Gait::Drift, "how it swims (darting, drifting or lurking)", c)?;
        } else if c.eat("lurking") {
            once(&mut gait, Gait::Lurk, "how it swims (darting, drifting or lurking)", c)?;
        } else if c.eat_all("turning smoothly") {
            flag(&mut smooth, "turning smoothly", c)?;
        } else if let Some(v) = c.attempt(|c| {
            if !c.eat_all("in a wave") {
                return None;
            }
            let n = c.num()?;
            if !(c.eat("units") || c.eat("unit")) {
                return None;
            }
            c.eat("high").then_some(n)
        }) {
            once(&mut wave, positive(v, "the wave height", c)?, "\"in a wave\"", c)?;
        } else if let Some(v) = c.attempt(|c| {
            if !c.eat_all("keeping its depth") {
                return None;
            }
            opt_within(c)
        }) {
            once(&mut band, positive(v, "the depth band", c)?, "\"keeping its depth\"", c)?;
        } else if let Some((t, d)) = c.attempt(|c| {
            if !(c.eat_all("nosing toward") || c.eat_all("nosing towards")) {
                return None;
            }
            c.article();
            let (t, _) = c.thing()?;
            Some((t, opt_within(c)?))
        }) {
            once(&mut nose, (t, positive(d, "the nosing distance", c)?), "\"nosing toward\"", c)?;
        } else if let Some((t, d, v)) = c.attempt(|c| {
            if !c.eat_all("lunging at") {
                return None;
            }
            c.article();
            let (t, _) = c.thing()?;
            let d = opt_within(c)?;
            Some((t, d, opt_speed(c)?))
        }) {
            let d = positive(d, "the lunge distance", c)?;
            once(&mut lunge, (t, d, positive(v, "the lunge speed", c)?), "\"lunging at\"", c)?;
        } else {
            return Err(c.leftover());
        }
    }
    let how = school || gait.is_some() || smooth || wave.is_some() || band.is_some() || nose.is_some() || lunge.is_some();
    if !how {
        c.i = save;
        return patrol(c, kind);
    }
    if side.is_some() {
        return Err(c.err("a fish that swims its own way turns at the edges: \"within N units\", not \"to each side\""));
    }
    for (t, what) in [(nose.as_ref().map(|n| &n.0), "nosing toward"), (lunge.as_ref().map(|l| &l.0), "lunging at")] {
        if t == Some(&kind) {
            return Err(c.err(format!("a {kind} can't go \"{what}\" itself")));
        }
    }
    let within = positive(need(within, "the bounds (\"within N units\")", c)?, "\"within\"", c)?;
    let speed = positive(need(speed, "a speed (\"at N units per second\")", c)?, "the speed", c)?;
    Ok(Standing::Swim(Swim {
        kind,
        within,
        speed,
        school,
        gait: gait.unwrap_or(Gait::Steady),
        smooth,
        wave,
        band,
        nose,
        lunge,
    }))
}

/// "patrols / swims N units to each side" or "within N units".
fn patrol(c: &mut Cur, kind: String) -> R<Standing> {
    c.eat("sideways");
    let side = c.attempt(|c| {
        let n = c.num()?;
        if !(c.eat("units") || c.eat("unit")) {
            return None;
        }
        c.eat_all("to each side").then_some(n)
    });
    let (mut speed, mut within) = (None, None);
    while !c.done() {
        if c.glue() {
            continue;
        }
        if let Some(v) = opt_speed(c) {
            once(&mut speed, v, "a speed", c)?;
        } else if let Some(v) = opt_within(c) {
            once(&mut within, v, "\"within\"", c)?;
        } else {
            return Err(c.leftover());
        }
    }
    let (range, edges) = match (side, within) {
        (Some(r), None) => (r, false),
        (None, Some(w)) => (w, true),
        (Some(_), Some(_)) => return Err(c.err("\"to each side\" and \"within\" are two ranges; give one")),
        (None, None) => {
            return Err(c.err("how far?").hint("\"patrols 3 units to each side\" or \"swims within 12 units\""))
        }
    };
    let range = positive(range, "the patrol range", c)?;
    let speed = positive(need(speed, "a speed (\"at N units per second\")", c)?, "the speed", c)?;
    Ok(Standing::Patrol { kind, range, speed, edges })
}

fn each(c: &mut Cur, kind: String) -> R<Standing> {
    if c.eat("moves") {
        if !(c.eat_all("toward the player") || c.eat_all("towards the player")) {
            return Err(c.err("moves where?").hint("\"moves toward the player\""));
        }
        let (mut speed, mut within, mut speedup) = (None, None, None);
        let (mut sideways, mut ground) = (false, false);
        while !c.done() {
            if c.glue() {
                continue;
            }
            if let Some(v) = opt_speed(c) {
                once(&mut speed, v, "a speed", c)?;
            } else if c.eat("sideways") {
                flag(&mut sideways, "sideways", c)?;
            } else if opt_ground(c) {
                flag(&mut ground, "across the ground", c)?;
            } else if let Some(v) = opt_within(c) {
                once(&mut within, positive(v, "\"within\"", c)?, "\"within\"", c)?;
            } else if let Some(v) = c.attempt(|c| {
                if !c.eat_all("speeding up by") {
                    return None;
                }
                let n = c.num()?;
                c.eat_all("percent per second").then_some(n)
            }) {
                once(&mut speedup, v, "\"speeding up\"", c)?;
            } else {
                return Err(c.leftover());
            }
        }
        if sideways && ground {
            return Err(c.err("\"sideways\" and \"across the ground\" contradict each other"));
        }
        let mode = if sideways {
            Mode::Sideways
        } else if ground {
            Mode::Ground
        } else {
            Mode::Plane
        };
        let speed = positive(need(speed, "a speed (\"at N units per second\")", c)?, "the speed", c)?;
        return Ok(Standing::Chase { kind, speed, mode, within, speedup });
    }
    if c.eat("swims") {
        return swims(c, kind);
    }
    if c.eat("patrols") {
        return patrol(c, kind);
    }
    if c.eat("hangs") {
        return hang(c, kind);
    }
    if c.eat("rises") {
        return rise(c, kind);
    }
    if c.eat("flies") {
        let height = c.attempt(|c| {
            if !c.eat_all("in a wave") {
                return None;
            }
            let n = c.num()?;
            if !(c.eat("units") || c.eat("unit")) {
                return None;
            }
            c.eat("high").then_some(n)
        });
        let Some(height) = height else {
            return Err(c.err("flies how?").hint("\"flies in a wave 1.2 units high\""));
        };
        let height = positive(height, "the wave height", c)?;
        let mut freq = None;
        while !c.done() {
            if c.glue() {
                continue;
            }
            if let Some(v) = c.attempt(|c| {
                if !c.eat("at") {
                    return None;
                }
                let n = c.num()?;
                c.eat_all("cycles per second").then_some(n)
            }) {
                once(&mut freq, v, "a frequency", c)?;
            } else {
                return Err(c.leftover());
            }
        }
        let freq = positive(need(freq, "a frequency (\"at N cycles per second\")", c)?, "the frequency", c)?;
        return Ok(Standing::Fly { kind, height, freq });
    }
    if c.eat("wanders") || c.eat("wander") {
        let (mut speed, mut turn, mut ground) = (None, None, false);
        while !c.done() {
            if c.glue() {
                continue;
            }
            if let Some(v) = opt_speed(c) {
                once(&mut speed, v, "a speed", c)?;
            } else if let Some(v) = c.attempt(|c| {
                if !c.eat("turning") {
                    return None;
                }
                opt_every(c)
            }) {
                once(&mut turn, positive(v, "the turn time", c)?, "\"turning every\"", c)?;
            } else if opt_ground(c) {
                flag(&mut ground, "across the ground", c)?;
            } else {
                return Err(c.leftover());
            }
        }
        let speed = positive(need(speed, "a speed (\"at N units per second\")", c)?, "the speed", c)?;
        let turn = need(turn, "how often it turns (\"turning every N seconds\")", c)?;
        return Ok(Standing::Wander { kind, speed, turn, ground });
    }
    if c.eat("marches") {
        if !c.eat("sideways") {
            return Err(c.err("marches how?").hint("\"marches sideways\""));
        }
        let (mut speed, mut within, mut step, mut max_mult, mut wave_pct) = (None, None, None, None, None);
        while !c.done() {
            if c.glue() {
                continue;
            }
            if let Some(v) = opt_speed(c) {
                once(&mut speed, v, "a speed", c)?;
            } else if let Some(v) = opt_within(c) {
                once(&mut within, positive(v, "\"within\"", c)?, "\"within\"", c)?;
            } else if let Some(v) = c.attempt(|c| {
                if !c.eat_all("stepping down") {
                    return None;
                }
                let n = c.num()?;
                (c.eat("units") || c.eat("unit")).then_some(n)
            }) {
                once(&mut step, v, "\"stepping down\"", c)?;
            } else if let Some(v) = c.attempt(|c| {
                if !c.eat_all("up to") {
                    return None;
                }
                let n = c.num()?;
                c.eat_all("times as fewer are left").then_some(n)
            }) {
                if v < 1.0 {
                    return Err(c.err("\"up to N times\" needs N of at least 1"));
                }
                once(&mut max_mult, v, "\"up to N times\"", c)?;
            } else if let Some(v) = c.attempt(|c| {
                let n = c.num()?;
                c.eat_all("percent faster each wave").then_some(n)
            }) {
                once(&mut wave_pct, v, "\"percent faster each wave\"", c)?;
            } else {
                return Err(c.leftover());
            }
        }
        let speed = positive(need(speed, "a speed (\"at N units per second\")", c)?, "the speed", c)?;
        let within = need(within, "the bounds (\"within N units\")", c)?;
        let step = need(step, "the step (\"stepping down N units\")", c)?;
        return Ok(Standing::March { kind, speed, within, step, max_mult, wave_pct });
    }
    if c.eat("shoots") {
        let Some(aim) = opt_aim(c, false) else {
            return Err(c.err("shoots where?").hint("\"shoots at the player\", \"shoots down\" or \"shoots up\""));
        };
        let (mut speed, mut every, mut within, mut sprite, mut sound) = (None, None, None, None, None);
        while !c.done() {
            if c.glue() {
                continue;
            }
            if let Some(v) = opt_speed(c) {
                once(&mut speed, v, "a speed", c)?;
            } else if let Some(v) = opt_every(c) {
                once(&mut every, positive(v, "the time between shots", c)?, "\"every\"", c)?;
            } else if let Some(v) = opt_within(c) {
                once(&mut within, positive(v, "\"within\"", c)?, "\"within\"", c)?;
            } else if let Some(s) = opt_using(c) {
                once(&mut sprite, s, "\"using\"", c)?;
            } else if let Some(s) = opt_sound(c) {
                once(&mut sound, s, "a sound", c)?;
            } else {
                return Err(c.leftover());
            }
        }
        let speed = positive(need(speed, "a speed (\"at N units per second\")", c)?, "the speed", c)?;
        let every = need(every, "how often (\"every N seconds\")", c)?;
        return Ok(Standing::EnemyShoot {
            kind,
            aim,
            speed,
            every,
            within,
            sprite: sprite.unwrap_or_else(|| "bomb".into()),
            sound,
        });
    }
    Err(c.err(format!("don't know what each {kind} does: \"{}\"", c.rest()))
        .hint("moves toward the player, patrols, swims, flies in a wave, wanders, marches sideways, shoots, hangs or rises"))
}

// ---- triggers --------------------------------------------------------------

fn trigger(c: &mut Cur) -> R<Option<Trigger>> {
    let t = if c.eat_all("at the start") {
        Trigger::Start
    } else if c.eat_all("every frame") {
        Trigger::EveryFrame
    } else if c.is("every") {
        match opt_every(c) {
            Some(v) => Trigger::Every(positive(v, "the period", c)?),
            None => return Err(c.err("every how often?").hint("\"Every 2 seconds, ...\" or \"Every frame, ...\"")),
        }
    } else if c.eat("when") {
        when(c)?
    } else {
        return Ok(None);
    };
    if !matches!(c.t.get(c.i), Some(Tok::Comma)) {
        return Err(c.err(format!("expected a comma after the trigger, found \"{}\"", c.rest()))
            .hint("write the trigger, a comma, then what happens"));
    }
    c.i += 1;
    Ok(Some(t))
}

fn when(c: &mut Cur) -> R<Trigger> {
    if c.eat_all("the player") {
        if c.eat("touches") || c.eat("hits") {
            c.article();
            let Some((thing, _)) = c.thing() else {
                return Err(c.err("touches what?"));
            };
            let none_left = c.attempt(|c| {
                if !c.eat_all("and no") {
                    return None;
                }
                let (t, _) = c.thing()?;
                if !(c.eat_all("are left") || c.eat_all("is left")) {
                    return None;
                }
                Some(t)
            });
            return Ok(Trigger::Touch { thing, none_left });
        }
        if c.eat_all("lands on") {
            c.article();
            let Some((thing, _)) = c.thing() else {
                return Err(c.err("lands on what?"));
            };
            return Ok(Trigger::LandsOn(thing));
        }
        if c.eat_all("falls below") {
            let n = need(c.num(), "a height (\"falls below -10\")", c)?;
            return Ok(Trigger::PlayerBelow(n));
        }
        if c.eat("presses") {
            let k = need(c.key(), "a key (space, enter, shift, a letter or an arrow)", c)?;
            return Ok(Trigger::Press(k));
        }
        return Err(c.err(format!("don't understand \"when the player {}\"", c.rest()))
            .hint("touches, lands on, falls below or presses"));
    }
    if c.eat("no") {
        let Some((thing, _)) = c.thing() else {
            return Err(c.err("no what?"));
        };
        if !(c.eat_all("are left") || c.eat_all("is left")) {
            return Err(c.err("expected \"are left\""));
        }
        return Ok(Trigger::NoneLeft(thing));
    }
    if c.eat_all("the score reaches") {
        let n = need(c.num(), "a number", c)?;
        return Ok(Trigger::Score(integer(n, "the score", c)?));
    }
    if c.eat_all("the time reaches") {
        let n = need(c.num(), "a number of seconds", c)?;
        if !(c.eat("seconds") || c.eat("second")) {
            return Err(c.err("expected \"seconds\""));
        }
        return Ok(Trigger::Time(positive(n, "the time", c)?));
    }
    c.article();
    let hooked = c.eat("hooked");
    let Some((a, _)) = c.thing() else {
        return Err(c.err(format!("don't understand \"when {}\"", c.rest())));
    };
    if hooked {
        if c.eat_all("stays at the bottom for") {
            let n = need(c.num(), "a number of seconds", c)?;
            if !(c.eat("seconds") || c.eat("second")) {
                return Err(c.err("expected \"seconds\""));
            }
            return Ok(Trigger::AtBottom { kind: a, secs: positive(n, "the time", c)? });
        }
        if !c.eat("reaches") {
            return Err(c.err(format!("what does the hooked {a} do?"))
                .hint("\"When a hooked perch reaches the hole, ...\" or \"... stays at the bottom for 3 seconds, ...\""));
        }
        c.article();
        let Some((place, _)) = c.thing() else {
            return Err(c.err("reaches what?").hint("\"reaches the hole\""));
        };
        return Ok(Trigger::Landed { kind: a, place });
    }
    if c.eat("hits") || c.eat("touches") {
        let b = if c.eat_all("the player") {
            Party::Player
        } else {
            c.article();
            match c.thing() {
                Some((t, _)) => Party::Thing(t),
                None => return Err(c.err(format!("{a} hits what?"))),
            }
        };
        let cond = c.attempt(|c| {
            if !c.eat_all("and no") {
                return None;
            }
            let (t, _) = c.thing()?;
            if c.eat_all("are left") || c.eat_all("is left") {
                Some(Cond::NoneLeft(t))
            } else if c.eat_all("is hooked") || c.eat_all("are hooked") {
                Some(Cond::NoneHooked(t))
            } else {
                None
            }
        });
        return Ok(Trigger::Hit { a, b, cond });
    }
    if c.eat_all("falls below") {
        let n = need(c.num(), "a height", c)?;
        return Ok(Trigger::ThingBelow(a, n));
    }
    Err(c.err(format!("don't understand what the {a} does: \"{}\"", c.rest())).hint("hits, touches or falls below"))
}

// ---- actions ---------------------------------------------------------------

fn starts_action(t: &[Tok]) -> bool {
    let w = |k: usize| match t.get(k) {
        Some(Tok::Word(w)) => w.as_str(),
        _ => "",
    };
    let head = if w(0) == "the" && w(1) == "player" { w(2) } else { w(0) };
    matches!(
        head,
        "destroy" | "add" | "has" | "loses" | "wins" | "goes" | "play" | "show" | "bounce" | "squash" | "restart" | "start"
    ) || (w(0) == "a" && (w(1) == "random" || w(1) == "new"))
        || (w(0) == "it" && (w(1) == "bites" || w(1) == "gets"))
        || (w(0) == "the" && w(2) == "bites")
}

/// Split the action list into pieces; verbless pieces join the one before.
fn pieces(t: &[Tok]) -> Vec<Vec<Tok>> {
    let mut raw: Vec<Vec<Tok>> = vec![Vec::new()];
    for tok in t {
        let split = matches!(tok, Tok::Comma) || matches!(tok, Tok::Word(w) if w == "and" || w == "then");
        if split {
            raw.push(Vec::new());
        } else {
            raw.last_mut().expect("nonempty").push(tok.clone());
        }
    }
    let mut out: Vec<Vec<Tok>> = Vec::new();
    for p in raw.into_iter().filter(|p| !p.is_empty()) {
        if starts_action(&p) || out.is_empty() {
            out.push(p);
        } else {
            let last = out.last_mut().expect("nonempty");
            last.push(Tok::Comma);
            last.extend(p);
        }
    }
    out
}

fn words_of(t: &[Tok]) -> String {
    let mut s = String::new();
    for tok in t {
        if !s.is_empty() && *tok != Tok::Comma {
            s.push(' ');
        }
        s.push_str(&tok.to_string());
    }
    s
}

fn target(c: &mut Cur, trig: &Trigger, verb: &str) -> R<Target> {
    if c.eat("it") {
        if trig.it().is_none() {
            return Err(c.err(format!("\"{verb} it\": \"it\" has nothing to refer to here")));
        }
        return Ok(Target::It);
    }
    if c.eat("both") {
        match trig {
            Trigger::Hit { b: Party::Thing(_), .. } => return Ok(Target::Both),
            _ => return Err(c.err("\"both\" needs a trigger with two things (\"when a shot hits an alien\")")),
        }
    }
    if c.eat_all("the player") {
        return Ok(Target::Player);
    }
    let the = c.eat("the");
    let Some((tag, plural)) = c.thing() else {
        return Err(c.err(format!("{verb} what? \"{}\"", c.rest())));
    };
    if plural {
        return Ok(Target::All(tag));
    }
    if !the || !trig.named().contains(&tag.as_str()) {
        return Err(c.err(format!("which {tag}? the trigger doesn't name one"))
            .hint(format!("say \"it\", or \"the {tag}s\" for all of them")));
    }
    Ok(Target::Named(tag))
}

fn color(q: &str) -> Option<[f32; 3]> {
    let h = q.strip_prefix('#')?;
    let digits: Vec<u32> = h.chars().map(|ch| ch.to_digit(16)).collect::<Option<_>>()?;
    let v: Vec<u32> = match digits.len() {
        3 => digits.iter().map(|d| d * 17).collect(),
        6 => digits.chunks(2).map(|p| p[0] * 16 + p[1]).collect(),
        _ => return None,
    };
    let f = |x: u32| (x as f32 / 255.0 * 1000.0).round() / 1000.0;
    Some([f(v[0]), f(v[1]), f(v[2])])
}

fn template(q: &str, c: &Cur) -> R<Vec<Part>> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut rest = q;
    while let Some(open) = rest.find('{') {
        text.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('}') else {
            return Err(c.err(format!("an unclosed {{ in \"{q}\"")));
        };
        let inner = rest[open + 1..open + close].trim();
        if !text.is_empty() {
            parts.push(Part::Text(std::mem::take(&mut text)));
        }
        let words: Vec<&str> = inner.split_whitespace().collect();
        let part = match words.as_slice() {
            ["score"] => Part::Score,
            ["lives"] => Part::Lives,
            ["wave"] => Part::Wave,
            ["time"] => Part::Time,
            ["countdown", n] => match n.parse::<f32>() {
                Ok(v) if v > 0.0 => Part::Countdown(v),
                _ => return Err(c.err(format!("{{countdown N}} needs a positive number, got \"{n}\""))),
            },
            ["left", t] => match lexicon::thing(t) {
                Some((tag, _)) => Part::Left(tag.to_string()),
                None => return Err(c.err(format!("{{left X}}: \"{t}\" is not a thing"))),
            },
            _ => {
                return Err(c
                    .err(format!("unknown placeholder {{{inner}}}"))
                    .hint("{score} {lives} {wave} {time} {countdown 60} {left coin}"))
            }
        };
        parts.push(part);
        rest = &rest[open + close + 1..];
    }
    if rest.contains('}') {
        return Err(c.err(format!("a stray }} in \"{q}\"")));
    }
    text.push_str(rest);
    if !text.is_empty() {
        parts.push(Part::Text(text));
    }
    Ok(parts)
}

fn action(c: &mut Cur, trig: &Trigger) -> R<Action> {
    let player = c.eat_all("the player");
    if !player && c.eat("destroy") {
        let t = target(c, trig, "destroy")?;
        if t == Target::Player {
            return Err(c.err("the player can't be destroyed").hint("\"the player loses\" ends the game"));
        }
        return end(c, Action::Destroy(t));
    }
    if !player && c.eat("add") {
        let n = need(c.num(), "a number", c)?;
        let n = integer(n, "the points", c)?;
        if !(c.eat_all("to the score") || c.eat("points") || c.eat("point")) {
            return Err(c.err("add to what?").hint("\"add 1 to the score\" or \"add 10 points\""));
        }
        return end(c, Action::AddScore(n));
    }
    if c.eat("has") {
        let n = need(c.num(), "a number of lives", c)?;
        if !(c.eat("lives") || c.eat("life")) {
            return Err(c.err("has what? (\"has 3 lives\")"));
        }
        if *trig != Trigger::Start {
            return Err(c.err("\"the player has N lives\" belongs in \"At the start, ...\""));
        }
        let n = integer(n, "the lives", c)?;
        if n < 1 {
            return Err(c.err("the player needs at least one life"));
        }
        return end(c, Action::Lives(n));
    }
    if c.eat("loses") {
        if c.eat_all("a life") {
            let mut safe = None;
            while !c.done() {
                if c.glue() {
                    continue;
                }
                if let Some(v) = c.attempt(|c| {
                    if !c.eat_all("safe for") {
                        return None;
                    }
                    let n = c.num()?;
                    (c.eat("seconds") || c.eat("second")).then_some(n)
                }) {
                    once(&mut safe, positive(v, "the safe time", c)?, "\"safe for\"", c)?;
                } else {
                    return Err(c.leftover());
                }
            }
            return Ok(Action::LoseLife { safe });
        }
        let msg = c.quote();
        return end(c, Action::Lose(msg));
    }
    if c.eat("wins") {
        let msg = c.quote();
        return end(c, Action::Win(msg));
    }
    if c.eat_all("goes back to the start") {
        return end(c, Action::BackToStart);
    }
    if !player && (c.word_at(1) == Some("bites") || c.word_at(2) == Some("bites")) {
        let t = target(c, trig, "bite")?;
        if !c.eat("bites") {
            return Err(c.leftover());
        }
        if !matches!(trig, Trigger::Hit { b: Party::Thing(_), .. }) || !matches!(t, Target::It | Target::Named(_)) {
            return Err(c.err("a bite needs two things that touch").hint("\"When a hook touches a fish, the fish bites.\""));
        }
        return end(c, Action::Bite(t));
    }
    if !player && c.eat_all("it gets away") {
        if !matches!(trig, Trigger::AtBottom { .. } | Trigger::Landed { .. }) {
            return Err(c.err("only a hooked thing gets away").hint("\"When a hooked perch stays at the bottom for 3 seconds, it gets away.\""));
        }
        return end(c, Action::GetAway(Target::It));
    }
    if !player && c.eat_all("a new") {
        let Some((kind, _)) = c.thing() else {
            return Err(c.err("a new what?"));
        };
        if trig.it() != Some(kind.as_str()) {
            return Err(c.err(format!("a new {kind}: the trigger is not about a {kind}")).hint("\"When a hooked perch reaches the hole, ... a new perch comes back after 3 seconds\""));
        }
        if !c.eat("comes") {
            return Err(c.err(format!("what does the new {kind} do?")).hint("\"comes back after N seconds\" or \"comes in from the side after N seconds\""));
        }
        let side = if c.eat_all("in from the side") {
            true
        } else if c.eat("back") {
            false
        } else {
            return Err(c.err("comes where?").hint("\"comes back\" or \"comes in from the side\""));
        };
        let mut after = None;
        while !c.done() {
            if c.glue() {
                continue;
            }
            if let Some(v) = c.attempt(|c| {
                if !c.eat("after") {
                    return None;
                }
                let n = c.num()?;
                (c.eat("seconds") || c.eat("second")).then_some(n)
            }) {
                once(&mut after, v, "\"after\"", c)?;
            } else {
                return Err(c.leftover());
            }
        }
        let after = need(after, "when (\"after N seconds\")", c)?;
        if after < 0.0 {
            return Err(c.err("\"after\" can't be negative"));
        }
        return Ok(Action::ComeBack { kind, after, side });
    }
    if player {
        return Err(c.err(format!("don't understand \"the player {}\"", c.rest()))
            .hint("has N lives, loses a life, wins, loses, goes back to the start"));
    }
    if c.eat("play") {
        if c.eat_all("the music") {
            let Some(notes) = c.quote() else {
                return Err(c.err("the music needs quoted notes (\"C4 E4 G4\")"));
            };
            let (mut bpm, mut wave) = (None, None);
            while !c.done() {
                if c.glue() {
                    continue;
                }
                if let Some(v) = c.attempt(|c| {
                    if !c.eat("at") {
                        return None;
                    }
                    let n = c.num()?;
                    c.eat_all("beats per minute").then_some(n)
                }) {
                    once(&mut bpm, positive(v, "the tempo", c)?, "a tempo", c)?;
                } else if let Some(w) = c.attempt(|c| {
                    if !c.eat_all("as a") {
                        return None;
                    }
                    let w = c.word_at(0)?;
                    if !WAVES.contains(&w) {
                        return None;
                    }
                    c.i += 1;
                    c.eat("wave").then(|| w.to_string())
                }) {
                    once(&mut wave, w, "a wave", c)?;
                } else {
                    return Err(c.leftover());
                }
            }
            return Ok(Action::Music { notes, bpm, wave });
        }
        if !c.eat("the") {
            return Err(c.err("play what?").hint("\"play the coin sound\" or \"play the music ...\""));
        }
        let Some(w) = c.word_at(0) else {
            return Err(c.err("play what?"));
        };
        if c.word_at(1) == Some("sound") {
            if !SOUNDS.contains(&w) {
                return Err(c.err(format!("there is no \"{w}\" sound")).hint(format!("sounds: {}", SOUNDS.join(" "))));
            }
            c.i += 2;
            return end(c, Action::Sound(w.to_string()));
        }
        if c.word_at(1) == Some("animation") {
            if !ANIMS.contains(&w) {
                return Err(c.err(format!("there is no \"{w}\" animation")).hint(format!("animations: {}", ANIMS.join(" "))));
            }
            c.i += 2;
            if !c.eat("on") {
                return Err(c.err("play the animation on what? (\"on the player\")"));
            }
            let t = target(c, trig, "animate")?;
            if matches!(t, Target::Both | Target::All(_)) {
                return Err(c.err("an animation plays on one thing"));
            }
            return end(c, Action::Anim { anim: w.to_string(), target: t });
        }
        return Err(c.err(format!("play what? \"{}\"", c.rest())).hint("\"play the coin sound\", \"play the jump animation on the player\""));
    }
    if c.eat_all("show the shout meter") {
        if *trig != Trigger::Start {
            return Err(c.err("the shout meter is shown \"At the start, ...\" and stays"));
        }
        let mut place = None;
        while !c.done() {
            if c.glue() {
                continue;
            }
            let p = if c.eat_all("at the top left") {
                Place::TopLeft
            } else if c.eat_all("at the top right") {
                Place::TopRight
            } else if c.eat_all("at the top") {
                Place::Top
            } else if c.eat_all("in the middle") {
                Place::Middle
            } else if c.eat_all("at the bottom") {
                Place::Bottom
            } else {
                return Err(c.leftover());
            };
            once(&mut place, p, "a place", c)?;
        }
        return Ok(Action::Meter { place: place.unwrap_or(Place::Bottom) });
    }
    if c.eat("show") {
        let Some(q) = c.quote() else {
            return Err(c.err("show what? (a quoted text)"));
        };
        let parts = template(&q, c)?;
        let (mut place, mut col, mut secs) = (None, None, None);
        while !c.done() {
            if c.glue() {
                continue;
            }
            if let Some(v) = c.attempt(|c| {
                if !c.eat("for") {
                    return None;
                }
                let n = c.num()?;
                (c.eat("seconds") || c.eat("second")).then_some(n)
            }) {
                once(&mut secs, positive(v, "the time shown", c)?, "\"for N seconds\"", c)?;
                continue;
            }
            let p = if c.eat_all("at the top left") {
                Some(Place::TopLeft)
            } else if c.eat_all("at the top right") {
                Some(Place::TopRight)
            } else if c.eat_all("at the top") {
                Some(Place::Top)
            } else if c.eat_all("in the middle") {
                Some(Place::Middle)
            } else if c.eat_all("at the bottom") {
                Some(Place::Bottom)
            } else {
                None
            };
            if let Some(p) = p {
                once(&mut place, p, "a place", c)?;
            } else if let Some(q) = c.attempt(|c| {
                if !c.eat("in") {
                    return None;
                }
                c.quote()
            }) {
                let Some(rgb) = color(&q) else {
                    return Err(c.err(format!("\"{q}\" is not a colour")).hint("\"#rrggbb\" or \"#rgb\""));
                };
                once(&mut col, rgb, "a colour", c)?;
            } else {
                return Err(c.leftover());
            }
        }
        let live = secs.is_none() && *trig == Trigger::Start && parts.iter().any(|p| !matches!(p, Part::Text(_)));
        return Ok(Action::Show { parts, place: place.unwrap_or(Place::TopLeft), color: col, live, secs });
    }
    if c.eat_all("bounce the player up") {
        let v = need(opt_speed(c), "a speed (\"at N units per second\")", c)?;
        return end(c, Action::Bounce(positive(v, "the bounce", c)?));
    }
    if c.eat("squash") {
        let t = target(c, trig, "squash")?;
        if t == Target::Both {
            return Err(c.err("squash one thing at a time"));
        }
        return end(c, Action::Squash(t));
    }
    if c.eat_all("restart the game") {
        return end(c, Action::Restart);
    }
    if c.eat_all("start the next wave of") {
        let Some((tag, _)) = c.thing() else {
            return Err(c.err("the next wave of what?"));
        };
        return end(c, Action::NextWave(tag));
    }
    if c.eat_all("a random") {
        let Some((kind, _)) = c.thing() else {
            return Err(c.err("a random what?"));
        };
        if !c.eat("shoots") {
            return Err(c.err(format!("what does the random {kind} do? (\"shoots down ...\")")));
        }
        let Some(aim) = opt_aim(c, false) else {
            return Err(c.err("shoots where? (down, up or at the player)"));
        };
        let (mut speed, mut sprite, mut sound) = (None, None, None);
        while !c.done() {
            if c.glue() {
                continue;
            }
            if let Some(v) = opt_speed(c) {
                once(&mut speed, v, "a speed", c)?;
            } else if let Some(s) = opt_using(c) {
                once(&mut sprite, s, "\"using\"", c)?;
            } else if let Some(s) = opt_sound(c) {
                once(&mut sound, s, "a sound", c)?;
            } else {
                return Err(c.leftover());
            }
        }
        let speed = positive(need(speed, "a speed (\"at N units per second\")", c)?, "the speed", c)?;
        return Ok(Action::RandomShot { kind, aim, speed, sprite: sprite.unwrap_or_else(|| "bomb".into()), sound });
    }
    Err(c.err(format!("don't understand \"{}\"", c.rest()))
        .hint("actions: destroy, add, the player has/loses/wins/goes back, play, show, bounce, squash, restart, start the next wave, a random X shoots, it bites, a new X comes back"))
}

fn end(c: &Cur, a: Action) -> R<Action> {
    if c.done() {
        Ok(a)
    } else {
        Err(c.leftover())
    }
}

// ---- sentences ---------------------------------------------------------------

fn sentence(id: usize, toks: &[Tok], text: &str) -> R<Sentence> {
    // Modifiers may stand anywhere ("Quickly, each ...", "... safely").
    let mut mods = Vec::new();
    let mut t: Vec<Tok> = Vec::new();
    let mut skip_comma = false;
    for tok in toks {
        if let Tok::Word(w) = tok {
            if MODIFIERS.contains(&w.as_str()) {
                if !mods.contains(w) {
                    mods.push(w.clone());
                }
                skip_comma = t.is_empty();
                continue;
            }
        }
        if skip_comma && *tok == Tok::Comma {
            skip_comma = false;
            continue;
        }
        skip_comma = false;
        t.push(tok.clone());
    }
    while t.last() == Some(&Tok::Comma) {
        t.pop();
    }
    mods.sort();
    let mut c = Cur::new(&t, text);
    if c.done() {
        return Err(Diag::new(format!("an empty sentence: \"{text}\"")));
    }
    if let Some(trig) = trigger(&mut c)? {
        let rest = &t[c.i..];
        let ps = pieces(rest);
        if ps.is_empty() {
            return Err(c.err("the trigger is followed by nothing to do"));
        }
        let mut actions = Vec::new();
        for p in ps {
            let mut pc = Cur::new(&p, text);
            let act = action(&mut pc, &trig)?;
            actions.push(ActionItem { act, words: words_of(&p) });
        }
        if let Trigger::Press(_) = trig {
            // fine anywhere
        } else if trig == Trigger::Start {
            for a in &actions {
                if matches!(a.act, Action::Destroy(Target::It | Target::Both | Target::Named(_)) | Action::Restart) {
                    return Err(c.err(format!("\"{}\" makes no sense at the start", a.words)));
                }
            }
        }
        return Ok(Sentence { id, text: text.to_string(), mods, body: Body::Event { trigger: trig, actions } });
    }
    match standing(&mut c)? {
        Some(s) => Ok(Sentence { id, text: text.to_string(), mods, body: Body::Standing(s) }),
        None => Err(c.err(format!("don't understand \"{}\"", c.rest())).hint(
            "a sentence is a behaviour (Move the player ..., The player jumps ..., Each slime ..., Animate ...) \
             or a trigger and actions (At the start, / Every N seconds, / When ..., )",
        )),
    }
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn parse(src: &str, lenient: bool) -> Result<GameProgram, Vec<Diag>> {
    let tokens = lexicon::tokenize(src, lenient)?;
    let mut sentences = Vec::new();
    let mut diags = Vec::new();
    let mut cur: Vec<&Token> = Vec::new();
    let flush = |cur: &mut Vec<&Token>, end: usize, sentences: &mut Vec<Sentence>, diags: &mut Vec<Diag>| {
        if cur.is_empty() {
            return;
        }
        let text = collapse(&src[cur[0].start..end]);
        let toks: Vec<Tok> = cur.iter().map(|t| t.tok.clone()).collect();
        match sentence(sentences.len() + diags.len(), &toks, &text) {
            Ok(s) => sentences.push(s),
            Err(d) => diags.push(d),
        }
        cur.clear();
    };
    for t in &tokens {
        if t.tok == Tok::Period {
            flush(&mut cur, t.end, &mut sentences, &mut diags);
        } else {
            cur.push(t);
        }
    }
    if let Some(last) = cur.last() {
        let end = last.end;
        let text = collapse(&src[cur[0].start..end]);
        diags.push(Diag::new(format!("a sentence doesn't end with a period: \"{text}\"")).hint("end every sentence with ."));
    }
    if sentences.is_empty() && diags.is_empty() {
        diags.push(Diag::new("no sentences").hint("describe the game's behaviour in sentences ending with ."));
    }
    // "stays at the bottom" needs to know where the bottom is.
    let deepest = sentences.iter().any(|s| matches!(s.body, Body::Standing(Standing::Hang { sink: Some(_), .. })));
    for s in &sentences {
        if let Body::Event { trigger: Trigger::AtBottom { .. }, .. } = s.body {
            if !deepest {
                diags.push(
                    Diag::new(format!("\"{}\": the bottom is where a hook sinks to, and no hook sinks", s.text))
                        .hint("add \"The hook hangs on a line from the hole, sinking at 1.5 units per second down to -9.\""),
                );
            }
        }
    }
    // Ids follow the order sentences appear in, errors included.
    for (i, s) in sentences.iter_mut().enumerate() {
        s.id = i;
    }
    if diags.is_empty() {
        Ok(GameProgram { tokens, sentences })
    } else {
        Err(diags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(s: &str) -> Sentence {
        parse(s, false).unwrap_or_else(|d| panic!("{d:?}")).sentences.remove(0)
    }

    fn fails(s: &str) -> String {
        let d = parse(s, false).expect_err("should fail");
        d[0].to_string()
    }

    #[test]
    fn standing_options_in_any_order() {
        let a = one("Move the player sideways with the arrow keys at 7.5 units per second and face where it moves.");
        let b = one("Move the player sideways, face where it moves, at 7.5 units per second with the arrow keys.");
        assert_eq!(a.body, b.body);
    }

    #[test]
    fn verbless_piece_joins_previous_action() {
        let s = one("When the player touches a slime, the player loses a life, safe for 1.5 seconds.");
        let Body::Event { actions, .. } = s.body else { panic!() };
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].act, Action::LoseLife { safe: Some(1.5) });
    }

    #[test]
    fn trigger_keeps_its_and() {
        let s = one("When the player touches the door and no coins are left, the player wins.");
        let Body::Event { trigger, .. } = s.body else { panic!() };
        assert_eq!(trigger, Trigger::Touch { thing: "door".into(), none_left: Some("coin".into()) });
    }

    #[test]
    fn modifiers_are_collected() {
        let s = one("Quickly, each zombie moves toward the player at 2 units per second.");
        assert_eq!(s.mods, vec!["quickly".to_string()]);
    }

    #[test]
    fn leftover_words_are_errors() {
        assert!(fails("Each slime patrols 3 units to each side at 2 units per second at 3 units per second.").contains("twice"));
        assert!(fails("Move the player with the arrow keys at 7 units per second high.").contains("high"));
    }

    #[test]
    fn it_needs_a_referent() {
        assert!(fails("Every 2 seconds, destroy it.").contains("nothing to refer to"));
    }

    #[test]
    fn missing_period_is_an_error() {
        assert!(fails("At the start, play the coin sound").contains("period"));
    }

    #[test]
    fn placeholders_are_checked() {
        assert!(fails("At the start, show \"{scor}\".").contains("placeholder"));
        let s = one("At the start, show \"TIME {countdown 60}\" at the top right in \"#ff0\".");
        let Body::Event { actions, .. } = s.body else { panic!() };
        assert_eq!(
            actions[0].act,
            Action::Show {
                parts: vec![Part::Text("TIME ".into()), Part::Countdown(60.0)],
                place: Place::TopRight,
                color: Some([1.0, 1.0, 0.0]),
                live: true,
                secs: None
            }
        );
    }

    #[test]
    fn fishing_sentences_parse() {
        let s = one("The hook hangs on a line from the hole, sinking at 1.5 units per second down to -9.");
        assert_eq!(s.body, Body::Standing(Standing::Hang { kind: "hook".into(), anchor: "hole".into(), sink: Some((1.5, -9.0)) }));
        let s = one("The hook rises while the player shouts or holds space, at up to 8 units per second as loud as the shout.");
        assert_eq!(
            s.body,
            Body::Standing(Standing::Rise { kind: "hook".into(), speed: 8.0, voice: true, key: Some("space".into()), loud: true })
        );
        let s = one("Each hooked pike follows its hook, pulling it down at 3 units per second.");
        assert_eq!(s.body, Body::Standing(Standing::Hooked { kind: "pike".into(), pull: Some(3.0), fight: false, bursts: false }));
        let s = one("Each hooked pike follows its hook, fighting and pulling it down at 5 units per second in bursts.");
        assert_eq!(s.body, Body::Standing(Standing::Hooked { kind: "pike".into(), pull: Some(5.0), fight: true, bursts: true }));
        let s = parse(
            "The hook hangs on a line from the hole, sinking at 1.5 units per second down to -9.
             When a hooked pike stays at the bottom for 3 seconds, it gets away and show \"IT GOT AWAY\" in the middle for 1.5 seconds.",
            false,
        )
        .unwrap()
        .sentences
        .remove(1);
        let Body::Event { trigger, actions } = s.body else { panic!() };
        assert_eq!(trigger, Trigger::AtBottom { kind: "pike".into(), secs: 3.0 });
        assert_eq!(actions[0].act, Action::GetAway(Target::It));
        assert!(matches!(actions[1].act, Action::Show { secs: Some(1.5), live: false, .. }));
        let s = one("Each perch swims sideways within 12 units at 2 units per second.");
        assert_eq!(s.body, Body::Standing(Standing::Patrol { kind: "perch".into(), range: 12.0, speed: 2.0, edges: true }));
        let s = one("When a hook touches a fish and no fish is hooked, the fish bites.");
        let Body::Event { trigger, actions } = s.body else { panic!() };
        assert_eq!(trigger, Trigger::Hit { a: "hook".into(), b: Party::Thing("fish".into()), cond: Some(Cond::NoneHooked("fish".into())) });
        assert_eq!(actions[0].act, Action::Bite(Target::Named("fish".into())));
        let s = one("When a hooked roach reaches the hole, destroy it and a new roach comes in from the side after 2 seconds.");
        let Body::Event { trigger, actions } = s.body else { panic!() };
        assert_eq!(trigger, Trigger::Landed { kind: "roach".into(), place: "hole".into() });
        assert_eq!(actions[1].act, Action::ComeBack { kind: "roach".into(), after: 2.0, side: true });
        let s = one("At the start, show the shout meter at the bottom.");
        let Body::Event { actions, .. } = s.body else { panic!() };
        assert_eq!(actions[0].act, Action::Meter { place: Place::Bottom });
    }

    #[test]
    fn swim_sentences_parse_in_any_order() {
        let s = one("Each pike swims within 16 units, lunging at the hook within 4 units at 9 units per second, lurking, at 0.6 units per second and keeping its depth within 0.5 units.");
        assert_eq!(
            s.body,
            Body::Standing(Standing::Swim(Swim {
                kind: "pike".into(),
                within: 16.0,
                speed: 0.6,
                school: false,
                gait: Gait::Lurk,
                smooth: false,
                wave: None,
                band: Some(0.5),
                nose: None,
                lunge: Some(("hook".into(), 4.0, 9.0)),
            }))
        );
        let s = one("Each roach swims in a shoal within 16 units at 1.5 units per second, drifting, turning smoothly and in a wave 0.6 units high.");
        let Body::Standing(Standing::Swim(w)) = s.body else { panic!() };
        assert!(w.school && w.smooth && w.gait == Gait::Drift && w.wave == Some(0.6) && w.nose.is_none());
        let s = one("Each perch swims in a school within 16 units at 2 units per second, darting and nosing toward the hook within 4 units.");
        let Body::Standing(Standing::Swim(w)) = s.body else { panic!() };
        assert_eq!((w.gait, w.nose), (Gait::Dart, Some(("hook".into(), 4.0))));
        // "while it is hooked"
        let s = one("Animate each perch with \"a\" and \"b\" at 12 frames per second while it is hooked.");
        assert!(matches!(s.body, Body::Standing(Standing::Animate { hooked: true, walking: false, .. })));
    }

    #[test]
    fn fishing_sentences_are_checked() {
        assert!(fails("The hook rises at 8 units per second.").contains("rises when"));
        assert!(fails("The hook rises while the player holds space, at 8 units per second as loud as the shout.").contains("shouts"));
        assert!(fails("The hook rises while the player shouts, at up to 8 units per second.").contains("up to"));
        assert!(fails("When a shot hits an alien, a new perch comes back after 2 seconds.").contains("not about a perch"));
        assert!(fails("Every 2 seconds, show the shout meter.").contains("At the start"));
        assert!(fails("When the player touches a coin, it bites.").contains("two things"));
        assert!(fails("When the player touches a coin, it gets away.").contains("hooked"));
        assert!(fails("When a hooked perch stays at the bottom for 3 seconds, it gets away.").contains("no hook sinks"));
        assert!(fails("Each hooked perch follows its hook, in bursts.").contains("needs a pull"));
    }

    #[test]
    fn lives_only_at_the_start() {
        assert!(fails("When the player touches a heart, the player has 3 lives.").contains("At the start"));
    }
}
