use clap::{Parser, Subcommand};

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
    targets: Vec<String>,
  },
}

fn main() {
  let args = Args::parse();

  match &args.command {
    Command::Build { targets } => {
      println!("Building {}", targets.join(" "));
    }
  }
}
