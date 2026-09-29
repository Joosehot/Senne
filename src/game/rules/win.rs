//! "the player wins [\"LEVEL CLEAR\"]"
//!
//! Sets the status, plays the win sound and shows the banner (plus
//! " - PRESS R" when a key restarts the game). Guarded, so it happens once.
//! Variants: `inline` or `helper` (a `win` method); `helper_reuse` judges.

use super::{block, f, s, At, Emitted, GamePattern, Helper, Node, Uses};
use crate::game::codegen::Gen;
use crate::game::parser::Action;

pub struct Win;

/// End the game (won or lost) with `banner`, inline or through the helper method.
pub fn end_game(g: &mut Gen, won: bool, banner: &str, helper: bool) -> Vec<String> {
    let (method, status, sound) = if won { ("win", "Status::Won", "win") } else { ("lose", "Status::Lost", "lose") };
    let [x, y] = g.t.banner;
    let size = f(g.t.banner_size);
    let show = |text: &str| format!("cx.text_at(\"banner\", {text}, {}, {}, {size});", f(x), f(y));
    if helper {
        let mut body = block("if self.status != Status::Playing", vec!["return;".into()]);
        body.push(format!("self.status = {status};"));
        body.push(format!("cx.play_sound(\"{sound}\");"));
        body.push(show("banner"));
        g.helper(method, block(format!("fn {method}(&mut self, cx: &mut Ctx, banner: &str)"), body));
        vec![format!("self.{method}(cx, {});", s(banner))]
    } else {
        block(
            "if self.status == Status::Playing",
            vec![format!("self.status = {status};"), format!("cx.play_sound(\"{sound}\");"), show(&s(banner))],
        )
    }
}

impl GamePattern for Win {
    fn name(&self) -> &'static str {
        "win"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["inline", "helper"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Win(_), .. })
    }
    fn uses(&self, variant: &str, _n: &Node) -> Uses {
        Uses { helper: Some((Helper::Win, variant == "helper")), ..Uses::default() }
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Win(msg), .. } = n else { unreachable!() };
        let banner = g.banner(true, msg.as_deref());
        Emitted::inline(end_game(g, true, &banner, variant == "helper"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&Win, "When no coins are left, the player wins \"LEVEL CLEAR\"."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Win, "When no coins are left, the player loses."));
    }

    #[test]
    fn banner_mentions_the_restart_key() {
        let c = code(
            "When no coins are left, the player wins.\n\
             When the player presses r, restart the game.",
        );
        assert!(c.contains("\"YOU WIN! - PRESS R\""), "{c}");
    }

    #[test]
    fn two_wins_share_a_helper() {
        let c = code(
            "When no coins are left, the player wins.\n\
             When the player touches a flag, the player wins.",
        );
        assert!(c.contains("fn win(&mut self, cx: &mut Ctx, banner: &str)"), "{c}");
        assert_eq!(c.matches("self.win(cx, ").count(), 2, "{c}");
    }
}
