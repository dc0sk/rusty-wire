use std::path::PathBuf;

fn main() {
    let output_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/tui-doc-snapshots/index.html"));

    if let Err(err) = rusty_wire::tui::write_doc_snapshots_html(&output_path) {
        eprintln!("Failed to write TUI doc snapshots HTML: {err}");
        std::process::exit(1);
    }

    println!("Wrote {}", output_path.display());
}
