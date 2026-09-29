//! "the player has 3 lives" (only "At the start")

use super::{At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::Action;

pub struct Lives;

impl GamePattern for Lives {
    fn name(&self) -> &'static str {
        "lives"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["set"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Lives(_), .. })
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Lives(v), .. } = n else { unreachable!() };
        let lives = g.field("lives", "i64", &g.t.lives.to_string());
        Emitted::inline(vec![format!("{lives} = {v};")])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Lives, "At the start, the player has 3 lives."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Lives, "At the start, play the coin sound."));
    }
}
