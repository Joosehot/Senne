//! "print it", "print \"done\"", "print the total". Lists print one item per
//! line (`each_line`) or as a debug list (`debug`, shorter, uglier).

use super::{Env, Out, Pattern, Variant};
use crate::codegen::Gen;
use crate::lexicon::Verb;
use crate::model::{fmt_text, Scalar, Ty, Value};
use crate::parser::Clause;

pub struct Print;

impl Pattern for Print {
    fn name(&self) -> &'static str {
        "print"
    }
    fn all_variants(&self) -> &'static [&'static str] {
        &["display", "each_line", "debug"]
    }
    fn matches(&self, c: &Clause) -> bool {
        c.verb == Verb::Print
    }
    fn variants(&self, c: &Clause, env: &Env) -> Vec<Variant> {
        match env.ty {
            Ty::Unit if c.literal.is_none() => vec![],
            Ty::Unit | Ty::One(_) => vec![Variant::new("display", env.ty).sink()],
            Ty::Many(_) => vec![
                Variant::new("each_line", env.ty).sink(),
                Variant::new("debug", env.ty).sink(),
            ],
        }
    }
    fn emit(&self, variant: &str, c: &Clause, g: &mut Gen, input: &Value) -> Out {
        print_value(g, "println", variant, c, input);
        Out::Done(if input.ty == Ty::Unit { input.clone() } else { input.sunk() })
    }
}

/// Shared by print and log.
pub fn print_value(g: &mut Gen, mac: &str, variant: &str, c: &Clause, input: &Value) {
    let prefix = c.literal.as_deref().map(|l| {
        let t = fmt_text(l);
        if t.ends_with(' ') { t } else { format!("{t} ") }
    });
    let v = &input.name;
    match input.ty {
        Ty::Unit => {
            let text = fmt_text(c.literal.as_deref().unwrap_or_default());
            g.line(format!("{mac}!(\"{text}\");"));
        }
        Ty::One(s) => {
            let p = prefix.unwrap_or_default();
            if s == Scalar::Path {
                g.line(format!("{mac}!(\"{p}{{}}\", {v}.display());"));
            } else {
                g.line(format!("{mac}!(\"{p}{{{v}}}\");"));
            }
        }
        Ty::Many(s) => {
            if let Some(p) = &prefix {
                g.line(format!("{mac}!(\"{}\");", p.trim_end()));
            }
            if variant == "debug" {
                g.line(format!("{mac}!(\"{{{v}:?}}\");"));
            } else {
                let item = g.fresh(s.item_name());
                g.open(format!("for {item} in &{v} {{"));
                if s == Scalar::Path {
                    g.line(format!("{mac}!(\"{{}}\", {item}.display());"));
                } else {
                    g.line(format!("{mac}!(\"{{{item}}}\");"));
                }
                g.close("}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::*;

    #[test]
    fn lists_have_two_styles() {
        let c = clause("print them");
        assert_eq!(names(Print.variants(&c, &env(Ty::Many(Scalar::Int)))), vec!["each_line", "debug"]);
    }

    #[test]
    fn nothing_to_print() {
        let c = clause("print it");
        assert!(Print.variants(&c, &env(Ty::Unit)).is_empty());
    }
}
