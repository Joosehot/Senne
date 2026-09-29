//! `--explain` for games: per sentence, every unit with the candidates the
//! search weighed (winner marked), then the judges and the total.

use super::parser::GameProgram;
use super::rules::Facts;
use super::search::{candidates, profile, units, Outcome};
use crate::config::Config;
use std::fmt::Write as _;

pub fn explain(p: &GameProgram, out: &Outcome, cfg: &Config) -> String {
    let mut s = String::new();
    let toks: Vec<String> = p.tokens.iter().map(|t| t.tok.to_string()).collect();
    let _ = writeln!(s, "tokens: {}", toks.join(" "));
    let facts = Facts::of(p);
    let mut last_sentence = usize::MAX;
    for (i, u) in units(p).iter().enumerate() {
        let prof = profile(u.sentence, cfg);
        if u.sentence.id != last_sentence {
            last_sentence = u.sentence.id;
            let mods = if u.sentence.mods.is_empty() { String::new() } else { format!(" [{}]", u.sentence.mods.join(", ")) };
            let _ = writeln!(s);
            let _ = writeln!(
                s,
                "#{} \"{}\"{mods}   (profile safety {:.1} brevity {:.1} perf {:.1})",
                u.sentence.id + 1,
                u.sentence.text,
                prof.safety,
                prof.brevity,
                prof.perf
            );
        }
        let chosen = &out.choices[i];
        let _ = writeln!(s, "  \"{}\"", u.words);
        if let Ok(mut cands) = candidates(u, &facts, &prof, cfg) {
            cands.sort_by(|a, b| b.score.total_cmp(&a.score));
            for c in cands {
                let mark = if c.pattern == chosen.pattern && c.variant == chosen.variant { "=>" } else { "  " };
                let _ = writeln!(s, "    {mark} {:+.2}  {}", c.score, c.label());
            }
        }
    }
    let _ = writeln!(s);
    if out.judge_notes.is_empty() {
        let _ = writeln!(s, "judges: (none fired)");
    } else {
        let _ = writeln!(s, "judges: {}", out.judge_notes.join("; "));
    }
    let margin = out.runner_up.map(|r| format!(", runner-up {r:.2} (margin {:.2})", out.score - r)).unwrap_or_default();
    let _ = writeln!(s, "total: {:.2}{margin}, {} candidates scored", out.score, out.explored);
    s
}
