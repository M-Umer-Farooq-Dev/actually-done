use actually_done::{
    process::INTERRUPTED,
    render,
    run::{run, Options},
};
use clap::Parser;
use std::{
    io::{self, Write},
    path::PathBuf,
    sync::atomic::Ordering,
};

#[derive(Parser)]
#[command(version, about = "Compare agent completion claims with local evidence")]
struct Cli {
    #[arg(default_value = ".")]
    path: PathBuf,
    #[arg(long)]
    session: Option<String>,
    #[arg(long,value_parser=["auto","claude","codex"])]
    provider: Option<String>,
    #[arg(long)]
    test: Option<String>,
    #[arg(long)]
    test_timeout: Option<f64>,
    #[arg(long)]
    guess_test: bool,
    #[arg(long)]
    no_git: bool,
    #[arg(long)]
    no_test: bool,
    #[arg(long, conflicts_with = "quiet")]
    json: bool,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long, default_value_t = 0)]
    prompt_index: usize,
    #[arg(short = 'q', long)]
    quiet: bool,
    #[arg(short = 'v', long)]
    verbose: bool,
}
fn execute(cli: Cli) -> Result<i32, String> {
    if cli.test_timeout.is_some_and(|n| !n.is_finite() || n <= 0.) {
        return Err("Invalid config: test_timeout must be a positive finite number".into());
    }
    let out = cli.out.clone();
    let receipt = run(Options {
        path: cli.path,
        session: cli.session,
        provider: cli.provider,
        test: cli.test,
        timeout: cli.test_timeout,
        guess: cli.guess_test,
        no_git: cli.no_git,
        no_test: cli.no_test,
        prompt_index: cli.prompt_index,
        out: cli.out,
    })?;
    if INTERRUPTED.load(Ordering::Relaxed) {
        return Err("interrupted".into());
    }
    if let Some(out) = out {
        std::fs::write(out, render::human(&receipt, true))
            .map_err(|e| format!("Cannot write receipt: {e}"))?;
    }
    if cli.verbose && receipt.parse_warnings > 0 {
        eprintln!(
            "Skipped {} malformed transcript records",
            receipt.parse_warnings
        );
    }
    let output = if cli.json {
        serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())? + "\n"
    } else if cli.quiet {
        format!("{} (exit {})\n", receipt.verdict, receipt.exit_code)
    } else {
        render::human(&receipt, false)
    };
    if INTERRUPTED.load(Ordering::Relaxed) {
        return Err("interrupted".into());
    }
    io::stdout()
        .lock()
        .write_all(output.as_bytes())
        .map_err(|e| format!("Cannot write output: {e}"))?;
    Ok(receipt.exit_code)
}
fn main() {
    let cli = Cli::parse();
    if let Err(e) = ctrlc::set_handler(|| INTERRUPTED.store(true, Ordering::Relaxed)) {
        eprintln!("Cannot install interrupt handler: {e}");
        std::process::exit(2);
    }
    let code = match execute(cli) {
        Ok(code) => code,
        Err(e) if e == "interrupted" || INTERRUPTED.load(Ordering::Relaxed) => 130,
        Err(e) => {
            eprintln!("actually-done: {e}");
            2
        }
    };
    std::process::exit(code);
}
