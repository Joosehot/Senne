//! "play the music \"C4 E4 G4 C5\" [at N beats per minute] [as a square wave]"
//! (defaults from `[game.tuning]`).

use super::{f, s, At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::Action;

pub struct Music;

impl GamePattern for Music {
    fn name(&self) -> &'static str {
        "music"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["play"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Music { .. }, .. })
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Music { notes, bpm, wave }, .. } = n else { unreachable!() };
        let bpm = bpm.unwrap_or(g.t.music_bpm);
        let wave = wave.clone().unwrap_or_else(|| g.t.music_wave.clone());
        Emitted::inline(vec![format!("cx.play_music({}, {}, {});", f(bpm), s(notes), s(&wave))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Music, "At the start, play the music \"C4 E4 G4 C5\" at 132 beats per minute as a square wave."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Music, "At the start, play the win sound."));
    }
}
