//! "When the score reaches 50, ..." / "When the time reaches 60 seconds, ..."
//! — edge-triggered like `none_left`: once when the value gets there.

use super::{block, f, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Trigger;

pub struct Reaches;

impl GamePattern for Reaches {
    fn name(&self) -> &'static str {
        "reaches"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["edge"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Trigger(Trigger::Score(_) | Trigger::Time(_)))
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        let (flag, cond) = match n {
            Node::Trigger(Trigger::Score(v)) => {
                let score = g.field("score", "i64", "0");
                (g.fresh(&format!("score_{v}"), "bool", "false"), format!("{score} >= {v}"))
            }
            Node::Trigger(Trigger::Time(t)) => {
                let clock = g.clock();
                let name = format!("time_{}", f(*t).replace(['.', '-'], "_"));
                (g.fresh(&name, "bool", "false"), format!("{clock} >= {}", f(*t)))
            }
            _ => unreachable!(),
        };
        let mut l = vec![format!("let reached = {cond};")];
        l.extend(block(format!("if reached && !{flag}"), body));
        l.push(format!("{flag} = reached;"));
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Reaches, "When the score reaches 50, the player wins."));
        assert!(matches(&Reaches, "When the time reaches 60 seconds, the player wins."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Reaches, "When no coins are left, the player wins."));
    }
}
