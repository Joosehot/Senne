//! Winning choices -> one Rust file shaped like Droid's `games/coin_runner`:
//! a struct with typed state, `impl Game` with `init` / `update` /
//! `on_collision` / `status`, and `main` handing the type to `droid::run`.
//!
//! Patterns write their lines into sections of [`Gen`]; shared machinery
//! (the game clock, the hoisted player position, blinking while safe,
//! projectile clean-up, the start position) is asked for by name and written
//! once, in a fixed place, by [`render`].

use super::parser::{Body, GameProgram, Place as TextPlace};
use super::rules::{self, block, f, ind, site, At, Emitted, Facts, Node, Section, Stats};
use super::search::Outcome;
use crate::config::{Config, GameTuning};
use std::collections::{BTreeMap, BTreeSet};

/// Shared machinery a pattern can ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Feature {
    /// `self.clock += dt` while playing.
    Clock,
    /// `let player = cx.pos("player");` once per frame.
    Hoist,
    /// Blink the player while `self.safe > 0`.
    Blink,
    /// `self.start = cx.pos("player")` in init.
    Start,
    /// Scene things queued to come back ("a new perch comes back after 3 seconds").
    Comebacks,
}

#[derive(Clone, Debug, PartialEq)]
struct Field {
    name: String,
    ty: String,
    default: String,
}

pub struct Gen<'a> {
    pub t: &'a GameTuning,
    pub facts: &'a Facts,
    pub stats: Stats,
    /// The comment for the unit being emitted (None with `comments: false`).
    pub note: Option<String>,
    fields: Vec<Field>,
    pub init: Vec<String>,
    pub keys: Vec<String>,
    pub hud: Vec<String>,
    pub update: Vec<String>,
    pub land: Vec<String>,
    pub collide: Vec<String>,
    pub late: Vec<String>,
    helpers: BTreeMap<String, Vec<String>>,
    features: BTreeSet<Feature>,
    cleanup: BTreeSet<String>,
    slots: BTreeMap<TextPlace, u32>,
    texts: u32,
    flashes: BTreeSet<TextPlace>,
}

impl<'a> Gen<'a> {
    pub fn new(t: &'a GameTuning, facts: &'a Facts, stats: Stats) -> Gen<'a> {
        Gen {
            t,
            facts,
            stats,
            note: None,
            fields: vec![Field { name: "status".into(), ty: "Status".into(), default: "Status::Playing".into() }],
            init: Vec::new(),
            keys: Vec::new(),
            hud: Vec::new(),
            update: Vec::new(),
            land: Vec::new(),
            collide: Vec::new(),
            late: Vec::new(),
            helpers: BTreeMap::new(),
            features: BTreeSet::new(),
            cleanup: BTreeSet::new(),
            slots: BTreeMap::new(),
            texts: 0,
            flashes: BTreeSet::new(),
        }
    }

    /// A struct field (idempotent); returns `self.<name>`.
    pub fn field(&mut self, name: &str, ty: &str, default: &str) -> String {
        match self.fields.iter().find(|x| x.name == name) {
            Some(existing) => {
                debug_assert_eq!(existing.ty, ty, "field {name} declared with two types");
            }
            None => self.fields.push(Field { name: name.into(), ty: ty.into(), default: default.into() }),
        }
        format!("self.{name}")
    }

    /// A new field that nobody else has claimed: `base`, `base_2`, ...
    pub fn fresh(&mut self, base: &str, ty: &str, default: &str) -> String {
        let mut name = base.to_string();
        let mut k = 2;
        while self.fields.iter().any(|x| x.name == name) {
            name = format!("{base}_{k}");
            k += 1;
        }
        self.field(&name, ty, default)
    }

    pub fn need(&mut self, feat: Feature) {
        match feat {
            Feature::Clock => {
                self.field("clock", "f32", "0.0");
            }
            Feature::Blink | Feature::Start => {}
            Feature::Hoist => {}
            Feature::Comebacks => {
                self.field("clock", "f32", "0.0");
                self.field("comebacks", "Vec<(f32, String, bool)>", "Vec::new()");
            }
        }
        if feat == Feature::Start {
            self.field("start", "Vec3", "Vec3::ZERO");
        }
        self.features.insert(feat);
    }

    /// The game clock (seconds of play), as an expression.
    pub fn clock(&mut self) -> String {
        self.need(Feature::Clock);
        "self.clock".into()
    }

    /// Remove projectiles with this tag once they are far from the player.
    pub fn cleanup(&mut self, tag: &str) {
        self.cleanup.insert(tag.to_string());
    }

    /// A method on the game struct, written once.
    pub fn helper(&mut self, name: &str, lines: Vec<String>) {
        self.helpers.entry(name.to_string()).or_insert(lines);
    }

    /// A new text id.
    pub fn text_id(&mut self) -> String {
        let id = format!("text_{}", self.texts);
        self.texts += 1;
        id
    }

    /// Screen position for the next line of text at `place` (lines stack down).
    pub fn slot(&mut self, place: TextPlace) -> (f32, f32) {
        let n = self.slots.entry(place).or_insert(0);
        let k = *n;
        *n += 1;
        let [x, y] = match place {
            TextPlace::TopLeft => self.t.top_left,
            TextPlace::TopRight => self.t.top_right,
            TextPlace::Top => self.t.top,
            TextPlace::Middle => self.t.middle,
            TextPlace::Bottom => self.t.bottom,
        };
        let dy = if place == TextPlace::Bottom { -self.t.line_gap } else { self.t.line_gap };
        let y = ((y + dy * k as f32) * 1000.0).round() / 1000.0;
        (x, y)
    }

    /// A message shown for a while at `place` ("show ... for N seconds"): its text id and the
    /// field holding when it goes. Messages at one place share them (a new one replaces the old).
    pub fn flash(&mut self, place: TextPlace) -> (String, String) {
        let key = flash_key(place);
        self.clock();
        let until = self.field(&format!("flash_{key}"), "f32", "0.0");
        self.flashes.insert(place);
        (format!("flash_{key}"), until)
    }

    /// Screen position of `place` (its first line).
    pub fn place_xy(&self, place: TextPlace) -> (f32, f32) {
        let [x, y] = match place {
            TextPlace::TopLeft => self.t.top_left,
            TextPlace::TopRight => self.t.top_right,
            TextPlace::Top => self.t.top,
            TextPlace::Middle => self.t.middle,
            TextPlace::Bottom => self.t.bottom,
        };
        (x, y)
    }

    /// The end-of-game banner: "YOU WIN!" / "GAME OVER" or the sentence's own,
    /// plus " - PRESS R" when a key restarts the game.
    pub fn banner(&self, won: bool, custom: Option<&str>) -> String {
        let base = custom.map(str::to_string).unwrap_or_else(|| if won { "YOU WIN!" } else { "GAME OVER" }.to_string());
        match &self.facts.restart_key {
            Some(k) => format!("{base} - PRESS {}", k.to_uppercase()),
            None => base,
        }
    }

    /// Lines for the comment of the unit being emitted.
    pub fn comment(&self) -> Vec<String> {
        self.note.iter().cloned().collect()
    }

    fn place(&mut self, e: Emitted, comment: Vec<String>) {
        if e.lines.is_empty() {
            return;
        }
        let mut lines = comment;
        lines.extend(e.lines);
        match e.section {
            Section::Init => self.init.extend(lines),
            Section::Keys => self.keys.extend(lines),
            Section::Update | Section::Inline => self.update.extend(lines),
            Section::Land => self.land.extend(lines),
            Section::Collide => self.collide.extend(lines),
            Section::Late => self.late.extend(lines),
        }
    }
}

fn flash_key(p: TextPlace) -> &'static str {
    match p {
        TextPlace::TopLeft => "top_left",
        TextPlace::TopRight => "top_right",
        TextPlace::Top => "top",
        TextPlace::Middle => "middle",
        TextPlace::Bottom => "bottom",
    }
}

pub struct GenOptions {
    pub struct_name: String,
    pub comments: bool,
}

/// Does `word` appear as a whole identifier in `lines`?
fn uses_ident(lines: &[String], word: &str) -> bool {
    lines.iter().any(|l| {
        let b = l.as_bytes();
        let mut i = 0;
        while let Some(p) = l[i..].find(word) {
            let s = i + p;
            let e = s + word.len();
            let before = s == 0 || !(b[s - 1].is_ascii_alphanumeric() || b[s - 1] == b'_' || b[s - 1] == b'.');
            let after = e >= b.len() || !(b[e].is_ascii_alphanumeric() || b[e] == b'_');
            if before && after {
                return true;
            }
            i = e;
        }
        false
    })
}

pub fn generate(prog: &GameProgram, out: &Outcome, cfg: &Config, opts: &GenOptions) -> String {
    let facts = Facts::of(prog);
    let t = &cfg.game.tuning;
    let mut g = Gen::new(t, &facts, out.stats);
    let mut unit = 0;
    let note = |text: &str, label: &str| -> Option<String> {
        opts.comments.then(|| format!("// \"{text}\" [{label}]"))
    };
    for s in &prog.sentences {
        match &s.body {
            Body::Standing(st) => {
                let ch = &out.choices[unit];
                unit += 1;
                g.note = note(&s.text, &ch.label());
                let node = Node::Standing(st);
                let e = rules::pattern(ch.pattern).emit(ch.variant, &node, &mut g, &At::plain(rules::Place::Update), vec![]);
                let c = g.comment();
                g.place(e, c);
            }
            Body::Event { trigger, actions } => {
                let tch = out.choices[unit].clone();
                unit += 1;
                let mut at = site(trigger);
                let mut body = Vec::new();
                for (i, a) in actions.iter().enumerate() {
                    let ch = &out.choices[unit];
                    unit += 1;
                    at.last = i + 1 == actions.len();
                    g.note = note(&s.text, &ch.label());
                    let e = rules::pattern(ch.pattern).emit(ch.variant, &Node::Action { act: &a.act, trigger }, &mut g, &at, vec![]);
                    if opts.comments && !e.lines.is_empty() {
                        body.push(format!("// {} [{}]", a.words, ch.label()));
                    }
                    body.extend(e.lines);
                }
                g.note = note(&s.text, &tch.label());
                let e = rules::pattern(tch.pattern).emit(tch.variant, &Node::Trigger(trigger), &mut g, &at, body);
                let c = g.comment();
                g.place(e, c);
            }
        }
    }
    render(&g, &opts.struct_name)
}

fn render(g: &Gen, name: &str) -> String {
    let t = g.t;
    let mut o: Vec<String> = Vec::new();
    o.push(format!("//! {name}: written by Senne from the game's sentences (behavior.txt)."));
    o.push("//! Each block is commented with the sentence that asked for it and the [pattern/variant]".into());
    o.push("//! that wrote it. Change the sentences, not this file.".into());
    o.push(String::new());
    o.push("use droid::prelude::*;".into());
    if g.fields.iter().any(|x| x.ty.contains("BTreeMap")) {
        o.push("use std::collections::BTreeMap;".into());
    }
    o.push(String::new());

    // struct + Default
    o.push(format!("struct {name} {{"));
    for x in &g.fields {
        o.push(format!("    {}: {},", x.name, x.ty));
    }
    o.push("}".into());
    o.push(String::new());
    o.push(format!("impl Default for {name} {{"));
    o.push("    fn default() -> Self {".into());
    o.push(format!("        {name} {{"));
    for x in &g.fields {
        o.push(format!("            {}: {},", x.name, x.default));
    }
    o.push("        }".into());
    o.push("    }".into());
    o.push("}".into());
    o.push(String::new());

    if !g.helpers.is_empty() {
        o.push(format!("impl {name} {{"));
        let mut first = true;
        for lines in g.helpers.values() {
            if !first {
                o.push(String::new());
            }
            first = false;
            o.extend(ind(lines.clone(), 1));
        }
        o.push("}".into());
        o.push(String::new());
    }

    o.push(format!("impl Game for {name} {{"));
    let mut fns: Vec<Vec<String>> = Vec::new();

    // init
    let mut init = Vec::new();
    if g.features.contains(&Feature::Start) {
        init.push("self.start = cx.pos(\"player\");".to_string());
    }
    init.extend(g.init.iter().cloned());
    if !init.is_empty() {
        let cx = if uses_ident(&init, "cx") { "cx" } else { "_cx" };
        fns.push(block(format!("fn init(&mut self, {cx}: &mut Ctx)"), init));
    }

    // update
    let mut playing = Vec::new();
    if g.features.contains(&Feature::Clock) {
        playing.push("self.clock += dt;".to_string());
    }
    if g.features.contains(&Feature::Hoist) {
        playing.push("let player = cx.pos(\"player\");".to_string());
    }
    if g.features.contains(&Feature::Blink) {
        playing.push("// blink while safe after losing a life".into());
        playing.extend(block(
            "if self.safe > 0.0",
            vec![
                "self.safe -= dt;".into(),
                format!("let lit = self.safe <= 0.0 || (self.safe * {}) as i64 % 2 == 0;", f(t.blink_rate)),
                format!("cx.set_color(\"player\", if lit {{ Vec3::ONE }} else {{ Vec3::splat({}) }});", f(t.blink_dim)),
            ],
        ));
    }
    for place in &g.flashes {
        let key = flash_key(*place);
        playing.push(format!("// the message at the {} goes when its time is up", key.replace('_', " ")));
        playing.extend(block(
            format!("if self.flash_{key} > 0.0 && self.clock >= self.flash_{key}"),
            vec![format!("cx.remove_text(\"flash_{key}\");"), format!("self.flash_{key} = 0.0;")],
        ));
    }
    if g.features.contains(&Feature::Comebacks) {
        playing.push("// scene things that come back once their time is up".into());
        playing.push(
            "let due: Vec<(String, bool)> = self.comebacks.iter().filter(|c| c.0 <= self.clock).map(|c| (c.1.clone(), c.2)).collect();"
                .into(),
        );
        playing.push("self.comebacks.retain(|c| c.0 > self.clock);".into());
        playing.extend(block(
            "for (name, side) in due",
            block(
                "if let Some(n) = cx.respawn(&name)",
                block(
                    "if side",
                    vec![
                        format!(
                            "let x = if cx.random(0.0, 1.0) < 0.5 {{ {} }} else {{ {} }};",
                            f(-t.side_x),
                            f(t.side_x)
                        ),
                        "let y = cx.pos(&n).y;".into(),
                        "cx.set_pos(&n, vec2(x, y));".into(),
                    ],
                ),
            ),
        ));
    }
    playing.extend(g.update.iter().cloned());
    playing.extend(g.late.iter().cloned());
    for tag in &g.cleanup {
        playing.push(format!("// projectiles far from the player are gone ({tag})"));
        playing.extend(block(
            format!("for n in cx.find_tag({})", rules::s(tag)),
            block(
                format!("if (cx.pos(&n) - cx.pos(\"player\")).length() > {}", f(t.projectile_range)),
                vec!["cx.destroy(&n);".into()],
            ),
        ));
    }
    let mut update: Vec<String> = Vec::new();
    update.extend(g.keys.iter().cloned());
    update.extend(g.hud.iter().cloned());
    if !playing.is_empty() {
        update.extend(block("if self.status != Status::Playing", vec!["return;".into()]));
        update.extend(playing);
    }
    if !update.is_empty() {
        let cx = if uses_ident(&update, "cx") { "cx" } else { "_cx" };
        let dt = if uses_ident(&update, "dt") { "dt" } else { "_dt" };
        fns.push(block(format!("fn update(&mut self, {cx}: &mut Ctx, {dt}: f32)"), update));
    }

    // on_collision
    if !g.land.is_empty() || !g.collide.is_empty() {
        let mut body = block("if self.status != Status::Playing", vec!["return;".into()]);
        body.extend(g.land.iter().cloned());
        body.extend(g.collide.iter().cloned());
        let cx = if uses_ident(&body, "cx") { "cx" } else { "_cx" };
        let a = if uses_ident(&body, "a") { "a" } else { "_a" };
        let b = if uses_ident(&body, "b") { "b" } else { "_b" };
        fns.push(block(format!("fn on_collision(&mut self, {cx}: &mut Ctx, {a}: &str, {b}: &str)"), body));
    }

    fns.push(block("fn status(&self) -> Status", vec!["self.status".into()]));
    for (i, fun) in fns.into_iter().enumerate() {
        if i > 0 {
            o.push(String::new());
        }
        o.extend(ind(fun, 1));
    }
    o.push("}".into());
    o.push(String::new());
    o.push("fn main() {".into());
    o.push(format!("    droid::run::<{name}>(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/game\"));"));
    o.push("}".into());
    let mut s = o.join("\n");
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ident_detection_respects_word_boundaries() {
        let l = vec!["let dtx = self.dt + cx.time();".to_string()];
        assert!(!uses_ident(&l, "dt"));
        assert!(uses_ident(&l, "cx"));
        assert!(uses_ident(&["a == \"player\"".to_string()], "a"));
    }
}
