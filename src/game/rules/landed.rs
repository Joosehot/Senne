//! "When a hooked perch reaches the hole, ..." — every frame, each hooked
//! perch whose hook is within `[game.tuning] land_margin` of the first hole's
//! height is unhooked and the actions run with "it" = the perch. Unhooking
//! first means it fires once per catch even if the actions keep the perch.

use super::{block, f, hooked_map, s, with_anchor, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Trigger;

pub struct Landed;

impl GamePattern for Landed {
    fn name(&self) -> &'static str {
        "landed"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["scan"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Trigger(Trigger::Landed { .. }))
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        let Node::Trigger(Trigger::Landed { kind, place }) = n else { unreachable!() };
        let map = hooked_map(g);
        let mut inner = vec![format!("{map}.remove(&n);")];
        inner.extend(body);
        let check = block(
            format!("if cx.has_tag(&n, {}) && cx.pos(&hook).y >= top.y - {}", s(kind), f(g.t.land_margin)),
            inner,
        );
        let l = with_anchor(place, block(format!("for (n, hook) in {map}.clone()"), check));
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&Landed, "When a hooked perch reaches the hole, add 10 points."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Landed, "When a hook touches a perch, add 10 points."));
    }

    #[test]
    fn a_catch_is_unhooked_then_handled() {
        let c = code("When a hooked perch reaches the hole, destroy it.");
        let un = c.find("self.hooked.remove(&n);").unwrap();
        let destroy = c.find("cx.destroy(&n);").unwrap();
        assert!(un < destroy, "{c}");
    }
}
