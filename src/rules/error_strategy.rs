//! How a fallible clause deals with its error. Not a pattern of its own:
//! the search pairs every fallible variant with each strategy allowed here.
//!
//! - an "if it fails, ..." handler forces `handle`
//! - a requested fallback ("or 0") forces `or_default`
//! - otherwise `propagate` (`?`) competes with `expect` (panic); workers
//!   can't use `?` because a thread can't return from `run()`

use super::{retry, Choice, Env, Fallible, Msg};
use crate::codegen::{mentions, Gen};
use crate::lexicon::Verb;
use crate::model::{fallback_expr, fmt_text, Ty, Value};
use crate::parser::Clause;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Strategy {
    Propagate,
    Expect,
    OrDefault,
    Handle,
}

pub const STRATEGIES: &[Strategy] =
    &[Strategy::Propagate, Strategy::Expect, Strategy::OrDefault, Strategy::Handle];

impl Strategy {
    pub fn key(self) -> &'static str {
        match self {
            Strategy::Propagate => "propagate",
            Strategy::Expect => "expect",
            Strategy::OrDefault => "or_default",
            Strategy::Handle => "handle",
        }
    }
}

/// Strategies allowed for a fallible clause whose success value is `ok`,
/// or why there are none.
pub fn valid(c: &Clause, env: &Env, ok: Ty) -> Result<Vec<Strategy>, String> {
    if let Some(fb) = &c.fallback {
        if ok != Ty::Unit && fallback_expr(fb, ok).is_none() {
            return Err(format!(
                "the fallback in \"{}\" doesn't fit {}",
                c.words,
                ok.describe()
            ));
        }
    }
    if !c.on_error.is_empty() {
        for h in &c.on_error {
            if !matches!(h.verb, Verb::Log | Verb::Print | Verb::Continue | Verb::Stop | Verb::Wait) {
                return Err(format!(
                    "\"{}\" can't run inside an error handler (only log, print, wait, continue or stop)",
                    h.words
                ));
            }
            if h.verb == Verb::Stop && env.in_worker {
                return Err(format!(
                    "\"{}\": a worker thread can't stop run(); say \"log the error and continue\"",
                    h.words
                ));
            }
        }
        // "if it fails, give up" is just `?`
        if c.on_error.iter().all(|h| h.verb == Verb::Stop) && !env.in_worker {
            return Ok(vec![Strategy::Propagate]);
        }
        return Ok(vec![Strategy::Handle]);
    }
    if c.fallback.is_some() {
        return Ok(vec![Strategy::OrDefault]);
    }
    if env.in_worker {
        Ok(vec![Strategy::Expect])
    } else {
        Ok(vec![Strategy::Propagate, Strategy::Expect])
    }
}

/// `.expect("...")`, or a `panic!` with arguments when the message needs them.
fn expect_call(msg: &Msg) -> String {
    let literal = msg.fmt.replace("{{", "").replace("}}", "");
    if msg.args.is_empty() && !literal.contains('{') {
        let plain = msg.fmt.replace("{{", "{").replace("}}", "}");
        format!(".expect(\"{plain}\")")
    } else {
        format!(".unwrap_or_else(|err| panic!(\"{}: {{err}}\"{}))", msg.fmt, args(msg))
    }
}

fn args(msg: &Msg) -> String {
    msg.args.iter().map(|a| format!(", {a}")).collect()
}

/// Bind the result of `f` according to the chosen strategy. Returns the new
/// value, or `input` (marked used) when the operation only has an effect.
pub fn bind(g: &mut Gen, c: &Clause, choice: &Choice, f: Fallible, input: &Value) -> Value {
    let unit = f.ok == Ty::Unit;
    let name = if unit {
        String::new()
    } else {
        let base = f.name.clone().unwrap_or_else(|| f.ok.name().to_string());
        g.fresh(&base)
    };
    let mut expr = f.expr.clone();
    if let (Some(style), Some(spec)) = (choice.retry, &c.retry) {
        let base = if unit { "attempt" } else { name.as_str() };
        expr = retry::wrap(g, style, spec, &expr, base);
    }
    let strategy = choice.strategy.expect("fallible clause has a strategy");
    match strategy {
        Strategy::Propagate => {
            g.returns_result = true;
            if unit {
                g.line(format!("{expr}?;"));
            } else {
                g.line(format!("let {name} = {expr}?;"));
            }
        }
        Strategy::Expect => {
            let tail = expect_call(&f.msg);
            if unit {
                g.line(format!("{expr}{tail};"));
            } else {
                g.line(format!("let {name} = {expr}{tail};"));
            }
        }
        Strategy::OrDefault => {
            if unit {
                g.line(format!("let _ = {expr};"));
            } else {
                let fb = c.fallback.as_ref().expect("or_default needs a fallback");
                let value = fallback_expr(fb, f.ok).expect("search checked the fallback type");
                let tail = match fb {
                    crate::parser::Fallback::Default => ".unwrap_or_default()".to_string(),
                    _ if matches!(f.ok, Ty::One(crate::model::Scalar::Str)) => {
                        format!(".unwrap_or_else(|_| {value})")
                    }
                    _ => format!(".unwrap_or({value})"),
                };
                g.line(format!("let {name} = {expr}{tail};"));
            }
        }
        Strategy::Handle => handle(g, c, &f, &expr, &name),
    }
    // An effect (write, mkdir) only "uses" the value if it mentions it.
    if unit {
        if mentions(&expr, &input.name) || mentions(&f.expr, &input.name) {
            input.sunk()
        } else {
            input.clone()
        }
    } else {
        g.value(name, f.ok, false)
    }
}

/// `match` with the sentence's own "if it fails, ..." clauses in the Err arm.
fn handle(g: &mut Gen, c: &Clause, f: &Fallible, expr: &str, name: &str) {
    let mut stmts: Vec<String> = Vec::new();
    let mut diverges = false;
    let mut uses_err = false;
    for h in &c.on_error {
        match h.verb {
            Verb::Log | Verb::Print => {
                let mac = if h.verb == Verb::Log { "eprintln" } else { "println" };
                let line = match &h.literal {
                    Some(text) => format!("{mac}!(\"{}: {{err}}\");", fmt_text(text)),
                    None => format!("{mac}!(\"{}: {{err}}\"{});", f.msg.fmt, args(&f.msg)),
                };
                stmts.push(line);
                uses_err = true;
            }
            Verb::Wait => {
                if let Some(ms) = h.millis {
                    g.import("std::thread");
                    g.import("std::time::Duration");
                    stmts.push(format!("thread::sleep(Duration::from_millis({ms}));"));
                }
            }
            Verb::Continue => {
                if g.in_loop {
                    stmts.push("continue;".into());
                    diverges = true;
                    break;
                }
            }
            Verb::Stop => {
                g.returns_result = true;
                stmts.push("return Err(err.into());".into());
                uses_err = true;
                diverges = true;
                break;
            }
            _ => unreachable!("search rejects other handler verbs"),
        }
    }
    let err = if uses_err { "err" } else { "_" };
    if f.ok == Ty::Unit {
        if stmts.is_empty() {
            g.line(format!("let _ = {expr};"));
        } else {
            if uses_err {
                g.open(format!("if let Err(err) = {expr} {{"));
            } else {
                g.open(format!("if {expr}.is_err() {{"));
            }
            for s in &stmts {
                g.line(s);
            }
            g.close("}");
        }
        return;
    }
    let fallback = match &c.fallback {
        Some(fb) => fallback_expr(fb, f.ok).expect("search checked the fallback type"),
        None => f.ok.default_expr(),
    };
    if fallback.contains("PathBuf") {
        g.import("std::path::PathBuf");
    }
    g.open(format!("let {name} = match {expr} {{"));
    g.line("Ok(value) => value,");
    if stmts.is_empty() {
        g.line(format!("Err({err}) => {fallback},"));
    } else {
        g.open(format!("Err({err}) => {{"));
        for s in &stmts {
            g.line(s);
        }
        if !diverges {
            g.line(&fallback);
        }
        g.close("}");
    }
    g.close("};");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Scalar;
    use crate::rules::test_util::*;

    #[test]
    fn handler_forces_handle() {
        let c = clause("parse it as an integer, and if it fails, log the error and continue");
        let got = valid(&c, &env(Ty::One(Scalar::Str)), Ty::One(Scalar::Int)).unwrap();
        assert_eq!(got, vec![Strategy::Handle]);
    }

    #[test]
    fn workers_cannot_propagate() {
        let c = clause("parse it as an integer");
        let e = Env { in_worker: true, ..env(Ty::One(Scalar::Str)) };
        assert_eq!(valid(&c, &e, Ty::One(Scalar::Int)).unwrap(), vec![Strategy::Expect]);
    }

    #[test]
    fn mismatched_fallback_is_rejected() {
        let c = clause("parse it as an integer or \"abc\"");
        assert!(valid(&c, &env(Ty::One(Scalar::Str)), Ty::One(Scalar::Int)).is_err());
    }
}
