//! "Each bat flies in a wave N units high at N cycles per second."
//!
//! Variants: `velocity` (vertical velocity is the wave's derivative: no
//! state, left to the physics step) and `anchored` (remember each one's
//! height and phase, set the position exactly).

use super::{block, f, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Standing;

pub struct Fly;

impl GamePattern for Fly {
    fn name(&self) -> &'static str {
        "fly"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["velocity", "anchored"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Fly { .. }))
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Fly { kind, height, freq }) = n else { unreachable!() };
        let w = std::f32::consts::TAU * freq;
        let mut l = vec!["let t = cx.time();".to_string()];
        let body = if variant == "anchored" {
            let map = g.fresh(&format!("fly_{kind}"), "BTreeMap<String, (f32, f32)>", "BTreeMap::new()");
            vec![
                "let p = cx.pos(&n);".to_string(),
                format!("let (home, t0) = *{map}.entry(n.clone()).or_insert((p.y, t));"),
                format!("cx.set_pos(&n, vec3(p.x, home + ((t - t0) * {}).sin() * {}, p.z));", f(w), f(*height)),
            ]
        } else {
            vec![format!("cx.set_vel_y(&n, (t * {}).cos() * {});", f(w), f(height * w))]
        };
        l.extend(block(format!("for n in cx.find_tag({})", s(kind)), body));
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(&Fly, "Each bat flies in a wave 1.2 units high at 2 cycles per second."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Fly, "Each slime patrols 3 units to each side at 2.2 units per second."));
    }
}
