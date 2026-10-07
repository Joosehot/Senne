//! "The player jigs the hook with the up arrow or w, lifting it N units[, and it flutters back
//! down]."
//!
//! A jig only twitches the lure in a short band above where it rests: each press lifts it at
//! most to N units above its resting depth (never higher, so presses can't add up), and it
//! falls back to that depth. The resting depth is where the hook hangs: the lowest it has been
//! since the jigging began (so it follows a hook that sinks, and resets once the hook has been
//! still for `[game.tuning] jig_secs`). A press while the hook is above the band (reeled up)
//! does nothing, and nothing happens while a catch is on the hook (only the shout reels it).
//!
//! A twitch notes when (the game's map of jigged things, hook -> clock); for `jig_secs` after
//! it the hook counts as jigged ("a jigged hook", "while it is jigged"). The hook sinks back by
//! its own "hangs ... sinking" sentence; without one it settles back over `jig_settle_secs`.
//! "it flutters back down" sways it from side to side on the way (`flutter_sway` units at
//! `flutter_rate` cycles per second, fading out over the jigged time), and the line follows it.
//! Variants: `twitch` (the lift is a quick rise over `jig_lift_secs`: smooth on screen) and
//! `snap` (the hook jumps up on the press: no in-between frames).

use super::{block, f, hooked_map, is_jigged, jigged_map, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Standing;

pub struct Jig;

impl GamePattern for Jig {
    fn name(&self) -> &'static str {
        "jig"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["twitch", "snap"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Jig { .. }))
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Jig { kind, keys, lift, flutter }) = n else { unreachable!() };
        let t = g.t;
        let lift_s = f(*lift);
        let jigging = is_jigged(g, "&n");
        let map = jigged_map(g);
        let rest = g.fresh(&format!("rest_{kind}"), "BTreeMap<String, f32>", "BTreeMap::new()");
        let clock = g.clock();
        let pressed = keys.iter().map(|k| format!("cx.key_pressed({})", s(k))).collect::<Vec<_>>().join(" || ");
        let mut l = vec![format!("let jig = {pressed};")];
        let mut each = vec![
            "let y = cx.pos(&n).y;".to_string(),
            "// where it rests: the lowest it has been since the jigging began".into(),
            format!("let rest = match {rest}.get(&n) {{"),
            format!("    Some(r) if {jigging} => r.min(y),"),
            "    _ => y,".into(),
            "};".into(),
            format!("{rest}.insert(n.clone(), rest);"),
        ];
        if g.facts.hook_depth.is_none() {
            each.push("// nothing sinks it: it settles back to where it rests".into());
            each.extend(block(
                "if y > rest",
                vec![format!("cx.move_by(&n, vec2(0.0, -(y - rest).min({} * dt)));", f(lift / t.jig_settle_secs))],
            ));
        }
        // a twitch only within the band above its rest, and never with a catch on the hook
        let mut can = format!("jig && y - rest < {lift_s}");
        if g.facts.hooks {
            let hooked = hooked_map(g);
            can.push_str(&format!(" && !{hooked}.values().any(|h| *h == n)"));
        }
        let mut on_press = vec![format!("{map}.insert(n.clone(), {clock});")];
        if variant == "snap" {
            on_press.push("// up to the top of the band, no higher".into());
            on_press.push(format!("cx.move_by(&n, vec2(0.0, rest + {lift_s} - y));"));
        }
        each.extend(block(format!("if {can}"), on_press));
        let mut since = vec![format!("let t = {clock} - t0;")];
        if variant == "twitch" {
            since.push("// the twitch: a quick rise to the top of the band, no higher".into());
            since.extend(block(
                format!("if t < {}", f(t.jig_lift_secs)),
                vec![
                    format!("let room = (rest + {lift_s} - cx.pos(&n).y).max(0.0);"),
                    format!("cx.move_by(&n, vec2(0.0, ({} * dt).min(room)));", f(lift / t.jig_lift_secs)),
                ],
            ));
        }
        if *flutter {
            let w = t.flutter_rate * std::f32::consts::TAU;
            since.push("// it flutters back down: a sway from side to side that fades".into());
            since.extend(block(
                format!("if t < {}", f(t.jig_secs)),
                vec![
                    format!("let fade = 1.0 - t / {};", f(t.jig_secs)),
                    format!("cx.move_by(&n, vec2((t * {}).cos() * {} * fade * dt, 0.0));", f(w), f(t.flutter_sway * w)),
                ],
            ));
        }
        if variant == "twitch" || *flutter {
            each.extend(block(format!("if let Some(t0) = {map}.get(&n).copied()"), since));
        }
        l.extend(block(format!("for n in cx.find_tag({})", s(kind)), each));
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    const HANG: &str = "The hook hangs on a line from the hole, sinking at 1.5 units per second down to -8.\n";

    #[test]
    fn applies() {
        assert!(matches(&Jig, "The player jigs the hook with the up arrow, lifting it 0.8 units."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Jig, "The hook rises while the player holds space, at 8 units per second."));
    }

    #[test]
    fn every_key_twitches_the_hook_and_it_flutters() {
        let c = code("The player jigs the hook with the up arrow or w, lifting it 0.8 units, and it flutters back down.");
        assert!(c.contains("let jig = cx.key_pressed(\"up\") || cx.key_pressed(\"w\");"), "{c}");
        assert!(c.contains("self.jigged.insert(n.clone(), self.clock);"), "{c}");
        assert!(c.contains("it flutters back down"), "{c}");
    }

    #[test]
    fn without_flutter_there_is_no_sway() {
        let c = code("The player jigs the hook with w, lifting it 1 unit.");
        assert!(c.contains("cx.key_pressed(\"w\")") && !c.contains("flutters"), "{c}");
    }

    #[test]
    fn a_jig_never_lifts_the_hook_above_the_band_over_its_rest() {
        let c = code(&format!("{HANG}The player jigs the hook with w, lifting it 0.8 units."));
        // the rest follows the hook down while it is jigged, and starts again where it is after
        assert!(c.contains("Some(r) if self.jigged.get(&n).is_some_and(|t| self.clock - *t < 2.5) => r.min(y),"), "{c}");
        // a press inside the band only, lifting it at most to the top of the band
        assert!(c.contains("if jig && y - rest < 0.8 {"), "{c}");
        assert!(c.contains("rest + 0.8 - "), "{c}");
        // it sinks by the hang sentence: no settling of its own
        assert!(!c.contains("settles back"), "{c}");
    }

    #[test]
    fn nothing_sinks_the_hook_so_it_settles_back_itself() {
        let c = code("The player jigs the hook with w, lifting it 0.8 units.");
        assert!(c.contains("it settles back to where it rests"), "{c}");
    }

    #[test]
    fn a_hook_with_a_catch_on_it_is_not_jigged() {
        let c = code(&format!(
            "{HANG}The player jigs the hook with w, lifting it 0.8 units.\n\
             When the hook is jigged within 2 units of a fish and no fish is hooked, the fish bites."
        ));
        assert!(c.contains("if jig && y - rest < 0.8 && !self.hooked.values().any(|h| *h == n) {"), "{c}");
    }
}
