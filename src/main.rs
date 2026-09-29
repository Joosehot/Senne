use anyhow::{bail, Context, Result};
use clap::Parser;
use senne::config::Config;
use senne::{compile, explain, lexicon, Options};
use std::io::Read;
use std::path::PathBuf;

/// Senne: describe the code in plain words, get deterministic Rust.
///
///   senne "read the file \"n.txt\", for each line parse it as an integer, then sum them"
///   senne --game -f behavior.txt --struct CoinRunner > src/main.rs
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// The sentence. Omit to read it from --file or stdin.
    sentence: Option<String>,
    /// Read the sentence from a file.
    #[arg(short, long)]
    file: Option<PathBuf>,
    /// Write the Rust here instead of stdout.
    #[arg(short, long)]
    out: Option<PathBuf>,
    /// Show tokens, clauses and every candidate the search weighed (stderr).
    #[arg(long)]
    explain: bool,
    /// No per-statement comments.
    #[arg(long)]
    bare: bool,
    /// Ignore unknown words instead of failing.
    #[arg(long)]
    lenient: bool,
    /// Use this rules.toml instead of the built-in one.
    #[arg(long)]
    rules: Option<PathBuf>,
    /// List every word and phrase Senne knows (with --game: the game language's words).
    #[arg(long)]
    vocabulary: bool,
    /// Game sentences (Droid's behavior.txt) in, a Droid game's main.rs out.
    #[arg(long)]
    game: bool,
    /// With --game: the name of the generated game type.
    #[arg(long = "struct", value_name = "NAME", default_value = "DroidGame")]
    struct_name: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.vocabulary && cli.game {
        for w in senne::game::vocabulary() {
            println!("{w}");
        }
        return Ok(());
    }
    if cli.vocabulary {
        for w in lexicon::vocabulary() {
            println!("{w}");
        }
        return Ok(());
    }
    let cfg = match &cli.rules {
        Some(p) => Config::load(p)?,
        None => Config::builtin(),
    };
    let sentence = match (&cli.sentence, &cli.file) {
        (Some(s), _) => s.clone(),
        (None, Some(f)) => std::fs::read_to_string(f).with_context(|| format!("reading {}", f.display()))?,
        (None, None) => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s)?;
            s
        }
    };
    if cli.game {
        let opts = senne::game::GameOptions { struct_name: cli.struct_name.clone(), lenient: cli.lenient, comments: !cli.bare };
        let compiled = match senne::game::compile(&sentence, &cfg, &opts) {
            Ok(c) => c,
            Err(diags) => {
                for d in &diags {
                    eprintln!("{d}");
                }
                bail!("no game code generated ({} problem{})", diags.len(), if diags.len() == 1 { "" } else { "s" });
            }
        };
        if cli.explain {
            eprint!("{}", compiled.explain);
            eprintln!();
        }
        match &cli.out {
            Some(p) => std::fs::write(p, &compiled.code).with_context(|| format!("writing {}", p.display()))?,
            None => print!("{}", compiled.code),
        }
        return Ok(());
    }
    let opts = Options { lenient: cli.lenient, comments: !cli.bare };
    let compiled = match compile(&sentence, &cfg, &opts) {
        Ok(c) => c,
        Err(diags) => {
            for d in &diags {
                eprintln!("{d}");
            }
            bail!("no code generated ({} problem{})", diags.len(), if diags.len() == 1 { "" } else { "s" });
        }
    };
    if cli.explain {
        eprint!("{}", explain::explain(&compiled.program, &compiled.outcome, &cfg));
        eprintln!();
    }
    match &cli.out {
        Some(p) => std::fs::write(p, &compiled.code).with_context(|| format!("writing {}", p.display()))?,
        None => print!("{}", compiled.code),
    }
    Ok(())
}
