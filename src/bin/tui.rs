use rusty_wire::ui::tui::{render_tui_scaffold, TuiAction, TuiFocus, TuiState};

fn main() {
    let mut state = TuiState::default();
    state
        .update(TuiAction::SetStatusMessage(Some(
            "TUI scaffold ready".to_string(),
        )))
        .expect("failed to set status message");
    state
        .update(TuiAction::SetFocus(TuiFocus::Inputs))
        .expect("failed to set focus");
    state
        .update(TuiAction::RunCalculation)
        .expect("failed to run scaffold calculation");

    println!("{}", render_tui_scaffold(&state));
}
