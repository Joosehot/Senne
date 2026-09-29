//! "the player goes back to the start" — where it stood when the game began.

use super::{At, Emitted, GamePattern, Node};
use crate::game::codegen::{Feature, Gen};
use crate::game::parser::Action;

pub struct BackToStart;

impl GamePattern for BackToStart {
    fn name(&self) -> &'static str {
        "back_to_start"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["reset"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::BackToStart, .. })
    }
    fn emit(&self, _variant: &str, _n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        g.need(Feature::Start);
        Emitted::inline(vec!["cx.set_pos(\"player\", self.start);".into(), "cx.set_vel(\"player\", Vec3::ZERO);".into()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&BackToStart, "When the player falls below -12, the player goes back to the start."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&BackToStart, "When the player falls below -12, restart the game."));
    }
}
