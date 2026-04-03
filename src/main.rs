mod app;
mod bands;
mod calculations;
mod cli;
mod export;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        cli::run_from_args(&args[1..]);
    } else {
        cli::run_interactive();
    }
}

