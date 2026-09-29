//! "Move the player [sideways | across the ground] with the arrow keys at N
//! units per second [and face where it moves] [smoothly] [within N units]."
//!
//! Variants: `axis` (sideways: one axis, nothing to choose), `raw` (both
//! axes as read: diagonals are faster) and `normalized` (diagonals the same
//! speed as straight lines).

use super::{block, f, At, Emitted, Facts, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::{Mode, Standing};

pub struct MovePlayer;

impl GamePattern for MovePlayer {
    fn name(&self) -> &'static str {
        "move_player"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["axis", "raw", "normalized"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Move { .. }))
    }
    fn variants(&self, n: &Node, _f: &Facts) -> Vec<&'static str> {
        match n {
            Node::Standing(Standing::Move { mode: Mode::Sideways, .. }) => vec!["axis"],
            _ => vec!["raw", "normalized"],
        }
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Move { mode, speed, face, smooth, within }) = n else { unreachable!() };
        let mut l = Vec::new();
        let sideways = *mode == Mode::Sideways;
        if sideways {
            l.push(format!("let want = cx.axis_x() * {};", f(*speed)));
        } else if variant == "normalized" {
            l.push(format!("let want = vec2(cx.axis_x(), cx.axis_y()).normalize_or_zero() * {};", f(*speed)));
        } else {
            l.push(format!("let want = vec2(cx.axis_x(), cx.axis_y()) * {};", f(*speed)));
        }
        // (velocity component setter, target expression, current-velocity component)
        let axes: Vec<(&str, String, &str)> = match mode {
            Mode::Sideways => vec![("set_vel_x", "want".into(), "x")],
            Mode::Plane => vec![("set_vel_x", "want.x".into(), "x"), ("set_vel_y", "want.y".into(), "y")],
            Mode::Ground => vec![("set_vel_x", "want.x".into(), "x"), ("set_vel_z", "-want.y".into(), "z")],
        };
        if *smooth {
            l.push(format!("let ease = 1.0 - (-{} * dt).exp();", f(g.t.smoothing_rate)));
            l.push("let v = cx.vel(\"player\");".into());
            for (set, want, c) in &axes {
                l.push(format!("cx.{set}(\"player\", v.{c} + ({want} - v.{c}) * ease);"));
            }
        } else {
            for (set, want, _) in &axes {
                l.push(format!("cx.{set}(\"player\", {want});"));
            }
        }
        if *face {
            let x = if sideways { "want" } else { "want.x" };
            l.extend(block(format!("if {x} != 0.0"), vec![format!("cx.set_flip_x(\"player\", {x} < 0.0);")]));
        }
        if g.facts.needs_aim {
            let aim = g.field("aim", "Vec3", "Vec3::X");
            let (cond, val) = match mode {
                Mode::Sideways => ("want != 0.0", "vec2(want.signum(), 0.0)".to_string()),
                Mode::Plane => ("want != Vec3::ZERO", "want.normalize()".to_string()),
                Mode::Ground => ("want != Vec3::ZERO", "vec3(want.x, 0.0, -want.y).normalize()".to_string()),
            };
            l.extend(block(format!("if {cond}"), vec![format!("{aim} = {val};")]));
        }
        if let Some(w) = within {
            l.push("let p = cx.pos(\"player\");".into());
            l.extend(block(
                format!("if p.x.abs() > {}", f(*w)),
                vec![
                    format!("cx.set_pos(\"player\", vec3(p.x.clamp(-{w}, {w}), p.y, p.z));", w = f(*w)),
                    "cx.set_vel_x(\"player\", 0.0);".into(),
                ],
            ));
        }
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&MovePlayer, "Move the player with the arrow keys at 7 units per second."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&MovePlayer, "Each zombie moves toward the player at 2 units per second."));
    }

    #[test]
    fn simply_takes_raw_diagonals() {
        let plain = code("Move the player with the arrow keys at 7 units per second.");
        let terse = code("Simply move the player with the arrow keys at 7 units per second.");
        assert!(plain.contains("normalize_or_zero"), "{plain}");
        assert!(!terse.contains("normalize_or_zero"), "{terse}");
    }
}
