//! TUI front-end for Rusty Wire — ratatui + crossterm.
//!
//! Architecture:
//! ```text
//! src/bin/tui.rs  →  tui::run()
//!                         │
//!                    event loop
//!                         │
//!                  handle_key(key)
//!                         │
//!                AppAction dispatch
//!                         │
//!                 apply_action()      ← pure, no I/O
//!                         │
//!                    AppState
//!                         │
//!                    render()
//!                         │
//!              ratatui widget tree
//! ```
//!
//! **Keybindings**
//!
//! | Key | Action |
//! |-----|--------|
//! | `↑` / `k` | Select previous config field |
//! | `↓` / `j` | Select next config field |
//! | `←` / `h` | Decrease selected field value |
//! | `→` / `l` | Increase selected field value |
//! | `r` / `Enter` | Run calculation |
//! | `Tab` | Toggle focus between config and results panels |
//! | `q` / `Esc` | Quit |
//! | `Ctrl-C` | Quit |
//! | `PgUp` / `PgDn` | Scroll results (results panel focused) |

use std::io::{self, Stdout};
use std::panic;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Terminal;

use crate::app::{
    apply_action, results_display_document, AntennaModel, AppAction, AppState, CalcMode, UnitSystem,
};
use crate::bands::ITURegion;
use crate::calculations::TransformerRatio;

// ---------------------------------------------------------------------------
// Preset tables — values the user cycles through with ←/→
// ---------------------------------------------------------------------------

const VF_PRESETS: &[f64] = &[0.50, 0.60, 0.66, 0.70, 0.80, 0.85, 0.90, 0.95, 0.97, 1.00];
const WIRE_MIN_PRESETS: &[f64] = &[5.0, 8.0, 10.0, 12.0, 15.0, 20.0];
const WIRE_MAX_PRESETS: &[f64] = &[20.0, 25.0, 30.0, 35.0, 40.0, 50.0, 60.0, 80.0, 100.0];
const TRANSFORMER_RATIOS: &[TransformerRatio] = &[
    TransformerRatio::R1To1,
    TransformerRatio::R1To2,
    TransformerRatio::R1To4,
    TransformerRatio::R1To5,
    TransformerRatio::R1To6,
    TransformerRatio::R1To9,
    TransformerRatio::R1To16,
    TransformerRatio::R1To49,
    TransformerRatio::R1To56,
    TransformerRatio::R1To64,
];

/// Named band presets that work in all three ITU regions.
///
/// Indices are 1-based.  All selected indices exist across all three regions
/// (they are the common HF amateur allocations).
const BAND_PRESETS: &[(&str, &[usize])] = &[
    ("40m–10m (7 bands)", &[4, 5, 6, 7, 8, 9, 10]),
    ("80m–10m (8 bands)", &[2, 4, 5, 6, 7, 8, 9, 10]),
    ("160m–10m (9 bands)", &[1, 2, 4, 5, 6, 7, 8, 9, 10]),
    ("20m–10m (5 bands)", &[6, 7, 8, 9, 10]),
    ("Contest 80/40/20/15/10", &[2, 4, 6, 8, 10]),
];

// ---------------------------------------------------------------------------
// TUI-local types
// ---------------------------------------------------------------------------

/// Which panel currently receives keyboard input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Config,
    Results,
}

/// Editable fields shown in the configuration panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigField {
    Mode,
    AntennaModel,
    ItuRegion,
    Bands,
    VelocityFactor,
    TransformerRatio,
    Units,
    WireMin,
    WireMax,
}

impl ConfigField {
    const ALL: &'static [Self] = &[
        Self::Mode,
        Self::AntennaModel,
        Self::ItuRegion,
        Self::Bands,
        Self::VelocityFactor,
        Self::TransformerRatio,
        Self::Units,
        Self::WireMin,
        Self::WireMax,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Mode => "Mode",
            Self::AntennaModel => "Antenna",
            Self::ItuRegion => "ITU Region",
            Self::Bands => "Bands",
            Self::VelocityFactor => "Vel. Factor",
            Self::TransformerRatio => "Transformer",
            Self::Units => "Units",
            Self::WireMin => "Wire Min",
            Self::WireMax => "Wire Max",
        }
    }
}

/// All TUI-local state that is NOT part of the app-layer `AppState`.
struct TuiState {
    app: AppState,
    focus: Focus,
    /// Index into `ConfigField::ALL`.
    field_idx: usize,
    /// Index into `BAND_PRESETS`.
    band_preset_idx: usize,
    /// Index into `VF_PRESETS`.
    vf_idx: usize,
    /// Index into `TRANSFORMER_RATIOS`.
    ratio_idx: usize,
    /// Index into `WIRE_MIN_PRESETS`.
    wire_min_idx: usize,
    /// Index into `WIRE_MAX_PRESETS`.
    wire_max_idx: usize,
    /// Vertical scroll offset for the results panel.
    results_scroll: u16,
    /// Set to `true` to exit the event loop.
    quit: bool,
}

impl TuiState {
    fn new() -> Self {
        let app = AppState::default();
        // Derive preset indices from the default AppConfig values.
        let vf = app.config.velocity_factor;
        let vf_idx = VF_PRESETS
            .iter()
            .position(|&v| (v - vf).abs() < 1e-9)
            .unwrap_or(7); // 0.95 is at index 7
        let ratio = app.config.transformer_ratio;
        let ratio_idx = TRANSFORMER_RATIOS
            .iter()
            .position(|&r| r == ratio)
            .unwrap_or(0);
        // DEFAULT_NON_RESONANT_CONFIG defaults: 8.0 / 35.0 m
        let wire_min_idx = WIRE_MIN_PRESETS
            .iter()
            .position(|&v| (v - app.config.wire_min_m).abs() < 0.5)
            .unwrap_or(1); // 8.0 m
        let wire_max_idx = WIRE_MAX_PRESETS
            .iter()
            .position(|&v| (v - app.config.wire_max_m).abs() < 0.5)
            .unwrap_or(3); // 35.0 m
        Self {
            app,
            focus: Focus::Config,
            field_idx: 0,
            band_preset_idx: 0, // 40m–10m default
            vf_idx,
            ratio_idx,
            wire_min_idx,
            wire_max_idx,
            results_scroll: 0,
            quit: false,
        }
    }

    fn current_field(&self) -> ConfigField {
        ConfigField::ALL[self.field_idx]
    }

    /// Return (label, value, is_selected) for every config field.
    fn all_field_values(&self) -> Vec<(String, String, bool)> {
        let c = &self.app.config;
        ConfigField::ALL
            .iter()
            .enumerate()
            .map(|(i, &field)| {
                let value: String = match field {
                    ConfigField::Mode => match c.mode {
                        CalcMode::Resonant => "Resonant".into(),
                        CalcMode::NonResonant => "Non-resonant".into(),
                    },
                    ConfigField::AntennaModel => match c.antenna_model {
                        None => "All".into(),
                        Some(AntennaModel::Dipole) => "Dipole".into(),
                        Some(AntennaModel::InvertedVDipole) => "Inverted-V".into(),
                        Some(AntennaModel::EndFedHalfWave) => "EFHW".into(),
                        Some(AntennaModel::FullWaveLoop) => "Loop".into(),
                        Some(AntennaModel::OffCenterFedDipole) => "OCFD".into(),
                    },
                    ConfigField::ItuRegion => match c.itu_region {
                        ITURegion::Region1 => "1 (EU/AF/ME)".into(),
                        ITURegion::Region2 => "2 (Americas)".into(),
                        ITURegion::Region3 => "3 (Asia-Pac)".into(),
                    },
                    ConfigField::Bands => BAND_PRESETS[self.band_preset_idx].0.into(),
                    ConfigField::VelocityFactor => format!("{:.2}", VF_PRESETS[self.vf_idx]),
                    ConfigField::TransformerRatio => {
                        TRANSFORMER_RATIOS[self.ratio_idx].as_label().into()
                    }
                    ConfigField::Units => match c.units {
                        UnitSystem::Metric => "Metric (m)".into(),
                        UnitSystem::Imperial => "Imperial (ft)".into(),
                        UnitSystem::Both => "Both".into(),
                    },
                    ConfigField::WireMin => {
                        format!("{:.1} m", WIRE_MIN_PRESETS[self.wire_min_idx])
                    }
                    ConfigField::WireMax => {
                        format!("{:.1} m", WIRE_MAX_PRESETS[self.wire_max_idx])
                    }
                };
                let selected = i == self.field_idx && self.focus == Focus::Config;
                (field.label().to_string(), value, selected)
            })
            .collect()
    }

    /// Compute the `AppAction` for incrementing or decrementing the selected
    /// config field.  Mutates preset indices as a side-effect.
    fn compute_action(&mut self, forward: bool) -> AppAction {
        // Copy all needed config values up front to avoid borrow conflicts.
        let mode = self.app.config.mode;
        let antenna = self.app.config.antenna_model;
        let region = self.app.config.itu_region;
        let units = self.app.config.units;

        match self.current_field() {
            ConfigField::Mode => AppAction::SetMode(match mode {
                CalcMode::Resonant => CalcMode::NonResonant,
                CalcMode::NonResonant => CalcMode::Resonant,
            }),
            ConfigField::AntennaModel => {
                const MODELS: &[Option<AntennaModel>] = &[
                    None,
                    Some(AntennaModel::Dipole),
                    Some(AntennaModel::InvertedVDipole),
                    Some(AntennaModel::EndFedHalfWave),
                    Some(AntennaModel::FullWaveLoop),
                    Some(AntennaModel::OffCenterFedDipole),
                ];
                let pos = MODELS.iter().position(|m| *m == antenna).unwrap_or(0);
                let next = if forward {
                    (pos + 1) % MODELS.len()
                } else {
                    pos.checked_sub(1).unwrap_or(MODELS.len() - 1)
                };
                AppAction::SetAntennaModel(MODELS[next])
            }
            ConfigField::ItuRegion => {
                const REGIONS: &[ITURegion] =
                    &[ITURegion::Region1, ITURegion::Region2, ITURegion::Region3];
                let pos = REGIONS.iter().position(|&r| r == region).unwrap_or(0);
                let next = if forward {
                    (pos + 1) % REGIONS.len()
                } else {
                    pos.checked_sub(1).unwrap_or(REGIONS.len() - 1)
                };
                AppAction::SetItuRegion(REGIONS[next])
            }
            ConfigField::Bands => {
                if forward {
                    self.band_preset_idx = (self.band_preset_idx + 1) % BAND_PRESETS.len();
                } else {
                    self.band_preset_idx = self
                        .band_preset_idx
                        .checked_sub(1)
                        .unwrap_or(BAND_PRESETS.len() - 1);
                }
                AppAction::SetBandIndices(BAND_PRESETS[self.band_preset_idx].1.to_vec())
            }
            ConfigField::VelocityFactor => {
                if forward {
                    self.vf_idx = (self.vf_idx + 1).min(VF_PRESETS.len() - 1);
                } else if self.vf_idx > 0 {
                    self.vf_idx -= 1;
                }
                AppAction::SetVelocityFactor(VF_PRESETS[self.vf_idx])
            }
            ConfigField::TransformerRatio => {
                if forward {
                    self.ratio_idx = (self.ratio_idx + 1) % TRANSFORMER_RATIOS.len();
                } else {
                    self.ratio_idx = self
                        .ratio_idx
                        .checked_sub(1)
                        .unwrap_or(TRANSFORMER_RATIOS.len() - 1);
                }
                AppAction::SetTransformerRatio(TRANSFORMER_RATIOS[self.ratio_idx])
            }
            ConfigField::Units => {
                const ORDER: &[UnitSystem] =
                    &[UnitSystem::Both, UnitSystem::Metric, UnitSystem::Imperial];
                let pos = ORDER.iter().position(|&u| u == units).unwrap_or(0);
                let next = if forward {
                    (pos + 1) % ORDER.len()
                } else {
                    pos.checked_sub(1).unwrap_or(ORDER.len() - 1)
                };
                AppAction::SetUnits(ORDER[next])
            }
            ConfigField::WireMin => {
                if forward {
                    self.wire_min_idx = (self.wire_min_idx + 1).min(WIRE_MIN_PRESETS.len() - 1);
                } else if self.wire_min_idx > 0 {
                    self.wire_min_idx -= 1;
                }
                AppAction::SetWireMin(WIRE_MIN_PRESETS[self.wire_min_idx])
            }
            ConfigField::WireMax => {
                if forward {
                    self.wire_max_idx = (self.wire_max_idx + 1).min(WIRE_MAX_PRESETS.len() - 1);
                } else if self.wire_max_idx > 0 {
                    self.wire_max_idx -= 1;
                }
                AppAction::SetWireMax(WIRE_MAX_PRESETS[self.wire_max_idx])
            }
        }
    }

    fn dispatch(&mut self, action: AppAction) {
        self.app = apply_action(self.app.clone(), action);
    }

    fn run_calculation(&mut self) {
        self.results_scroll = 0;
        self.dispatch(AppAction::RunCalculation);
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Global shortcuts — active regardless of focused panel.
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.quit = true;
                return;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.quit = true;
                return;
            }
            KeyCode::Char('r') | KeyCode::Enter => {
                self.run_calculation();
                return;
            }
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Config => Focus::Results,
                    Focus::Results => Focus::Config,
                };
                return;
            }
            _ => {}
        }

        // Panel-specific shortcuts.
        match self.focus {
            Focus::Config => match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    self.field_idx = (self.field_idx + 1) % ConfigField::ALL.len();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.field_idx = self
                        .field_idx
                        .checked_sub(1)
                        .unwrap_or(ConfigField::ALL.len() - 1);
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    let action = self.compute_action(true);
                    self.dispatch(action);
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    let action = self.compute_action(false);
                    self.dispatch(action);
                }
                _ => {}
            },
            Focus::Results => match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    self.results_scroll = self.results_scroll.saturating_add(1);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.results_scroll = self.results_scroll.saturating_sub(1);
                }
                KeyCode::PageDown => {
                    self.results_scroll = self.results_scroll.saturating_add(10);
                }
                KeyCode::PageUp => {
                    self.results_scroll = self.results_scroll.saturating_sub(10);
                }
                _ => {}
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render(f: &mut ratatui::Frame, state: &TuiState) {
    let area = f.area();

    // Enforce a minimum usable size.
    if area.width < 60 || area.height < 12 {
        let msg = Paragraph::new("Terminal too small — resize to at least 60×12")
            .style(Style::default().fg(Color::Red));
        f.render_widget(msg, area);
        return;
    }

    // Outer: title (1) | body | hints (1)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_title(f, outer[0]);

    // Body: config (38%) | results (62%)
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(outer[1]);

    render_config_panel(f, panels[0], state);
    render_results_panel(f, panels[1], state);
    render_hints(f, outer[2], state);
}

fn render_title(f: &mut ratatui::Frame, area: Rect) {
    let title = Paragraph::new(format!(
        " Rusty Wire TUI v{}  —  wire antenna calculator",
        env!("CARGO_PKG_VERSION")
    ))
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(title, area);
}

fn render_config_panel(f: &mut ratatui::Frame, area: Rect, state: &TuiState) {
    let focused = state.focus == Focus::Config;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Configuration  (←→ change  ↑↓ select) ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = state
        .all_field_values()
        .into_iter()
        .map(|(label, value, selected)| {
            let (prefix, style) = if selected {
                (
                    "► ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ("  ", Style::default().fg(Color::White))
            };
            let line = Line::from(vec![
                Span::styled(prefix.to_string(), style),
                Span::styled(format!("{:<12}", label), style),
                Span::styled(value, style),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, inner);
}

fn render_results_panel(f: &mut ratatui::Frame, area: Rect, state: &TuiState) {
    let focused = state.focus == Focus::Results;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Results  (↑↓/PgUp/Dn scroll) ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line<'static>> = if let Some(ref err) = state.app.error {
        vec![
            Line::from(Span::styled(
                "Error:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                err.to_string(),
                Style::default().fg(Color::Red),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Adjust configuration above and press r to retry.",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else if let Some(ref results) = state.app.results {
        let doc = results_display_document(results);
        let mut out: Vec<Line<'static>> = Vec::new();

        out.push(Line::from(Span::styled(
            doc.overview_heading.to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        for l in &doc.overview_header_lines {
            out.push(Line::from(l.clone()));
        }
        out.push(Line::from(""));

        for band_view in &doc.band_views {
            out.push(Line::from(Span::styled(
                band_view.title.clone(),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));
            for l in &band_view.lines {
                out.push(Line::from(l.clone()));
            }
            out.push(Line::from(""));
        }

        for l in &doc.summary_lines {
            out.push(Line::from(l.clone()));
        }
        out.push(Line::from(""));

        for section in &doc.sections {
            for (i, l) in section.lines.iter().enumerate() {
                let style = if i == 0 {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                out.push(Line::from(vec![Span::styled(l.clone(), style)]));
            }
            out.push(Line::from(""));
        }

        for w in &doc.warning_lines {
            out.push(Line::from(Span::styled(
                w.clone(),
                Style::default().fg(Color::Yellow),
            )));
        }

        out
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No results yet.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Configure your antenna above, then press r to calculate.",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((state.results_scroll, 0));
    f.render_widget(para, inner);
}

fn render_hints(f: &mut ratatui::Frame, area: Rect, state: &TuiState) {
    let text = match state.focus {
        Focus::Config => " ↑↓/jk:select  ←→/hl:change  r:run  Tab:→results  q:quit",
        Focus::Results => " ↑↓/jk:scroll  PgUp/Dn:page  r:run  Tab:→config   q:quit",
    };
    let para = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(para, area);
}

// ---------------------------------------------------------------------------
// Terminal lifecycle
// ---------------------------------------------------------------------------

type Term = Terminal<CrosstermBackend<Stdout>>;

fn setup_terminal() -> Result<Term, Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Term) -> Result<(), Box<dyn std::error::Error>> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Launch the TUI.
///
/// Sets up the crossterm/ratatui terminal, runs the event loop until the
/// user quits, then restores the terminal.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Panic hook: always restore the terminal before printing the panic message.
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let mut terminal = setup_terminal()?;
    let mut state = TuiState::new();

    // Run an initial calculation so the results panel is populated immediately.
    state.run_calculation();

    loop {
        terminal.draw(|f| render(f, &state))?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                state.handle_key(key);
            }
        }

        if state.quit {
            break;
        }
    }

    restore_terminal(&mut terminal)?;
    Ok(())
}
