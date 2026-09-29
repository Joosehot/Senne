//! "Each blob wanders at N units per second, turning every N seconds
//! [across the ground]."
//!
//! Variants: `shared_timer` (one timer; all of them turn together) and
//! `own_timer` (each has its own, jittered by `[game.tuning] wander_jitter`,
//! so they don't move in lockstep).

use super::{block, f, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Standing;

pub struct Wander;

fn turn(speed: f32, ground: bool) -> Vec<String> {
    let mut v = vec!["let a = cx.random(0.0, std::f32::consts::TAU);".to_string()];
    if ground {
        v.push(format!("cx.set_vel_x(&n, a.cos() * {});", f(speed)));
        v.push(format!("cx.set_vel_z(&n, a.sin() * {});", f(speed)));
    } else {
        v.push(format!("cx.set_vel(&n, vec2(a.cos(), a.sin()) * {});", f(speed)));
    }
    v
}

impl GamePattern for Wander {
    fn name(&self) -> &'static str {
        "wander"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["shared_timer", "own_timer"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Wander { .. }))
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Wander { kind, speed, turn: every, ground }) = n else { unreachable!() };
        let mut l = Vec::new();
        if variant == "own_timer" {
            let map = g.fresh(&format!("wander_{kind}"), "BTreeMap<String, f32>", "BTreeMap::new()");
            let j = g.t.wander_jitter;
            let mut fire = vec![format!("*left = cx.random({}, {});", f(every * (1.0 - j)), f(every * (1.0 + j)))];
            fire.extend(turn(*speed, *ground));
            let mut body = vec![format!("let left = {map}.entry(n.clone()).or_insert(0.0);"), "*left -= dt;".into()];
            body.extend(block("if *left <= 0.0", fire));
            l.extend(block(format!("for n in cx.find_tag({})", s(kind)), body));
        } else {
            let timer = g.fresh(&format!("wander_{kind}"), "f32", "0.0");
            l.push(format!("{timer} -= dt;"));
            let mut fire = vec![format!("{timer} += {};", f(*every))];
            fire.extend(block(format!("for n in cx.find_tag({})", s(kind)), turn(*speed, *ground)));
            l.extend(block(format!("if {timer} <= 0.0"), fire));
        }
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Wander, "Each blob wanders at 2 units per second, turning every 1.5 seconds."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Wander, "Each zombie moves toward the player at 2 units per second."));
    }
}
