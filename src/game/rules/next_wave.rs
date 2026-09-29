//! "start the next wave of aliens" — the scene's aliens come back and the
//! wave count goes up (marching can speed up per wave; {wave} shows it).

use super::{s, At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::Action;

pub struct NextWave;

impl GamePattern for NextWave {
    fn name(&self) -> &'static str {
        "next_wave"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["respawn"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::NextWave(_), .. })
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::NextWave(tag), .. } = n else { unreachable!() };
        let wave = g.field("wave", "i64", "1");
        Emitted::inline(vec![format!("{wave} += 1;"), format!("cx.respawn_tag({});", s(tag))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&NextWave, "When no aliens are left, start the next wave of aliens."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&NextWave, "When no aliens are left, restart the game."));
    }
}
