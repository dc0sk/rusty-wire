use rusty_wire::ui::tui::{TuiAction, TuiFocus, TuiState};

fn main() {
    let mut state = TuiState::default();
    state.update(TuiAction::SetStatusMessage(Some(
        "TUI scaffold ready".to_string(),
    )));
    state.update(TuiAction::SetFocus(TuiFocus::Results));

    let draft = state.to_request_draft();
    println!("Rusty Wire TUI scaffold");
    println!("  focus: {:?}", state.focus);
    println!("  mode: {:?}", draft.mode);
    println!("  region: {}", draft.itu_region.short_name());
    println!("  bands: {}", draft.band_indices.len());
}