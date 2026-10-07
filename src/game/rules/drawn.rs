//! "Fish are drawn to a jigged hook within N units at N units per second."
//!
//! While a hook is jigged (and nothing is caught on it), every one of the kind that isn't hooked (or shy after getting away)
//! and is within N units of it turns toward it and swims at it, slowing in the last
//! `[game.tuning] draw_ease` units so it noses up to it. This runs after the kind's own
//! swimming sentence (write it after), so it wins while it lasts; once the hook sits still
//! longer than `jig_secs`, the fish lose interest and their own swimming takes them back.
//! Variants: `steer` (the velocity eases toward the hook at `swim_turn_rate`: a turn, not a
//! snap) and `direct` (the velocity points straight at it at once).

use super::{block, f, hooked_map, is_jigged, s, shy_map, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Standing;

pub struct Drawn;

impl GamePattern for Drawn {
    fn name(&self) -> &'static str {
        "drawn"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["steer", "direct"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Drawn { .. }))
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Drawn { kind, target, within, speed }) = n else { unreachable!() };
        let t = g.t;
        let mut jigged = is_jigged(g, "h");
        if g.facts.hooks {
            // a hook with a catch on it draws no more
            let map = hooked_map(g);
            jigged.push_str(&format!(" && !{map}.values().any(|k| k == h)"));
        }
        let mut l = vec![format!(
            "let lure = cx.find_tag({}).into_iter().find(|h| {jigged}).map(|h| cx.pos(&h));",
            s(target)
        )];
        let mut each = Vec::new();
        if g.facts.hooks {
            let map = hooked_map(g);
            each.extend(block(format!("if {map}.contains_key(&n)"), vec!["continue;".into()]));
        }
        if g.facts.shy {
            let shy = shy_map(g);
            let clock = g.clock();
            each.extend(block(format!("if {shy}.get(&n).is_some_and(|t| *t > {clock})"), vec!["continue;".into()]));
        }
        each.push("let p = cx.pos(&n);".into());
        each.push("let d = h - p;".into());
        let mut near = vec![format!(
            "let want = d.normalize_or_zero() * {} * (d.length() / {}).min(1.0);",
            f(*speed),
            f(t.draw_ease)
        )];
        if variant == "steer" {
            near.push("let v = cx.vel(&n);".into());
            near.push(format!("let v = v + (want - v) * (1.0 - (-{} * dt).exp());", f(t.swim_turn_rate)));
        } else {
            near.push("let v = want;".into());
        }
        near.push("cx.set_vel(&n, vec2(v.x, v.y));".into());
        near.push("// it turns to face the lure".into());
        near.extend(block(format!("if d.x.abs() > {}", f(t.face_speed)), vec!["cx.set_flip_x(&n, d.x < 0.0);".into()]));
        each.extend(block(format!("if d.length() < {}", f(*within)), near));
        l.extend(block("if let Some(h) = lure", block(format!("for n in cx.find_tag({})", s(kind)), each)));
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    const JIG: &str = "The player jigs the hook with the up arrow, lifting it 0.8 units.\n";

    #[test]
    fn applies() {
        assert!(matches(&Drawn, &format!("{JIG}Fish are drawn to a jigged hook within 5 units at 2 units per second.")));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Drawn, "Each perch swims within 15 units at 2 units per second."));
    }

    #[test]
    fn only_a_jigged_hook_draws_them() {
        let c = code(&format!("{JIG}Fish are drawn to a jigged hook within 5 units at 2 units per second."));
        assert!(c.contains("self.jigged.get(h).is_some_and(|t| self.clock - *t < "), "{c}");
        assert!(c.contains("if d.length() < 5.0"), "{c}");
    }

    #[test]
    fn a_hook_with_a_catch_draws_no_more() {
        let c = code(&format!(
            "{JIG}Fish are drawn to a jigged hook within 5 units at 2 units per second.
             When a jigged hook touches a fish and no fish is hooked, the fish bites."
        ));
        assert!(c.contains("&& !self.hooked.values().any(|k| k == h)"), "{c}");
    }

    #[test]
    fn nothing_jigs_the_hook_is_an_error() {
        let e = crate::game::parser::parse("Fish are drawn to a jigged hook within 5 units at 2 units per second.", false).unwrap_err();
        assert!(format!("{e:?}").contains("nothing jigs a hook"), "{e:?}");
    }
}
