//! "Animate each slime with "slime_1" and "slime_2" at N frames per second."
//! "Animate the player with "hero_1" and "hero_2" at N frames per second while it walks."
//! "Animate each perch with "perch_struggle_1" and "perch_struggle_2" at N frames per second while it is hooked."
//! "Animate each hook with "jig_1" and "jig_2" at N frames per second while it is jigged."
//!
//! "while it is hooked" animates only the ones on a hook; another sentence for
//! the same kind without it is the default and skips them, whatever the order.
//! Variants: `clock` (pick the frame from the time every frame) and `engine`
//! (hand the frames to Droid's sprite animator once at the start; only when
//! nothing decides per frame: not "while it walks" or "while it is hooked", not
//! for a kind that also has a hooked animation, and not for a kind that comes
//! back in waves, since new ones would not be animated). "while it is jigged" plays the frames
//! while the thing is jigged and shows the first frame otherwise.

use super::{block, f, is_jigged, s, At, Emitted, Facts, GamePattern, Node, Section};
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
        let Node::Standing(Standing::Animate { who, walking, hooked, jigged, .. }) = n else { return vec![] };
        let respawns = matches!(who, Who::Each(k) if facts.respawned.contains(k) || facts.hooked_anim.contains(k));
        if *walking || *hooked || *jigged || respawns {
            vec!["clock"]
        } else {
            vec!["clock", "engine"]
        }
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Animate { who, frames, fps, walking, hooked, jigged }) = n else { unreachable!() };
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
            Who::Each(k) if *jigged => {
                let on = is_jigged(g, "&n");
                let set = vec![format!("cx.set_sprite(&n, if {on} {{ frame }} else {{ {} }});", s(&frames[0]))];
                l.extend(block(format!("for n in cx.find_tag({})", s(k)), set));
            }
            Who::Each(k) if *hooked => {
                let map = super::hooked_map(g);
                let set = block(format!("if {map}.contains_key(&n)"), vec!["cx.set_sprite(&n, frame);".into()]);
                l.extend(block(format!("for n in cx.find_tag({})", s(k)), set));
            }
            Who::Each(k) if g.facts.hooked_anim.contains(k) => {
                let map = super::hooked_map(g);
                let mut body = block(format!("if {map}.contains_key(&n)"), vec!["continue;".into()]);
                body.push("cx.set_sprite(&n, frame);".into());
                l.extend(block(format!("for n in cx.find_tag({})", s(k)), body));
            }
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

    #[test]
    fn hooked_frames_only_for_the_hooked_and_the_default_skips_them() {
        // the hooked sentence first: the default must still leave the hooked ones alone
        let c = code(
            "Animate each perch with \"p_struggle_1\" and \"p_struggle_2\" at 12 frames per second while it is hooked.
             Animate each perch with \"p_swim_1\" and \"p_swim_2\" at 8 frames per second.",
        );
        let hooked = c.find("let frame = [\"p_struggle_1\"").expect("hooked animation");
        let swim = c.find("let frame = [\"p_swim_1\"").expect("default animation");
        assert!(c[hooked..swim].contains("if self.hooked.contains_key(&n) {\n                cx.set_sprite(&n, frame);"), "{c}");
        assert!(c[swim..].contains("if self.hooked.contains_key(&n) {\n                continue;"), "{c}");
        assert!(!c.contains("[animate/engine]"), "{c}");
    }

    #[test]
    fn without_a_hooked_sentence_the_default_may_use_the_engine() {
        let c = code("Animate each perch with \"p_swim_1\" and \"p_swim_2\" at 8 frames per second.");
        assert!(!c.contains("contains_key"), "{c}");
    }

    #[test]
    fn jigged_frames_play_only_while_it_is_jigged() {
        let c = code(
            "The player jigs the hook with w, lifting it 1 unit.\n\
             Animate each hook with \"jig_1\" and \"jig_2\" at 12 frames per second while it is jigged.",
        );
        assert!(c.contains("cx.set_sprite(&n, if self.jigged.get(&n).is_some_and("), "{c}");
        assert!(c.contains("{ frame } else { \"jig_1\" }"), "{c}");
    }

    #[test]
    fn while_it_is_hooked_is_not_for_the_player() {
        let e = crate::game::parser::parse(
            "Animate the player with \"a\" and \"b\" at 8 frames per second while it is hooked.",
            false,
        )
        .unwrap_err();
        assert!(format!("{e:?}").contains("while it is hooked"), "{e:?}");
    }
}
