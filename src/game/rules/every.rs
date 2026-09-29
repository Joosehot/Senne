//! "Every N seconds, ..." — a timer field counts down while playing; the
//! first time is N seconds in, and leftover time carries over so the rhythm
//! does not drift.

use super::{block, f, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Trigger;

pub struct Every;

impl GamePattern for Every {
    fn name(&self) -> &'static str {
        "every"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["countdown"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Trigger(Trigger::Every(_)))
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        let Node::Trigger(Trigger::Every(period)) = n else { unreachable!() };
        let timer = g.fresh("timer", "f32", &f(*period));
        let mut inner = vec![format!("{timer} += {};", f(*period))];
        inner.extend(body);
        let mut l = vec![format!("{timer} -= dt;")];
        l.extend(block(format!("if {timer} <= 0.0"), inner));
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Every, "Every 1.6 seconds, play the laser sound."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Every, "Every frame, play the laser sound."));
    }
}
