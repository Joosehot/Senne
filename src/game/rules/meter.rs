//! "At the start, show the shout meter at the bottom." — a HUD bar of the
//! microphone's loudness (`cx.voice()`), `[game.tuning] meter_width` cells
//! after `meter_label`, coloured `meter_on` while it counts as a shout and
//! `meter_off` below. Kept up to date before the playing check, like the
//! other HUD lines. Variants: `every_frame` (rewrite it each frame) and
//! `on_change` (only when the bar's length changes).

use super::{block, f, s, At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::Action;

pub struct Meter;

impl GamePattern for Meter {
    fn name(&self) -> &'static str {
        "meter"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["every_frame", "on_change"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Meter { .. }, .. })
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Meter { place }, .. } = n else { unreachable!() };
        let id = g.text_id();
        let (x, y) = g.slot(*place);
        let t = g.t;
        let (cells, size, th) = (t.meter_width, f(t.hud_size), f(t.shout_threshold));
        let ([r1, g1, b1], [r0, g0, b0]) = (t.meter_on, t.meter_off);
        let label = s(&format!("{} {{}}{{}}", t.meter_label));
        let draw = vec![
            format!(
                "cx.text_at(\"{id}\", &format!({label}, \"#\".repeat(bars), \"-\".repeat({cells} - bars)), {}, {}, {size});",
                f(x),
                f(y)
            ),
            format!(
                "cx.text_color(\"{id}\", if shout >= {th} {{ vec3({}, {}, {}) }} else {{ vec3({}, {}, {}) }});",
                f(r1),
                f(g1),
                f(b1),
                f(r0),
                f(g0),
                f(b0)
            ),
        ];
        let mut hud = g.comment();
        hud.push("let shout = cx.voice();".into());
        hud.push(format!("let bars = ((shout * {}).round() as usize).min({cells});", f(cells as f32)));
        if variant == "on_change" {
            let shown = g.fresh(&format!("{id}_bars"), "usize", "usize::MAX");
            let mut body = draw;
            body.push(format!("{shown} = bars;"));
            hud.extend(block(format!("if bars != {shown}"), body));
        } else {
            hud.extend(draw);
        }
        g.hud.extend(hud);
        Emitted::inline(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&Meter, "At the start, show the shout meter at the bottom."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Meter, "At the start, show \"SCORE {score}\" at the bottom."));
    }

    #[test]
    fn the_meter_is_a_live_hud_line() {
        let c = code("At the start, show the shout meter at the bottom.");
        let update = &c[c.find("fn update").unwrap()..];
        assert!(update.contains("cx.voice()") && update.contains("\"SHOUT {}{}\""), "{c}");
    }
}
