//! "it bites" / "the fish bites" (in "When a hook touches a fish, ...") — the
//! fish is hooked on the trigger's other thing: the game's map of catches
//! (hooked thing -> hook) gets the pair. "and no fish is hooked" on the
//! trigger keeps it to one catch at a time.

use super::{hooked_at_map, hooked_map, At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::Action;

pub struct Bite;

impl GamePattern for Bite {
    fn name(&self) -> &'static str {
        "bite"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["map"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Bite(_), .. })
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Bite(t), .. } = n else { unreachable!() };
        let fish = at.one(t);
        let hook = at.named.iter().map(|(_, e)| e.clone()).find(|e| *e != fish).expect("parser: two things touch");
        let map = hooked_map(g);
        let at = hooked_at_map(g);
        let clock = g.clock();
        Emitted::inline(vec![
            format!("{map}.insert({fish}.to_string(), {hook}.to_string());"),
            format!("{at}.insert({fish}.to_string(), {clock});"),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&Bite, "When a hook touches a fish and no fish is hooked, the fish bites."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Bite, "When a hook touches a fish, destroy it."));
    }

    #[test]
    fn one_catch_at_a_time() {
        let c = code("When a hook touches a fish and no fish is hooked, it bites.");
        assert!(c.contains("!self.hooked.keys().any(|n| cx.has_tag(n, \"fish\"))"), "{c}");
        assert!(c.contains("self.hooked.insert(b.to_string(), a.to_string());"), "{c}");
    }
}
