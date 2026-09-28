//! Judges: rules that look past a single clause.

use super::{Choice, Judge, Stats, Strategy};
use crate::config::{Axes, JudgeConfig};

/// A clause that can panic on bad input. Breakable: a sentence asking for
/// brevity ("simply", "just") forgives part of the penalty.
pub struct NoPanic;

impl Judge for NoPanic {
    fn name(&self) -> &'static str {
        "no_panic"
    }
    fn local(&self, choice: &Choice, profile: &Axes, cfg: &JudgeConfig) -> Option<(f32, String)> {
        if choice.strategy != Some(Strategy::Expect) {
            return None;
        }
        let forgiven = if cfg.breakable { cfg.discount * profile.brevity_share() } else { 0.0 };
        let penalty = -cfg.weight * (1.0 - forgiven);
        Some((penalty, format!("no_panic {penalty:+.2} (panics on error)")))
    }
}

/// Using `?` anywhere makes run() return a Result; that costs brevity.
pub struct ResultSignature;

impl Judge for ResultSignature {
    fn name(&self) -> &'static str {
        "result_signature"
    }
    fn global(&self, stats: &Stats, cfg: &JudgeConfig) -> Option<(f32, String)> {
        if stats.propagate == 0 {
            return None;
        }
        let penalty = -cfg.weight * stats.mean_brevity().max(0.0);
        Some((penalty, format!("result_signature {penalty:+.2} (run() returns Result)")))
    }
}

/// One function should handle errors one way.
pub struct ConsistentErrors;

impl Judge for ConsistentErrors {
    fn name(&self) -> &'static str {
        "consistent_errors"
    }
    fn global(&self, stats: &Stats, cfg: &JudgeConfig) -> Option<(f32, String)> {
        if stats.propagate > 0 && stats.expect > 0 {
            Some((-cfg.weight, format!("consistent_errors {:+.2} (mixes ? and expect)", -cfg.weight)))
        } else {
            None
        }
    }
}

/// A shared retry helper pays off from the second call site.
pub struct HelperReuse;

impl Judge for HelperReuse {
    fn name(&self) -> &'static str {
        "helper_reuse"
    }
    fn global(&self, stats: &Stats, cfg: &JudgeConfig) -> Option<(f32, String)> {
        match stats.helper_uses {
            0 => None,
            1 => Some((-cfg.weight, format!("helper_reuse {:+.2} (helper used once)", -cfg.weight))),
            n => {
                let bonus = cfg.weight * (n - 1) as f32;
                Some((bonus, format!("helper_reuse {bonus:+.2} (helper shared by {n} steps)")))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn choice(strategy: Strategy) -> Choice {
        Choice {
            pattern: "read_file",
            variant: "to_string",
            strategy: Some(strategy),
            retry: None,
            score: 0.0,
            notes: vec![],
        }
    }

    #[test]
    fn no_panic_fires_on_expect_only() {
        let cfg = Config::builtin();
        let j = cfg.judge("no_panic");
        assert!(NoPanic.local(&choice(Strategy::Expect), &cfg.profile, j).is_some());
        assert!(NoPanic.local(&choice(Strategy::Propagate), &cfg.profile, j).is_none());
    }

    #[test]
    fn brevity_forgives_panics() {
        let cfg = Config::builtin();
        let j = cfg.judge("no_panic");
        let terse = cfg.profile.add(&cfg.modifiers["simply"]);
        let (strict, _) = NoPanic.local(&choice(Strategy::Expect), &cfg.profile, j).unwrap();
        let (lenient, _) = NoPanic.local(&choice(Strategy::Expect), &terse, j).unwrap();
        assert!(lenient > strict);
    }

    #[test]
    fn consistent_errors_fires_on_mix() {
        let cfg = Config::builtin();
        let j = cfg.judge("consistent_errors");
        let mixed = Stats { propagate: 1, expect: 1, ..Stats::default() };
        let clean = Stats { propagate: 2, ..Stats::default() };
        assert!(ConsistentErrors.global(&mixed, j).is_some());
        assert!(ConsistentErrors.global(&clean, j).is_none());
    }

    #[test]
    fn helper_reuse_rewards_sharing() {
        let cfg = Config::builtin();
        let j = cfg.judge("helper_reuse");
        let once = Stats { helper_uses: 1, ..Stats::default() };
        let twice = Stats { helper_uses: 2, ..Stats::default() };
        assert!(HelperReuse.global(&once, j).unwrap().0 < 0.0);
        assert!(HelperReuse.global(&twice, j).unwrap().0 > 0.0);
    }
}
