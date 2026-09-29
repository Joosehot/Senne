//! "When no aliens are left, ..." — edge-triggered: fires once when the
//! count reaches zero, re-arms when there are some again (e.g. a new wave).

use super::{block, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Trigger;

pub struct NoneLeft;

impl GamePattern for NoneLeft {
    fn name(&self) -> &'static str {
        "none_left"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["edge"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Trigger(Trigger::NoneLeft(_)))
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        let Node::Trigger(Trigger::NoneLeft(thing)) = n else { unreachable!() };
        let flag = g.fresh(&format!("no_{thing}_left"), "bool", "false");
        let mut l = vec![format!("let none = cx.count_tag({}) == 0;", s(thing))];
        l.extend(block(format!("if none && !{flag}"), body));
        l.push(format!("{flag} = none;"));
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&NoneLeft, "When no coins are left, the player wins."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&NoneLeft, "When the player touches the door and no coins are left, the player wins."));
    }
}
