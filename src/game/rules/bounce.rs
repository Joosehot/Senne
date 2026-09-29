//! "bounce the player up at N units per second"

use super::{f, At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::Action;

pub struct Bounce;

impl GamePattern for Bounce {
    fn name(&self) -> &'static str {
        "bounce"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["set_vel"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Bounce(_), .. })
    }
    fn emit(&self, _variant: &str, n: &Node, _g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Bounce(v), .. } = n else { unreachable!() };
        Emitted::inline(vec![format!("cx.set_vel_y(\"player\", {});", f(*v))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Bounce, "When the player lands on a slime, bounce the player up at 11 units per second."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Bounce, "When the player lands on a slime, squash it."));
    }
}
