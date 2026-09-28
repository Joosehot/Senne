//! "read the lines of \"urls.txt\"" -> Vec<String>. `collect` reads the whole
//! file then splits; `buffered` streams through a BufReader (better for large
//! files, more code).

use super::{path_arg, Env, Fallible, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::{Noun, Verb};
use crate::model::{Scalar, Ty, Value};
use crate::parser::Clause;

pub struct ReadLines;

impl Pattern for ReadLines {
    fn name(&self) -> &'static str {
        "read_lines"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["collect", "buffered"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Read && c.has(Noun::Lines)
    }
    fn variants(&self, _c: &Clause, _env: &Env) -> Vec<Variant> {
        let out = Ty::Many(Scalar::Str);
        vec![
            Variant::new("collect", out).fallible(),
            Variant::new("buffered", out).fallible(),
        ]
    }
    fn emit(&self, variant: &str, c: &Clause, g: &mut Gen, input: &Value) -> Out {
        let p = path_arg(c, input, g, "path", "read", true);
        let expr = match variant {
            "collect" => {
                g.import("std::fs");
                format!(
                    "fs::read_to_string({}).map(|text| text.lines().map(str::to_owned).collect::<Vec<_>>())",
                    p.expr
                )
            }
            _ => {
                g.import("std::fs::File");
                g.import("std::io::{BufRead, BufReader}");
                format!(
                    "File::open({}).and_then(|file| BufReader::new(file).lines().collect::<Result<Vec<_>, _>>())",
                    p.expr
                )
            }
        };
        Out::Fallible(Fallible { expr, ok: Ty::Many(Scalar::Str), msg: p.msg, name: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn matches_lines() {
        assert!(ReadLines.matches(&clause("read the lines of \"a.txt\"")));
    }

    #[test]
    fn ignores_plain_read() {
        assert!(!ReadLines.matches(&clause("read the file \"a.txt\"")));
    }
}
