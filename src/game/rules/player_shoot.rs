//! "The player shoots up|down|forward [across the ground] with space at N
//! units per second [every N seconds][, using "shot"][, with the shoot sound]."
//!
//! Holding the key fires repeatedly with a cooldown. Variants: `countdown`
//! (a timer field counts down by dt) and `timestamp` (remember when the last
//! shot left and compare with the clock). Shots are tagged `shot`; ones far
//! from the player are removed.

use super::{block, f, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::{Aim, Standing};

pub struct PlayerShoot;

/// Spawn one projectile tagged `tag` at `from` flying along the unit vector `dir` (both expressions).
pub fn projectile(g: &mut Gen, tag: &str, sprite: &str, from: &str, dir: &str, speed: f32, sound: &Option<String>) -> Vec<String> {
    g.cleanup(tag);
    let mut v = vec![
        format!(
            "let shot = cx.spawn_sprite({t}, &[{t}], {}, {from} + {dir} * {}, Vec3::splat({}), BodyKind::Kinematic);",
            s(sprite),
            f(g.t.shot_offset),
            f(g.t.shot_size),
            t = s(tag)
        ),
        "cx.set_solid(&shot, false);".into(),
        format!("cx.set_vel(&shot, {dir} * {});", f(speed)),
    ];
    if let Some(snd) = sound {
        v.push(format!("cx.play_sound({});", s(snd)));
    }
    v
}

impl GamePattern for PlayerShoot {
    fn name(&self) -> &'static str {
        "player_shoot"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["countdown", "timestamp"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Shoot { .. }))
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Shoot { aim, ground: _, key, speed, every, sprite, sound }) = n else { unreachable!() };
        let every = every.unwrap_or(g.t.shot_cooldown);
        let dir = match aim {
            Aim::Up => "vec2(0.0, 1.0)".to_string(),
            Aim::Down => "vec2(0.0, -1.0)".to_string(),
            Aim::Forward | Aim::AtPlayer => g.field("aim", "Vec3", "Vec3::X"),
        };
        let mut fire = vec![format!("let dir = {dir};")];
        let mut l = Vec::new();
        let head = if variant == "timestamp" {
            let last = g.fresh("last_shot", "f32", "f32::NEG_INFINITY");
            fire.insert(0, format!("{last} = cx.time();"));
            format!("if cx.key({}) && cx.time() - {last} >= {}", s(key), f(every))
        } else {
            let timer = g.fresh("shot_timer", "f32", "0.0");
            l.push(format!("{timer} -= dt;"));
            fire.insert(0, format!("{timer} = {};", f(every)));
            format!("if cx.key({}) && {timer} <= 0.0", s(key))
        };
        fire.extend(projectile(g, "shot", sprite, "cx.pos(\"player\")", "dir", *speed, sound));
        l.extend(block(head, fire));
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&PlayerShoot, "The player shoots up with space at 22 units per second every 0.3 seconds."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&PlayerShoot, "Each ghost shoots at the player at 6 units per second every 2 seconds."));
    }

    #[test]
    fn forward_uses_the_remembered_aim() {
        let c = code(
            "Move the player with the arrow keys at 7 units per second.\n\
             The player shoots forward with space at 20 units per second.",
        );
        assert!(c.contains("self.aim = want.normalize();"), "{c}");
        assert!(c.contains("let dir = self.aim;"), "{c}");
    }
}
