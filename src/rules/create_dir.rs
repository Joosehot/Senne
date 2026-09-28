//! "create the directory \"out\" if missing" -> fs::create_dir_all.

use super::{path_arg, Env, Fallible, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::{Noun, Verb};
use crate::model::{Ty, Value};
use crate::parser::Clause;

pub struct CreateDir;

impl Pattern for CreateDir {
    fn name(&self) -> &'static str {
        "create_dir"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["create_dir_all"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Create && c.has(Noun::Dir)
    }
    fn variants(&self, _c: &Clause, env: &Env) -> Vec<Variant> {
        vec![Variant::new("create_dir_all", env.ty).fallible().sink()]
    }
    fn emit(&self, _variant: &str, c: &Clause, g: &mut Gen, input: &Value) -> Out {
        g.import("std::fs");
        let p = path_arg(c, input, g, "dir", "create", false);
        Out::Fallible(Fallible {
            expr: format!("fs::create_dir_all({})", p.expr),
            ok: Ty::Unit,
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
    fn matches_directory() {
        assert!(CreateDir.matches(&clause("create the directory \"out\" if missing")));
    }

    #[test]
    fn ignores_other_creations() {
        assert!(!CreateDir.matches(&clause("create the file \"a.txt\"")));
    }
}
