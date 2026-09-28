//! The value flowing through a sentence ("it"), and its Rust type.

use crate::parser::Fallback;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scalar {
    Str,
    Int,
    Float,
    Path,
    Usize,
}

impl Scalar {
    pub fn rust(self) -> &'static str {
        match self {
            Scalar::Str => "String",
            Scalar::Int => "i64",
            Scalar::Float => "f64",
            Scalar::Path => "PathBuf",
            Scalar::Usize => "usize",
        }
    }
    pub fn is_numeric(self) -> bool {
        matches!(self, Scalar::Int | Scalar::Float | Scalar::Usize)
    }
    /// Copy types are iterated by value.
    pub fn is_copy(self) -> bool {
        self.is_numeric()
    }
    pub fn one_name(self) -> &'static str {
        match self {
            Scalar::Str => "text",
            Scalar::Int => "n",
            Scalar::Float => "x",
            Scalar::Path => "path",
            Scalar::Usize => "count",
        }
    }
    pub fn item_name(self) -> &'static str {
        match self {
            Scalar::Str => "line",
            Scalar::Int => "n",
            Scalar::Float => "x",
            Scalar::Path => "path",
            Scalar::Usize => "n",
        }
    }
    pub fn many_name(self) -> &'static str {
        match self {
            Scalar::Str => "lines",
            Scalar::Int => "numbers",
            Scalar::Float => "values",
            Scalar::Path => "paths",
            Scalar::Usize => "counts",
        }
    }
    pub fn default_expr(self) -> &'static str {
        match self {
            Scalar::Str => "String::new()",
            Scalar::Int => "0",
            Scalar::Float => "0.0",
            Scalar::Path => "PathBuf::new()",
            Scalar::Usize => "0",
        }
    }
    pub fn describe(self) -> &'static str {
        match self {
            Scalar::Str => "text",
            Scalar::Int => "an integer",
            Scalar::Float => "a decimal number",
            Scalar::Path => "a path",
            Scalar::Usize => "a count",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ty {
    Unit,
    One(Scalar),
    Many(Scalar),
}

impl Ty {
    pub fn rust(self) -> String {
        match self {
            Ty::Unit => "()".into(),
            Ty::One(s) => s.rust().into(),
            Ty::Many(s) => format!("Vec<{}>", s.rust()),
        }
    }
    pub fn default_expr(self) -> String {
        match self {
            Ty::Unit => "()".into(),
            Ty::One(s) => s.default_expr().into(),
            Ty::Many(_) => "Vec::new()".into(),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Ty::Unit => "result",
            Ty::One(Scalar::Str) => "contents",
            Ty::One(s) => s.one_name(),
            Ty::Many(s) => s.many_name(),
        }
    }
    pub fn describe(self) -> String {
        match self {
            Ty::Unit => "nothing yet".into(),
            Ty::One(s) => s.describe().into(),
            Ty::Many(s) => format!("a list of {}", s.describe().trim_start_matches("a ").trim_start_matches("an ")),
        }
    }
}

/// A value in scope in the generated code.
#[derive(Clone, Debug, PartialEq)]
pub struct Value {
    pub name: String,
    pub ty: Ty,
    /// `&str` / `&PathBuf` rather than an owned value.
    pub borrowed: bool,
    /// A sink (print, write) used it after it was made.
    pub consumed: bool,
    /// Bumped on every new value, so loops can tell if the body made one.
    pub version: u32,
    /// Bound with `let mut`.
    pub mutable: bool,
}

impl Value {
    pub fn unit() -> Value {
        Value { name: String::new(), ty: Ty::Unit, borrowed: false, consumed: false, version: 0, mutable: false }
    }
    pub fn scalar(&self) -> Option<Scalar> {
        match self.ty {
            Ty::One(s) | Ty::Many(s) => Some(s),
            Ty::Unit => None,
        }
    }
    /// Expression turning this value into an owned one.
    pub fn owned(&self) -> String {
        match (self.borrowed, self.ty) {
            (true, Ty::One(Scalar::Str)) => format!("{}.to_string()", self.name),
            (true, Ty::One(Scalar::Path)) => format!("{}.to_path_buf()", self.name),
            (true, _) => format!("{}.clone()", self.name),
            (false, _) => self.name.clone(),
        }
    }
    pub fn sunk(&self) -> Value {
        Value { consumed: true, ..self.clone() }
    }
}

/// Name for a list collected from values named `name` in a loop body.
pub fn plural(name: &str) -> String {
    let base = name.trim_end_matches(|c: char| c.is_ascii_digit());
    match base {
        "n" => "numbers".into(),
        "x" => "values".into(),
        "contents" | "text" => "texts".into(),
        "count" => "counts".into(),
        b if b.ends_with('s') => format!("{b}_list"),
        b => format!("{b}s"),
    }
}

/// The Rust expression for a requested fallback, if it fits the type.
pub fn fallback_expr(fb: &Fallback, ty: Ty) -> Option<String> {
    match (fb, ty) {
        (Fallback::Default, _) => Some(ty.default_expr()),
        (Fallback::Num(n), Ty::One(Scalar::Int | Scalar::Usize)) => Some(n.to_string()),
        (Fallback::Num(n), Ty::One(Scalar::Float)) => Some(format!("{n}.0")),
        (Fallback::Num(n), Ty::One(Scalar::Str)) => Some(format!("{:?}.to_string()", n.to_string())),
        (Fallback::Text(t), Ty::One(Scalar::Str)) => Some(format!("{t:?}.to_string()")),
        (Fallback::Text(t), Ty::One(Scalar::Int | Scalar::Usize)) => {
            t.trim().parse::<i64>().ok().map(|n| n.to_string())
        }
        (Fallback::Text(t), Ty::One(Scalar::Float)) => {
            t.trim().parse::<f64>().ok().map(|x| format!("{x:?}"))
        }
        _ => None,
    }
}

/// `s` as a Rust string literal.
pub fn rust_str(s: &str) -> String {
    format!("{s:?}")
}

/// `s` as text inside a format string literal (quotes escaped, braces doubled).
pub fn fmt_text(s: &str) -> String {
    let lit = rust_str(s);
    lit[1..lit.len() - 1].replace('{', "{{").replace('}', "}}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_must_fit_type() {
        let fb = Fallback::Text("abc".into());
        assert_eq!(fallback_expr(&fb, Ty::One(Scalar::Int)), None);
        assert_eq!(fallback_expr(&Fallback::Num(0), Ty::One(Scalar::Int)), Some("0".into()));
    }

    #[test]
    fn format_text_escapes_braces_and_quotes() {
        assert_eq!(fmt_text("a{b}\"c"), "a{{b}}\\\"c");
    }
}
