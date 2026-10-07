//! "Each perch swims in a school within 16 units at 2 units per second, darting,
//! keeping its depth within 1.5 units and nosing toward the hook within 4 units."
//! "Each roach swims in a shoal within 16 units at 1.5 units per second, drifting,
//! turning smoothly, in a wave 0.6 units high and keeping its depth within 1 unit."
//! "Each pike swims within 16 units at 0.6 units per second, lurking, keeping its depth
//! within 0.5 units and lunging at the hook within 4 units at 9 units per second."
//!
//! A fish that swims its own way. Each one keeps a small state (the depth it started
//! at, its heading, its speed, the last burst or turn it decided, its lunge) and its
//! own clock, a little faster or slower than the game's and offset by a seed made
//! of its name, so no two move alike and every run is the same. Every frame it:
//!
//! 1. picks a speed from its gait: steady; darting (burst, glide, pause, maybe a turn
//!    at the burst); drifting (a slow swell); lurking (almost still, now and then a turn);
//! 2. in a school, turns with the first of its school (by name) and closes in on the
//!    others while keeping loose spacing;
//! 3. turns back at x = +-within (one that comes in from the side swims in);
//! 4. noses toward the thing when it is near (within its depth band), or lunges at it: a dash of `lunge_secs`
//!    straight at it (so the lunge is what touches the hook), then a glide back;
//! 5. eases its velocity to that (slowly when "turning smoothly": it slows before it
//!    turns), rises and sinks toward its depth target (its own drift within the band,
//!    plus the wave), and faces where it moves.
//!
//! Hooked ones don't swim. Numbers are the sentence's or `[game.tuning]`'s.
//! One variant, `steer`: the state lives in a map per kind and the velocity is set.

use super::{block, f, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::{Gait, Standing};

pub struct Swim;

impl GamePattern for Swim {
    fn name(&self) -> &'static str {
        "swim"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["steer"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Swim(_)))
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Swim(w)) = n else { unreachable!() };
        let t = g.t;
        let clock = g.clock();
        let hooked = if g.facts.hooks { Some(super::hooked_map(g)) } else { None };
        let map = g.fresh(&format!("swim_{}", w.kind), "BTreeMap<String, (f32, f32, f32, f32, f32, f32)>", "BTreeMap::new()");
        let speed = w.speed;
        let rate = if w.smooth { t.swim_smooth_turn } else { t.swim_turn_rate };
        let uses_t = w.gait != Gait::Steady || w.band.is_some() || w.wave.is_some();

        let mut l = Vec::new();
        // what it noses toward or lunges at: the first one there is
        let target = w.nose.as_ref().map(|x| &x.0).or(w.lunge.as_ref().map(|x| &x.0));
        if let Some(th) = target {
            l.push(format!("let near = cx.find_tag({}).first().map(|h| cx.pos(h));", s(th)));
        }
        let mut body = Vec::new();
        if w.school {
            let filter = match &hooked {
                Some(m) => format!(".filter(|n| !{m}.contains_key(n))"),
                None => String::new(),
            };
            l.push(format!(
                "let school: Vec<(String, Vec3)> = cx.find_tag({}).into_iter(){filter}.map(|n| {{ let p = cx.pos(&n); (n, p) }}).collect();",
                s(&w.kind)
            ));
        } else {
            if let Some(m) = &hooked {
                body.extend(block(format!("if {m}.contains_key(&n)"), vec!["continue;".into()]));
            }
            body.push("let p = cx.pos(&n);".into());
        }
        body.push(format!(
            "let (home, dir, vx, beat, lunge, wait) = *{map}.entry(n.clone()).or_insert((p.y, if p.x > 0.0 {{ -1.0 }} else {{ 1.0 }}, 0.0, -1.0, 0.0, 0.0));"
        ));
        if uses_t {
            body.push("// its own clock, from a seed made of its name".into());
            body.push("let seed = (n.bytes().fold(17u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b))) % 1000) as f32 / 1000.0;".into());
            body.push(format!(
                "let t = {clock} * ({} + {} * seed) + seed * 7.0;",
                f(1.0 - t.swim_clock_jitter / 2.0),
                f(t.swim_clock_jitter)
            ));
        }
        // 1. the gait
        let turn_at = |cycle: f32, chance: f32| {
            vec![
                format!("let cycle = t / {};", f(cycle)),
                format!(
                    "let turn = cycle.floor() != beat && ((cycle.floor() + seed * 31.0).sin() * 437.585).fract().abs() < {};",
                    f(chance)
                ),
                "let beat = cycle.floor();".to_string(),
                "let dir = if turn { -dir } else { dir };".to_string(),
            ]
        };
        match w.gait {
            Gait::Steady => {
                body.push(format!("let go = {};", f(speed)));
                body.push(format!("let rate = {};", f(rate)));
            }
            Gait::Dart => {
                body.push("// darting: a quick burst, then a glide that slows into a short pause".into());
                body.extend(turn_at(t.dart_cycle, t.dart_turn_chance));
                body.push(format!("let burst = cycle.fract() * {} < {};", f(t.dart_cycle), f(t.dart_secs)));
                body.push(format!("let go = if burst {{ {} }} else {{ 0.0 }};", f(speed * t.dart_boost)));
                body.push(format!("let rate = if burst {{ {} }} else {{ {} }};", f(t.dart_rate), f(t.glide_rate)));
            }
            Gait::Drift => {
                body.push("// drifting: calm, the speed swells and eases".into());
                body.push(format!("let go = {} * (1.0 + {} * (t * {}).sin());", f(speed), f(t.drift_swing), f(t.drift_rate)));
                body.push(format!("let rate = {};", f(rate)));
            }
            Gait::Lurk => {
                body.push("// lurking: hovers almost still, the fins drifting it slowly; now and then it turns".into());
                body.extend(turn_at(t.lurk_turn_secs, t.lurk_turn_chance));
                body.push(format!(
                    "let go = {} * ({} + {} * (0.5 + 0.5 * (t * {}).sin()));",
                    f(speed),
                    f(t.lurk_low),
                    f(1.0 - t.lurk_low),
                    f(t.lurk_rate)
                ));
                body.push(format!("let rate = {};", f(rate)));
            }
        }
        // 2. the school
        if w.school {
            body.push("// the school: turn with its first fish, close in on the others, keep loose spacing".into());
            body.push("let (mut mid, mut count, mut push, mut lead) = (Vec3::ZERO, 0.0, Vec3::ZERO, None);".into());
            body.extend(block(
                "for (o, q) in &school",
                {
                    let mut v = vec!["let d = *q - p;".to_string()];
                    let mut inner = vec!["if lead.is_none() {".to_string(), "    lead = Some(o.clone());".into(), "}".into()];
                    let mut other = vec!["mid += *q;".to_string(), "count += 1.0;".into()];
                    other.extend(block(
                        format!("if d.length() < {}", f(t.school_spacing)),
                        vec![format!("push -= d.normalize_or_zero() * ({} - d.length());", f(t.school_spacing))],
                    ));
                    inner.extend(block("if *o != n", other));
                    v.extend(block(format!("if d.length() < {}", f(t.school_radius)), inner));
                    v
                },
            ));
            body.push(format!("let dir = lead.filter(|o| *o != n).and_then(|o| {map}.get(&o)).map_or(dir, |o| o.1);"));
            body.push(format!(
                "let pull = if count > 0.0 {{ (mid * (1.0 / count) - p) * {} }} else {{ Vec3::ZERO }} + push * {};",
                f(t.school_pull),
                f(t.school_push)
            ));
        }
        // 3. the edges
        body.push(format!(
            "let dir = if p.x > {r} {{ -1.0 }} else if p.x < -{r} {{ 1.0 }} else {{ dir }};",
            r = f(w.within)
        ));
        // 4. nosing toward
        if let Some((_, reach)) = &w.nose {
            body.push("// curious: noses toward it when it is near".into());
            body.push(format!("let nose = near.filter(|h| (*h - p).length() < {});", f(*reach)));
            body.push("let dir = nose.map_or(dir, |h| if h.x < p.x { -1.0 } else { 1.0 });".into());
            body.push(format!(
                "let go = nose.map_or(go, |h| ((h.x - p.x).abs() * {}).min({}));",
                f(t.nose_ease),
                f(speed * t.nose_speed)
            ));
        }
        // 5. velocity and depth
        body.push("let vx = vx + (dir * go - vx) * (1.0 - (-rate * dt).exp());".into());
        let mut ty = "home".to_string();
        if let Some(band) = w.band {
            ty = format!("home + (t * {} + seed * 6.283).sin() * {}", f(t.depth_rate), f(band * t.depth_share));
        }
        if let Some(h) = w.wave {
            ty = format!("{ty} + (t * {}).sin() * {}", f(t.swim_wave_rate * std::f32::consts::TAU), f(h / 2.0));
        }
        if let Some(band) = w.band {
            body.push(format!("let ty = ({ty}).clamp(home - {b}, home + {b});", b = f(band)));
        } else {
            body.push(format!("let ty = {ty};"));
        }
        if w.nose.is_some() {
            // it noses toward it within its own depth band, if it keeps one
            match w.band {
                Some(b) => body.push(format!("let ty = nose.map_or(ty, |h| h.y.clamp(home - {b}, home + {b}));", b = f(b))),
                None => body.push("let ty = nose.map_or(ty, |h| h.y);".into()),
            }
        }
        body.push(format!("let vy = (ty - p.y) * {};", f(t.depth_follow)));
        if let Some((_, reach, dash)) = &w.lunge {
            body.push("// lunging: a sudden dash at it when it comes near, then a glide back".into());
            body.push(format!(
                "let start = lunge <= 0.0 && {clock} >= wait && near.is_some_and(|h| (h - p).length() < {});",
                f(*reach)
            ));
            body.push(format!(
                "let (lunge, wait) = if start {{ ({ls}, {clock} + {next}) }} else {{ ((lunge - dt).max(0.0), wait) }};",
                ls = f(t.lunge_secs),
                next = f(t.lunge_secs + t.lunge_rest)
            ));
            body.push(format!(
                "let dash = near.filter(|_| lunge > 0.0).map(|h| (h - p).normalize_or_zero() * {});",
                f(*dash)
            ));
            body.push("let (vx, vy) = dash.map_or((vx, vy), |d| (d.x, d.y));".into());
        }
        let v = if w.school { "vec2(vx + pull.x, vy + pull.y)" } else { "vec2(vx, vy)" };
        body.push(format!("cx.set_vel(&n, {v});"));
        body.extend(block(format!("if vx.abs() > {}", f(t.face_speed)), vec!["cx.set_flip_x(&n, vx < 0.0);".into()]));
        body.push(format!("{map}.insert(n, (home, dir, vx, beat, lunge, wait));"));
        if w.school {
            l.extend(block("for (n, p) in school.clone()", body));
        } else {
            l.extend(block(format!("for n in cx.find_tag({})", s(&w.kind)), body));
        }
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    const PERCH: &str = "Each perch swims in a school within 16 units at 2 units per second, darting, keeping its depth within 1.5 units and nosing toward the hook within 4 units.";
    const ROACH: &str = "Each roach swims in a shoal within 16 units at 1.5 units per second, drifting, turning smoothly, in a wave 0.6 units high and keeping its depth within 1 unit.";
    const PIKE: &str = "Each pike swims within 16 units at 0.6 units per second, lurking, keeping its depth within 0.5 units and lunging at the hook within 4 units at 9 units per second.";

    #[test]
    fn applies() {
        assert!(matches(&Swim, PERCH));
        assert!(matches(&Swim, ROACH));
        assert!(matches(&Swim, PIKE));
    }

    #[test]
    fn does_not_apply() {
        // no way of swimming given: a plain patrol
        assert!(!matches(&Swim, "Each perch swims within 16 units at 2 units per second."));
        assert!(matches(&super::super::patrol::Patrol, "Each perch swims within 16 units at 2 units per second."));
    }

    #[test]
    fn a_school_darts_and_noses_toward_the_hook() {
        let c = code(PERCH);
        assert!(c.contains("[swim/steer]"), "{c}");
        assert!(c.contains("let school: Vec<(String, Vec3)> = cx.find_tag(\"perch\")"), "{c}");
        assert!(c.contains("let burst = cycle.fract() * 1.8 < 0.35;"), "{c}");
        assert!(c.contains("let nose = near.filter(|h| (*h - p).length() < 4.0);"), "{c}");
        assert!(c.contains(".clamp(home - 1.5, home + 1.5)"), "{c}");
        assert!(c.contains("let ty = nose.map_or(ty, |h| h.y.clamp(home - 1.5, home + 1.5));"), "{c}");
        assert!(c.contains("cx.set_flip_x(&n, vx < 0.0);"), "{c}");
    }

    #[test]
    fn a_smooth_turner_eases_slowly() {
        let c = code(ROACH);
        assert!(c.contains("let rate = 1.2;"), "{c}");
        assert!(c.contains("(t * 1.5707964).sin() * 0.3"), "{c}");
    }

    #[test]
    fn a_lurker_lunges_straight_at_the_hook() {
        let c = code(PIKE);
        assert!(c.contains("near.is_some_and(|h| (h - p).length() < 4.0)"), "{c}");
        assert!(c.contains("(h - p).normalize_or_zero() * 9.0"), "{c}");
        assert!(!c.contains("school"), "{c}");
    }

    #[test]
    fn hooked_ones_do_not_swim() {
        let c = code(&format!("{PIKE}\nEach hooked pike follows its hook."));
        assert!(c.contains("if self.hooked.contains_key(&n) {\n                continue;"), "{c}");
        let c = code(&format!("{PERCH}\nEach hooked perch follows its hook."));
        assert!(c.contains(".filter(|n| !self.hooked.contains_key(n))"), "{c}");
    }

    #[test]
    fn honest_failures() {
        let err = |src: &str| format!("{:?}", crate::game::parser::parse(src, false).unwrap_err());
        assert!(err("Each perch swims 3 units to each side at 2 units per second, darting.").contains("within N units"));
        assert!(err("Each perch swims within 16 units, darting.").contains("speed"));
        assert!(err("Each perch swims within 16 units at 2 units per second, darting and lurking.").contains("given twice"));
        assert!(err("Each pike swims within 16 units at 1 unit per second, lunging at the pike within 4 units at 9 units per second.").contains("itself"));
    }
}
