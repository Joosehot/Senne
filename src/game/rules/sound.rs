//! "play the coin sound"

use super::{s, At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::Action;

pub struct Sound;

impl GamePattern for Sound {
    fn name(&self) -> &'static str {
        "sound"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["effect"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Sound(_), .. })
    }
    fn emit(&self, _variant: &str, n: &Node, _g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Sound(name), .. } = n else { unreachable!() };
        Emitted::inline(vec![format!("cx.play_sound({});", s(name))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Sound, "When the player touches a coin, play the coin sound."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Sound, "At the start, play the music \"C4 E4\"."));
    }
}
