//! The v0 proof:
//! 1. determinism: every example compiles to byte-identical Rust, run after run
//! 2. stability: the output matches the checked-in golden file
//! 3. validity: every golden file compiles with `rustc -D warnings`
//!
//! Regenerate goldens after an intended change with `SENNE_BLESS=1 cargo test`.

use senne::config::Config;
use senne::{compile, Options};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn examples() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut out: Vec<(String, String)> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .map(|p| {
            let stem = p.file_stem().unwrap().to_string_lossy().into_owned();
            (stem, fs::read_to_string(&p).unwrap())
        })
        .collect();
    out.sort();
    assert!(!out.is_empty(), "no examples found");
    out
}

fn golden_path(stem: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/out").join(format!("{stem}.rs"))
}

#[test]
fn examples_are_deterministic_and_match_goldens() {
    let cfg = Config::builtin();
    let bless = std::env::var_os("SENNE_BLESS").is_some();
    let mut failures = Vec::new();
    for (stem, sentence) in examples() {
        let first = compile(&sentence, &cfg, &Options::default())
            .unwrap_or_else(|d| panic!("{stem}: {d:?}"))
            .code;
        for _ in 0..5 {
            let again = compile(&sentence, &cfg, &Options::default()).unwrap().code;
            assert_eq!(first, again, "{stem}: output changed between runs");
        }
        let path = golden_path(&stem);
        if bless {
            fs::write(&path, &first).unwrap();
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(golden) if golden.replace("\r\n", "\n") == first => {}
            Ok(_) => failures.push(format!("{stem}: differs from {}", path.display())),
            Err(_) => failures.push(format!("{stem}: missing {}", path.display())),
        }
    }
    assert!(failures.is_empty(), "{}\n(run with SENNE_BLESS=1 to accept)", failures.join("\n"));
}

#[test]
fn goldens_compile_without_warnings() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let tmp = std::env::temp_dir().join(format!("senne-golden-{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap();
    let mut failures = Vec::new();
    for (stem, _) in examples() {
        let path = golden_path(&stem);
        if !path.exists() {
            continue; // the other test reports it
        }
        let out = Command::new(&rustc)
            .args(["--edition", "2021", "--crate-type", "lib", "--emit", "metadata", "-D", "warnings"])
            .arg("--crate-name")
            .arg(format!("golden_{}", stem.replace('-', "_")))
            .arg("--out-dir")
            .arg(&tmp)
            .arg(&path)
            .output()
            .expect("running rustc");
        if !out.status.success() {
            failures.push(format!("{stem}:\n{}", String::from_utf8_lossy(&out.stderr)));
        }
    }
    let _ = fs::remove_dir_all(&tmp);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn unknown_words_fail_with_a_hint() {
    let cfg = Config::builtin();
    let err = compile("retyr reading \"a.txt\"", &cfg, &Options::default()).err().unwrap();
    assert!(err[0].message.contains("retyr"));
    assert!(err[0].hint.as_deref().unwrap().contains("retry"));
}

#[test]
fn lenient_mode_ignores_unknown_words() {
    let cfg = Config::builtin();
    let opts = Options { lenient: true, ..Options::default() };
    assert!(compile("kindly read the file \"a.txt\"", &cfg, &opts).is_ok());
}

#[test]
fn modifiers_change_the_tradeoff() {
    let cfg = Config::builtin();
    let plain = compile("read the file \"a.txt\" and print it", &cfg, &Options::default()).unwrap().code;
    let terse = compile("simply read the file \"a.txt\" and print it", &cfg, &Options::default()).unwrap().code;
    assert!(plain.contains("?;"), "default should propagate:\n{plain}");
    assert!(terse.contains(".expect("), "\"simply\" should accept a panic:\n{terse}");
}
