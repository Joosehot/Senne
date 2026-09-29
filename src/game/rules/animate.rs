//! "Animate each slime with "slime_1" and "slime_2" at N frames per second."
//! "Animate the player with "hero_1" and "hero_2" at N frames per second while it walks."
//!
//! Variants: `clock` (pick the frame from the time every frame) and `engine`
//! (hand the frames to Droid's sprite animator once at the start; only when
//! nothing decides per frame: not "while it walks", and not for a kind that
//! comes back in waves, since new ones would not be animated).

use super::{block, f, s, At, Emitted, Facts, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::{Mode, Standing, Who};

pub struct Animate;

impl GamePattern for Animate {
    fn name(&self) -> &'static str {
        "animate"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["clock", "engine"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Animate { .. }))
    }
    fn variants(&self, n: &Node, facts: &Facts) -> Vec<&'static str> {
        let Node::Standing(Standing::Animate { who, walking, .. }) = n else { return vec![] };
        let respawns = matches!(who, Who::Each(k) if facts.respawned.contains(k));
        if *walking || respawns {
            vec!["clock"]
        } else {
            vec!["clock", "engine"]
        }
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Animate { who, frames, fps, walking }) = n else { unreachable!() };
        let list = frames.iter().map(|x| s(x)).collect::<Vec<_>>().join(", ");
        if variant == "engine" {
            let l = match who {
                Who::Player => vec![format!("cx.animate_sprite(\"player\", &[{list}], {});", f(*fps))],
                Who::Each(k) => block(
                    format!("for n in cx.find_tag({})", s(k)),
                    vec![format!("cx.animate_sprite(&n, &[{list}], {});", f(*fps))],
                ),
            };
            return Emitted::to(Section::Init, l);
        }
        let mut l = vec![format!("let frame = [{list}][(cx.time() * {}) as usize % {}];", f(*fps), frames.len())];
        match who {
            Who::Each(k) => l.extend(block(format!("for n in cx.find_tag({})", s(k)), vec!["cx.set_sprite(&n, frame);".into()])),
            Who::Player if *walking => {
                l.push("let v = cx.vel(\"player\");".into());
                let (speed, grounded) = match g.facts.move_mode {
                    Some(Mode::Sideways) => ("v.x.abs()", " && cx.grounded(\"player\")"),
                    Some(Mode::Plane) => ("vec2(v.x, v.y).length()", ""),
                    Some(Mode::Ground) => ("vec2(v.x, v.z).length()", ""),
                    None => ("v.length()", ""),
                };
                l.push(format!("let walking = {speed} > {}{grounded};", f(g.t.walk_speed)));
                l.push(format!("cx.set_sprite(\"player\", if walking {{ frame }} else {{ {} }});", s(&frames[0])));
            }
            Who::Player => l.push("cx.set_sprite(\"player\", frame);".into()),
        }
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&Animate, "Animate each slime with \"slime_1\" and \"slime_2\" at 4 frames per second."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Animate, "Each slime patrols 3 units to each side at 2 units per second."));
    }

    #[test]
    fn walking_needs_the_clock_variant() {
        let c = code("Animate the player with \"a\" and \"b\" at 8 frames per second while it walks.");
        assert!(c.contains("[animate/clock]") && c.contains("let walking"), "{c}");
    }
}
