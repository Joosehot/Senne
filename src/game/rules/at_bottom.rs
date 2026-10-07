//! "When a hooked perch stays at the bottom for 3 seconds, ..." — every
//! frame, each hooked perch whose hook is within `[game.tuning]
//! bottom_margin` of the lowest point hooks sink to ("sinking ... down to D")
//! counts the time it has been there; once it reaches the given seconds the
//! count starts over and the actions run with "it" = the perch. A weak shout
//! lets the fish drag the hook down, so this is where it gets away.

use super::{block, f, hooked_map, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Trigger;

pub struct AtBottom;

impl GamePattern for AtBottom {
    fn name(&self) -> &'static str {
        "at_bottom"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["timer"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Trigger(Trigger::AtBottom { .. }))
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        let Node::Trigger(Trigger::AtBottom { kind, secs }) = n else { unreachable!() };
        let depth = g.facts.hook_depth.expect("parser: a hook sinks somewhere");
        let map = hooked_map(g);
        let held = g.fresh(&format!("bottom_{kind}"), "BTreeMap<String, f32>", "BTreeMap::new()");
        let mut inner = block(format!("if !cx.has_tag(&n, {})", s(kind)), vec!["continue;".into()]);
        inner.push(format!("let down = cx.pos(&hook).y <= {};", f(depth + g.t.bottom_margin)));
        inner.push(format!("let held = if down {{ {held}.get(&n).copied().unwrap_or(0.0) + dt }} else {{ 0.0 }};"));
        inner.push(format!("{held}.insert(n.clone(), held);"));
        let mut fire = vec![format!("{held}.remove(&n);")];
        fire.extend(body);
        inner.extend(block(format!("if held >= {}", f(*secs)), fire));
        Emitted::to(Section::Update, block(format!("for (n, hook) in {map}.clone()"), inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    const HANG: &str = "The hook hangs on a line from the hole, sinking at 1.5 units per second down to -9.\n";

    #[test]
    fn applies() {
        assert!(matches(&AtBottom, &format!("{HANG}When a hooked pike stays at the bottom for 3 seconds, it gets away.")));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&AtBottom, "When a hooked pike reaches the hole, add 30 points."));
    }

    #[test]
    fn the_bottom_is_where_hooks_sink_to() {
        let c = code(&format!("{HANG}When a hooked pike stays at the bottom for 3 seconds, it gets away."));
        assert!(c.contains("let down = cx.pos(&hook).y <= -8.8;"), "{c}");
        assert!(c.contains("if held >= 3.0 {"), "{c}");
    }
}
