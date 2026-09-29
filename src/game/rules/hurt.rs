//! "the player loses a life[, safe for N seconds]"
//!
//! Ignored while safe. Otherwise one life goes, the hurt sound plays, and
//! either the game is lost (no lives left) or the player is safe for a while
//! (blinking; default `[game.tuning] safe_seconds`). Variants: `inline` or
//! `helper` (a `hurt` method); `helper_reuse` judges, counting the "lose"
//! inside each hurt block too.

use super::win::end_game;
use super::{block, f, At, Emitted, GamePattern, Helper, Node, Uses};
use crate::game::codegen::{Feature, Gen};
use crate::game::parser::Action;

pub struct Hurt;

impl GamePattern for Hurt {
    fn name(&self) -> &'static str {
        "hurt"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["inline", "helper"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::LoseLife { .. }, .. })
    }
    fn uses(&self, variant: &str, _n: &Node) -> Uses {
        Uses { helper: Some((Helper::Hurt, variant == "helper")), ..Uses::default() }
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::LoseLife { safe }, .. } = n else { unreachable!() };
        let safe = safe.unwrap_or(g.t.safe_seconds);
        let lives = g.field("lives", "i64", &g.t.lives.to_string());
        let safe_field = g.field("safe", "f32", "0.0");
        g.need(Feature::Blink);
        let banner = g.banner(false, None);
        let lose = end_game(g, false, &banner, g.stats.embedded_lose_helper());
        let body = |safe_value: String| {
            let mut v = vec![format!("{lives} -= 1;"), "cx.play_sound(\"hurt\");".to_string()];
            v.extend(block(format!("if {lives} <= 0"), lose.clone()));
            v.extend(block("else", vec![format!("{safe_field} = {safe_value};")]));
            super::jump::join_else(v)
        };
        if variant == "helper" {
            let mut m = block(format!("if {safe_field} > 0.0"), vec!["return;".into()]);
            m.extend(body("safe".into()));
            g.helper("hurt", block("fn hurt(&mut self, cx: &mut Ctx, safe: f32)", m));
            Emitted::inline(vec![format!("self.hurt(cx, {});", f(safe))])
        } else {
            Emitted::inline(block(format!("if {safe_field} <= 0.0"), body(f(safe))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&Hurt, "When the player touches a slime, the player loses a life, safe for 1.5 seconds."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Hurt, "When the player touches a slime, the player loses."));
    }

    #[test]
    fn losing_lives_blinks_and_can_lose() {
        let c = code("When the player touches a slime, the player loses a life.");
        assert!(c.contains("blink while safe"), "{c}");
        assert!(c.contains("self.status = Status::Lost;"), "{c}");
    }
}
