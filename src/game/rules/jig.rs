//! "The player jigs the hook with the up arrow, w or space, lifting it N units[, and it
//! flutters back down]."
//!
//! A press of any of the keys twitches every hook up N units and notes when (the game's map of
//! jigged things, hook -> clock); for `[game.tuning] jig_secs` after a twitch the hook counts as
//! jigged ("a jigged hook", "while it is jigged"). The hook sinks back by its own "hangs ...
//! sinking" sentence; "it flutters back down" sways it from side to side on the way
//! (`flutter_sway` units at `flutter_rate` cycles per second, fading out over the jigged time),
//! and the line follows it. Variants: `twitch` (the lift is a quick rise over `jig_lift_secs`:
//! smooth on screen) and `snap` (the hook jumps up N units on the press: no in-between frames).

use super::{block, f, jigged_map, s, At, Emitted, GamePattern, Node, Section};
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
        let map = jigged_map(g);
        let clock = g.clock();
        let pressed = keys.iter().map(|k| format!("cx.key_pressed({})", s(k))).collect::<Vec<_>>().join(" || ");
        let mut l = vec![format!("let jig = {pressed};")];
        let mut each = Vec::new();
        let mut on_press = vec![format!("{map}.insert(n.clone(), {clock});")];
        if variant == "snap" {
            on_press.push(format!("cx.move_by(&n, vec2(0.0, {}));", f(*lift)));
        }
        each.extend(block("if jig", on_press));
        let mut since = vec![format!("let t = {clock} - t0;")];
        if variant == "twitch" {
            since.push("// the twitch: a quick rise".into());
            since.extend(block(
                format!("if t < {}", f(t.jig_lift_secs)),
                vec![format!("cx.move_by(&n, vec2(0.0, {} * dt));", f(lift / t.jig_lift_secs))],
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
        each.extend(block(format!("if let Some(t0) = {map}.get(&n).copied()"), since));
        l.extend(block(format!("for n in cx.find_tag({})", s(kind)), each));
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

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
        let c = code("The player jigs the hook with the up arrow, w or space, lifting it 0.8 units, and it flutters back down.");
        assert!(c.contains("let jig = cx.key_pressed(\"up\") || cx.key_pressed(\"w\") || cx.key_pressed(\"space\");"), "{c}");
        assert!(c.contains("self.jigged.insert(n.clone(), self.clock);"), "{c}");
        assert!(c.contains("it flutters back down"), "{c}");
    }

    #[test]
    fn without_flutter_there_is_no_sway() {
        let c = code("The player jigs the hook with w, lifting it 1 unit.");
        assert!(c.contains("cx.key_pressed(\"w\")") && !c.contains("flutters"), "{c}");
    }
}
