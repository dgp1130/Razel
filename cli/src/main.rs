mod target_pattern;

use clap::{Parser, Subcommand};
use target_pattern::TargetPattern;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "Razel", version)]
struct Args {
  #[command(subcommand)]
  command: Command,
}

#[derive(Subcommand)]
enum Command {
  #[command(about = "Build some targets.")]
  Build {
    patterns: Vec<String>,
  },
}

fn main() -> ExitCode {
  let args = Args::parse();

  match &args.command {
    Command::Build { patterns } => {
      // Parse target patterns.
      let (patterns, errors): (Vec<_>, Vec<_>) = patterns.iter()
          .map(|target| TargetPattern::parse(target))
          .partition(|result| result.is_ok());

      // Fail with any parsing errors.
      if errors.len() != 0 {
        for result in errors {
          eprintln!("ERROR: {}", result.unwrap_err().0);
        }
        return ExitCode::FAILURE;
      }

      // Print targets being built.
      println!(
        "Building targets: {}",
        patterns.into_iter()
            .map(|lbl| format!("{}", lbl.unwrap()))
            .collect::<Vec<_>>()
            .join(" "),
      );
      ExitCode::SUCCESS
    }
  }
}
