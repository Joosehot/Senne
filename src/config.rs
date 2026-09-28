//! `rules.toml`: every number the engine uses. Rule code only names keys.

use anyhow::{bail, Context as _, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// The built-in configuration, so the binary works without a rules file.
pub const DEFAULT_RULES: &str = include_str!("../rules.toml");

/// A point in tradeoff space. Used both for what a choice offers and for
/// what a clause asks for (its profile).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
pub struct Axes {
    #[serde(default)]
    pub safety: f32,
    #[serde(default)]
    pub brevity: f32,
    #[serde(default)]
    pub perf: f32,
}

impl Axes {
    pub fn dot(&self, other: &Axes) -> f32 {
        self.safety * other.safety + self.brevity * other.brevity + self.perf * other.perf
    }
    pub fn add(&self, other: &Axes) -> Axes {
        Axes {
            safety: self.safety + other.safety,
            brevity: self.brevity + other.brevity,
            perf: self.perf + other.perf,
        }
    }
    /// Share of the profile that asks for brevity, 0..=1. Breakable judges
    /// are forgiven in proportion to it.
    pub fn brevity_share(&self) -> f32 {
        let total = self.safety.max(0.0) + self.brevity.max(0.0) + self.perf.max(0.0);
        if total <= 0.0 {
            0.0
        } else {
            self.brevity.max(0.0) / total
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct SearchConfig {
    pub beam: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RetryConfig {
    pub default_times: u32,
    pub base_delay_ms: u64,
    pub fixed_delay_ms: u64,
    pub styles: BTreeMap<String, Axes>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RuleConfig {
    pub weight: f32,
    pub variants: BTreeMap<String, Axes>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct JudgeConfig {
    pub weight: f32,
    pub breakable: bool,
    pub discount: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub search: SearchConfig,
    pub profile: Axes,
    pub modifiers: BTreeMap<String, Axes>,
    pub strategies: BTreeMap<String, Axes>,
    pub retry: RetryConfig,
    pub judges: BTreeMap<String, JudgeConfig>,
    pub rules: BTreeMap<String, RuleConfig>,
}

impl Config {
    pub fn parse(text: &str) -> Result<Config> {
        let cfg: Config = toml::from_str(text).context("parsing rules.toml")?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn load(path: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        Config::parse(&text)
    }

    pub fn builtin() -> Config {
        Config::parse(DEFAULT_RULES).expect("built-in rules.toml is valid")
    }

    /// Every pattern, variant, strategy, retry style, modifier and judge the
    /// code names must have an entry, so a typo fails at load time.
    fn validate(&self) -> Result<()> {
        for p in crate::rules::registry() {
            let Some(rc) = self.rules.get(p.name()) else {
                bail!("rules.toml: missing [rules.{}]", p.name());
            };
            for v in p.all_variants() {
                if !rc.variants.contains_key(*v) {
                    bail!("rules.toml: missing variant rules.{}.variants.{v}", p.name());
                }
            }
        }
        for s in crate::rules::STRATEGIES {
            if !self.strategies.contains_key(s.key()) {
                bail!("rules.toml: missing [strategies.{}]", s.key());
            }
        }
        for s in crate::rules::RETRY_STYLES {
            if !self.retry.styles.contains_key(s.key()) {
                bail!("rules.toml: missing [retry.styles.{}]", s.key());
            }
        }
        for m in crate::lexicon::MODIFIER_KEYS {
            if !self.modifiers.contains_key(*m) {
                bail!("rules.toml: missing [modifiers.{m}]");
            }
        }
        for j in crate::rules::judges() {
            if !self.judges.contains_key(j.name()) {
                bail!("rules.toml: missing [judges.{}]", j.name());
            }
        }
        Ok(())
    }

    pub fn rule(&self, name: &str) -> &RuleConfig {
        &self.rules[name]
    }

    pub fn judge(&self, name: &str) -> &JudgeConfig {
        &self.judges[name]
    }

    /// Score of one pattern variant under a profile.
    pub fn variant_score(&self, rule: &str, variant: &str, profile: &Axes) -> f32 {
        let rc = self.rule(rule);
        rc.weight * rc.variants[variant].dot(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_load() {
        let cfg = Config::builtin();
        assert!(cfg.search.beam > 0);
    }

    #[test]
    fn missing_variant_is_rejected() {
        let broken = DEFAULT_RULES.replace("variants.atomic", "variants.atomik");
        assert!(Config::parse(&broken).is_err());
    }
}
