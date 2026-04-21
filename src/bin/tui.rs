use std::process;

fn main() {
    if let Err(err) = rusty_wire::tui::run() {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}
