//! "a new perch comes back after 3 seconds" / "... comes in from the side
//! after 3 seconds" — "it"'s scene entity is queued and spawned again from
//! the scene (Droid's `cx.respawn`) once the game clock passes; "from the
//! side" puts it `[game.tuning] side_x` left or right at its own height.

use super::{f, At, Emitted, GamePattern, Node, Target};
use crate::game::codegen::{Feature, Gen};
use crate::game::parser::Action;

pub struct ComeBack;

impl GamePattern for ComeBack {
    fn name(&self) -> &'static str {
        "come_back"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["queue"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::ComeBack { .. }, .. })
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::ComeBack { after, side, .. }, .. } = n else { unreachable!() };
        let it = at.one(&Target::It);
        let clock = g.clock();
        g.need(Feature::Comebacks);
        Emitted::inline(vec![format!("self.comebacks.push(({clock} + {}, String::from({it}), {side}));", f(*after))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&ComeBack, "When a hooked perch reaches the hole, a new perch comes back after 3 seconds."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&ComeBack, "When a hooked perch reaches the hole, destroy it."));
    }

    #[test]
    fn queued_then_respawned() {
        let c = code("When a hooked perch reaches the hole, destroy it and a new perch comes in from the side after 3 seconds.");
        assert!(c.contains("self.comebacks.push((self.clock + 3.0, String::from(&n), true));"), "{c}");
        assert!(c.contains("cx.respawn(&name)"), "{c}");
        assert!(c.contains("self.clock += dt;"), "{c}");
    }
}
