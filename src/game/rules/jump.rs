//! "The player jumps with space at N units per second[, twice at N units per
//! second][, higher while held][, with the jump sound]."
//!
//! Variants: `on_press` (jump the frame the key goes down, if grounded) and
//! `buffered` (a press shortly before landing still counts, and the player
//! may jump shortly after walking off a ledge; times from `[game.tuning]`).

use super::{block, f, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Standing;

pub struct Jump;

fn launch(speed: f32, sound: &Option<String>) -> Vec<String> {
    let mut v = vec![format!("cx.set_vel_y(\"player\", {});", f(speed))];
    if let Some(snd) = sound {
        v.push(format!("cx.play_sound({});", s(snd)));
    }
    v
}

impl GamePattern for Jump {
    fn name(&self) -> &'static str {
        "jump"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["on_press", "buffered"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Jump { .. }))
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Jump { key, speed, air, held, sound }) = n else { unreachable!() };
        let k = s(key);
        let air_jumped = air.map(|_| g.field("air_jumped", "bool", "false"));
        let mut l = Vec::new();
        if variant == "buffered" {
            let buffer = g.field("jump_buffer", "f32", "0.0");
            let coyote = g.field("coyote", "f32", "0.0");
            let mut on_ground = vec![format!("{coyote} = {};", f(g.t.coyote_time))];
            if let Some(aj) = &air_jumped {
                on_ground.push(format!("{aj} = false;"));
            }
            l.extend(block("if cx.grounded(\"player\")", on_ground));
            l.extend(block("else", vec![format!("{coyote} -= dt;")]));
            l.extend(block(format!("if cx.key_pressed({k})"), vec![format!("{buffer} = {};", f(g.t.jump_buffer))]));
            l.extend(block("else", vec![format!("{buffer} -= dt;")]));
            let mut jump = vec![format!("{buffer} = 0.0;"), format!("{coyote} = 0.0;")];
            jump.extend(launch(*speed, sound));
            l.extend(block(format!("if {buffer} > 0.0 && {coyote} > 0.0"), jump));
            if let (Some(aj), Some(second)) = (&air_jumped, air) {
                let mut again = vec![format!("{aj} = true;"), format!("{buffer} = 0.0;")];
                again.extend(launch(*second, sound));
                l.extend(block(format!("else if cx.key_pressed({k}) && {coyote} <= 0.0 && !{aj}"), again));
            }
        } else {
            match (&air_jumped, air) {
                (Some(aj), Some(second)) => {
                    l.push("let grounded = cx.grounded(\"player\");".into());
                    l.extend(block("if grounded", vec![format!("{aj} = false;")]));
                    let mut inner = block("if grounded", launch(*speed, sound));
                    let mut again = vec![format!("{aj} = true;")];
                    again.extend(launch(*second, sound));
                    inner.extend(block(format!("else if !{aj}"), again));
                    l.extend(block(format!("if cx.key_pressed({k})"), inner));
                }
                _ => {
                    l.extend(block(format!("if cx.key_pressed({k}) && cx.grounded(\"player\")"), launch(*speed, sound)));
                }
            }
        }
        if *held {
            l.push("let rise = cx.vel(\"player\").y;".into());
            l.extend(block(
                format!("if rise > 0.0 && !cx.key({k})"),
                vec![format!("cx.set_vel_y(\"player\", rise - {} * dt);", f(g.t.jump_release_gravity))],
            ));
        }
        // "if {} else {}" pairs are emitted as separate blocks; join them.
        Emitted::to(Section::Update, join_else(l))
    }
}

/// `}` followed by `else ... {` on the next line -> `} else ... {`.
pub fn join_else(lines: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in lines {
        let t = l.trim_start();
        if t.starts_with("else") {
            if let Some(prev) = out.last_mut() {
                if prev.trim_end().ends_with('}') {
                    prev.push(' ');
                    prev.push_str(t);
                    continue;
                }
            }
        }
        out.push(l);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&Jump, "The player jumps with space at 14 units per second."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Jump, "The player shoots up with space at 20 units per second."));
    }

    #[test]
    fn else_joins_the_closing_brace() {
        let c = code("Simply, the player jumps with space at 14 units per second, twice at 12 units per second.");
        assert!(c.contains("} else if !self.air_jumped {"), "{c}");
    }
}
