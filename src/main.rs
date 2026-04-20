use std::env;
use std::process;

fn main() {
    if !rusty_wire::run_cli(&env::args().collect::<Vec<String>>()) {
        process::exit(1);
    }
}
