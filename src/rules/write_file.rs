//! "write it to \"out.txt\"". `plain` is fs::write; `atomic` writes a temp
//! file and renames it over the target, so a crash never leaves half a file.

use super::{path_arg, Env, Fallible, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::{Modifier, Verb};
use crate::model::{Scalar, Ty, Value};
use crate::parser::Clause;

pub struct WriteFile;

impl Pattern for WriteFile {
    fn name(&self) -> &'static str {
        "write_file"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["plain", "atomic"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Write
    }
    fn variants(&self, c: &Clause, env: &Env) -> Vec<Variant> {
        let writable = matches!(env.ty, Ty::One(s) | Ty::Many(s) if s != Scalar::Path);
        if !writable {
            return vec![];
        }
        let plain = Variant::new("plain", env.ty).fallible().sink();
        let atomic = Variant::new("atomic", env.ty).fallible().sink();
        if c.has_mod(Modifier::Atomically) {
            vec![atomic]
        } else {
            vec![plain, atomic]
        }
    }
    fn emit(&self, variant: &str, c: &Clause, g: &mut Gen, input: &Value) -> Out {
        g.import("std::fs");
        let data = match input.ty {
            Ty::One(Scalar::Str) if input.borrowed => input.name.clone(),
            Ty::One(Scalar::Str) => format!("&{}", input.name),
            Ty::Many(Scalar::Str) => format!("{}.join(\"\\n\")", input.name),
            Ty::One(_) => format!("{}.to_string()", input.name),
            _ => format!(
                "{}.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(\"\\n\")",
                input.name
            ),
        };
        let p = path_arg(c, input, g, "out_path", "write", false);
        let expr = if variant == "atomic" {
            g.import("std::path::Path");
            let tmp = g.fresh("tmp");
            g.line(format!("let {tmp} = {}.with_extension(\"tmp\");", p.path_expr));
            format!("fs::write(&{tmp}, {data}).and_then(|()| fs::rename(&{tmp}, {}))", p.expr)
        } else {
            format!("fs::write({}, {data})", p.expr)
        };
        Out::Fallible(Fallible { expr, ok: Ty::Unit, msg: p.msg, name: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn atomically_forces_atomic() {
        let c = clause("atomically write it to \"out.txt\"");
        assert_eq!(names(WriteFile.variants(&c, &env(Ty::One(Scalar::Str)))), vec!["atomic"]);
    }

    #[test]
    fn nothing_to_write() {
        let c = clause("write it to \"out.txt\"");
        assert!(WriteFile.variants(&c, &env(Ty::Unit)).is_empty());
    }
}
