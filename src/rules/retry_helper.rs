//! "retry this safely three times": nothing named to retry, so emit a
//! reusable `retry` function with the count and delays baked in.

use super::retry::{self, RetryStyle};
use super::{Env, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::Verb;
use crate::model::{Ty, Value};
use crate::parser::Clause;

pub struct RetryHelper;

impl Pattern for RetryHelper {
    fn name(&self) -> &'static str {
        "retry_helper"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["helper"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Retry
    }
    fn variants(&self, _c: &Clause, _env: &Env) -> Vec<Variant> {
        vec![Variant::new("helper", Ty::Unit).sink()]
    }
    fn emit(&self, _variant: &str, c: &Clause, g: &mut Gen, input: &Value) -> Out {
        let spec = c.retry.as_ref().expect("parser attaches the spec");
        let style = g.choice(c.id).retry.expect("search picks a retry style");
        let retry::Resolved { times, delay_ms } = retry::resolve(g, style, spec);
        g.import("std::thread");
        g.import("std::time::Duration");
        let (doc, sleep) = match style {
            RetryStyle::HelperBackoff => (
                format!("doubling the delay between attempts from {delay_ms} ms"),
                "thread::sleep(Duration::from_millis(DELAY_MS << (attempt - 1)));",
            ),
            _ => (
                format!("waiting {delay_ms} ms between attempts"),
                "thread::sleep(Duration::from_millis(DELAY_MS));",
            ),
        };
        g.add_helper(
            "retry",
            format!(
                "\
/// Runs `op` up to {times} times, {doc}.
pub fn retry<T, E>(mut op: impl FnMut() -> Result<T, E>) -> Result<T, E> {{
    const ATTEMPTS: u32 = {times};
    const DELAY_MS: u64 = {delay_ms};
    let mut attempt = 1;
    loop {{
        match op() {{
            Ok(value) => return Ok(value),
            Err(err) if attempt >= ATTEMPTS => return Err(err),
            Err(_) => {{
                {sleep}
                attempt += 1;
            }}
        }}
    }}
}}"
            ),
        );
        Out::Done(input.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn matches_standalone_retry() {
        assert!(RetryHelper.matches(&clause("retry this three times")));
    }

    #[test]
    fn ignores_other_verbs() {
        assert!(!RetryHelper.matches(&clause("read the file \"a\"")));
    }
}
