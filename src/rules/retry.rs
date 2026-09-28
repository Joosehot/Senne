//! Retry: wraps a fallible expression so it runs up to N times before the
//! error strategy sees the error. Styles: an inline loop or a shared helper
//! function, each with a fixed delay or exponential backoff.

use crate::codegen::Gen;
use crate::parser::RetrySpec;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RetryStyle {
    LoopFixed,
    LoopBackoff,
    HelperFixed,
    HelperBackoff,
}

pub const RETRY_STYLES: &[RetryStyle] = &[
    RetryStyle::LoopFixed,
    RetryStyle::LoopBackoff,
    RetryStyle::HelperFixed,
    RetryStyle::HelperBackoff,
];

impl RetryStyle {
    pub fn key(self) -> &'static str {
        match self {
            RetryStyle::LoopFixed => "loop_fixed",
            RetryStyle::LoopBackoff => "loop_backoff",
            RetryStyle::HelperFixed => "helper_fixed",
            RetryStyle::HelperBackoff => "helper_backoff",
        }
    }
    pub fn is_helper(self) -> bool {
        matches!(self, RetryStyle::HelperFixed | RetryStyle::HelperBackoff)
    }
    pub fn backoff(self) -> bool {
        matches!(self, RetryStyle::LoopBackoff | RetryStyle::HelperBackoff)
    }
}

/// Styles the sentence allows. "with backoff" forces backoff; an explicit
/// wait ("200ms between attempts") without it forces a fixed delay.
pub fn valid(spec: &RetrySpec, standalone: bool) -> Vec<RetryStyle> {
    RETRY_STYLES
        .iter()
        .copied()
        .filter(|s| !standalone || s.is_helper())
        .filter(|s| {
            if spec.backoff {
                s.backoff()
            } else if spec.delay_ms.is_some() {
                !s.backoff()
            } else {
                true
            }
        })
        .collect()
}

pub struct Resolved {
    pub times: u32,
    pub delay_ms: u64,
}

pub fn resolve(g: &Gen, style: RetryStyle, spec: &RetrySpec) -> Resolved {
    let r = &g.cfg.retry;
    Resolved {
        times: if spec.times == 0 { r.default_times } else { spec.times },
        delay_ms: spec
            .delay_ms
            .unwrap_or(if style.backoff() { r.base_delay_ms } else { r.fixed_delay_ms }),
    }
}

/// Wrap `expr`; returns the expression holding the final `Result`.
pub fn wrap(g: &mut Gen, style: RetryStyle, spec: &RetrySpec, expr: &str, base: &str) -> String {
    let Resolved { times, delay_ms } = resolve(g, style, spec);
    g.import("std::thread");
    g.import("std::time::Duration");
    if style.is_helper() {
        g.add_helper("retry", shared_helper());
        return format!("retry({times}, {delay_ms}, {}, || {expr})", style.backoff());
    }
    let var = g.fresh(&format!("{base}_result"));
    let sleep = if style.backoff() {
        format!("thread::sleep(Duration::from_millis({delay_ms} << (attempt - 1)));")
    } else {
        format!("thread::sleep(Duration::from_millis({delay_ms}));")
    };
    g.open(format!("let {var} = {{"));
    g.line("let mut attempt = 1;");
    g.open("loop {");
    g.open(format!("match {expr} {{"));
    g.line("Ok(value) => break Ok(value),");
    g.line(format!("Err(err) if attempt >= {times} => break Err(err),"));
    g.open("Err(_) => {");
    g.line(sleep);
    g.line("attempt += 1;");
    g.close("}");
    g.close("}");
    g.close("}");
    g.close("};");
    var
}

fn shared_helper() -> String {
    "\
/// Runs `op` up to `attempts` times, sleeping `delay_ms` between tries
/// (doubling it after every failure when `backoff` is set).
fn retry<T, E>(
    attempts: u32,
    delay_ms: u64,
    backoff: bool,
    mut op: impl FnMut() -> Result<T, E>,
) -> Result<T, E> {
    let mut attempt = 1;
    loop {
        match op() {
            Ok(value) => return Ok(value),
            Err(err) if attempt >= attempts => return Err(err),
            Err(_) => {
                let factor = if backoff { 1 << (attempt - 1) } else { 1 };
                thread::sleep(Duration::from_millis(delay_ms * factor));
                attempt += 1;
            }
        }
    }
}"
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(backoff: bool, delay: Option<u64>) -> RetrySpec {
        RetrySpec { times: 3, backoff, delay_ms: delay }
    }

    #[test]
    fn backoff_forces_backoff_styles() {
        assert!(valid(&spec(true, None), false).iter().all(|s| s.backoff()));
    }

    #[test]
    fn explicit_wait_forces_fixed() {
        assert!(valid(&spec(false, Some(200)), false).iter().all(|s| !s.backoff()));
    }

    #[test]
    fn standalone_is_helper_only() {
        assert!(valid(&spec(false, None), true).iter().all(|s| s.is_helper()));
    }
}
