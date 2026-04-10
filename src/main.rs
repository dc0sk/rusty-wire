mod app;
mod bands;
mod calculations;
mod cli;
mod export;

use std::env;

fn main() {
    cli::run_from_args(&env::args().collect::<Vec<String>>());
}
