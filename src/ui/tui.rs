#![allow(dead_code)]

use std::error::Error;
use std::io::{self, Stdout};
use std::time::Duration;

use crate::app::{
    app_results_view_model, execute_request_checked, AntennaModel, AppError, AppRequest,
    AppRequestDraft, AppResults, AppResultsViewModel, CalcMode, ExportFormat, UnitSystem,
    DEFAULT_BAND_SELECTION,
};
use crate::bands::ITURegion;
use crate::calculations::{TransformerRatio, DEFAULT_NON_RESONANT_CONFIG};
use crate::export::{default_output_name, export_results, to_csv, to_json, to_markdown, to_txt};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiFocus {
    Inputs,
    Results,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiInputField {
    BandIndices,
    VelocityFactor,
    WireMinM,
    WireMaxM,
}

impl TuiInputField {
    fn label(self) -> &'static str {
        match self {
            TuiInputField::BandIndices => "bands",
            TuiInputField::VelocityFactor => "velocity_factor",
            TuiInputField::WireMinM => "wire_min_m",
            TuiInputField::WireMaxM => "wire_max_m",
        }
    }

    fn next(self) -> Self {
        match self {
            TuiInputField::BandIndices => TuiInputField::VelocityFactor,
            TuiInputField::VelocityFactor => TuiInputField::WireMinM,
            TuiInputField::WireMinM => TuiInputField::WireMaxM,
            TuiInputField::WireMaxM => TuiInputField::BandIndices,
        }
    }

    fn previous(self) -> Self {
        match self {
            TuiInputField::BandIndices => TuiInputField::WireMaxM,
            TuiInputField::VelocityFactor => TuiInputField::BandIndices,
            TuiInputField::WireMinM => TuiInputField::VelocityFactor,
            TuiInputField::WireMaxM => TuiInputField::WireMinM,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TuiAction {
    SetFocus(TuiFocus),
    SetStatusMessage(Option<String>),
    SetBandIndices(Vec<usize>),
    SetBandIndicesAndRunCalculation(Vec<usize>),
    SetMode(CalcMode),
    SetModeAndRunCalculation(CalcMode),
    SetAntennaModel(Option<AntennaModel>),
    SetAntennaModelAndRunCalculation(Option<AntennaModel>),
    SetVelocityFactor(f64),
    SetVelocityFactorAndRunCalculation(f64),
    SetTransformerRatio(TransformerRatio),
    SetTransformerRatioAndRunCalculation(TransformerRatio),
    SetWireWindow { min_m: f64, max_m: f64 },
    SetWireWindowAndRunCalculation { min_m: f64, max_m: f64 },
    SetDefaultUnits(UnitSystem),
    SetSelectedUnits(Option<UnitSystem>),
    SetSelectedUnitsAndRunCalculation(Option<UnitSystem>),
    SetRegion(ITURegion),
    SetRegionAndRunCalculation(ITURegion),
    RunCalculation,
}

#[derive(Debug, Clone)]
pub struct TuiSummaryPanelState {
    pub overview_heading: String,
    pub summary_lines: Vec<String>,
    pub band_count: usize,
}

#[derive(Debug, Clone)]
pub struct TuiWarningsPanelState {
    pub warning_lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TuiSectionPanelState {
    pub heading: Option<String>,
    pub line_count: usize,
}

#[derive(Debug, Clone)]
pub struct TuiResultsPanelState {
    pub summary: TuiSummaryPanelState,
    pub warnings: TuiWarningsPanelState,
    pub sections: Vec<TuiSectionPanelState>,
}

impl TuiResultsPanelState {
    fn from_app_view_model(view_model: &AppResultsViewModel) -> Self {
        Self {
            summary: TuiSummaryPanelState {
                overview_heading: view_model.display_document.overview_heading.to_string(),
                summary_lines: view_model.display_document.summary_lines.clone(),
                band_count: view_model.display_document.band_views.len(),
            },
            warnings: TuiWarningsPanelState {
                warning_lines: view_model.display_document.warning_lines.clone(),
            },
            sections: view_model
                .display_document
                .sections
                .iter()
                .map(|section| TuiSectionPanelState {
                    heading: section.lines.first().cloned(),
                    line_count: section.lines.len(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TuiState {
    pub focus: TuiFocus,
    pub status_message: Option<String>,
    pub app_results_view_model: Option<AppResultsViewModel>,
    pub app_results: Option<AppResults>,
    pub results_panel: Option<TuiResultsPanelState>,
    pub band_indices: Vec<usize>,
    pub mode: CalcMode,
    pub antenna_model: Option<AntennaModel>,
    pub velocity_factor: f64,
    pub transformer_ratio: TransformerRatio,
    pub wire_min_m: f64,
    pub wire_max_m: f64,
    pub default_units: UnitSystem,
    pub selected_units: Option<UnitSystem>,
    pub itu_region: ITURegion,
    pub active_input_field: TuiInputField,
    pub editing_input: bool,
    pub input_buffer: String,
    pub export_format: ExportFormat,
    pub export_output_path: String,
    pub export_preview: Option<Vec<String>>,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            focus: TuiFocus::Inputs,
            status_message: None,
            app_results_view_model: None,
            app_results: None,
            results_panel: None,
            band_indices: DEFAULT_BAND_SELECTION.to_vec(),
            mode: CalcMode::Resonant,
            antenna_model: None,
            velocity_factor: 0.95,
            transformer_ratio: TransformerRatio::R1To1,
            wire_min_m: DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            wire_max_m: DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            default_units: UnitSystem::Both,
            selected_units: None,
            itu_region: ITURegion::Region1,
            active_input_field: TuiInputField::BandIndices,
            editing_input: false,
            input_buffer: String::new(),
            export_format: ExportFormat::Txt,
            export_output_path: default_output_name(ExportFormat::Txt).to_string(),
            export_preview: None,
        }
    }
}

impl TuiState {
    fn run_calculation(&mut self) -> Result<(), AppError> {
        let response = execute_request_checked(self.to_request())?;
        self.app_results = Some(response.results.clone());
        let view_model = app_results_view_model(&response.results);
        self.results_panel = Some(TuiResultsPanelState::from_app_view_model(&view_model));
        self.app_results_view_model = Some(view_model);
        self.focus = TuiFocus::Results;
        self.status_message = Some("Calculation complete".to_string());
        Ok(())
    }

    pub fn to_request(&self) -> AppRequest {
        AppRequest::from_draft(self.to_request_draft())
    }

    pub fn to_request_draft(&self) -> AppRequestDraft {
        AppRequestDraft {
            band_indices: self.band_indices.clone(),
            velocity_factor: self.velocity_factor,
            mode: self.mode,
            wire_min_m: self.wire_min_m,
            wire_max_m: self.wire_max_m,
            default_units: self.default_units,
            selected_units: self.selected_units,
            itu_region: self.itu_region,
            transformer_ratio: self.transformer_ratio,
            antenna_model: self.antenna_model,
        }
    }

    pub fn set_focus(&mut self, focus: TuiFocus) {
        self.focus = focus;
    }

    pub fn update(&mut self, action: TuiAction) -> Result<(), AppError> {
        match action {
            TuiAction::SetFocus(focus) => self.focus = focus,
            TuiAction::SetStatusMessage(message) => self.status_message = message,
            TuiAction::SetBandIndices(band_indices) => self.band_indices = band_indices,
            TuiAction::SetBandIndicesAndRunCalculation(band_indices) => {
                self.band_indices = band_indices;
                return self.run_calculation();
            }
            TuiAction::SetMode(mode) => self.mode = mode,
            TuiAction::SetModeAndRunCalculation(mode) => {
                self.mode = mode;
                return self.run_calculation();
            }
            TuiAction::SetAntennaModel(antenna_model) => self.antenna_model = antenna_model,
            TuiAction::SetAntennaModelAndRunCalculation(antenna_model) => {
                self.antenna_model = antenna_model;
                return self.run_calculation();
            }
            TuiAction::SetVelocityFactor(velocity_factor) => {
                self.velocity_factor = velocity_factor;
            }
            TuiAction::SetVelocityFactorAndRunCalculation(velocity_factor) => {
                self.velocity_factor = velocity_factor;
                return self.run_calculation();
            }
            TuiAction::SetTransformerRatio(transformer_ratio) => {
                self.transformer_ratio = transformer_ratio;
            }
            TuiAction::SetTransformerRatioAndRunCalculation(transformer_ratio) => {
                self.transformer_ratio = transformer_ratio;
                return self.run_calculation();
            }
            TuiAction::SetWireWindow { min_m, max_m } => {
                self.wire_min_m = min_m;
                self.wire_max_m = max_m;
            }
            TuiAction::SetWireWindowAndRunCalculation { min_m, max_m } => {
                self.wire_min_m = min_m;
                self.wire_max_m = max_m;
                return self.run_calculation();
            }
            TuiAction::SetDefaultUnits(default_units) => self.default_units = default_units,
            TuiAction::SetSelectedUnits(selected_units) => {
                self.selected_units = selected_units;
            }
            TuiAction::SetSelectedUnitsAndRunCalculation(selected_units) => {
                self.selected_units = selected_units;
                return self.run_calculation();
            }
            TuiAction::SetRegion(itu_region) => self.itu_region = itu_region,
            TuiAction::SetRegionAndRunCalculation(itu_region) => {
                self.itu_region = itu_region;
                return self.run_calculation();
            }
            TuiAction::RunCalculation => return self.run_calculation(),
        }

        Ok(())
    }

    fn begin_input_edit(&mut self) {
        self.input_buffer = self.active_input_display_value();
        self.editing_input = true;
        self.status_message = Some(format!(
            "Editing {}. Enter to apply, Esc to cancel",
            self.active_input_field.label()
        ));
    }

    fn cancel_input_edit(&mut self) {
        self.editing_input = false;
        self.input_buffer.clear();
        self.status_message = Some("Input edit canceled".to_string());
    }

    fn active_input_display_value(&self) -> String {
        match self.active_input_field {
            TuiInputField::BandIndices => self
                .band_indices
                .iter()
                .map(|idx| idx.to_string())
                .collect::<Vec<_>>()
                .join(","),
            TuiInputField::VelocityFactor => format!("{:.2}", self.velocity_factor),
            TuiInputField::WireMinM => format!("{:.2}", self.wire_min_m),
            TuiInputField::WireMaxM => format!("{:.2}", self.wire_max_m),
        }
    }

    fn apply_input_edit(&mut self) {
        let raw = self.input_buffer.trim();
        if raw.is_empty() {
            self.status_message = Some("Input value cannot be empty".to_string());
            return;
        }

        match self.active_input_field {
            TuiInputField::BandIndices => match parse_band_indices(raw) {
                Some(indices) => {
                    if self
                        .update(TuiAction::SetBandIndicesAndRunCalculation(indices.clone()))
                        .is_ok()
                    {
                        self.status_message = Some(format!("Updated bands: {:?}", indices));
                    }
                }
                None => {
                    self.status_message = Some(
                        "Invalid band list. Use comma-separated indices like 4,5,6".to_string(),
                    );
                }
            },
            TuiInputField::VelocityFactor => match raw.parse::<f64>() {
                Ok(value) => {
                    if self
                        .update(TuiAction::SetVelocityFactorAndRunCalculation(value))
                        .is_err()
                    {
                        self.status_message =
                            Some("Velocity factor out of range. Use 0.50..1.00".to_string());
                    }
                }
                Err(_) => {
                    self.status_message =
                        Some("Invalid velocity factor. Example: 0.95".to_string());
                }
            },
            TuiInputField::WireMinM => match raw.parse::<f64>() {
                Ok(value) => {
                    if self
                        .update(TuiAction::SetWireWindowAndRunCalculation {
                            min_m: value,
                            max_m: self.wire_max_m,
                        })
                        .is_err()
                    {
                        self.status_message =
                            Some("Invalid wire window. Ensure min <= max and both > 0".to_string());
                    }
                }
                Err(_) => {
                    self.status_message = Some("Invalid minimum wire length".to_string());
                }
            },
            TuiInputField::WireMaxM => match raw.parse::<f64>() {
                Ok(value) => {
                    if self
                        .update(TuiAction::SetWireWindowAndRunCalculation {
                            min_m: self.wire_min_m,
                            max_m: value,
                        })
                        .is_err()
                    {
                        self.status_message =
                            Some("Invalid wire window. Ensure min <= max and both > 0".to_string());
                    }
                }
                Err(_) => {
                    self.status_message = Some("Invalid maximum wire length".to_string());
                }
            },
        }

        self.editing_input = false;
        self.input_buffer.clear();
    }

    fn cycle_export_format(&mut self) {
        self.export_format = match self.export_format {
            ExportFormat::Csv => ExportFormat::Json,
            ExportFormat::Json => ExportFormat::Markdown,
            ExportFormat::Markdown => ExportFormat::Txt,
            ExportFormat::Txt => ExportFormat::Csv,
        };
        self.export_output_path = default_output_name(self.export_format).to_string();
        self.status_message = Some(format!("Export format: {:?}", self.export_format));
    }

    fn build_export_document(&self) -> Result<String, String> {
        let Some(results) = self.app_results.as_ref() else {
            return Err("No calculation results available for export".to_string());
        };

        let content = match self.export_format {
            ExportFormat::Csv => to_csv(
                &results.calculations,
                self.app_results_view_model
                    .as_ref()
                    .and_then(|view| view.export_recommendation.as_ref()),
                results.config.units,
                results.config.wire_min_m,
                results.config.wire_max_m,
            ),
            ExportFormat::Json => to_json(
                &results.calculations,
                self.app_results_view_model
                    .as_ref()
                    .and_then(|view| view.export_recommendation.as_ref()),
                results.config.units,
                results.config.wire_min_m,
                results.config.wire_max_m,
            ),
            ExportFormat::Markdown => to_markdown(
                &results.calculations,
                self.app_results_view_model
                    .as_ref()
                    .and_then(|view| view.export_recommendation.as_ref()),
                results.config.units,
                results.config.wire_min_m,
                results.config.wire_max_m,
            ),
            ExportFormat::Txt => to_txt(
                &results.calculations,
                self.app_results_view_model
                    .as_ref()
                    .and_then(|view| view.export_recommendation.as_ref()),
                results.config.units,
                results.config.wire_min_m,
                results.config.wire_max_m,
            ),
        };

        Ok(content)
    }

    fn build_export_preview_lines(&self) -> Result<Vec<String>, String> {
        let document = self.build_export_document()?;
        let mut lines = Vec::new();
        for (idx, line) in document.lines().take(8).enumerate() {
            lines.push(format!("{}: {}", idx + 1, line));
        }
        if lines.is_empty() {
            lines.push("(empty export document)".to_string());
        }
        Ok(lines)
    }

    fn refresh_export_preview(&mut self) {
        match self.build_export_preview_lines() {
            Ok(lines) => {
                self.export_preview = Some(lines);
                self.status_message = Some("Export preview refreshed".to_string());
            }
            Err(err) => {
                self.export_preview = None;
                self.status_message = Some(err);
            }
        }
    }

    fn write_export_file(&mut self) {
        let Some(results) = self.app_results.as_ref() else {
            self.status_message = Some("Run a calculation before exporting".to_string());
            return;
        };

        match export_results(
            self.export_format,
            &self.export_output_path,
            &results.calculations,
            self.app_results_view_model
                .as_ref()
                .and_then(|view| view.export_recommendation.as_ref()),
            results.config.units,
            results.config.wire_min_m,
            results.config.wire_max_m,
        ) {
            Ok(_) => {
                self.status_message = Some(format!("Exported to {}", self.export_output_path));
            }
            Err(err) => {
                self.status_message = Some(format!("Export failed: {err}"));
            }
        }
    }
}

struct TuiTerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TuiTerminalSession {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TuiTerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn next_focus(focus: TuiFocus) -> TuiFocus {
    match focus {
        TuiFocus::Inputs => TuiFocus::Results,
        TuiFocus::Results => TuiFocus::Export,
        TuiFocus::Export => TuiFocus::Inputs,
    }
}

fn toggle_mode(mode: CalcMode) -> CalcMode {
    match mode {
        CalcMode::Resonant => CalcMode::NonResonant,
        CalcMode::NonResonant => CalcMode::Resonant,
    }
}

fn format_units(units: Option<UnitSystem>, default_units: UnitSystem) -> String {
    match units {
        Some(selected) => format!("selected {selected:?} (default {default_units:?})"),
        None => format!("default {default_units:?}"),
    }
}

fn input_lines(state: &TuiState) -> Vec<String> {
    if state.editing_input {
        return vec![
            format!("editing: {}", state.active_input_field.label()),
            format!("buffer: {}", state.input_buffer),
            "Enter apply | Esc cancel | Backspace delete".to_string(),
        ];
    }

    vec![
        format!("mode: {:?} (m toggle)", state.mode),
        format!("region: {:?}", state.itu_region),
        format!(
            "bands [{}]: {:?}",
            TuiInputField::BandIndices.label(),
            state.band_indices
        ),
        format!("velocity factor: {:.2}", state.velocity_factor),
        format!("transformer: {:?}", state.transformer_ratio),
        format!(
            "wire min [{}]: {:.1}m",
            TuiInputField::WireMinM.label(),
            state.wire_min_m
        ),
        format!(
            "wire max [{}]: {:.1}m",
            TuiInputField::WireMaxM.label(),
            state.wire_max_m
        ),
        format!(
            "units: {}",
            format_units(state.selected_units, state.default_units)
        ),
        format!("antenna: {:?}", state.antenna_model),
        format!("active field: {}", state.active_input_field.label()),
        "j/k choose field, i edit field, Enter apply".to_string(),
    ]
}

fn export_lines(state: &TuiState) -> Vec<String> {
    let mut lines = vec!["shared app request draft:".to_string()];
    let draft = state.to_request_draft();
    lines.push(format!("  bands: {:?}", draft.band_indices));
    lines.push(format!("  mode: {:?}", draft.mode));
    lines.push(format!("  region: {:?}", draft.itu_region));
    lines.push(format!(
        "  units: {:?}",
        draft.selected_units.unwrap_or(draft.default_units)
    ));
    lines.push(format!("format: {:?}", state.export_format));
    lines.push(format!("output: {}", state.export_output_path));
    lines.push("t cycle format | p preview | e export".to_string());

    if let Some(preview) = state.export_preview.as_ref() {
        lines.push("preview:".to_string());
        lines.extend(preview.iter().map(|line| format!("  {line}")));
    }

    lines
}

fn status_line(state: &TuiState) -> String {
    state
        .status_message
        .clone()
        .unwrap_or_else(|| "Tab focus, r run, m toggle mode, q quit".to_string())
}

fn focus_title(title: &str, focus: TuiFocus, current_focus: TuiFocus) -> String {
    if focus == current_focus {
        format!("> {title}")
    } else {
        title.to_string()
    }
}

fn panel_block(title: String, focused: bool) -> Block<'static> {
    let border_style = if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
}

fn paragraph_from_lines(title: String, focused: bool, lines: Vec<String>) -> Paragraph<'static> {
    let text_lines: Vec<Line<'static>> = if lines.is_empty() {
        vec![Line::from("(empty)")]
    } else {
        lines
            .into_iter()
            .map(|line| Line::from(Span::raw(line)))
            .collect()
    };

    Paragraph::new(text_lines)
        .block(panel_block(title, focused))
        .wrap(Wrap { trim: false })
}

fn render_results_column(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let Some(panel) = state.results_panel.as_ref() else {
        let empty = paragraph_from_lines(
            focus_title("Results", TuiFocus::Results, state.focus),
            state.focus == TuiFocus::Results,
            vec!["no results available".to_string()],
        );
        frame.render_widget(empty, area);
        return;
    };

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(35),
            Constraint::Percentage(25),
        ])
        .split(area);

    let summary = paragraph_from_lines(
        focus_title("Summary", TuiFocus::Results, state.focus),
        state.focus == TuiFocus::Results,
        render_summary_panel(&panel.summary),
    );
    let sections = paragraph_from_lines(
        "Sections".to_string(),
        false,
        render_sections_panel(&panel.sections),
    );
    let warnings = paragraph_from_lines(
        "Warnings".to_string(),
        false,
        render_warnings_panel(&panel.warnings),
    );

    frame.render_widget(summary, vertical[0]);
    frame.render_widget(sections, vertical[1]);
    frame.render_widget(warnings, vertical[2]);
}

pub fn render_tui_frame(frame: &mut Frame<'_>, state: &TuiState) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(frame.area());

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(43),
            Constraint::Percentage(23),
        ])
        .split(layout[0]);

    let inputs = paragraph_from_lines(
        focus_title("Inputs", TuiFocus::Inputs, state.focus),
        state.focus == TuiFocus::Inputs,
        input_lines(state),
    );
    let export = paragraph_from_lines(
        focus_title("Export", TuiFocus::Export, state.focus),
        state.focus == TuiFocus::Export,
        export_lines(state),
    );
    let status = Paragraph::new(vec![Line::from(status_line(state))])
        .block(Block::default().title("Status").borders(Borders::ALL))
        .wrap(Wrap { trim: false });

    frame.render_widget(inputs, columns[0]);
    render_results_column(frame, columns[1], state);
    frame.render_widget(export, columns[2]);
    frame.render_widget(status, layout[1]);
}

fn handle_key_event(state: &mut TuiState, key_code: KeyCode) -> Result<bool, AppError> {
    if state.editing_input {
        match key_code {
            KeyCode::Esc => state.cancel_input_edit(),
            KeyCode::Enter => state.apply_input_edit(),
            KeyCode::Backspace => {
                state.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                if c.is_ascii_graphic() || c == ' ' {
                    state.input_buffer.push(c);
                }
            }
            _ => {}
        }
        return Ok(false);
    }

    match key_code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
        KeyCode::Tab => state.update(TuiAction::SetFocus(next_focus(state.focus)))?,
        KeyCode::BackTab => state.update(TuiAction::SetFocus(match state.focus {
            TuiFocus::Inputs => TuiFocus::Export,
            TuiFocus::Results => TuiFocus::Inputs,
            TuiFocus::Export => TuiFocus::Results,
        }))?,
        KeyCode::Char('r') => state.update(TuiAction::RunCalculation)?,
        KeyCode::Char('m') => {
            state.update(TuiAction::SetModeAndRunCalculation(toggle_mode(state.mode)))?
        }
        KeyCode::Char('f') => {
            let next = if state.velocity_factor >= 0.95 {
                0.66
            } else {
                0.95
            };
            state.update(TuiAction::SetVelocityFactorAndRunCalculation(next))?;
        }
        KeyCode::Char('u') => {
            let next = match state.selected_units {
                None => Some(UnitSystem::Metric),
                Some(UnitSystem::Metric) => Some(UnitSystem::Imperial),
                Some(UnitSystem::Imperial) => Some(UnitSystem::Both),
                Some(UnitSystem::Both) => None,
            };
            state.update(TuiAction::SetSelectedUnitsAndRunCalculation(next))?;
        }
        KeyCode::Char('g') => {
            let next = match state.itu_region {
                ITURegion::Region1 => ITURegion::Region2,
                ITURegion::Region2 => ITURegion::Region3,
                ITURegion::Region3 => ITURegion::Region1,
            };
            state.update(TuiAction::SetRegionAndRunCalculation(next))?;
        }
        KeyCode::Char('j') if state.focus == TuiFocus::Inputs => {
            state.active_input_field = state.active_input_field.next();
            state.status_message = Some(format!(
                "Selected input field: {}",
                state.active_input_field.label()
            ));
        }
        KeyCode::Char('k') if state.focus == TuiFocus::Inputs => {
            state.active_input_field = state.active_input_field.previous();
            state.status_message = Some(format!(
                "Selected input field: {}",
                state.active_input_field.label()
            ));
        }
        KeyCode::Char('i') if state.focus == TuiFocus::Inputs => {
            state.begin_input_edit();
        }
        KeyCode::Enter if state.focus == TuiFocus::Inputs => {
            state.begin_input_edit();
        }
        KeyCode::Char('t') if state.focus == TuiFocus::Export => {
            state.cycle_export_format();
        }
        KeyCode::Char('p') if state.focus == TuiFocus::Export => {
            state.refresh_export_preview();
        }
        KeyCode::Char('e') if state.focus == TuiFocus::Export => {
            state.write_export_file();
        }
        _ => {}
    }

    Ok(false)
}

pub fn run_tui_app() -> Result<(), Box<dyn Error>> {
    let mut state = TuiState::default();
    state.update(TuiAction::SetStatusMessage(Some(
        "Tab focus, j/k pick input, i edit, r run, t/p/e export, q quit".to_string(),
    )))?;
    state.update(TuiAction::RunCalculation)?;

    let mut session = TuiTerminalSession::enter()?;

    loop {
        session
            .terminal_mut()
            .draw(|frame| render_tui_frame(frame, &state))?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        if handle_key_event(&mut state, key.code)? {
            break;
        }
    }

    Ok(())
}

pub fn render_summary_panel(panel: &TuiSummaryPanelState) -> Vec<String> {
    let mut lines = vec![format!("  heading: {}", panel.overview_heading)];
    lines.push(format!("  bands: {}", panel.band_count));
    lines.extend(panel.summary_lines.iter().map(|line| format!("  {line}")));
    lines
}

pub fn render_warnings_panel(panel: &TuiWarningsPanelState) -> Vec<String> {
    if panel.warning_lines.is_empty() {
        return Vec::new();
    }

    let mut lines = vec!["  warnings:".to_string()];
    lines.extend(panel.warning_lines.iter().map(|line| format!("    {line}")));
    lines
}

pub fn render_sections_panel(sections: &[TuiSectionPanelState]) -> Vec<String> {
    let mut lines = vec![format!("  sections: {}", sections.len())];
    lines.extend(sections.iter().enumerate().map(|(idx, section)| {
        let heading = section
            .heading
            .as_deref()
            .unwrap_or("(section without heading)");
        format!(
            "    {}. {} [{} lines]",
            idx + 1,
            heading,
            section.line_count
        )
    }));
    lines
}

pub fn render_panel_block(title: &str, lines: &[String]) -> Vec<String> {
    let mut block = vec![format!("[{title}]")];
    if lines.is_empty() {
        block.push("  (empty)".to_string());
    } else {
        block.extend(lines.iter().cloned());
    }
    block
}

pub fn render_tui_layout(state: &TuiState) -> String {
    let mut lines = vec![
        "Rusty Wire TUI".to_string(),
        format!("Focus: {:?}", state.focus),
    ];

    if let Some(panel) = state.results_panel.as_ref() {
        let summary_lines = render_summary_panel(&panel.summary);
        let section_lines = render_sections_panel(&panel.sections);
        let warning_lines = render_warnings_panel(&panel.warnings);

        lines.extend(render_panel_block("Summary", &summary_lines));
        lines.extend(render_panel_block("Sections", &section_lines));
        lines.extend(render_panel_block("Warnings", &warning_lines));
    } else {
        lines.push("[Summary]".to_string());
        lines.push("  no results available".to_string());
    }

    lines.join("\n")
}

pub fn render_tui_scaffold(state: &TuiState) -> String {
    render_tui_layout(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tui_state_defaults_to_inputs_focus() {
        let state = TuiState::default();

        assert_eq!(state.focus, TuiFocus::Inputs);
        assert_eq!(state.band_indices, DEFAULT_BAND_SELECTION.to_vec());
        assert_eq!(state.mode, CalcMode::Resonant);
        assert!(state.selected_units.is_none());
        assert!(state.results_panel.is_none());
    }

    #[test]
    fn tui_state_translates_to_request_draft() {
        let state = TuiState {
            focus: TuiFocus::Results,
            status_message: Some("ready".to_string()),
            app_results_view_model: None,
            app_results: None,
            results_panel: None,
            band_indices: vec![4, 6],
            mode: CalcMode::NonResonant,
            antenna_model: Some(AntennaModel::EndFedHalfWave),
            velocity_factor: 0.9,
            transformer_ratio: TransformerRatio::R1To56,
            wire_min_m: 12.0,
            wire_max_m: 24.0,
            default_units: UnitSystem::Metric,
            selected_units: Some(UnitSystem::Both),
            itu_region: ITURegion::Region2,
            active_input_field: TuiInputField::BandIndices,
            editing_input: false,
            input_buffer: String::new(),
            export_format: ExportFormat::Csv,
            export_output_path: "out.csv".to_string(),
            export_preview: None,
        };

        let draft = state.to_request_draft();

        assert_eq!(draft.band_indices, vec![4, 6]);
        assert_eq!(draft.mode, CalcMode::NonResonant);
        assert_eq!(draft.selected_units, Some(UnitSystem::Both));
        assert_eq!(draft.itu_region, ITURegion::Region2);
        assert_eq!(draft.antenna_model, Some(AntennaModel::EndFedHalfWave));
    }

    #[test]
    fn tui_state_update_applies_actions() {
        let mut state = TuiState::default();

        state
            .update(TuiAction::SetFocus(TuiFocus::Results))
            .unwrap();
        state
            .update(TuiAction::SetStatusMessage(Some("updated".to_string())))
            .unwrap();
        state
            .update(TuiAction::SetMode(CalcMode::NonResonant))
            .unwrap();
        state.update(TuiAction::SetVelocityFactor(0.9)).unwrap();
        state
            .update(TuiAction::SetRegion(ITURegion::Region3))
            .unwrap();
        state
            .update(TuiAction::SetSelectedUnits(Some(UnitSystem::Both)))
            .unwrap();

        assert_eq!(state.focus, TuiFocus::Results);
        assert_eq!(state.status_message.as_deref(), Some("updated"));
        assert_eq!(state.mode, CalcMode::NonResonant);
        assert_eq!(state.velocity_factor, 0.9);
        assert_eq!(state.itu_region, ITURegion::Region3);
        assert_eq!(state.selected_units, Some(UnitSystem::Both));
    }

    #[test]
    fn tui_state_run_calculation_stores_results_view_models() {
        let mut state = TuiState::default();

        state.update(TuiAction::RunCalculation).unwrap();

        assert_eq!(state.focus, TuiFocus::Results);
        assert_eq!(
            state.status_message.as_deref(),
            Some("Calculation complete")
        );
        assert!(state.app_results_view_model.is_some());
        let panel = state
            .results_panel
            .as_ref()
            .expect("expected results panel state");
        assert_eq!(panel.summary.overview_heading, "Resonant Overview:");
        assert!(!panel.summary.summary_lines.is_empty());
        assert!(panel.summary.band_count > 0);
        assert!(!panel.sections.is_empty());
    }

    #[test]
    fn tui_state_recalculates_after_mode_change() {
        let mut state = TuiState::default();

        state
            .update(TuiAction::SetModeAndRunCalculation(CalcMode::NonResonant))
            .unwrap();

        let panel = state
            .results_panel
            .as_ref()
            .expect("expected results panel state");
        assert_eq!(
            panel.summary.overview_heading,
            "Non-resonant Overview (band context):"
        );
        assert_eq!(state.focus, TuiFocus::Results);
    }

    #[test]
    fn tui_state_recalculates_after_band_change() {
        let mut state = TuiState::default();

        state
            .update(TuiAction::SetBandIndicesAndRunCalculation(vec![4, 6]))
            .unwrap();

        let panel = state
            .results_panel
            .as_ref()
            .expect("expected results panel state");
        assert_eq!(state.band_indices, vec![4, 6]);
        assert_eq!(panel.summary.band_count, 2);
    }

    #[test]
    fn tui_state_recalculates_after_region_change() {
        let mut state = TuiState::default();

        state
            .update(TuiAction::SetRegionAndRunCalculation(ITURegion::Region2))
            .unwrap();

        assert_eq!(state.itu_region, ITURegion::Region2);
        assert!(state.results_panel.is_some());
    }

    #[test]
    fn tui_state_recalculates_after_antenna_change() {
        let mut state = TuiState::default();

        state
            .update(TuiAction::SetAntennaModelAndRunCalculation(Some(
                AntennaModel::EndFedHalfWave,
            )))
            .unwrap();

        assert_eq!(state.antenna_model, Some(AntennaModel::EndFedHalfWave));
        let panel = state
            .results_panel
            .as_ref()
            .expect("expected results panel state");
        assert!(panel.summary.overview_heading.contains("Resonant Overview"));
    }

    #[test]
    fn tui_state_recalculates_after_wire_window_change() {
        let mut state = TuiState::default();

        state
            .update(TuiAction::SetMode(CalcMode::NonResonant))
            .unwrap();
        state
            .update(TuiAction::SetWireWindowAndRunCalculation {
                min_m: 12.0,
                max_m: 24.0,
            })
            .unwrap();

        assert_eq!(state.wire_min_m, 12.0);
        assert_eq!(state.wire_max_m, 24.0);
        let panel = state
            .results_panel
            .as_ref()
            .expect("expected results panel state");
        assert_eq!(
            panel.summary.overview_heading,
            "Non-resonant Overview (band context):"
        );
    }

    #[test]
    fn tui_state_recalculates_after_transformer_change() {
        let mut state = TuiState::default();

        state
            .update(TuiAction::SetTransformerRatioAndRunCalculation(
                TransformerRatio::R1To9,
            ))
            .unwrap();

        assert_eq!(state.transformer_ratio, TransformerRatio::R1To9);
        assert!(state.results_panel.is_some());
    }

    #[test]
    fn tui_state_recalculates_after_selected_units_change() {
        let mut state = TuiState::default();

        state
            .update(TuiAction::SetSelectedUnitsAndRunCalculation(Some(
                UnitSystem::Imperial,
            )))
            .unwrap();

        assert_eq!(state.selected_units, Some(UnitSystem::Imperial));
        assert!(state.results_panel.is_some());
    }

    #[test]
    fn tui_results_panel_state_splits_render_contract() {
        let mut state = TuiState::default();

        state.update(TuiAction::RunCalculation).unwrap();

        let panel = state
            .results_panel
            .as_ref()
            .expect("expected results panel state");
        assert!(!panel.summary.summary_lines.is_empty());
        assert!(panel.warnings.warning_lines.is_empty());
        assert!(panel.sections.iter().all(|section| section.line_count > 0));
    }

    #[test]
    fn render_tui_scaffold_uses_tui_render_contract() {
        let mut state = TuiState::default();

        state.update(TuiAction::RunCalculation).unwrap();

        let rendered = render_tui_scaffold(&state);
        assert!(rendered.contains("Rusty Wire TUI"));
        assert!(rendered.contains("[Summary]"));
        assert!(rendered.contains("[Sections]"));
        assert!(rendered.contains("[Warnings]"));
        assert!(rendered.contains("heading: Resonant Overview:"));
        assert!(rendered.contains("bands: 7"));
        assert!(rendered.contains("sections: 2"));
    }

    #[test]
    fn render_panel_helpers_use_split_contract() {
        let panel = TuiResultsPanelState {
            summary: TuiSummaryPanelState {
                overview_heading: "Example".to_string(),
                summary_lines: vec!["line one".to_string()],
                band_count: 2,
            },
            warnings: TuiWarningsPanelState {
                warning_lines: vec!["warn".to_string()],
            },
            sections: vec![TuiSectionPanelState {
                heading: Some("Section A".to_string()),
                line_count: 3,
            }],
        };

        let summary_lines = render_summary_panel(&panel.summary);
        let warning_lines = render_warnings_panel(&panel.warnings);
        let section_lines = render_sections_panel(&panel.sections);

        assert!(summary_lines
            .iter()
            .any(|line| line.contains("heading: Example")));
        assert!(warning_lines.iter().any(|line| line.contains("warn")));
        assert!(section_lines
            .iter()
            .any(|line| line.contains("Section A [3 lines]")));
    }

    #[test]
    fn render_panel_block_wraps_lines() {
        let block = render_panel_block("Demo", &["  alpha".to_string(), "  beta".to_string()]);
        assert_eq!(block[0], "[Demo]");
        assert!(block.iter().any(|line| line.contains("alpha")));
    }

    #[test]
    fn input_lines_reflect_current_tui_inputs() {
        let state = TuiState {
            mode: CalcMode::NonResonant,
            itu_region: ITURegion::Region3,
            band_indices: vec![5, 7],
            velocity_factor: 0.82,
            transformer_ratio: TransformerRatio::R1To49,
            wire_min_m: 10.0,
            wire_max_m: 18.0,
            selected_units: Some(UnitSystem::Imperial),
            ..TuiState::default()
        };

        let lines = input_lines(&state);

        assert!(lines.iter().any(|line| line.contains("NonResonant")));
        assert!(lines.iter().any(|line| line.contains("Region3")));
        assert!(lines.iter().any(|line| line.contains("[5, 7]")));
        assert!(lines.iter().any(|line| line.contains("0.82")));
        assert!(lines.iter().any(|line| line.contains("Imperial")));
    }

    #[test]
    fn export_lines_use_request_draft_state() {
        let state = TuiState {
            mode: CalcMode::NonResonant,
            itu_region: ITURegion::Region2,
            band_indices: vec![4, 8],
            selected_units: Some(UnitSystem::Metric),
            export_format: ExportFormat::Json,
            export_output_path: "demo.json".to_string(),
            ..TuiState::default()
        };

        let lines = export_lines(&state);

        assert!(lines.iter().any(|line| line.contains("[4, 8]")));
        assert!(lines.iter().any(|line| line.contains("NonResonant")));
        assert!(lines.iter().any(|line| line.contains("Region2")));
        assert!(lines.iter().any(|line| line.contains("Metric")));
        assert!(lines.iter().any(|line| line.contains("Json")));
        assert!(lines.iter().any(|line| line.contains("demo.json")));
    }

    #[test]
    fn handle_key_event_cycles_focus() {
        let mut state = TuiState::default();

        let should_quit = handle_key_event(&mut state, KeyCode::Tab).unwrap();

        assert!(!should_quit);
        assert_eq!(state.focus, TuiFocus::Results);
    }

    #[test]
    fn handle_key_event_quits_on_q() {
        let mut state = TuiState::default();

        let should_quit = handle_key_event(&mut state, KeyCode::Char('q')).unwrap();

        assert!(should_quit);
    }

    #[test]
    fn handle_key_event_runs_calculation_for_mode_toggle() {
        let mut state = TuiState::default();

        let should_quit = handle_key_event(&mut state, KeyCode::Char('m')).unwrap();

        assert!(!should_quit);
        assert_eq!(state.mode, CalcMode::NonResonant);
        assert!(state.results_panel.is_some());
    }

    #[test]
    fn handle_key_event_starts_input_edit_in_inputs_focus() {
        let mut state = TuiState::default();
        state.focus = TuiFocus::Inputs;

        let should_quit = handle_key_event(&mut state, KeyCode::Char('i')).unwrap();

        assert!(!should_quit);
        assert!(state.editing_input);
        assert!(!state.input_buffer.is_empty());
    }

    #[test]
    fn handle_key_event_cycles_input_fields() {
        let mut state = TuiState::default();
        state.focus = TuiFocus::Inputs;

        handle_key_event(&mut state, KeyCode::Char('j')).unwrap();
        assert_eq!(state.active_input_field, TuiInputField::VelocityFactor);

        handle_key_event(&mut state, KeyCode::Char('k')).unwrap();
        assert_eq!(state.active_input_field, TuiInputField::BandIndices);
    }

    #[test]
    fn apply_input_edit_updates_velocity_and_runs() {
        let mut state = TuiState::default();
        state.active_input_field = TuiInputField::VelocityFactor;
        state.editing_input = true;
        state.input_buffer = "0.88".to_string();

        let should_quit = handle_key_event(&mut state, KeyCode::Enter).unwrap();

        assert!(!should_quit);
        assert!((state.velocity_factor - 0.88).abs() < 0.0001);
        assert!(state.results_panel.is_some());
    }

    #[test]
    fn refresh_export_preview_populates_preview_lines() {
        let mut state = TuiState::default();
        state.update(TuiAction::RunCalculation).unwrap();
        state.export_format = ExportFormat::Txt;

        state.refresh_export_preview();

        assert!(state.export_preview.is_some());
        assert!(!state.export_preview.unwrap().is_empty());
    }
}

fn parse_band_indices(raw: &str) -> Option<Vec<usize>> {
    let mut indices = Vec::new();
    for token in raw.split(',') {
        let parsed = token.trim().parse::<usize>().ok()?;
        indices.push(parsed);
    }

    if indices.is_empty() {
        None
    } else {
        Some(indices)
    }
}
