//! "The hook hangs [by its eye] on a line from the hole | the rod tip [down through the
//! hole][, sinking at N units per second down to D]."
//!
//! "by its eye": the line ends at the hook's eye, the top middle of its box (half its height
//! above its position), so a sprite whose eye is at its top edge hangs from it; otherwise the
//! line ends at its middle. "from the rod tip": the line starts at the thing tagged "rod_tip".
//! "down through the hole": the hook rises no higher than the hole (it is landed there);
//! otherwise no higher than where the line starts.
//!
//! Every hook sinks by itself (while above depth D), and after every other
//! behaviour this frame it is kept between D and the hole, springs back
//! under the hole when something pulled it sideways (`[game.tuning]
//! line_spring`), and a thin box is stretched and turned from the hole to it.
//! Variants for the line: `named` (the line is called "<hook>_line" and found
//! by that name: no state, but a scene thing with that name would confuse it)
//! and `tracked` (remember the name the engine gave each hook's line).

use super::{block, f, s, with_anchor, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Standing;

pub struct Hang;

impl GamePattern for Hang {
    fn name(&self) -> &'static str {
        "hang"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["named", "tracked"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::Hang { .. }))
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::Hang { kind, anchor, eye, through, sink }) = n else { unreachable!() };
        let t = g.t;
        let (w, [r, gr, b], z) = (f(t.line_width), t.line_color, f(t.line_depth));
        let spawn = format!(
            "cx.spawn_shape(&format!(\"{{n}}_line\"), &[\"line\"], \"box\", top, Vec3::ONE, vec3({}, {}, {}), BodyKind::None)",
            f(r),
            f(gr),
            f(b)
        );
        let mut each = vec!["let p = cx.pos(&n);".to_string()];
        let ceil = if through.is_some() { "gate.y" } else { "top.y" };
        let clamp = match sink {
            Some((_, depth)) => format!("let y = p.y.clamp({}, {ceil});", f(*depth)),
            None => format!("let y = p.y.min({ceil});"),
        };
        each.push(clamp);
        each.push(format!("let x = top.x + (p.x - top.x) * (-{} * dt).exp();", f(t.line_spring)));
        each.push("cx.set_pos(&n, vec2(x, y));".into());
        if variant == "named" {
            each.push("let line = format!(\"{n}_line\");".into());
            each.extend(block("if !cx.exists(&line)", vec![format!("{spawn};")]));
        } else {
            let map = g.fresh(&format!("line_{kind}"), "BTreeMap<String, String>", "BTreeMap::new()");
            each.push(format!("let line = match {map}.get(&n) {{"));
            each.push("    Some(l) if cx.exists(l) => l.clone(),".into());
            each.push("    _ => {".into());
            each.push(format!("        let l = {spawn};"));
            each.push(format!("        {map}.insert(n.clone(), l.clone());"));
            each.push("        l".into());
            each.push("    }".into());
            each.push("};".into());
        }
        if *eye {
            each.push("// the line ends at its eye: the top middle of the hook".into());
            each.push("let y = y + cx.scale(&n).y * 0.5;".into());
        }
        each.push(format!("cx.set_pos(&line, vec3((top.x + x) * 0.5, (top.y + y) * 0.5, {z}));"));
        each.push(format!("cx.set_scale(&line, vec3({w}, vec2(x - top.x, y - top.y).length().max({w}), {w}));"));
        each.push("cx.set_rotation(&line, vec3(0.0, 0.0, (x - top.x).atan2(top.y - y).to_degrees()));".into());
        let mut hooks = block(format!("for n in cx.find_tag({})", s(kind)), each);
        if let Some(gate) = through {
            hooks = block(format!("if let Some(gate) = cx.find_tag({}).first().map(|h| cx.pos(h))", s(gate)), hooks);
        }
        let late = with_anchor(anchor, hooks);
        if let Some(note) = g.comment().first() {
            g.late.push(note.clone());
        }
        g.late.extend(late);
        let mut l = Vec::new();
        if let Some((speed, depth)) = sink {
            l.extend(block(
                format!("for n in cx.find_tag({})", s(kind)),
                block(
                    format!("if cx.pos(&n).y > {}", f(*depth)),
                    vec![format!("cx.move_by(&n, vec2(0.0, -{} * dt));", f(*speed))],
                ),
            ));
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
        assert!(matches(&Hang, "The hook hangs on a line from the hole, sinking at 1.5 units per second down to -9."));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&Hang, "The hook rises while the player shouts, at 8 units per second."));
    }

    #[test]
    fn line_is_drawn_after_everything_moved() {
        let c = code(
            "The hook hangs on a line from the hole, sinking at 1.5 units per second down to -9.\n\
             The hook rises while the player holds space, at 8 units per second.",
        );
        let rise = c.find("cx.key(\"space\")").unwrap();
        let line = c.find("cx.set_scale(&line").unwrap();
        assert!(rise < line, "{c}");
        assert!(c.contains("p.y.clamp(-9.0, top.y)"), "{c}");
        assert!(c.contains("cx.set_rotation(&line"), "{c}");
    }

    #[test]
    fn by_its_eye_from_the_rod_tip_through_the_hole() {
        let c = code("The hook hangs by its eye on a line from the rod tip down through the hole, sinking at 1.5 units per second down to -8.");
        assert!(c.contains("cx.find_tag(\"rod_tip\")"), "{c}");
        assert!(c.contains("if let Some(gate) = cx.find_tag(\"hole\")"), "{c}");
        assert!(c.contains("p.y.clamp(-8.0, gate.y)"), "{c}");
        assert!(c.contains("let y = y + cx.scale(&n).y * 0.5;"), "{c}");
    }
}
