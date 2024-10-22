mod host;
mod package_loader;
mod target_pattern;

use clap::{Parser, Subcommand};
use host::fs_host::FsHost;
use package_loader::PackageLoader;
use std::{path::PathBuf, process::ExitCode};
use target_pattern::TargetPattern;
use v8::{Context, ContextScope, FunctionCallbackArguments, FunctionTemplate, HandleScope, Isolate, Local, ReturnValue};

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
      let pkgs = match loader.resolve_packages(patterns.clone()) {
        Ok(pkgs) => pkgs,
        Err(error) => {
          eprintln!("ERROR: Failed to resolve packages\n{}", error);
          return ExitCode::FAILURE;
        },
      };

      // Create the JavaScript engine.
      let platform = v8::new_default_platform(0, false).make_shared();
      v8::V8::initialize_platform(platform);
      v8::V8::initialize();
      let mut isolate = Isolate::new(Default::default());
      isolate.add_message_listener(error_handler);
      let mut scope = HandleScope::new(&mut isolate);
      let context = Context::new(&mut scope, Default::default());
      let scope = &mut ContextScope::new(&mut scope, context);

      // Assign global `print` function.
      let print_tmpl = FunctionTemplate::new(scope, print);
      let print = print_tmpl.get_function(scope).unwrap();
      let print_name = v8::String::new(scope, "print").unwrap();
      context.global(scope).set(scope, print_name.into(), print.into());

      // Create a function to execute JavaScript.
      let mut exec = |code: &str| {
        let code = v8::String::new(scope, code).unwrap();

        let script = v8::Script::compile(scope, code, None).unwrap();
        script.run(scope).unwrap();
      };

      // Load packages.
      let load_result = loader.load_packages(
        &mut exec,
        &pkgs.iter()
          .map(|pkg| pkg.as_path())
          .collect(),
      );
      if let Err(error) = load_result {
        eprintln!("ERROR: Failed to load packages\n{}", error);
        return ExitCode::FAILURE;
      }

      ExitCode::SUCCESS
    }
  }
}

fn print(
  scope: &mut HandleScope,
  args: FunctionCallbackArguments,
  mut _return_value: ReturnValue,
) {
  if args.length() == 0 { return; }

  let data = args.get(0);
  let data = data.to_string(scope).unwrap();
  let data = data.to_rust_string_lossy(scope);
  println!("LOG: {}", data);
}

extern "C" fn error_handler(msg: Local<v8::Message>, _value: Local<v8::Value>) {
  let scope = unsafe { &mut v8::CallbackScope::new(msg) };
  eprintln!("ERROR: {}", msg.get(scope).to_rust_string_lossy(scope));
}
