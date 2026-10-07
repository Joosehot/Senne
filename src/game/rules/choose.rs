//! "The player chooses easy, normal or hard at the start with 1, 2 and 3[, or normal with space
//! or after 5 seconds]."
//!
//! The game holds (its clock stopped, nothing moves) and shows "1 EASY  2 NORMAL  3 HARD" at
//! `[game.tuning] choose_at` until a key picks the level (`self.level`: 0 easy, 1 normal, 2
//! hard), or the default's key or wait does. What a level changes is said by "The harder the
//! game, ..." (`harder.rs`). A restart asks again.

use super::{block, f, level, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::{Standing, LEVELS};

pub struct Choose;

impl GamePattern for Choose {
    fn name(&self) -> &'static str {
        "choose"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["prompt"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Choose { .. }))
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Choose { keys, default }) = n else { unreachable!() };
        let t = g.t;
        let lv = level(g);
        let choosing = g.field("choosing", "bool", "true");
        let mut pick = String::from("let pick = ");
        for (i, k) in keys.iter().enumerate() {
            pick.push_str(&format!("if cx.key_pressed({}) {{ Some({i}) }} else ", s(k)));
        }
        let mut body = Vec::new();
        if let Some((d, key, wait)) = default {
            if let Some(k) = key {
                pick.push_str(&format!("if cx.key_pressed({}) {{ Some({d}) }} else ", s(k)));
            }
            if let Some(w) = wait {
                let waited = g.field("choose_secs", "f32", "0.0");
                body.push(format!("{waited} += dt;"));
                pick.push_str(&format!("if {waited} >= {} {{ Some({d}) }} else ", f(*w)));
            }
        }
        pick.push_str("{ None };");
        body.push(pick);
        let prompt: Vec<String> = keys.iter().zip(LEVELS).map(|(k, l)| format!("{} {}", k.to_uppercase(), l.to_uppercase())).collect();
        let [x, y] = t.choose_at;
        body.extend(block(
            "match pick",
            vec![
                "Some(l) => {".into(),
                format!("    {lv} = l;"),
                format!("    {choosing} = false;"),
                "    cx.remove_text(\"choose\");".into(),
                "}".into(),
                "None => {".into(),
                format!("    cx.text_at(\"choose\", {}, {}, {}, {});", s(&prompt.join("  ")), f(x), f(y), f(t.choose_size)),
                "    return;".into(),
                "}".into(),
            ],
        ));
        let mut l = vec!["// the game waits until the player picks a level".to_string()];
        l.extend(block(format!("if {choosing}"), body));
        Emitted::to(Section::Gate, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&Choose, "The player chooses easy, normal or hard at the start with 1, 2 and 3."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Choose, "When the player presses r, restart the game."));
    }

    #[test]
    fn the_game_waits_for_a_level() {
        let c = code("The player chooses easy, normal or hard at the start with 1, 2 and 3, or normal with space or after 5 seconds.\nWhen the time reaches 60 seconds, the player wins.");
        assert!(c.contains("level: usize,") && c.contains("level: 1,"), "{c}");
        assert!(
            c.contains(
                "let pick = if cx.key_pressed(\"1\") { Some(0) } else if cx.key_pressed(\"2\") { Some(1) } else if cx.key_pressed(\"3\") { Some(2) } \
                 else if cx.key_pressed(\"space\") { Some(1) } else if self.choose_secs >= 5.0 { Some(1) } else { None };"
            ),
            "{c}"
        );
        assert!(c.contains("\"1 EASY  2 NORMAL  3 HARD\""), "{c}");
        // it holds the game before the clock moves
        assert!(c.find("if self.choosing {").unwrap() < c.find("self.clock += dt;").unwrap(), "{c}");
    }

    #[test]
    fn a_key_picks_one_level_only() {
        let e = crate::game::parser::parse("The player chooses easy, normal or hard at the start with 1, 1 and 3.", false).unwrap_err();
        assert!(format!("{e:?}").contains("picks two levels"), "{e:?}");
    }
}
