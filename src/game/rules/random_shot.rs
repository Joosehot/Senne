//! "a random alien shoots down|up|at the player at N units per second[, using
//! \"bomb\"][, with the laser sound]" — one of them, picked with the
//! engine's random numbers, fires a projectile tagged `bomb` (space-invader
//! bombs: "Every 1.2 seconds, a random alien shoots down ...").

use super::player_shoot::projectile;
use super::{block, s, At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::{Action, Aim};

pub struct RandomShot;

impl GamePattern for RandomShot {
    fn name(&self) -> &'static str {
        "random_shot"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["pick"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::RandomShot { .. }, .. })
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::RandomShot { kind, aim, speed, sprite, sound }, .. } = n else { unreachable!() };
        let mut body = vec![
            "let pick = &group[(cx.random(0.0, group.len() as f32) as usize).min(group.len() - 1)];".to_string(),
            "let from = cx.pos(pick);".into(),
        ];
        body.push(match aim {
            Aim::AtPlayer => "let dir = (cx.pos(\"player\") - from).normalize_or_zero();".to_string(),
            Aim::Up => "let dir = vec2(0.0, 1.0);".to_string(),
            Aim::Down | Aim::Forward => "let dir = vec2(0.0, -1.0);".to_string(),
        });
        body.extend(projectile(g, "bomb", sprite, "from", "dir", *speed, sound));
        let mut l = vec![format!("let group = cx.find_tag({});", s(kind))];
        l.extend(block("if !group.is_empty()", body));
        Emitted::inline(l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&RandomShot, "Every 1.2 seconds, a random alien shoots down at 7 units per second, using \"bomb\"."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&RandomShot, "Each alien shoots down at 7 units per second every 2 seconds."));
    }
}
