use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check if --tui flag is present
    if args.iter().any(|arg| arg == "--tui" || arg == "-t") {
        // Launch TUI mode
        match rusty_wire::run_tui() {
            Ok(()) => process::exit(0),
            Err(err) => {
                eprintln!("Error: {err}");
                process::exit(1);
            }
        }
    } else {
        // Launch CLI mode (skip program name)
        if !rusty_wire::run_cli(&args[1..]) {
            process::exit(1);
        }
    }
}
