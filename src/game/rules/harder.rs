//! "The harder the game, the stronger the fish pull, the less they bite, the louder the shout
//! must be, the sooner they get away and the shorter the time."
//!
//! Writes nothing itself: it tells the patterns it names to scale their sentence's numbers by
//! the chosen level ("The player chooses ...", `choose.rs`), each by its `[game.tuning]
//! level_*` table (easy, normal, hard):
//!
//! - "the stronger the fish pull": `hooked.rs`, the pull x `level_pull` and the burst x
//!   `level_burst`;
//! - "the less they bite": `bite.rs`, the chance x `level_bite`;
//! - "the louder the shout must be": `rise.rs` and `meter.rs`, the shout threshold x
//!   `level_shout`;
//! - "the sooner they get away": `at_bottom.rs`, the seconds x `level_escape`;
//! - "the shorter the time": `reaches.rs` ("when the time reaches N seconds") and `show.rs`
//!   ("{countdown N}"), N x `level_time`.

use super::{At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Standing;

pub struct HarderGame;

impl GamePattern for HarderGame {
    fn name(&self) -> &'static str {
        "harder"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["tables"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Harder(_)))
    }
    fn emit(&self, _variant: &str, _n: &Node, _g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        Emitted::to(Section::Update, Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    const GAME: &str = "The player chooses easy, normal or hard at the start with 1, 2 and 3.
        At the start, show \"TIME {countdown 90}\" at the top right.
        The hook hangs on a line from the hole, sinking at 1.5 units per second down to -8.
        The hook rises while the player shouts, at up to 8 units per second as loud as the shout.
        Each hooked pike follows its hook, pulling it down at 5 units per second in bursts.
        When a hooked pike stays at the bottom for 3 seconds, it gets away.
        When the time reaches 90 seconds, the player loses.\n";

    #[test]
    fn applies() {
        assert!(matches(&HarderGame, &format!("{GAME}The harder the game, the shorter the time.")));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&HarderGame, GAME));
    }

    #[test]
    fn every_named_number_scales_by_level() {
        let c = code(&format!(
            "{GAME}The harder the game, the stronger the fish pull, the louder the shout must be, the sooner they get away and the shorter the time."
        ));
        assert!(c.contains("cx.move_by(&hook, vec2(0.0, -[3.0, 5.0, 6.75][self.level] * dt));"), "{c}");
        assert!(c.contains("let surge = t % 1.6 < [0.675, 0.9, 1.125][self.level];"), "{c}");
        assert!(c.contains("if shout >= [0.15, 0.25, 0.35][self.level]"), "{c}");
        assert!(c.contains("if held >= [4.8, 3.0, 1.95][self.level] {"), "{c}");
        assert!(c.contains("let reached = self.clock >= [120.0, 90.0, 70.0][self.level];"), "{c}");
        assert!(c.contains("([120.0, 90.0, 70.0][self.level] - self.clock)"), "{c}");
    }

    #[test]
    fn without_it_the_numbers_stay() {
        let c = code(GAME);
        assert!(c.contains("cx.move_by(&hook, vec2(0.0, -5.0 * dt));"), "{c}");
        assert!(c.contains("let reached = self.clock >= 90.0;"), "{c}");
    }

    #[test]
    fn it_needs_a_choice_and_something_to_scale() {
        let e = crate::game::parser::parse("The harder the game, the shorter the time.\nWhen the time reaches 60 seconds, the player wins.", false)
            .unwrap_err();
        assert!(format!("{e:?}").contains("nothing chooses how hard"), "{e:?}");
        let e = crate::game::parser::parse(
            "The player chooses easy, normal or hard at the start with 1, 2 and 3.\nThe harder the game, the less they bite.",
            false,
        )
        .unwrap_err();
        assert!(format!("{e:?}").contains("nothing bites by chance"), "{e:?}");
    }
}
