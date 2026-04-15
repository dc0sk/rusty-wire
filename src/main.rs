use rusty_wire::cli;
use std::env;

fn main() {
    cli::run_from_args(&env::args().collect::<Vec<String>>());
}
