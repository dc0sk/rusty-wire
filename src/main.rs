mod app;
mod bands;
mod calculations;
mod cli;
mod export;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if !cli::run_from_args(&args) {
        process::exit(1);
    }
}
