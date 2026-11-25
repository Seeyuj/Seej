use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use seej_lab_workloads::{recover, run, Config, JournalMode, Load, INVALID_CONFIG};
use serde::Serialize;

#[derive(Parser)]
#[command(about = "Compare workload journals and checkpoint recovery")]
struct Cli {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    Run(RunArgs),
    Recover {
        #[arg(long)]
        dir: PathBuf,
    },
}

#[derive(Args)]
struct RunArgs {
    #[arg(long)]
    dir: PathBuf,
    #[arg(long, value_enum)]
    load: Load,
    #[arg(long, value_enum)]
    journal: JournalMode,
    #[arg(long)]
    seed: Option<u64>,
    #[arg(long)]
    keys: Option<u64>,
    #[arg(long)]
    ticks: Option<u64>,
    #[arg(long)]
    warmup_ticks: Option<u64>,
    #[arg(long)]
    checkpoint_at: Option<u64>,
    #[arg(long)]
    auto_writes: Option<u64>,
    #[arg(long)]
    commands: Option<u64>,
    #[arg(long)]
    hot_keys: Option<u64>,
    #[arg(long)]
    hot_percent: Option<u64>,
}

impl RunArgs {
    fn config(&self) -> Config {
        let cfg = Config::preset(self.load, self.journal);
        Config {
            seed: self.seed.unwrap_or(cfg.seed),
            keys: self.keys.unwrap_or(cfg.keys),
            ticks: self.ticks.unwrap_or(cfg.ticks),
            warmup_ticks: self.warmup_ticks.unwrap_or(cfg.warmup_ticks),
            checkpoint_at: self.checkpoint_at.unwrap_or(cfg.checkpoint_at),
            auto_writes: self.auto_writes.unwrap_or(cfg.auto_writes),
            commands: self.commands.unwrap_or(cfg.commands),
            hot_keys: self.hot_keys.unwrap_or(cfg.hot_keys),
            hot_percent: self.hot_percent.unwrap_or(cfg.hot_percent),
            ..cfg
        }
    }
}

fn print_summary(summary: impl Serialize) -> Result<(), String> {
    let mut out = io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, &summary).map_err(|e| e.to_string())?;
    out.write_all(b"\n")
        .and_then(|()| out.flush())
        .map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    let result = match Cli::parse().action {
        Action::Run(args) => run(&args.config(), &args.dir).and_then(print_summary),
        Action::Recover { dir } => recover(&dir).and_then(print_summary),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(if err.starts_with(INVALID_CONFIG) {
                2
            } else {
                1
            })
        }
    }
}
