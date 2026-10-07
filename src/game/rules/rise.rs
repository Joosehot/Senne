//! "The hook rises while the player shouts [or holds space], at [up to] N
//! units per second [as loud as the shout]."
//!
//! The shout is Droid's `cx.voice()` (microphone loudness 0..1); it counts
//! from `[game.tuning] shout_threshold`. A held key always gives the top
//! speed. Variants with "as loud as the shout": `linear` (speed = top x
//! loudness once over the threshold) and `ramp` (speed grows from 0 at the
//! threshold to the top at full voice: a murmur just over it barely moves).
//! Without it: `constant` (top speed while it counts as a shout). "The harder the game, the
//! louder the shout must be" scales the threshold by `level_shout`.

use super::{block, f, s, shout_threshold, At, Emitted, Facts, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Standing;

pub struct Rise;

impl GamePattern for Rise {
    fn name(&self) -> &'static str {
        "rise"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["linear", "ramp", "constant"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Rise { .. }))
    }
    fn variants(&self, n: &Node, _f: &Facts) -> Vec<&'static str> {
        match n {
            Node::Standing(Standing::Rise { loud: true, .. }) => vec!["linear", "ramp"],
            _ => vec!["constant"],
        }
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Rise { kind, speed, voice, key, .. }) = n else { unreachable!() };
        let (top, th) = (f(*speed), shout_threshold(g));
        let mut l = Vec::new();
        if *voice {
            l.push("let shout = cx.voice();".to_string());
            l.push(match variant {
                "linear" => format!("let lift = if shout >= {th} {{ shout * {top} }} else {{ 0.0 }};"),
                "ramp" => format!("let lift = ((shout - {th}) / (1.0 - {th})).clamp(0.0, 1.0) * {top};"),
                _ => format!("let lift = if shout >= {th} {{ {top} }} else {{ 0.0 }};"),
            });
            if let Some(k) = key {
                l.push(format!("let lift = if cx.key({}) {{ {top} }} else {{ lift }};", s(k)));
            }
        } else {
            let k = key.as_deref().expect("parser: a shout or a key");
            l.push(format!("let lift = if cx.key({}) {{ {top} }} else {{ 0.0 }};", s(k)));
        }
        l.extend(block(
            "if lift > 0.0",
            block(format!("for n in cx.find_tag({})", s(kind)), vec!["cx.move_by(&n, vec2(0.0, lift * dt));".into()]),
        ));
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    #[test]
    fn applies() {
        assert!(matches(
            &Rise,
            "The hook rises while the player shouts or holds space, at up to 8 units per second as loud as the shout."
        ));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Rise, "The hook hangs on a line from the hole."));
    }

    #[test]
    fn the_shout_reads_the_microphone() {
        let c = code("The hook rises while the player shouts or holds space, at up to 8 units per second as loud as the shout.");
        assert!(c.contains("let shout = cx.voice();"), "{c}");
        assert!(c.contains("shout * 8.0"), "{c}");
        assert!(c.contains("cx.key(\"space\")"), "{c}");
    }
}
