mod host;
mod package_loader;
mod target_pattern;

use clap::{Parser, Subcommand};
use host::fs_host::FsHost;
use package_loader::PackageLoader;
use std::{path::PathBuf, process::ExitCode};
use target_pattern::TargetPattern;

#[derive(Parser)]
#[command(name = "Razel", version)]
struct Args {
  #[command(subcommand)]
  command: Command,

  #[arg(long = "workspace", default_value = ".")]
  wksp_root: PathBuf,
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
      let host = FsHost::from(&args.wksp_root).unwrap();

      // Parse target patterns.
      let (patterns, errors): (Vec<_>, Vec<_>) = patterns.iter()
          .map(|target| TargetPattern::parse(target))
          .partition(|result| result.is_ok());

      // Fail with any parsing errors.
      if errors.len() != 0 {
        for result in errors {
          eprintln!(
            "ERROR: Failed to resolve target pattern\n{}",
            result.unwrap_err().0,
          );
        }
        return ExitCode::FAILURE;
      }

      // Extract patterns from the `Result` type.
      let patterns: Vec<_> = patterns.into_iter()
          .map(|result| result.unwrap())
          .collect();

      // Resolve packages from target patterns.
      let loader = PackageLoader::from(&host);
      match loader.resolve_packages(patterns.clone()) {
        Ok(pkgs) => pkgs,
        Err(error) => {
          eprintln!("ERROR: Failed to resolve packages\n{}", error);
          return ExitCode::FAILURE;
        },
      };

      ExitCode::SUCCESS
    }
  }
}
