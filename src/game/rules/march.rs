//! "Each alien marches sideways at N units per second within N units,
//! stepping down N units[, up to N times as fewer are left][, N percent
//! faster each wave]."
//!
//! The group moves as one: when the leading edge would pass `within`, all
//! step down and turn. Variants: `move_by` (move the positions each frame)
//! and `velocity` (set their velocity; the physics step moves them).

use super::{block, f, s, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::Gen;
use crate::game::parser::Standing;

pub struct March;

impl GamePattern for March {
    fn name(&self) -> &'static str {
        "march"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["move_by", "velocity"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Standing(Standing::March { .. }))
    }
    fn emit(&self, variant: &str, n: &Node, g: &mut Gen, _at: &At, _body: Vec<String>) -> Emitted {
        let Node::Standing(Standing::March { kind, speed, within, step, max_mult, wave_pct }) = n else { unreachable!() };
        let dir = g.fresh(&format!("march_{kind}"), "f32", "1.0");
        let most = g.fresh(&format!("most_{kind}"), "f32", "0.0");
        let mut body = vec!["let left = group.len() as f32;".to_string(), format!("{most} = {most}.max(left);")];
        let mut speed_expr = f(*speed);
        if let Some(m) = max_mult {
            speed_expr.push_str(&format!(" * (1.0 + {} * (1.0 - left / {most}))", f(m - 1.0)));
        }
        if let Some(p) = wave_pct {
            let wave = g.field("wave", "i64", "1");
            speed_expr.push_str(&format!(" * (1.0 + {} * ({wave} - 1) as f32)", f(p / 100.0)));
        }
        body.push(format!("let speed = {speed_expr};"));
        body.push(format!("let step = {dir} * speed * dt;"));
        body.push("let xs: Vec<f32> = group.iter().map(|n| cx.pos(n).x).collect();".into());
        body.push("let lo = xs.iter().copied().fold(f32::INFINITY, f32::min);".into());
        body.push("let hi = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);".into());
        let mut turn = vec![format!("{dir} = -{dir};")];
        let mut down = vec![format!("cx.move_by(n, vec2(0.0, -{}));", f(*step))];
        if variant == "velocity" {
            down.push("cx.set_vel_x(n, 0.0);".into());
        }
        turn.extend(block("for n in &group", down));
        body.extend(block(format!("if (step > 0.0 && hi + step > {w}) || (step < 0.0 && lo + step < -{w})", w = f(*within)), turn));
        let go = if variant == "velocity" {
            block("for n in &group", vec![format!("cx.set_vel_x(n, {dir} * speed);")])
        } else {
            block("for n in &group", vec!["cx.move_by(n, vec2(step, 0.0));".into()])
        };
        body.extend(block("else", go));
        let mut l = vec![format!("let group = cx.find_tag({});", s(kind))];
        l.extend(block("if !group.is_empty()", super::jump::join_else(body)));
        Emitted::to(Section::Update, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::matches;

    #[test]
    fn applies() {
        assert!(matches(
            &March,
            "Each alien marches sideways at 1.4 units per second within 12.5 units, stepping down 0.6 units."
        ));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&March, "Each alien shoots down at 5 units per second every 2 seconds."));
    }
}
