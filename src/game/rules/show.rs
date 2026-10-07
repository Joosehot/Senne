//! "show \"COINS {score} / 14\" [at the top left | ...] [in \"#ffffff\"]"
//!
//! Placeholders: {score} {lives} {wave} {time} {countdown N} {left coin}.
//! Shown "At the start" with placeholders, the text is a HUD line kept up to
//! date every frame (before the playing check, so the last values show on
//! the end screen). Variants for that: `every_frame` (rewrite it each frame)
//! and `on_change` (remember the last text, rewrite only when it changes).
//! Anywhere else it is shown once, when the trigger fires (`once`).

use super::{block, f, maybe_leveled, s, At, Emitted, Facts, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::{Action, Part};

pub struct Show;

/// The text as a Rust expression of type `&str`-compatible (a literal or `&format!(..)`),
/// and the owned `String` form for `on_change`.
fn text_expr(g: &mut Gen, parts: &[Part]) -> (String, String) {
    let mut fmt = String::new();
    let mut args = Vec::new();
    for p in parts {
        let arg = match p {
            Part::Text(t) => {
                fmt.push_str(&t.replace('{', "{{").replace('}', "}}"));
                continue;
            }
            Part::Score => g.field("score", "i64", "0"),
            Part::Lives => g.field("lives", "i64", &g.t.lives.to_string()),
            Part::Wave => g.field("wave", "i64", "1"),
            Part::Time => format!("{} as i64", g.clock()),
            Part::Countdown(n) => {
                // "the harder the game, ... the shorter the time": N x level_time
                let (on, table) = (g.facts.harder.time, g.t.level_time);
                let from = maybe_leveled(g, *n, on, table);
                format!("({from} - {}).max(0.0).ceil() as i64", g.clock())
            }
            Part::Left(t) => format!("cx.count_tag({})", s(t)),
        };
        fmt.push_str("{}");
        args.push(arg);
    }
    if args.is_empty() {
        let lit = s(&fmt.replace("{{", "{").replace("}}", "}"));
        (lit.clone(), format!("{lit}.to_string()"))
    } else {
        let call = format!("format!({}, {})", s(&fmt), args.join(", "));
        (format!("&{call}"), call)
    }
}

impl GamePattern for Show {
    fn name(&self) -> &'static str {
        "show"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["once", "every_frame", "on_change"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Show { .. }, .. })
    }
    fn variants(&self, n: &Node, _f: &Facts) -> Vec<&'static str> {
        match n {
            Node::Action { act: Action::Show { live: true, .. }, .. } => vec!["every_frame", "on_change"],
            _ => vec!["once"],
        }
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Show { parts, place, color, secs, .. }, .. } = n else { unreachable!() };
        if let Some(secs) = secs {
            let (id, until) = g.flash(*place);
            let (x, y) = g.place_xy(*place);
            let (text, _) = text_expr(g, parts);
            let mut l = vec![format!("cx.text_at(\"{id}\", {text}, {}, {}, {});", f(x), f(y), f(g.t.flash_size))];
            if let Some([r, gr, b]) = color {
                l.push(format!("cx.text_color(\"{id}\", vec3({}, {}, {}));", f(*r), f(*gr), f(*b)));
            }
            let clock = g.clock();
            l.push(format!("{until} = {clock} + {};", f(*secs)));
            return Emitted::inline(l);
        }
        let id = g.text_id();
        let (x, y) = g.slot(*place);
        let size = f(g.t.hud_size);
        let (text, owned) = text_expr(g, parts);
        let put = |t: &str| format!("cx.text_at(\"{id}\", {t}, {}, {}, {size});", f(x), f(y));
        let mut l = vec![put(&text)];
        if let Some([r, gr, b]) = color {
            l.push(format!("cx.text_color(\"{id}\", vec3({}, {}, {}));", f(*r), f(*gr), f(*b)));
        }
        let mut hud = g.comment();
        match variant {
            "every_frame" => hud.push(put(&text)),
            "on_change" => {
                let shown = g.fresh(&format!("{id}_shown"), "String", "String::new()");
                hud.push(format!("let text = {owned};"));
                hud.extend(block(format!("if text != {shown}"), vec![put("&text"), format!("{shown} = text;")]));
            }
            _ => {}
        }
        if variant != "once" {
            g.hud.extend(hud);
        }
        Emitted::inline(l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&Show, "At the start, show \"COINS {score} / 14\" at the top left."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Show, "At the start, play the coin sound."));
    }

    #[test]
    fn a_message_for_a_while_goes_again() {
        let c = code("When the player touches a coin, show \"NICE\" in the middle for 1.5 seconds.");
        assert!(c.contains("self.flash_middle = self.clock + 1.5;"), "{c}");
        assert!(c.contains("cx.remove_text(\"flash_middle\");"), "{c}");
    }

    #[test]
    fn hud_lines_stay_live() {
        let c = code("At the start, show \"SCORE {score}\".");
        assert!(c.contains("format!(\"SCORE {}\", self.score)"), "{c}");
        let update = &c[c.find("fn update").unwrap()..];
        assert!(update.contains("cx.text_at(\"text_0\""), "{c}");
    }
}
