//! "At the start, show the shout meter at the bottom." — a big bar of the microphone's
//! loudness (`cx.voice()`), drawn with Droid's screen rectangles (`cx.rect_at`) before the
//! playing check, like the other HUD lines.
//!
//! The bar is `[game.tuning] meter_size` of the screen (a third wide), centred at the top, the
//! middle or the bottom (at the left or right edge for the corners), in a `meter_border` frame,
//! with `meter_label` ("SHOUT!") above it. It fills with the loudness, its colour going from
//! `meter_low` (green) to `meter_mid` (yellow) at the shout threshold and on to `meter_high`
//! (red) at full voice; a `meter_mark` line marks where a shout starts to count (the threshold,
//! by level when "the louder the shout must be"). While a fish is on, the frame and the label
//! pulse toward `meter_high` `meter_pulse` times a second.

use super::{f, hooked_map, s, shout_threshold, At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::{Action, Place};

pub struct Meter;

fn v3([r, g, b]: [f32; 3]) -> String {
    format!("vec3({}, {}, {})", f(r), f(g), f(b))
}

impl GamePattern for Meter {
    fn name(&self) -> &'static str {
        "meter"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["bar"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Meter { .. }, .. })
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Meter { place }, .. } = n else { unreachable!() };
        let id = g.text_id();
        let (_, py) = g.slot(*place);
        let t = g.t;
        let [w, h] = t.meter_size;
        let b = t.meter_border;
        let x = match place {
            Place::TopLeft => t.top_left[0],
            Place::TopRight => 1.0 - t.top_left[0] - w,
            _ => (1.0 - w) / 2.0,
        };
        // the label above the bar, one bar height up; at the bottom the bar ends at the place
        let label_y = if *place == Place::Bottom { py - 2.0 * h } else { py };
        let y = label_y + h;
        let r = |v: f32| f((v * 10000.0).round() / 10000.0);
        let th = shout_threshold(g);
        let mut hud = g.comment();
        hud.push("let shout = cx.voice();".into());
        hud.push(format!("let th = {th};"));
        hud.push("let fill = shout.clamp(0.0, 1.0);".into());
        hud.push("// green when quiet, yellow where a shout starts to count, red at full voice".into());
        hud.push(format!(
            "let color = if fill < th {{ {low} + ({mid} - {low}) * (fill / th) }} else {{ {mid} + ({high} - {mid}) * ((fill - th) / (1.0 - th)) }};",
            low = v3(t.meter_low),
            mid = v3(t.meter_mid),
            high = v3(t.meter_high)
        ));
        let frame = if g.facts.hooks {
            let hooked = hooked_map(g);
            hud.push("// it pulses while a fish is on".into());
            hud.push(format!(
                "let pulse = if {hooked}.is_empty() {{ 0.0 }} else {{ 0.5 + 0.5 * (cx.time() * {}).sin() }};",
                f(t.meter_pulse * std::f32::consts::TAU)
            ));
            hud.push(format!("let frame = {} + ({} - {}) * pulse;", v3(t.meter_frame), v3(t.meter_high), v3(t.meter_frame)));
            "frame".to_string()
        } else {
            v3(t.meter_frame)
        };
        hud.push(format!("cx.rect_at(\"{id}_frame\", {}, {}, {}, {}, {frame});", r(x - b), r(y - b), r(w + 2.0 * b), r(h + 2.0 * b)));
        hud.push(format!("cx.rect_at(\"{id}_back\", {}, {}, {}, {}, {});", r(x), r(y), r(w), r(h), v3(t.meter_back)));
        hud.push(format!("cx.rect_at(\"{id}_fill\", {}, {}, {} * fill, {}, color);", r(x), r(y), r(w), r(h)));
        hud.push(format!(
            "cx.rect_at(\"{id}_mark\", {} + {} * th, {}, {}, {}, {});",
            r(x - b / 2.0),
            r(w),
            r(y - b),
            r(b),
            r(h + 2.0 * b),
            v3(t.meter_mark)
        ));
        hud.push(format!("cx.text_at(\"{id}\", {}, {}, {}, {});", s(&t.meter_label), r(x), r(label_y), f(t.meter_label_size)));
        hud.push(format!("cx.text_color(\"{id}\", {frame});"));
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
    fn the_meter_is_a_wide_bar_with_a_mark_where_a_shout_counts() {
        let c = code("At the start, show the shout meter at the bottom.");
        let update = &c[c.find("fn update").unwrap()..];
        assert!(update.contains("cx.voice()") && update.contains("\"SHOUT!\""), "{c}");
        // a third of the screen, centred, ending at the bottom place
        assert!(update.contains("cx.rect_at(\"text_0_fill\", 0.33, 0.86, 0.34 * fill, 0.06, color);"), "{c}");
        assert!(update.contains("cx.rect_at(\"text_0_mark\", 0.327 + 0.34 * th,"), "{c}");
        assert!(update.contains("let th = 0.25;"), "{c}");
        // no catches in this game: nothing to pulse for
        assert!(!update.contains("pulse"), "{c}");
    }

    #[test]
    fn it_pulses_while_a_fish_is_on() {
        let c = code("At the start, show the shout meter at the bottom.\nWhen a hook touches a fish and no fish is hooked, the fish bites.");
        assert!(c.contains("let pulse = if self.hooked.is_empty() { 0.0 } else {"), "{c}");
        assert!(c.contains("cx.text_color(\"text_0\", frame);"), "{c}");
    }
}
