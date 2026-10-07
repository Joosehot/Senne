//! "Each hooked perch follows its hook[, fighting][, pulling it down at N
//! units per second[ in bursts]]."
//!
//! A hooked thing hangs `[game.tuning] hooked_drop` below the hook it bit and
//! stops its own movement. A pull drags the hook down every second, so a weak
//! shout loses ground; "in bursts" pulls for `burst_secs` and rests for
//! `rest_secs` (counted from the bite, so it surges as it bites), and a shout
//! in the rest gains ground. "fighting" makes it dart from side to side
//! (`thrash_width` at `thrash_rate`, `rest_thrash` of that while resting),
//! turn to face where it darts, and jerk the hook sideways (`hook_jerk` of the
//! dart; the line springs it back). "The harder the game, the stronger the fish pull" scales
//! the pull by `level_pull` and the burst by `level_burst`. Variants: `copy` (loop over a copy of
//! every catch) and `filtered` (copy only this kind's catches first).

use super::{block, f, hooked_at_map, hooked_map, maybe_leveled, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Standing;

pub struct Hooked;

impl GamePattern for Hooked {
    fn name(&self) -> &'static str {
        "hooked"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["copy", "filtered"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Hooked { .. }))
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Hooked { kind, pull, fight, bursts }) = n else { unreachable!() };
        let map = hooked_map(g);
        let t = g.t;
        let mut body = Vec::new();
        if *fight || *bursts {
            let since = hooked_at_map(g);
            let clock = g.clock();
            body.push(format!("let t = {clock} - {since}.get(&fish).copied().unwrap_or({clock});"));
        }
        let stronger = g.facts.harder.pull;
        if *bursts {
            let burst = maybe_leveled(g, t.burst_secs, stronger, t.level_burst);
            body.push(format!("let surge = t % {} < {burst};", f(((t.burst_secs + t.rest_secs) * 1000.0).round() / 1000.0)));
        }
        if let Some(p) = pull {
            let p = maybe_leveled(g, *p, stronger, t.level_pull);
            let pull_line = format!("cx.move_by(&hook, vec2(0.0, -{p} * dt));");
            if *bursts {
                body.extend(block("if surge", vec![pull_line]));
            } else {
                body.push(pull_line);
            }
        }
        let drop = f(t.hooked_drop);
        if *fight {
            let wild = if *bursts { format!("if surge {{ 1.0 }} else {{ {} }}", f(t.rest_thrash)) } else { "1.0".into() };
            body.push(format!("let wild = {wild};"));
            body.push(format!("let phase = t * {};", f(t.thrash_rate * std::f32::consts::TAU)));
            body.push(format!("let dart = phase.sin() * {} * wild;", f(t.thrash_width)));
            body.push("// it jerks the hook with it (the line springs it back)".into());
            body.push(format!(
                "cx.move_by(&hook, vec2(phase.cos() * {} * wild * dt, 0.0));",
                f(t.thrash_width * t.thrash_rate * std::f32::consts::TAU * t.hook_jerk)
            ));
            body.push(format!("let at = cx.pos(&hook) + vec2(dart, -{drop});"));
            body.push("cx.set_pos(&fish, at);".into());
            body.push("cx.set_flip_x(&fish, phase.cos() < 0.0);".into());
        } else {
            body.push(format!("let at = cx.pos(&hook) - vec2(0.0, {drop});"));
            body.push("cx.set_pos(&fish, at);".into());
        }
        body.push("cx.set_vel(&fish, Vec3::ZERO);".into());
        let mut l = Vec::new();
        if variant == "copy" {
            let mut inner = block(format!("if !cx.has_tag(&fish, {})", s(kind)), vec!["continue;".into()]);
            inner.extend(body);
            l.extend(block(format!("for (fish, hook) in {map}.clone()"), inner));
        } else {
            l.push(format!(
                "let caught: Vec<(String, String)> = {map}.iter().filter(|(n, _)| cx.has_tag(n, {})).map(|(n, h)| (n.clone(), h.clone())).collect();",
                s(kind)
            ));
            l.extend(block("for (fish, hook) in caught", body));
        }
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&Hooked, "Each hooked pike follows its hook, pulling it down at 3 units per second."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Hooked, "Each pike swims within 12 units at 3 units per second."));
    }

    #[test]
    fn the_pull_drags_the_hook_down() {
        let c = code("Each hooked pike follows its hook, pulling it down at 3 units per second.");
        assert!(c.contains("cx.move_by(&hook, vec2(0.0, -3.0 * dt));"), "{c}");
        assert!(c.contains("hooked: BTreeMap<String, String>"), "{c}");
        assert!(!c.contains("surge"), "{c}");
    }

    #[test]
    fn a_fight_surges_and_thrashes() {
        let c = code("Each hooked pike follows its hook, fighting and pulling it down at 5 units per second in bursts.");
        assert!(c.contains("let surge = t % 1.6 < 0.9;"), "{c}");
        assert!(c.contains("if surge {\n"), "{c}");
        assert!(c.contains("cx.set_flip_x(&fish, phase.cos() < 0.0);"), "{c}");
        assert!(c.contains("self.hooked_at.get(&fish)"), "{c}");
    }
}
