//! "read the file", "load \"config.toml\"" -> the whole file as a String.

use super::{path_arg, Env, Fallible, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::{Noun, Verb};
use crate::model::{Scalar, Ty, Value};
use crate::parser::Clause;

pub struct ReadFile;

impl Pattern for ReadFile {
    fn name(&self) -> &'static str {
        "read_file"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["to_string"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Read && ![Noun::Lines, Noun::Env, Noun::Dir, Noun::Files].iter().any(|n| c.has(*n))
    }
    fn variants(&self, _c: &Clause, _env: &Env) -> Vec<Variant> {
        vec![Variant::new("to_string", Ty::One(Scalar::Str)).fallible()]
    }
    fn emit(&self, _variant: &str, c: &Clause, g: &mut Gen, input: &Value) -> Out {
        g.import("std::fs");
        let p = path_arg(c, input, g, "path", "read", true);
        Out::Fallible(Fallible {
            expr: format!("fs::read_to_string({})", p.expr),
            ok: Ty::One(Scalar::Str),
            msg: p.msg,
            name: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn matches_read_file() {
        assert!(ReadFile.matches(&clause("read the file \"a.txt\"")));
    }

    #[test]
    fn leaves_lines_and_env_to_others() {
        assert!(!ReadFile.matches(&clause("read the lines of \"a.txt\"")));
        assert!(!ReadFile.matches(&clause("read the environment variable PORT")));
    }
}
