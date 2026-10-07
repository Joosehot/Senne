//! "When the hook is jigged within 2 units of a fish [and no fish is hooked], ..." — once per
//! jig: in the frame a hook is twitched (its time in the game's map of jigged things is this
//! frame's), the actions run for every one of the kind within the distance of it, nearest or
//! not, with "it" / "the fish" = that one and "the hook" = the hook. "and no fish is hooked"
//! stops at the first catch.
//!
//! For the actions it also binds, when they use them: `near` (1 at the lure, 0 at the edge of
//! the distance), and the rhythm of the jigging: `steady` (this jig came `[game.tuning]
//! rhythm_min`..`rhythm_max` seconds after the last one, within `rhythm_slack` of the gap
//! before: an even beat) and `frantic` (it came sooner than `frantic_secs`). See `bite.rs`.

use super::{block, f, hooked_map, jigged_map, s, shy_map, At, Emitted, GamePattern, Node, Section};
use crate::game::codegen::{uses_ident, Gen};
use crate::game::parser::{Cond, Trigger};

pub struct JigNear;

impl GamePattern for JigNear {
    fn name(&self) -> &'static str {
        "jig_near"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["each_jig"]
    }
    fn matches(&self, n: &Node) -> bool {
        matches!(n, Node::Trigger(Trigger::Jigged { .. }))
    }
    fn emit(&self, _variant: &str, n: &Node, g: &mut Gen, _at: &At, body: Vec<String>) -> Emitted {
        let Node::Trigger(Trigger::Jigged { a, b, within, cond }) = n else { unreachable!() };
        let t = g.t;
        let map = jigged_map(g);
        let clock = g.clock();
        let mut outer = block(format!("if {map}.get(&h).copied() != Some({clock})"), vec!["continue;".into()]);
        outer.push("let lure = cx.pos(&h);".into());
        let (steady, frantic) = (uses_ident(&body, "steady"), uses_ident(&body, "frantic"));
        if steady || frantic {
            let beat = g.fresh(&format!("beat_{a}"), "BTreeMap<String, (f32, f32)>", "BTreeMap::new()");
            outer.push("// the rhythm: the time since the last jig, and the gap before that".into());
            outer.push(format!("let (last, before) = {beat}.get(&h).copied().unwrap_or((f32::NEG_INFINITY, 0.0));"));
            outer.push(format!("let gap = {clock} - last;"));
            outer.push(format!("{beat}.insert(h.clone(), ({clock}, gap));"));
            if steady {
                outer.push(format!(
                    "let steady = ({}..={}).contains(&gap) && (gap - before).abs() <= {};",
                    f(t.rhythm_min),
                    f(t.rhythm_max),
                    f(t.rhythm_slack)
                ));
            }
            if frantic {
                outer.push(format!("let frantic = gap < {};", f(t.frantic_secs)));
            }
        }
        let mut inner = Vec::new();
        match cond {
            None => {}
            Some(Cond::NoneHooked(k)) => {
                let hooked = hooked_map(g);
                inner.extend(block(format!("if {hooked}.keys().any(|k| cx.has_tag(k, {}))", s(k)), vec!["break;".into()]));
            }
            Some(Cond::NoneLeft(k)) => {
                inner.extend(block(format!("if cx.count_tag({}) > 0", s(k)), vec!["break;".into()]));
            }
        }
        if g.facts.hooks {
            let hooked = hooked_map(g);
            inner.extend(block(format!("if {hooked}.contains_key(&n)"), vec!["continue;".into()]));
        }
        if g.facts.shy {
            // one that just got away doesn't bite again yet
            let shy = shy_map(g);
            inner.extend(block(format!("if {shy}.get(&n).is_some_and(|t| *t > {clock})"), vec!["continue;".into()]));
        }
        inner.push("let d = (cx.pos(&n) - lure).length();".into());
        inner.extend(block(format!("if d >= {}", f(*within)), vec!["continue;".into()]));
        if uses_ident(&body, "near") {
            inner.push(format!("let near = 1.0 - d / {};", f(*within)));
        }
        inner.extend(body);
        outer.extend(block(format!("for n in cx.find_tag({})", s(b)), inner));
        Emitted::to(Section::Update, block(format!("for h in cx.find_tag({})", s(a)), outer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::test_util::{code, matches};

    const JIG: &str = "The player jigs the hook with the up arrow, lifting it 0.8 units.\n";

    #[test]
    fn applies() {
        assert!(matches(&JigNear, &format!("{JIG}When the hook is jigged within 2 units of a fish, the fish bites.")));
    }

    #[test]
    fn does_not_apply() {
        assert!(!matches(&JigNear, &format!("{JIG}When a jigged hook touches a fish, the fish bites.")));
    }

    #[test]
    fn once_per_jig_for_every_one_near() {
        let c = code(&format!("{JIG}When the hook is jigged within 2 units of a fish and no fish is hooked, the fish bites."));
        assert!(c.contains("if self.jigged.get(&h).copied() != Some(self.clock) {"), "{c}");
        assert!(c.contains("for n in cx.find_tag(\"fish\") {"), "{c}");
        assert!(c.contains("if d >= 2.0 {"), "{c}");
        assert!(c.contains("break;"), "{c}");
        // nothing asks for the rhythm or the distance: none of it is written
        assert!(!c.contains("let steady") && !c.contains("let near") && !c.contains("beat_hook"), "{c}");
    }

    #[test]
    fn the_rhythm_is_measured_only_when_the_chance_uses_it() {
        let c = code(&format!(
            "{JIG}When the hook is jigged within 2 units of a fish, the fish bites with a chance of 30 percent, \
             more for a steady rhythm and less for frantic jigging."
        ));
        assert!(c.contains("let gap = self.clock - last;"), "{c}");
        assert!(c.contains("let steady = (0.35..=1.2).contains(&gap) && (gap - before).abs() <= 0.25;"), "{c}");
        assert!(c.contains("let frantic = gap < 0.3;"), "{c}");
    }
}
