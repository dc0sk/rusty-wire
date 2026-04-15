use rusty_wire::ui::tui::{TuiAction, TuiFocus, TuiState};

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

    let panel = state
        .results_panel
        .as_ref()
        .expect("expected results panel state after calculation");
    println!("Rusty Wire TUI scaffold");
    println!("  focus: {:?}", state.focus);
    println!("  heading: {}", panel.summary.overview_heading);
    println!("  sections: {}", panel.sections.len());
    println!("  bands: {}", panel.summary.band_count);
    for line in &panel.summary.summary_lines {
        println!("  {line}");
    }
    if !panel.warnings.warning_lines.is_empty() {
        println!("  warnings:");
        for line in &panel.warnings.warning_lines {
            println!("    {line}");
        }
    }
}
