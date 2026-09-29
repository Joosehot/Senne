//! "Each slime patrols N units to each side at N units per second."
//!
//! Variants: `bounds` (remember where each one started and turn at the
//! edges: exact, costs a map) and `sine` (velocity follows a cosine whose
//! swing is the patrol range: no state, drifts if something pushes it).

use super::{block, f, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Standing;

pub struct Patrol;

impl GamePattern for Patrol {
    fn name(&self) -> &'static str {
        "patrol"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["bounds", "sine"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Patrol { .. }))
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Patrol { kind, range, speed }) = n else { unreachable!() };
        let mut l = Vec::new();
        if variant == "bounds" {
            let map = g.fresh(&format!("patrol_{kind}"), "BTreeMap<String, (f32, f32)>", "BTreeMap::new()");
            let body = vec![
                "let x = cx.pos(&n).x;".to_string(),
                format!("let (home, dir) = *{map}.entry(n.clone()).or_insert((x, 1.0));"),
                format!(
                    "let dir = if x > home + {r} {{ -1.0 }} else if x < home - {r} {{ 1.0 }} else {{ dir }};",
                    r = f(*range)
                ),
                format!("cx.set_vel_x(&n, dir * {});", f(*speed)),
                "cx.set_flip_x(&n, dir < 0.0);".into(),
                format!("{map}.insert(n, (home, dir));"),
            ];
            l.extend(block(format!("for n in cx.find_tag({})", s(kind)), body));
        } else {
            l.push("let t = cx.time();".into());
            let body = vec![
                format!("let v = (t * {}).cos() * {};", f(speed / range), f(*speed)),
                "cx.set_vel_x(&n, v);".into(),
                "cx.set_flip_x(&n, v < 0.0);".into(),
            ];
            l.extend(block(format!("for n in cx.find_tag({})", s(kind)), body));
        }
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Patrol, "Each slime patrols 3 units to each side at 2.2 units per second."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Patrol, "Each bat flies in a wave 1.2 units high at 2 cycles per second."));
    }
}
