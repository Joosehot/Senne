//! "destroy it" / "destroy both" / "destroy the shot" / "destroy the bombs"

use super::{s, At, Emitted, GamePattern, Node};
use crate::game::codegen::Gen;
use crate::game::parser::{Action, Target};

pub struct Destroy;

impl GamePattern for Destroy {
    fn name(&self) -> &'static str {
        "destroy"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["despawn"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Action { act: Action::Destroy(_), .. })
    }
    fn emit(&self, _variant: &str, n: &Node, _g: &mut Gen, at: &At, _body: Vec<String>) -> Emitted {
        let Node::Action { act: Action::Destroy(t), .. } = n else { unreachable!() };
        let l = match t {
            Target::Both => vec!["cx.destroy(a);".to_string(), "cx.destroy(b);".to_string()],
            Target::All(tag) => vec![format!("cx.destroy_tag({});", s(tag))],
            one => vec![format!("cx.destroy({});", at.one(one))],
        };
        Emitted::inline(l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Destroy, "When a shot hits an alien, destroy both."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Destroy, "When a shot hits an alien, add 10 points."));
    }
}
