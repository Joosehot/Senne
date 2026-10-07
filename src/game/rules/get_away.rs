//! "it gets away" (when a hooked thing stays at the bottom or reaches the
//! hole) — it is unhooked, so it swims on by itself, and stays shy for
//! `[game.tuning] shy_secs`: a bite trigger ("and no fish is hooked") skips it
//! until then, so it can swim off the hook it just escaped.

use super::{f, hooked_map, shy_map, At, Emitted, GamePattern, Node, Target};
use crate::game::codegen::Gen;
use crate::game::parser::Action;

pub struct GetAway;

impl GamePattern for GetAway {
    fn name(&self) -> &'static str {
        "get_away"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["unhook"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::GetAway(_), .. })
    }
    fn emit(&self, _variant: &str, _n: &Node, g: &mut Gen, at: &At, _body: Vec<String>) -> Emitted {
        let it = at.one(&Target::It);
        let map = hooked_map(g);
        let shy = shy_map(g);
        let clock = g.clock();
        Emitted::inline(vec![
            format!("{map}.remove({it});"),
            format!("{shy}.insert(String::from({it}), {clock} + {});", f(g.t.shy_secs)),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    const SRC: &str = "The hook hangs on a line from the hole, sinking at 1.5 units per second down to -9.\n\
                       When a hook touches a fish and no fish is hooked, the fish bites.\n\
                       When a hooked pike stays at the bottom for 3 seconds, it gets away.";

    #[test]
    fn applies() {
        assert!(matches(&GetAway, "When a hooked pike reaches the hole, it gets away."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&GetAway, "When a hooked pike reaches the hole, destroy it."));
    }

    #[test]
    fn a_fish_that_got_away_is_shy() {
        let c = code(SRC);
        assert!(c.contains("self.shy.insert(String::from(&n), self.clock + 3.0);"), "{c}");
        assert!(c.contains("&& self.shy.get(b).map_or(true, |t| *t <= self.clock)"), "{c}");
    }
}
