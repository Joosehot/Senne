//! "Each slime patrols N units to each side at N units per second."
//! "Each perch swims within 12 units at 2 units per second."
//!
//! Variants: `bounds` (remember where each one started and turn at the
//! edges: exact, costs a map) and `sine` (velocity follows a cosine whose
//! swing is the patrol range: no state, drifts if something pushes it).
//! "within N units" turns at x = +-N wherever it started (new ones head for
//! the middle), so only `bounds` fits. Hooked things don't patrol.

use super::{block, f, s, At, Emitted, Facts, GamePattern, Node, Section};
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
    fn variants(&self, n: &Node, _f: &Facts) -> Vec<&'static str> {
        match n {
            Node::Standing(Standing::Patrol { edges: true, .. }) => vec!["bounds"],
            _ => self.all_variants().to_vec(),
        }
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Patrol { kind, range, speed, edges }) = n else { unreachable!() };
        let skip = if g.facts.hooks {
            let map = super::hooked_map(g);
            block(format!("if {map}.contains_key(&n)"), vec!["continue;".into()])
        } else {
            Vec::new()
        };
        let mut l = Vec::new();
        if *edges {
            let map = g.fresh(&format!("patrol_{kind}"), "BTreeMap<String, f32>", "BTreeMap::new()");
            let mut body = skip;
            body.extend([
                "let x = cx.pos(&n).x;".to_string(),
                format!("let dir = *{map}.entry(n.clone()).or_insert(if x > 0.0 {{ -1.0 }} else {{ 1.0 }});"),
                format!(
                    "let dir = if x > {r} {{ -1.0 }} else if x < -{r} {{ 1.0 }} else {{ dir }};",
                    r = f(*range)
                ),
                format!("cx.set_vel_x(&n, dir * {});", f(*speed)),
                "cx.set_flip_x(&n, dir < 0.0);".into(),
                format!("{map}.insert(n, dir);"),
            ]);
            l.extend(block(format!("for n in cx.find_tag({})", s(kind)), body));
        } else if variant == "bounds" {
            let map = g.fresh(&format!("patrol_{kind}"), "BTreeMap<String, (f32, f32)>", "BTreeMap::new()");
            let mut body = skip;
            body.extend([
                "let x = cx.pos(&n).x;".to_string(),
                format!("let (home, dir) = *{map}.entry(n.clone()).or_insert((x, 1.0));"),
                format!(
                    "let dir = if x > home + {r} {{ -1.0 }} else if x < home - {r} {{ 1.0 }} else {{ dir }};",
                    r = f(*range)
                ),
                format!("cx.set_vel_x(&n, dir * {});", f(*speed)),
                "cx.set_flip_x(&n, dir < 0.0);".into(),
                format!("{map}.insert(n, (home, dir));"),
            ]);
            l.extend(block(format!("for n in cx.find_tag({})", s(kind)), body));
        } else {
            l.push("let t = cx.time();".into());
            let mut body = skip;
            body.extend([
                format!("let v = (t * {}).cos() * {};", f(speed / range), f(*speed)),
                "cx.set_vel_x(&n, v);".into(),
                "cx.set_flip_x(&n, v < 0.0);".into(),
            ]);
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

    #[test]
    fn swimming_turns_at_the_edges_and_skips_hooked_ones() {
        let c = crate::game::rules::test_util::code(
            "Each perch swims within 12 units at 2 units per second.
             Each hooked perch follows its hook.",
        );
        assert!(c.contains("if x > 12.0 { -1.0 } else if x < -12.0 { 1.0 } else { dir }"), "{c}");
        assert!(c.contains("if self.hooked.contains_key(&n) {"), "{c}");
    }
}
