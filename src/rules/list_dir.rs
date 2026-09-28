//! "list the files in \"logs\"" -> Vec<PathBuf> (files only when "files"
//! is said, every entry for "directory").

use super::{path_arg, Env, Fallible, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::{Noun, Verb};
use crate::model::{Scalar, Ty, Value};
use crate::parser::Clause;

pub struct ListDir;

impl Pattern for ListDir {
    fn name(&self) -> &'static str {
        "list_dir"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["read_dir"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::List || (c.verb == Verb::Read && (c.has(Noun::Dir) || c.has(Noun::Files)))
    }
    fn variants(&self, _c: &Clause, _env: &Env) -> Vec<Variant> {
        vec![Variant::new("read_dir", Ty::Many(Scalar::Path)).fallible()]
    }
    fn emit(&self, _variant: &str, c: &Clause, g: &mut Gen, input: &Value) -> Out {
        g.import("std::fs");
        let p = path_arg(c, input, g, "dir", "list", true);
        let mut expr = format!(
            "fs::read_dir({}).and_then(|entries| entries.map(|entry| entry.map(|e| e.path())).collect::<Result<Vec<_>, _>>())",
            p.expr
        );
        if c.has(Noun::Files) {
            expr.push_str(".map(|paths| paths.into_iter().filter(|p| p.is_file()).collect::<Vec<_>>())");
        }
        Out::Fallible(Fallible { expr, ok: Ty::Many(Scalar::Path), msg: p.msg, name: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn matches_list() {
        assert!(ListDir.matches(&clause("list the files in \"logs\"")));
    }

    #[test]
    fn ignores_reading_one_file() {
        assert!(!ListDir.matches(&clause("read the file \"a.txt\"")));
    }
}
