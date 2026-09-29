//! "squash it" / "squash the player" / "squash the slimes" — a quick
//! squash-and-stretch (`[game.tuning] squash_amount`).

use super::{block, f, s, At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::{Action, Target};

pub struct Squash;

impl GamePattern for Squash {
    fn name(&self) -> &'static str {
        "squash"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["tween"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Squash(_), .. })
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Squash(t), .. } = n else { unreachable!() };
        let amount = f(g.t.squash_amount);
        let l = match t {
            Target::All(tag) => block(format!("for n in cx.find_tag({})", s(tag)), vec![format!("cx.squash(&n, {amount});")]),
            one => vec![format!("cx.squash({}, {amount});", at.one(one))],
        };
        Emitted::inline(l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Squash, "When the player lands on a slime, squash it."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Squash, "When the player lands on a slime, destroy it."));
    }
}
