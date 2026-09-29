//! "play the jump animation on the player" — a one-shot model animation (3D).

use super::{s, At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::Action;

pub struct PlayAnim;

impl GamePattern for PlayAnim {
    fn name(&self) -> &'static str {
        "play_anim"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["oneshot"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Anim { .. }, .. })
    }
    fn emit(&self, _variant: &str, n: &Node, _g: &mut Gen, at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Anim { anim, target }, .. } = n else { unreachable!() };
        Emitted::inline(vec![format!("cx.play_anim({}, {});", at.one(target), s(anim))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&PlayAnim, "When the player touches a gem, play the spin animation on the player."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&PlayAnim, "When the player touches a gem, play the pickup sound."));
    }
}
