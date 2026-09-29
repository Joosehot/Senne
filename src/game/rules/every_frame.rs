//! "Every frame, ..." — the actions run in `update` while playing.

use super::{At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Trigger;

pub struct EveryFrame;

impl GamePattern for EveryFrame {
    fn name(&self) -> &'static str {
        "every_frame"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["update"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Trigger(Trigger::EveryFrame))
    }
    fn emit(&self, _variant: &str, _n: &Node, _g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        Emitted::to(Section::Update, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&EveryFrame, "Every frame, add 1 to the score."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&EveryFrame, "Every 2 seconds, add 1 to the score."));
    }
}
