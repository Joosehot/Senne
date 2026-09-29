//! "the player loses [\"GAME OVER\"]" — like `win`, with the lose sound and
//! "GAME OVER". Variants: `inline` or `helper` (a `lose` method).

use super::win::end_game;
use super::{At, Emitted, GamePattern, Helper, Node, Uses};
use crate::game::codegen::Gen;
use crate::game::parser::Action;

pub struct Lose;

impl GamePattern for Lose {
    fn name(&self) -> &'static str {
        "lose"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["inline", "helper"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Lose(_), .. })
    }
    fn uses(&self, variant: &str, _n: &Node) -> Uses {
        Uses { helper: Some((Helper::Lose, variant == "helper")), ..Uses::default() }
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Lose(msg), .. } = n else { unreachable!() };
        let banner = g.banner(false, msg.as_deref());
        Emitted::inline(end_game(g, false, &banner, variant == "helper"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Lose, "When the player touches a spike, the player loses."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Lose, "When the player touches a spike, the player loses a life."));
    }
}
