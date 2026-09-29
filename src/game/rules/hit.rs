//! "When a shot hits an alien, ..." / "When a bomb hits the player, ..." —
//! in `on_collision`, which sees every pair both ways round, so one ordered
//! check is enough. "it" is the second thing (the first when the second is
//! the player); "the shot" / "the alien" name each side; "both" is both.

use super::{block, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::{Party, Trigger};

pub struct Hit;

impl GamePattern for Hit {
    fn name(&self) -> &'static str {
        "hit"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["has_tag"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Trigger(Trigger::Hit { .. }))
    }
    fn emit(&self, _variant: &str, n: &Node, _g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        let Node::Trigger(Trigger::Hit { a, b }) = n else { unreachable!() };
        let second = match b {
            Party::Player => "b == \"player\"".to_string(),
            Party::Thing(t) => format!("cx.has_tag(b, {})", s(t)),
        };
        Emitted::to(Section::Collide, block(format!("if cx.has_tag(a, {}) && {second}", s(a)), body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Hit, "When a shot hits an alien, destroy both."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Hit, "When the player touches an alien, the player loses."));
    }
}
