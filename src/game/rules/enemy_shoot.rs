//! "Each ghost shoots at the player|down|up at N units per second every N
//! seconds[ within N units][, using "bomb"][, with the laser sound]."
//!
//! One timer for the kind; each volley every one fires (those within range
//! when a range is given). Projectiles are tagged `bomb`. Variants when the
//! player's position is read: `hoisted` / `per_loop` (see `chase`); when it
//! is not (straight up or down, no range): `fixed`.

use super::chase::player_pos;
use super::player_shoot::projectile;
use super::{block, f, s, At, Emitted, Facts, GamePattern, Node, Section, Uses};
use crate::game::codegen::Gen;
use crate::game::parser::{Aim, Standing};

pub struct EnemyShoot;

fn reads_player(n: &Node) -> bool {
    matches!(n, Node::Standing(Standing::EnemyShoot { aim: Aim::AtPlayer, .. } | Standing::EnemyShoot { within: Some(_), .. }))
}

impl GamePattern for EnemyShoot {
    fn name(&self) -> &'static str {
        "enemy_shoot"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["hoisted", "per_loop", "fixed"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::EnemyShoot { .. }))
    }
    fn variants(&self, n: &Node, _f: &Facts) -> Vec<&'static str> {
        if reads_player(n) {
            vec!["hoisted", "per_loop"]
        } else {
            vec!["fixed"]
        }
    }
    fn uses(&self, variant: &str, n: &Node) -> Uses {
        Uses { hoist: reads_player(n).then_some(variant == "hoisted"), ..Uses::default() }
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::EnemyShoot { kind, aim, speed, every, within, sprite, sound }) = n else { unreachable!() };
        let timer = g.fresh(&format!("{kind}_fire"), "f32", &f(*every));
        let mut body = vec!["let from = cx.pos(&n);".to_string()];
        if reads_player(n) {
            let player = player_pos(variant, g);
            body.push(format!("let to = {player} - from;"));
        }
        if let Some(w) = within {
            body.extend(block(format!("if to.length() > {}", f(*w)), vec!["continue;".into()]));
        }
        body.push(match aim {
            Aim::AtPlayer => "let dir = to.normalize_or_zero();".to_string(),
            Aim::Up => "let dir = vec2(0.0, 1.0);".to_string(),
            Aim::Down | Aim::Forward => "let dir = vec2(0.0, -1.0);".to_string(),
        });
        body.extend(projectile(g, "bomb", sprite, "from", "dir", *speed, sound));
        let mut fire = vec![format!("{timer} += {};", f(*every))];
        fire.extend(block(format!("for n in cx.find_tag({})", s(kind)), body));
        let mut l = vec![format!("{timer} -= dt;")];
        l.extend(block(format!("if {timer} <= 0.0"), fire));
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(
            &EnemyShoot,
            "Each ghost shoots at the player at 6 units per second every 2 seconds within 10 units, using \"bomb\"."
        ));
    }

    #[test]
    fn straight_shots_do_not_read_the_player() {
        let c = crate::game::rules::test_util::code("Each alien shoots down at 5 units per second every 2 seconds.");
        assert!(c.contains("[enemy_shoot/fixed]") && !c.contains("cx.pos(\"player\") -"), "{c}");
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&EnemyShoot, "The player shoots up with space at 22 units per second."));
    }
}
