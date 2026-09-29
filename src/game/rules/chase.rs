//! "Each zombie moves toward the player at N units per second [sideways |
//! across the ground] [within N units] [speeding up by N percent per second]."
//!
//! Variants: `hoisted` (reads the player position once per frame, shared
//! with other loops) and `per_loop` (reads it inside this loop). The
//! `hoist_reuse` judge decides across the program.

use super::{block, f, s, At, Emitted, GamePattern, Node, Section, Uses};
use crate::game::codegen::{Feature, Gen};
use crate::game::parser::{Mode, Standing};

pub struct Chase;

/// The player position expression for a hoist variant.
pub fn player_pos(variant: &str, g: &mut Gen) -> String {
    if variant == "hoisted" {
        g.need(Feature::Hoist);
        "player".into()
    } else {
        "cx.pos(\"player\")".into()
    }
}

impl GamePattern for Chase {
    fn name(&self) -> &'static str {
        "chase"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["hoisted", "per_loop"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Chase { .. }))
    }
    fn uses(&self, variant: &str, _n: &Node) -> Uses {
        Uses { hoist: Some(variant == "hoisted"), ..Uses::default() }
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Chase { kind, speed, mode, within, speedup }) = n else { unreachable!() };
        let player = player_pos(variant, g);
        let mut l = Vec::new();
        let speed = match speedup {
            Some(pct) => {
                let clock = g.clock();
                l.push(format!("let speed = {} * (1.0 + {clock} * {});", f(*speed), f(pct / 100.0)));
                "speed".to_string()
            }
            None => f(*speed),
        };
        let stop: Vec<String> = match mode {
            Mode::Sideways => vec!["cx.set_vel_x(&n, 0.0);".into()],
            Mode::Plane => vec!["cx.set_vel(&n, Vec3::ZERO);".into()],
            Mode::Ground => vec!["cx.set_vel_x(&n, 0.0);".into(), "cx.set_vel_z(&n, 0.0);".into()],
        };
        let mut body = vec![format!("let to = {player} - cx.pos(&n);")];
        if let Some(w) = within {
            let mut far = stop.clone();
            far.push("continue;".into());
            body.extend(block(format!("if to.length() > {}", f(*w)), far));
        }
        match mode {
            Mode::Sideways => {
                body.extend(block(
                    format!("if to.x.abs() > {}", f(g.t.chase_dead_zone)),
                    vec![format!("cx.set_vel_x(&n, to.x.signum() * {speed});"), "cx.set_flip_x(&n, to.x < 0.0);".into()],
                ));
                body.extend(block("else", stop));
            }
            Mode::Plane => body.push(format!("cx.set_vel(&n, vec2(to.x, to.y).normalize_or_zero() * {speed});")),
            Mode::Ground => {
                body.push("let dir = vec3(to.x, 0.0, to.z).normalize_or_zero();".into());
                body.push(format!("cx.set_vel_x(&n, dir.x * {speed});"));
                body.push(format!("cx.set_vel_z(&n, dir.z * {speed});"));
            }
        }
        l.extend(block(format!("for n in cx.find_tag({})", s(kind)), super::jump::join_else(body)));
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(&Chase, "Each zombie moves toward the player at 2.3 units per second."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Chase, "Each blob wanders at 2 units per second, turning every 1.5 seconds."));
    }

    #[test]
    fn two_chasers_share_a_hoisted_read() {
        let c = code(
            "Each zombie moves toward the player at 2 units per second.\n\
             Each ghost moves toward the player at 1 units per second.",
        );
        assert_eq!(c.matches("let player = cx.pos(\"player\");").count(), 1, "{c}");
        assert!(c.contains("let to = player - cx.pos(&n);"), "{c}");
    }
}
