/// TUI front-end for Rusty Wire (ratatui-based).
///
/// This module is the entry point for the keyboard-driven terminal UI.
/// It is intentionally a stub until the ratatui dependency and widget tree
/// are added in the next TUI milestone.  All application logic lives in
/// `rusty_wire::app`; this module only owns rendering and input handling.
///
/// # Architecture
///
/// ```text
/// src/bin/tui.rs  →  tui::run()
///                         │
///                         ▼
///               event loop (crossterm)
///                         │
///             AppAction (user input)
///                         │
///                         ▼
///              app::apply_action()      ← pure, no I/O
///                         │
///                    AppState
///                         │
///                         ▼
///            ratatui widgets (render)
/// ```
///
/// # Adding ratatui
///
/// When real rendering is added:
/// 1. `cargo add ratatui` (and `crossterm` for the backend)
/// 2. Replace the `run()` stub below with:
///    - terminal setup / raw mode
///    - event loop reading `crossterm::event::Event`
///    - mapping events to `AppAction` variants
///    - calling `app::apply_action(state, action)`
///    - drawing the widget tree from `AppState`
///    - terminal teardown on exit / panic
use crate::app::{apply_action, AppAction, AppState};

/// Launch the TUI.
///
/// Returns when the user exits (e.g. presses `q` or `Ctrl-C`).
///
/// # Errors
///
/// Returns a boxed error if terminal initialisation or I/O fails.
///
/// # Stub behaviour
///
/// Until ratatui widgets are added this function prints a placeholder
/// message and exits cleanly, so the binary is always compilable.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise state with application defaults.
    let mut state = AppState::default();

    // Run an initial calculation so the TUI has something to display on first
    // render.  Errors here are soft — the TUI will show the error state.
    state = apply_action(state, AppAction::RunCalculation);

    // --- Stub: replace this section with the ratatui event loop ---
    eprintln!("Rusty Wire TUI — coming soon.");
    eprintln!(
        "State ready: {} band(s) configured.",
        state.config.band_indices.len()
    );
    if let Some(ref results) = state.results {
        eprintln!(
            "Initial calculation: {} band(s) computed.",
            results.calculations.len()
        );
    }
    if let Some(ref err) = state.error {
        eprintln!("Initial calculation error: {err}");
    }
    eprintln!("Run `rusty-wire --help` for the CLI interface.");
    // --- End stub ---

    Ok(())
}
