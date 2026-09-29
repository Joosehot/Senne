//! "add 1 to the score" / "add 10 points"

use super::{At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::Action;

pub struct Score;

impl GamePattern for Score {
    fn name(&self) -> &'static str {
        "score"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["add"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::AddScore(_), .. })
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::AddScore(v), .. } = n else { unreachable!() };
        let score = g.field("score", "i64", "0");
        Emitted::inline(vec![format!("{score} += {v};")])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Score, "When the player touches a coin, add 10 points."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Score, "When the player touches a coin, destroy it."));
    }
}
