//! "When the player lands on a slime, ..." — the player is above it
//! (`[game.tuning] stomp_height`) and falling. While the player is above it,
//! the pair's touches are skipped (`return`), so a stomp never also hurts:
//! this block is written before every touch.

use super::{block, f, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Trigger;

pub struct LandsOn;

impl GamePattern for LandsOn {
    fn name(&self) -> &'static str {
        "lands_on"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["above"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Trigger(Trigger::LandsOn(_)))
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        let Node::Trigger(Trigger::LandsOn(thing)) = n else { unreachable!() };
        let mut inner = block("if cx.vel(a).y <= 0.0", body);
        inner.push("return;".into());
        Emitted::to(
            Section::Land,
            block(
                format!("if a == \"player\" && cx.has_tag(b, {}) && cx.pos(a).y - cx.pos(b).y > {}", s(thing), f(g.t.stomp_height)),
                inner,
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&LandsOn, "When the player lands on a slime, destroy it."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&LandsOn, "When the player touches a slime, the player loses."));
    }

    #[test]
    fn landing_comes_before_touching() {
        let c = code(
            "When the player touches a slime, the player loses.\n\
             When the player lands on a slime, destroy it.",
        );
        let land = c.find("[lands_on/above]").unwrap();
        let touch = c.find("[touch/has_tag]").unwrap();
        assert!(land < touch, "{c}");
    }
}
