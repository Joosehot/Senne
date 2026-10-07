//! "When a shot hits an alien, ..." / "When a bomb hits the player, ..." —
//! in `on_collision`, which sees every pair both ways round, so one ordered
//! check is enough. "it" is the second thing (the first when the second is
//! the player); "the shot" / "the alien" name each side; "both" is both.
//! "and no coins are left" / "and no fish is hooked" add a condition; "a jigged hook" only
//! counts while the hook is jigged (see `jig.rs`).

use super::{block, hooked_map, is_jigged, s, shy_map, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::{Cond, Party, Trigger};

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
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        let Node::Trigger(Trigger::Hit { a, b, cond, jigged }) = n else { unreachable!() };
        let second = match b {
            Party::Player => "b == \"player\"".to_string(),
            Party::Thing(t) => format!("cx.has_tag(b, {})", s(t)),
        };
        let mut extra = match cond {
            None => String::new(),
            Some(Cond::NoneLeft(t)) => format!(" && cx.count_tag({}) == 0", s(t)),
            Some(Cond::NoneHooked(t)) => {
                let map = hooked_map(g);
                let mut c = format!(" && !{map}.keys().any(|n| cx.has_tag(n, {}))", s(t));
                if g.facts.shy {
                    // one that just got away doesn't bite again yet
                    let shy = shy_map(g);
                    let clock = g.clock();
                    c.push_str(&format!(" && {shy}.get(b).map_or(true, |t| *t <= {clock})"));
                }
                c
            }
        };
        if *jigged {
            // "a jigged hook": only while it was twitched a moment ago
            extra = format!(" && {}{extra}", is_jigged(g, "a"));
        }
        Emitted::to(Section::Collide, block(format!("if cx.has_tag(a, {}) && {second}{extra}", s(a)), body))
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
    fn a_jigged_hook_only_counts_while_it_is_jigged() {
        let c = crate::game::rules::test_util::code(
            "The player jigs the hook with the up arrow, lifting it 0.8 units.\n\
             When a jigged hook touches a fish and no fish is hooked, the fish bites.",
        );
        assert!(c.contains("cx.has_tag(a, \"hook\") && cx.has_tag(b, \"fish\") && self.jigged.get(a).is_some_and("), "{c}");
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Hit, "When the player touches an alien, the player loses."));
    }
}
