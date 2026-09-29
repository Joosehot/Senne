//! "restart the game" — Droid rebuilds the scene and calls `init` on a fresh
//! game. As the last action of an update or collision trigger it also returns,
//! so nothing else runs on the old state this frame.

use super::{At, Emitted, GamePattern, Node, Place};
use crate::game::codegen::Gen;
use crate::game::parser::Action;

pub struct Restart;

impl GamePattern for Restart {
    fn name(&self) -> &'static str {
        "restart"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["request"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Restart, .. })
    }
    fn emit(&self, _variant: &str, _n: &Node, _g: &mut Gen, at: &At, _body: Vec<String>) -> Emitted {
        let mut l = vec!["cx.restart();".to_string()];
        if at.last && at.place != Place::Init {
            l.push("return;".into());
        }
        Emitted::inline(l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Restart, "When the player presses r, restart the game."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Restart, "When the player presses r, play the click sound."));
    }
}
