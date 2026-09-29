//! The game domain's proof, like `golden.rs` for the Rust domain:
//! 1. determinism: every `examples/game/*.txt` compiles to byte-identical Rust, six times over
//! 2. stability: the output matches `examples/game/out/*.rs`
//! 3. validity: every golden type-checks with `rustc -D warnings` against a
//!    signature stub of Droid's API (`tests/game_stub/droid.rs`)
//!
//! Regenerate goldens after an intended change with `SENNE_BLESS=1 cargo test`.

use senne::config::{Config, DEFAULT_RULES};
use senne::game::{compile, vocabulary, GameOptions};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn examples() -> Vec<(String, String)> {
    let dir = root().join("examples/game");
    let mut out: Vec<(String, String)> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .map(|p| (p.file_stem().unwrap().to_string_lossy().into_owned(), fs::read_to_string(&p).unwrap()))
        .collect();
    out.sort();
    assert!(out.len() >= 6, "expected at least 6 game examples");
    out
}

fn golden_path(stem: &str) -> PathBuf {
    root().join("examples/game/out").join(format!("{stem}.rs"))
}

fn code(src: &str) -> String {
    compile(src, &Config::builtin(), &GameOptions::default()).unwrap_or_else(|d| panic!("{d:?}")).code
}

#[test]
fn game_examples_are_deterministic_and_match_goldens() {
    let cfg = Config::builtin();
    let bless = std::env::var_os("SENNE_BLESS").is_some();
    let mut failures = Vec::new();
    for (stem, text) in examples() {
        let first = compile(&text, &cfg, &GameOptions::default()).unwrap_or_else(|d| panic!("{stem}: {d:?}")).code;
        for _ in 0..5 {
            let again = compile(&text, &cfg, &GameOptions::default()).unwrap().code;
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
fn game_goldens_typecheck_against_the_droid_api() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let tmp = std::env::temp_dir().join(format!("senne-game-golden-{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap();
    let stub = Command::new(&rustc)
        .args(["--edition", "2021", "--crate-type", "lib", "--crate-name", "droid", "--out-dir"])
        .arg(&tmp)
        .arg(root().join("tests/game_stub/droid.rs"))
        .output()
        .expect("running rustc");
    assert!(stub.status.success(), "stub: {}", String::from_utf8_lossy(&stub.stderr));
    let mut failures = Vec::new();
    for (stem, _) in examples() {
        let path = golden_path(&stem);
        if !path.exists() {
            continue; // the other test reports it
        }
        let out = Command::new(&rustc)
            .args(["--edition", "2021", "--crate-type", "bin", "--emit", "metadata", "-D", "warnings"])
            .arg("--crate-name")
            .arg(format!("game_{stem}"))
            .arg("--extern")
            .arg(format!("droid={}", tmp.join("libdroid.rlib").display()))
            .arg("--out-dir")
            .arg(&tmp)
            .arg(&path)
            .env("CARGO_MANIFEST_DIR", &tmp)
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
fn every_block_names_its_sentence_and_rule() {
    for (stem, _) in examples() {
        let g = fs::read_to_string(golden_path(&stem)).unwrap_or_default();
        for line in g.lines().filter(|l| l.trim_start().starts_with("// \"")) {
            assert!(line.trim_end().ends_with(']') && line.contains(" ["), "{stem}: {line}");
        }
    }
}

#[test]
fn output_has_the_coin_runner_shape() {
    let c = code("When the player touches a coin, destroy it.");
    assert!(c.contains("use droid::prelude::*;"));
    assert!(c.contains("impl Game for DroidGame {"));
    assert!(c.contains("fn status(&self) -> Status {\n        self.status\n    }"));
    assert!(c.contains("droid::run::<DroidGame>(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/game\"));"));
}

#[test]
fn struct_name_is_used_and_checked() {
    let cfg = Config::builtin();
    let opts = GameOptions { struct_name: "CoinRunner".into(), ..GameOptions::default() };
    assert!(compile("Every frame, add 1 to the score.", &cfg, &opts).unwrap().code.contains("struct CoinRunner {"));
    let bad = GameOptions { struct_name: "Game".into(), ..GameOptions::default() };
    assert!(compile("Every frame, add 1 to the score.", &cfg, &bad).is_err());
}

#[test]
fn unknown_word_fails_with_a_did_you_mean() {
    let err = compile("Each slime patrolz 3 units to each side at 2 units per second.", &Config::builtin(), &GameOptions::default())
        .err()
        .unwrap();
    assert!(err[0].message.contains("patrolz"), "{err:?}");
    assert!(err[0].hint.as_deref().unwrap().contains("\"patrols\""), "{err:?}");
}

#[test]
fn lenient_skips_unknown_words() {
    let opts = GameOptions { lenient: true, ..GameOptions::default() };
    assert!(compile("Each slime happily patrols 3 units to each side at 2 units per second.", &Config::builtin(), &opts).is_ok());
}

#[test]
fn unused_words_are_errors_not_guesses() {
    let err = compile("Each slime patrols 3 units to each side at 2 units per second 5.", &Config::builtin(), &GameOptions::default())
        .err()
        .unwrap();
    assert!(err[0].message.contains("don't understand \"5\""), "{err:?}");
}

#[test]
fn every_problem_is_reported() {
    let err = compile("Each slime patrols.\nEvery 2 seconds, destroy it.", &Config::builtin(), &GameOptions::default())
        .err()
        .unwrap();
    assert_eq!(err.len(), 2, "{err:?}");
}

#[test]
fn modifiers_change_choices() {
    let pairs = [
        ("Move the player with the arrow keys at 7 units per second.", "normalize_or_zero", true),
        ("Simply move the player with the arrow keys at 7 units per second.", "normalize_or_zero", false),
        ("The player jumps with space at 12 units per second.", "jump_buffer", true),
        ("Simply, the player jumps with space at 12 units per second.", "jump_buffer", false),
        ("Each zombie moves toward the player at 2 units per second.", "let player = ", false),
        ("Quickly, each zombie moves toward the player at 2 units per second.", "let player = ", true),
        ("At the start, show \"S {score}\".", "text_0_shown", false),
        ("Quickly, at the start, show \"S {score}\".", "text_0_shown", true),
    ];
    for (src, needle, present) in pairs {
        assert_eq!(code(src).contains(needle), present, "{src}\n{}", code(src));
    }
}

#[test]
fn a_rules_file_changes_the_code() {
    // Same sentence, weights that make brevity win everywhere.
    let terse = DEFAULT_RULES.replace(
        "[game.profile]\nsafety = 1.0\nbrevity = 0.6\nperf = 0.4",
        "[game.profile]\nsafety = 0.1\nbrevity = 2.0\nperf = 0.1",
    );
    let cfg = Config::parse(&terse).unwrap();
    let c = compile("The player jumps with space at 12 units per second.", &cfg, &GameOptions::default()).unwrap().code;
    assert!(!c.contains("jump_buffer"), "{c}");
}

#[test]
fn tuning_numbers_come_from_rules_toml() {
    let changed = DEFAULT_RULES.replace("stomp_height = 0.25", "stomp_height = 0.75");
    let cfg = Config::parse(&changed).unwrap();
    let c = compile("When the player lands on a slime, destroy it.", &cfg, &GameOptions::default()).unwrap().code;
    assert!(c.contains("> 0.75"), "{c}");
}

#[test]
fn explain_shows_candidates_and_judges() {
    let src = "When no coins are left, the player wins.\nWhen the player touches a flag, the player wins.";
    let out = compile(src, &Config::builtin(), &GameOptions::default()).unwrap();
    assert!(out.explain.contains("=> "), "{}", out.explain);
    assert!(out.explain.contains("win/helper") && out.explain.contains("win/inline"), "{}", out.explain);
    assert!(out.explain.contains("total:"), "{}", out.explain);
}

#[test]
fn vocabulary_lists_things_and_verbs() {
    let v = vocabulary();
    for w in ["coin", "aliens", "patrols", "marches", "safely", "square"] {
        assert!(v.contains(&w), "{w}");
    }
}
