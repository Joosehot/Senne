//! `--explain`: tokens, clause tree, and for every clause the candidates the
//! search weighed, with the winner marked. The answer to "why this code?"

use crate::config::Config;
use crate::parser::{Clause, Program};
use crate::search::{by_score, candidates, Outcome};
use std::fmt::Write as _;

pub fn explain(prog: &Program, out: &Outcome, cfg: &Config) -> String {
    let mut s = String::new();
    let toks: Vec<String> = prog.tokens.iter().map(|t| t.to_string()).collect();
    let _ = writeln!(s, "tokens: {}", toks.join(" "));
    let _ = writeln!(s);
    for c in &prog.clauses {
        clause(&mut s, c, out, cfg, 0);
    }
    let _ = writeln!(s);
    if out.global_notes.is_empty() {
        let _ = writeln!(s, "judges: (none fired)");
    } else {
        let _ = writeln!(s, "judges: {}", out.global_notes.join("; "));
    }
    let margin = out
        .runner_up
        .map(|r| format!(", runner-up {r:.2} (margin {:.2})", out.score - r))
        .unwrap_or_default();
    let _ = writeln!(s, "total: {:.2}{margin}, {} candidates scored", out.score, out.explored);
    s
}

fn clause(s: &mut String, c: &Clause, out: &Outcome, cfg: &Config, depth: usize) {
    let pad = "  ".repeat(depth);
    let chosen = &out.choices[&c.id];
    let (env, prof) = &out.seen[&c.id];
    let _ = writeln!(
        s,
        "{pad}#{} \"{}\"   (it = {}; profile safety {:.1} brevity {:.1} perf {:.1})",
        c.id,
        c.words,
        env.ty.describe(),
        prof.safety,
        prof.brevity,
        prof.perf
    );
    if let Ok(mut cands) = candidates(c, env, prof, cfg) {
        cands.sort_by(by_score);
        for cand in cands {
            let mark = if cand.choice == *chosen { "=>" } else { "  " };
            let notes = if cand.choice.notes.is_empty() {
                String::new()
            } else {
                format!("   [{}]", cand.choice.notes.join("; "))
            };
            let _ = writeln!(s, "{pad}  {mark} {:+.2}  {}{notes}", cand.choice.score, cand.choice.label());
        }
    }
    for h in &c.on_error {
        let _ = writeln!(s, "{pad}    on error: \"{}\"", h.words);
    }
    for b in &c.body {
        clause(s, b, out, cfg, depth + 1);
    }
}
