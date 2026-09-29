//! Judges: rules that look across the whole game program.

use super::{Helper, Judge, Stats};
use crate::config::JudgeConfig;

/// Win, lose and hurt can be written inline or as a method. A method used
/// once is overhead; the same block inlined twice is duplication; mixing the
/// two for one block reads badly. Each costs `weight` (per extra copy for
/// duplication).
pub struct HelperReuse;

impl Judge for HelperReuse {
    fn name(&self) -> &'static str {
        "helper_reuse"
    }
    fn global(&self, st: &Stats, cfg: &JudgeConfig) -> Option<(f32, String)> {
        let w = cfg.weight;
        let mut total = 0.0;
        let mut notes = Vec::new();
        for (name, kind, [helper, inline]) in
            [("win", Helper::Win, st.win), ("lose", Helper::Lose, st.lose_sites()), ("hurt", Helper::Hurt, st.hurt)]
        {
            if helper == 1 && st.pending_of(kind) == 0 {
                total -= w;
                notes.push(format!("{name} helper used once"));
            }
            if inline >= 2 {
                total -= w * (inline - 1) as f32;
                notes.push(format!("{name} inlined {inline} times"));
            }
            if helper > 0 && inline > 0 {
                total -= w;
                notes.push(format!("{name} mixes helper and inline"));
            }
        }
        if notes.is_empty() {
            None
        } else {
            Some((total, format!("helper_reuse {total:+.2} ({})", notes.join(", "))))
        }
    }
}

/// Reading the player's position once per frame (hoisted) pays off from the
/// second loop that uses it; for one loop it is an extra line. Mixing
/// hoisted and per-loop reads costs too.
pub struct HoistReuse;

impl Judge for HoistReuse {
    fn name(&self) -> &'static str {
        "hoist_reuse"
    }
    fn global(&self, st: &Stats, cfg: &JudgeConfig) -> Option<(f32, String)> {
        let w = cfg.weight;
        let mut total = 0.0;
        let mut notes = Vec::new();
        if st.hoisted == 1 && st.per_loop == 0 && st.pending_hoist == 0 {
            total -= w;
            notes.push("hoisted for one loop".to_string());
        }
        if st.hoisted >= 2 {
            total += w * (st.hoisted - 1) as f32;
            notes.push(format!("hoisted read shared by {} loops", st.hoisted));
        }
        if st.hoisted > 0 && st.per_loop > 0 {
            total -= w;
            notes.push("mixes hoisted and per-loop reads".to_string());
        }
        if notes.is_empty() {
            None
        } else {
            Some((total, format!("hoist_reuse {total:+.2} ({})", notes.join(", "))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn helper_used_once_costs_and_duplication_costs() {
        let cfg = Config::builtin();
        let j = cfg.game_judge("helper_reuse");
        let once = Stats { win: [1, 0], ..Stats::default() };
        let twice_inline = Stats { win: [0, 2], ..Stats::default() };
        let twice_helper = Stats { win: [2, 0], ..Stats::default() };
        let mixed = Stats { win: [1, 1], ..Stats::default() };
        assert!(HelperReuse.global(&once, j).unwrap().0 < 0.0);
        assert!(HelperReuse.global(&twice_inline, j).unwrap().0 < 0.0);
        assert!(HelperReuse.global(&twice_helper, j).is_none());
        assert!(HelperReuse.global(&mixed, j).unwrap().0 < HelperReuse.global(&once, j).unwrap().0);
    }

    #[test]
    fn hurt_blocks_count_their_lose() {
        let cfg = Config::builtin();
        let j = cfg.game_judge("helper_reuse");
        // two inline hurt blocks each carry an inline "lose"
        let st = Stats { hurt: [0, 2], ..Stats::default() };
        assert_eq!(st.lose_sites(), [0, 2]);
        let (score, note) = HelperReuse.global(&st, j).unwrap();
        assert!(score < 0.0 && note.contains("lose inlined 2 times"), "{note}");
    }

    #[test]
    fn a_lone_helper_is_not_charged_while_sites_remain() {
        let cfg = Config::builtin();
        let j = cfg.game_judge("helper_reuse");
        let early = Stats { hurt: [1, 0], pending: [0, 0, 2], ..Stats::default() };
        assert!(HelperReuse.global(&early, j).is_none());
    }

    #[test]
    fn hoisting_pays_from_two_users() {
        let cfg = Config::builtin();
        let j = cfg.game_judge("hoist_reuse");
        let one = Stats { hoisted: 1, ..Stats::default() };
        let two = Stats { hoisted: 2, ..Stats::default() };
        let mixed = Stats { hoisted: 1, per_loop: 1, ..Stats::default() };
        assert!(HoistReuse.global(&one, j).unwrap().0 < 0.0);
        assert!(HoistReuse.global(&two, j).unwrap().0 > 0.0);
        assert!(HoistReuse.global(&mixed, j).unwrap().0 < 0.0);
        assert!(HoistReuse.global(&Stats { per_loop: 3, ..Stats::default() }, j).is_none());
    }
}
