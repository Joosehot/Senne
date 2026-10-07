//! "it bites" / "the fish bites[ with a chance of ...]" (in "When a hook touches a fish, ..." or
//! "When the hook is jigged within 2 units of a fish, ...") — the fish is hooked on the
//! trigger's other thing: the game's map of catches (hooked thing -> hook) gets the pair. "and
//! no fish is hooked" on the trigger keeps it to one catch at a time.
//!
//! "with a chance of 40 percent for a perch, 20 percent for a roach, 5 percent for a pike and 60
//! percent for a lunging pike, more for a steady rhythm, less for frantic jigging and less the
//! farther it is": once per jig, the fish bites with that chance (`Ctx::random`, repeatable with
//! Droid's `DROID_SEED`), and the trigger's other actions only run when it does. The chance is
//! the first that fits the fish (a lunging one's before its kind's own, which counts while its
//! swim sentence has it lunging), 0 for a fish none names; then x `[game.tuning] bite_steady` on
//! a steady rhythm, x `bite_frantic` when the jigging is frantic (the trigger measures both),
//! x how near it is (1 at the lure, 0 at the edge) and, when "the harder the game, ... the less
//! they bite", x `level_bite` of the chosen level. A still lure is never jigged, so it gets no
//! bites at all.

use super::{f, hooked_at_map, hooked_map, level, swim_map, At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::{Action, Chance};

pub struct Bite;

impl GamePattern for Bite {
    fn name(&self) -> &'static str {
        "bite"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["map"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Bite(..), .. })
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Bite(t, chance), .. } = n else { unreachable!() };
        let fish = at.one(t);
        let hook = at.named.iter().map(|(_, e)| e.clone()).find(|e| *e != fish).expect("parser: two things touch");
        let mut l = Vec::new();
        if let Some(ch) = chance {
            l.extend(by_chance(ch, &fish, g));
        }
        let map = hooked_map(g);
        let at = hooked_at_map(g);
        let clock = g.clock();
        l.push(format!("{map}.insert({fish}.to_string(), {hook}.to_string());"));
        l.push(format!("{at}.insert({fish}.to_string(), {clock});"));
        Emitted::inline(l)
    }
}

/// The roll: `continue`s the trigger's loop over the fish near the lure when it doesn't bite.
fn by_chance(ch: &Chance, fish: &str, g: &mut Gen) -> Vec<String> {
    let t = g.t;
    let mut arms: Vec<(String, f32)> = Vec::new();
    let mut rest = None;
    // a lunging one's chance first, then the kind's own, in the order written
    let ordered = ch.odds.iter().filter(|o| o.lunging).chain(ch.odds.iter().filter(|o| !o.lunging));
    for o in ordered {
        let p = (o.percent / 100.0 * 10000.0).round() / 10000.0;
        match &o.kind {
            None => {
                rest.get_or_insert(p);
            }
            Some(k) if o.lunging => {
                let swim = swim_map(g, k);
                arms.push((format!("cx.has_tag({fish}, {k:?}) && {swim}.get({fish}).is_some_and(|s| s.4 > 0.0)"), p));
            }
            Some(k) => arms.push((format!("cx.has_tag({fish}, {k:?})"), p)),
        }
    }
    let mut l = vec!["// it bites by chance, once per jig".to_string()];
    let last = f(rest.unwrap_or(0.0));
    if arms.is_empty() {
        l.push(format!("let chance = {last};"));
    } else {
        let mut e = String::from("let chance = ");
        for (i, (cond, p)) in arms.iter().enumerate() {
            if i > 0 {
                e.push_str(" else ");
            }
            e.push_str(&format!("if {cond} {{ {} }}", f(*p)));
        }
        e.push_str(&format!(" else {{ {last} }};"));
        l.push(e);
    }
    if ch.steady {
        l.push(format!("let chance = if steady {{ chance * {} }} else {{ chance }};", f(t.bite_steady)));
    }
    if ch.frantic {
        l.push(format!("let chance = if frantic {{ chance * {} }} else {{ chance }};", f(t.bite_frantic)));
    }
    if ch.farther {
        l.push("let chance = chance * near;".into());
    }
    if g.facts.levels && g.facts.harder.bite {
        let table = t.level_bite;
        let lv = level(g);
        l.push(format!("let chance = chance * [{}, {}, {}][{lv}];", f(table[0]), f(table[1]), f(table[2])));
    }
    l.push("if cx.random(0.0, 1.0) >= chance {".into());
    l.push("    continue;".into());
    l.push("}".into());
    l
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    const JIG: &str = "The player jigs the hook with the up arrow, lifting it 0.8 units.\n";

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
        assert!(!c.contains("cx.random"), "{c}");
    }

    #[test]
    fn each_kind_its_own_chance_by_rhythm_and_distance() {
        let c = code(&format!(
            "{JIG}When the hook is jigged within 2 units of a fish and no fish is hooked, the fish bites with a chance of \
             40 percent for a perch, 20 percent for a roach and 5 percent for a pike, more for a steady rhythm, \
             less for frantic jigging and less the farther it is, squash it."
        ));
        assert!(
            c.contains(
                "let chance = if cx.has_tag(n.as_str(), \"perch\") { 0.4 } else if cx.has_tag(n.as_str(), \"roach\") { 0.2 } \
                 else if cx.has_tag(n.as_str(), \"pike\") { 0.05 } else { 0.0 };"
            ),
            "{c}"
        );
        assert!(c.contains("let chance = if steady { chance * 1.6 } else { chance };"), "{c}");
        assert!(c.contains("let chance = if frantic { chance * 0.15 } else { chance };"), "{c}");
        assert!(c.contains("let chance = chance * near;"), "{c}");
        assert!(c.contains("if cx.random(0.0, 1.0) >= chance {"), "{c}");
        // the roll comes before the catch and the other actions
        let roll = c.find("cx.random(0.0, 1.0)").unwrap();
        assert!(roll < c.find("self.hooked.insert(").unwrap() && roll < c.find("cx.squash(").unwrap(), "{c}");
    }

    #[test]
    fn a_lunging_pike_counts_before_a_pike() {
        let c = code(&format!(
            "{JIG}Each pike swims within 15 units at 1 unit per second, lurking and lunging at the hook within 6 units at 9 units per second.\n\
             When the hook is jigged within 2 units of a fish, the fish bites with a chance of 5 percent for a pike and 60 percent for a lunging pike."
        ));
        assert!(
            c.contains(
                "if cx.has_tag(n.as_str(), \"pike\") && self.swim_pike.get(n.as_str()).is_some_and(|s| s.4 > 0.0) { 0.6 } \
                 else if cx.has_tag(n.as_str(), \"pike\") { 0.05 } else { 0.0 };"
            ),
            "{c}"
        );
    }

    #[test]
    fn one_chance_for_any_fish() {
        let c = code(&format!("{JIG}When the hook is jigged within 2 units of a fish, it bites with a chance per jig of 30 percent."));
        assert!(c.contains("let chance = 0.3;"), "{c}");
    }

    #[test]
    fn a_lunging_pike_needs_a_pike_that_lunges() {
        let e = crate::game::parser::parse(
            &format!("{JIG}When the hook is jigged within 2 units of a fish, the fish bites with a chance of 60 percent for a lunging pike."),
            false,
        )
        .unwrap_err();
        assert!(format!("{e:?}").contains("no pike lunges"), "{e:?}");
    }

    #[test]
    fn a_chance_needs_a_jig() {
        let e = crate::game::parser::parse("When a hook touches a fish, the fish bites with a chance of 30 percent.", false).unwrap_err();
        assert!(format!("{e:?}").contains("once per jig"), "{e:?}");
    }
}
