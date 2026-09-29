//! "At the start, ..." — the actions run in `init` (at the start and after
//! every restart, on a fresh game).

use super::{At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Trigger;

pub struct Start;

impl GamePattern for Start {
    fn name(&self) -> &'static str {
        "start"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["init"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Trigger(Trigger::Start))
    }
    fn emit(&self, _variant: &str, _n: &Node, _g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        Emitted::to(Section::Init, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Start, "At the start, play the coin sound."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Start, "Every frame, play the coin sound."));
    }
}
