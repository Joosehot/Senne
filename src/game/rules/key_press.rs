//! "When the player presses r, ..." — runs first in `update`, even after the
//! game is won or lost (so "restart the game" works on the end screen).

use super::{block, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Trigger;

pub struct KeyPress;

impl GamePattern for KeyPress {
    fn name(&self) -> &'static str {
        "key_press"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["pressed"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Trigger(Trigger::Press(_)))
    }
    fn emit(&self, _variant: &str, n: &Node, _g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        let Node::Trigger(Trigger::Press(k)) = n else { unreachable!() };
        Emitted::to(Section::Keys, block(format!("if cx.key_pressed({})", s(k)), body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&KeyPress, "When the player presses r, restart the game."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&KeyPress, "When the player touches a coin, restart the game."));
    }

    #[test]
    fn keys_run_before_the_playing_check() {
        let c = code(
            "When the player presses r, restart the game.\n\
             Every frame, add 1 to the score.",
        );
        assert!(c.find("key_pressed(\"r\")").unwrap() < c.find("if self.status != Status::Playing").unwrap(), "{c}");
    }
}
