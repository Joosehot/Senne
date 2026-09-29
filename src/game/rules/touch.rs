//! "When the player touches a coin[ and no keys are left], ..." — in
//! `on_collision`; "it" is the coin.

use super::{block, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Trigger;

pub struct Touch;

impl GamePattern for Touch {
    fn name(&self) -> &'static str {
        "touch"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["has_tag"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Trigger(Trigger::Touch { .. }))
    }
    fn emit(&self, _variant: &str, n: &Node, _g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        let Node::Trigger(Trigger::Touch { thing, none_left }) = n else { unreachable!() };
        let mut cond = format!("a == \"player\" && cx.has_tag(b, {})", s(thing));
        if let Some(t) = none_left {
            cond.push_str(&format!(" && cx.count_tag({}) == 0", s(t)));
        }
        Emitted::to(Section::Collide, block(format!("if {cond}"), body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Touch, "When the player touches a coin, destroy it."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Touch, "When the player lands on a slime, destroy it."));
    }
}
