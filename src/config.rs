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
    /// The game domain (`senne::game`): its own profile, tuning, rules and judges.
    pub game: GameConfig,
}

/// `[game]`: everything `senne::game` reads. Pattern code names keys only.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameConfig {
    pub profile: Axes,
    pub tuning: GameTuning,
    pub judges: BTreeMap<String, JudgeConfig>,
    pub rules: BTreeMap<String, RuleConfig>,
}

/// `[game.tuning]`: every constant the generated game code uses.
/// Unknown keys are rejected so a typo fails at load time.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameTuning {
    /// "smoothly": velocity eases toward the target at this rate (per second).
    pub smoothing_rate: f32,
    /// Buffered jump: a press this long before landing still jumps (seconds).
    pub jump_buffer: f32,
    /// Buffered jump: the player may still jump this long after leaving the ground (seconds).
    pub coyote_time: f32,
    /// "higher while held": extra downward acceleration while rising without the key.
    pub jump_release_gravity: f32,
    /// "while it walks": speed that counts as walking (units per second).
    pub walk_speed: f32,
    /// Player shots without "every N seconds": time between shots (seconds).
    pub shot_cooldown: f32,
    /// Size of spawned projectiles (units).
    pub shot_size: f32,
    /// Projectiles spawn this far from the shooter's centre (units).
    pub shot_offset: f32,
    /// Projectiles farther than this from the player are removed (units).
    pub projectile_range: f32,
    /// "lands on": the player's centre must be this far above the other's (units).
    pub stomp_height: f32,
    /// "squash it": squash-and-stretch amount (0..0.9).
    pub squash_amount: f32,
    /// "moves toward the player sideways": no movement inside this x distance (units).
    pub chase_dead_zone: f32,
    /// "wanders", own timers: each turn waits turn x (1 +- jitter).
    pub wander_jitter: f32,
    /// "loses a life" without "safe for": seconds of safety afterwards.
    pub safe_seconds: f32,
    /// "loses a life" without "has N lives": starting lives.
    pub lives: i64,
    /// Blinks per second while safe.
    pub blink_rate: f32,
    /// Brightness of the player on the dark half of a blink (0..1).
    pub blink_dim: f32,
    /// "play the music" without "at N beats per minute".
    pub music_bpm: f32,
    /// "play the music" without "as a W wave".
    pub music_wave: String,
    /// HUD text size (pixels).
    pub hud_size: f32,
    /// Win/lose banner size (pixels).
    pub banner_size: f32,
    /// Vertical gap between HUD lines at the same place (fraction of the screen).
    pub line_gap: f32,
    /// Screen positions (fractions of width, height from the top left).
    pub top_left: [f32; 2],
    pub top_right: [f32; 2],
    pub top: [f32; 2],
    pub middle: [f32; 2],
    pub bottom: [f32; 2],
    pub banner: [f32; 2],
    /// "while the player shouts": the microphone loudness (0..1) that counts as a shout.
    pub shout_threshold: f32,
    /// "hangs on a line": the line's thickness (units), colour and depth (z, behind the hook).
    pub line_width: f32,
    pub line_color: [f32; 3],
    pub line_depth: f32,
    /// A hooked thing hangs this far below its hook (units).
    pub hooked_drop: f32,
    /// "a hooked X reaches the Y": the hook counts as there within this height (units).
    pub land_margin: f32,
    /// "comes in from the side": how far from the middle it appears (units, either side).
    pub side_x: f32,
    /// "hangs on a line": a hook pulled sideways springs back under the hole at this rate (1/s).
    pub line_spring: f32,
    /// "in bursts": a hooked thing pulls this long, then rests this long (seconds).
    pub burst_secs: f32,
    pub rest_secs: f32,
    /// "fighting": it darts this far to each side (units) this many times a second, at
    /// `rest_thrash` of that while resting, and jerks its hook sideways by `hook_jerk` of it.
    pub thrash_width: f32,
    pub thrash_rate: f32,
    pub rest_thrash: f32,
    pub hook_jerk: f32,
    /// "stays at the bottom": the hook counts as at the bottom within this height (units).
    pub bottom_margin: f32,
    /// "it gets away": it won't bite again for this long (seconds).
    pub shy_secs: f32,
    /// "the shout meter": bar cells, label, colour at or above the shout threshold and below it.
    pub meter_width: i64,
    pub meter_label: String,
    pub meter_on: [f32; 3],
    pub meter_off: [f32; 3],
    /// "show ... for N seconds": text size of such messages (pixels).
    pub flash_size: f32,
    /// "swims ...": each fish runs its own clock, (1 +- jitter/2) as fast as the game's, from a
    /// seed made of its name; velocity eases to the target at `swim_turn_rate` (1/s), at
    /// `swim_smooth_turn` with "turning smoothly"; it faces left or right once it moves faster
    /// than `face_speed` that way.
    pub swim_clock_jitter: f32,
    pub swim_turn_rate: f32,
    pub swim_smooth_turn: f32,
    pub face_speed: f32,
    /// "darting": every `dart_cycle` seconds a burst of `dart_secs` at `dart_boost` x the speed
    /// (reached at `dart_rate`), then a glide that slows at `glide_rate` into a pause; at a burst
    /// it turns with the chance `dart_turn_chance`.
    pub dart_cycle: f32,
    pub dart_secs: f32,
    pub dart_boost: f32,
    pub dart_rate: f32,
    pub glide_rate: f32,
    pub dart_turn_chance: f32,
    /// "drifting": the speed swells by +- `drift_swing` of itself at `drift_rate` (rad/s).
    pub drift_swing: f32,
    pub drift_rate: f32,
    /// "lurking": hovers at `lurk_low`..1 x the speed, the fins beating at `lurk_rate` (rad/s);
    /// every `lurk_turn_secs` it turns with the chance `lurk_turn_chance`.
    pub lurk_low: f32,
    pub lurk_rate: f32,
    pub lurk_turn_secs: f32,
    pub lurk_turn_chance: f32,
    /// "keeping its depth": drifts over `depth_share` of the band at `depth_rate` (rad/s); its
    /// height follows the target at `depth_follow` (1/s).
    pub depth_share: f32,
    pub depth_rate: f32,
    pub depth_follow: f32,
    /// "in a wave N units high": cycles per second.
    pub swim_wave_rate: f32,
    /// "in a school": others within `school_radius` count; it closes in on them at `school_pull`
    /// (1/s) and is pushed apart at `school_push` (1/s) inside `school_spacing` (units).
    pub school_radius: f32,
    pub school_spacing: f32,
    pub school_pull: f32,
    pub school_push: f32,
    /// "nosing toward": at `nose_speed` x the speed, slowing within reach at `nose_ease` (1/s).
    pub nose_speed: f32,
    pub nose_ease: f32,
    /// "lunging at": the dash lasts `lunge_secs`, the next can come `lunge_rest` seconds after it.
    pub lunge_secs: f32,
    pub lunge_rest: f32,
    /// "jigs ... lifting it N units": the twitch rises over `jig_lift_secs`; the hook counts as
    /// jigged for `jig_secs` after a twitch. "it flutters back down": it sways `flutter_sway`
    /// units to each side at `flutter_rate` cycles per second, fading out over `jig_secs`.
    pub jig_lift_secs: f32,
    pub jig_secs: f32,
    pub flutter_sway: f32,
    pub flutter_rate: f32,
    /// "drawn to a jigged hook": a fish slows down within this distance of it (units).
    pub draw_ease: f32,
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
        self.validate_game()
    }

    /// The game domain is strict both ways: every pattern, variant and judge
    /// the code names must be present, and nothing else may be (a misspelt
    /// key would otherwise be silently ignored).
    fn validate_game(&self) -> Result<()> {
        let g = &self.game;
        let patterns = crate::game::rules::registry();
        for p in &patterns {
            let Some(rc) = g.rules.get(p.name()) else {
                bail!("rules.toml: missing [game.rules.{}]", p.name());
            };
            for v in p.all_variants() {
                if !rc.variants.contains_key(*v) {
                    bail!("rules.toml: missing variant game.rules.{}.variants.{v}", p.name());
                }
            }
            for v in rc.variants.keys() {
                if !p.all_variants().contains(&v.as_str()) {
                    bail!("rules.toml: game.rules.{}.variants.{v} is not a variant of {}", p.name(), p.name());
                }
            }
        }
        for name in g.rules.keys() {
            if !patterns.iter().any(|p| p.name() == name) {
                bail!("rules.toml: [game.rules.{name}] is not a game pattern");
            }
        }
        let judges = crate::game::rules::judges();
        for j in &judges {
            if !g.judges.contains_key(j.name()) {
                bail!("rules.toml: missing [game.judges.{}]", j.name());
            }
        }
        for name in g.judges.keys() {
            if !judges.iter().any(|j| j.name() == name) {
                bail!("rules.toml: [game.judges.{name}] is not a game judge");
            }
        }
        let t = &g.tuning;
        let positive = [
            ("smoothing_rate", t.smoothing_rate),
            ("jump_buffer", t.jump_buffer),
            ("coyote_time", t.coyote_time),
            ("walk_speed", t.walk_speed),
            ("shot_cooldown", t.shot_cooldown),
            ("shot_size", t.shot_size),
            ("projectile_range", t.projectile_range),
            ("safe_seconds", t.safe_seconds),
            ("blink_rate", t.blink_rate),
            ("music_bpm", t.music_bpm),
            ("hud_size", t.hud_size),
            ("banner_size", t.banner_size),
            ("swim_turn_rate", t.swim_turn_rate),
            ("swim_smooth_turn", t.swim_smooth_turn),
            ("dart_cycle", t.dart_cycle),
            ("dart_secs", t.dart_secs),
            ("dart_boost", t.dart_boost),
            ("dart_rate", t.dart_rate),
            ("glide_rate", t.glide_rate),
            ("drift_rate", t.drift_rate),
            ("lurk_rate", t.lurk_rate),
            ("lurk_turn_secs", t.lurk_turn_secs),
            ("depth_rate", t.depth_rate),
            ("depth_follow", t.depth_follow),
            ("swim_wave_rate", t.swim_wave_rate),
            ("school_radius", t.school_radius),
            ("nose_speed", t.nose_speed),
            ("nose_ease", t.nose_ease),
            ("lunge_secs", t.lunge_secs),
            ("jig_lift_secs", t.jig_lift_secs),
            ("jig_secs", t.jig_secs),
            ("flutter_sway", t.flutter_sway),
            ("flutter_rate", t.flutter_rate),
            ("draw_ease", t.draw_ease),
        ];
        for (k, v) in positive {
            if !(v > 0.0) {
                bail!("rules.toml: game.tuning.{k} must be > 0 (got {v})");
            }
        }
        let non_negative = [
            ("jump_release_gravity", t.jump_release_gravity),
            ("shot_offset", t.shot_offset),
            ("stomp_height", t.stomp_height),
            ("chase_dead_zone", t.chase_dead_zone),
            ("line_gap", t.line_gap),
            ("face_speed", t.face_speed),
            ("school_spacing", t.school_spacing),
            ("school_pull", t.school_pull),
            ("school_push", t.school_push),
            ("lunge_rest", t.lunge_rest),
        ];
        for (k, v) in non_negative {
            if !(v >= 0.0) {
                bail!("rules.toml: game.tuning.{k} must be >= 0 (got {v})");
            }
        }
        for (k, v) in [
            ("squash_amount", t.squash_amount),
            ("wander_jitter", t.wander_jitter),
            ("blink_dim", t.blink_dim),
            ("swim_clock_jitter", t.swim_clock_jitter),
            ("dart_turn_chance", t.dart_turn_chance),
            ("drift_swing", t.drift_swing),
            ("lurk_low", t.lurk_low),
            ("lurk_turn_chance", t.lurk_turn_chance),
            ("depth_share", t.depth_share),
        ] {
            if !(0.0..1.0).contains(&v) {
                bail!("rules.toml: game.tuning.{k} must be in 0..1 (got {v})");
            }
        }
        if t.jig_lift_secs >= t.jig_secs {
            bail!("rules.toml: game.tuning.jig_lift_secs must be shorter than jig_secs");
        }
        if t.dart_secs >= t.dart_cycle {
            bail!("rules.toml: game.tuning.dart_secs must be shorter than dart_cycle");
        }
        if t.lives < 1 {
            bail!("rules.toml: game.tuning.lives must be at least 1");
        }
        if !crate::game::lexicon::WAVES.contains(&t.music_wave.as_str()) {
            bail!("rules.toml: game.tuning.music_wave must be one of {:?}", crate::game::lexicon::WAVES);
        }
        for (k, p) in [
            ("top_left", t.top_left),
            ("top_right", t.top_right),
            ("top", t.top),
            ("middle", t.middle),
            ("bottom", t.bottom),
            ("banner", t.banner),
        ] {
            if !p.iter().all(|x| (0.0..=1.0).contains(x)) {
                bail!("rules.toml: game.tuning.{k} must be screen fractions in 0..1");
            }
        }
        Ok(())
    }

    pub fn game_rule(&self, name: &str) -> &RuleConfig {
        &self.game.rules[name]
    }

    pub fn game_judge(&self, name: &str) -> &JudgeConfig {
        &self.game.judges[name]
    }

    /// Score of one game pattern variant under a profile.
    pub fn game_variant_score(&self, rule: &str, variant: &str, profile: &Axes) -> f32 {
        let rc = self.game_rule(rule);
        rc.weight * rc.variants[variant].dot(profile)
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
    fn game_tuning_typo_is_rejected() {
        let broken = DEFAULT_RULES.replace("stomp_height =", "stomp_hieght =");
        let err = Config::parse(&broken).unwrap_err();
        assert!(format!("{err:#}").contains("stomp_hieght"), "{err:#}");
    }

    #[test]
    fn game_variant_typo_is_rejected() {
        let broken = DEFAULT_RULES.replace("variants.on_press", "variants.on_pres");
        assert!(Config::parse(&broken).is_err());
    }

    #[test]
    fn unknown_game_rule_is_rejected() {
        let broken = format!("{DEFAULT_RULES}\n[game.rules.teleport]\nweight = 1.0\nvariants.zap = {{ safety = 1.0 }}\n");
        assert!(Config::parse(&broken).is_err());
    }

    #[test]
    fn game_tuning_out_of_range_is_rejected() {
        let broken = DEFAULT_RULES.replace("blink_dim = 0.35", "blink_dim = 3.5");
        assert!(Config::parse(&broken).is_err());
    }

    #[test]
    fn missing_variant_is_rejected() {
        let broken = DEFAULT_RULES.replace("variants.atomic", "variants.atomik");
        assert!(Config::parse(&broken).is_err());
    }
}
