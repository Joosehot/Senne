//! "When the player falls below -12, ..." / "When an alien falls below -6, ..."
//! — checked every frame while playing; for a kind, "it" is the one that fell.

use super::{block, f, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Trigger;

pub struct FallsBelow;

impl GamePattern for FallsBelow {
    fn name(&self) -> &'static str {
        "falls_below"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["check"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Trigger(Trigger::PlayerBelow(_) | Trigger::ThingBelow(..)))
    }
    fn emit(&self, _variant: &str, n: &Node, _g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        let l = match n {
            Node::Trigger(Trigger::PlayerBelow(y)) => block(format!("if cx.pos(\"player\").y < {}", f(*y)), body),
            Node::Trigger(Trigger::ThingBelow(t, y)) => {
                block(format!("for n in cx.find_tag({})", s(t)), block(format!("if cx.pos(&n).y < {}", f(*y)), body))
            }
            _ => unreachable!(),
        };
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&FallsBelow, "When the player falls below -12, restart the game."));
        assert!(matches(&FallsBelow, "When an alien falls below -6, the player loses."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&FallsBelow, "When the player presses r, restart the game."));
    }
}
