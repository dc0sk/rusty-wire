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
//! | `e` | Export results as CSV (`rusty-wire-results.csv`) |
//! | `E` | Export results as JSON (`rusty-wire-results.json`) |
//! | `m` | Export results as Markdown (`rusty-wire-results.md`) |
//! | `t` | Export results as plain text (`rusty-wire-results.txt`) |
//! | `i` / `?` | Toggle project info popup |
//! | `Tab` | Toggle focus between config and results panels |
//! | `q` / `Esc` | Quit |
//! | `Ctrl-C` | Quit |
//! | `PgUp` / `PgDn` | Scroll results (results panel focused) |

use std::io::{self, Stdout};
use std::panic;
use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Terminal;

use crate::app::{
    apply_action, band_listing_view, parse_band_selection, results_display_document, AntennaModel,
    AppAction, AppState, CalcMode, ExportFormat, UnitSystem, STANDARD_ANTENNA_HEIGHTS_M,
};
use crate::band_presets::load_named_presets;
use crate::bands::ITURegion;
use crate::calculations::{GroundClass, TransformerRatio};
use crate::export::{default_output_name, export_results};

// ---------------------------------------------------------------------------
// Preset tables — values the user cycles through with ←/→
// ---------------------------------------------------------------------------

const VF_PRESETS: &[f64] = &[0.50, 0.60, 0.66, 0.70, 0.80, 0.85, 0.90, 0.95, 0.97, 1.00];
const WIRE_MIN_PRESETS: &[f64] = &[5.0, 8.0, 10.0, 12.0, 15.0, 20.0];
const WIRE_MAX_PRESETS: &[f64] = &[20.0, 25.0, 30.0, 35.0, 40.0, 50.0, 60.0, 80.0, 100.0];
const CONDUCTOR_DIAMETER_PRESETS: &[f64] = &[1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0];
const GROUND_CLASS_PRESETS: &[GroundClass] =
    &[GroundClass::Poor, GroundClass::Average, GroundClass::Good];
const STEP_PRESETS: &[f64] = &[0.01, 0.02, 0.05, 0.10, 0.25, 0.50, 1.00];
const PROJECT_URL: &str = env!("CARGO_PKG_REPOSITORY");
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
const BUILTIN_BAND_PRESETS: &[(&str, &str)] = &[
    ("40m–10m (7 bands)", "40m,30m,20m,17m,15m,12m,10m"),
    ("80m–10m (8 bands)", "80m,40m,30m,20m,17m,15m,12m,10m"),
    ("160m–10m (9 bands)", "160m,80m,40m,30m,20m,17m,15m,12m,10m"),
    ("20m–10m (5 bands)", "20m,17m,15m,12m,10m"),
    ("Contest 80/40/20/15/10", "80m,40m,20m,15m,10m"),
];

const DEFAULT_BAND_PRESET_CONFIG: &str = "bands.toml";

/// Named frequency presets for explicit multi-frequency runs.
const FREQUENCY_PRESETS: &[(&str, &[f64])] = &[
    ("Use bands", &[] as &[f64]), // sentinel: revert to band-based selection
    ("3.5 MHz", &[3.5]),
    ("7.074 MHz", &[7.074]),
    ("14.074 MHz", &[14.074]),
    ("3.5, 7.0 MHz", &[3.5, 7.0]),
    ("7.0, 14.0 MHz", &[7.0, 14.0]),
    ("3.5, 7.0, 14.0 MHz", &[3.5, 7.0, 14.0]),
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

#[derive(Debug, Clone)]
struct BandPresetChoice {
    label: String,
    selection: Option<String>,
}

impl BandPresetChoice {
    fn named(label: impl Into<String>, selection: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            selection: Some(selection.into()),
        }
    }

    fn custom() -> Self {
        Self {
            label: "Custom…".to_string(),
            selection: None,
        }
    }

    fn is_custom(&self) -> bool {
        self.selection.is_none()
    }
}

/// Editable fields shown in the configuration panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigField {
    Mode,
    AntennaModel,
    ItuRegion,
    Bands,
    CustomFrequencies,
    VelocityFactor,
    TransformerRatio,
    Units,
    WireMin,
    WireMax,
    AntennaHeight,
    GroundClassField,
    ConductorDiameter,
    StepSize,
}

impl ConfigField {
    const ALL: &'static [Self] = &[
        Self::Mode,
        Self::AntennaModel,
        Self::ItuRegion,
        Self::Bands,
        Self::CustomFrequencies,
        Self::VelocityFactor,
        Self::TransformerRatio,
        Self::Units,
        Self::WireMin,
        Self::WireMax,
        Self::AntennaHeight,
        Self::GroundClassField,
        Self::ConductorDiameter,
        Self::StepSize,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Mode => "Mode",
            Self::AntennaModel => "Antenna",
            Self::ItuRegion => "ITU Region",
            Self::Bands => "Bands",
            Self::CustomFrequencies => "Frequencies",
            Self::VelocityFactor => "Vel. Factor",
            Self::TransformerRatio => "Transformer",
            Self::Units => "Units",
            Self::WireMin => "Wire Min",
            Self::WireMax => "Wire Max",
            Self::AntennaHeight => "Height",
            Self::GroundClassField => "Ground",
            Self::ConductorDiameter => "Conductor",
            Self::StepSize => "Step",
        }
    }
}

/// All TUI-local state that is NOT part of the app-layer `AppState`.
struct TuiState {
    app: AppState,
    focus: Focus,
    /// Index into `ConfigField::ALL`.
    field_idx: usize,
    /// Available built-in and TOML-loaded band presets.
    band_presets: Vec<BandPresetChoice>,
    /// Index into `band_presets`.
    band_preset_idx: usize,
    /// Index into `VF_PRESETS`.
    vf_idx: usize,
    /// Index into `TRANSFORMER_RATIOS`.
    ratio_idx: usize,
    /// Index into `WIRE_MIN_PRESETS`.
    wire_min_idx: usize,
    /// Index into `WIRE_MAX_PRESETS`.
    wire_max_idx: usize,
    /// Index into `STANDARD_ANTENNA_HEIGHTS_M`.
    height_idx: usize,
    /// Index into `GROUND_CLASS_PRESETS`.
    ground_idx: usize,
    /// Index into `CONDUCTOR_DIAMETER_PRESETS`.
    conductor_idx: usize,
    /// Index into `STEP_PRESETS`.
    step_idx: usize,
    /// Index into `FREQUENCY_PRESETS`.
    freq_idx: usize,
    /// Vertical scroll offset for the results panel.
    results_scroll: u16,
    /// Whether the project-info popup is visible.
    show_info_popup: bool,
    /// Set to `true` to exit the event loop.
    quit: bool,
    /// Whether the band-checklist overlay is open.
    show_band_checklist: bool,
    /// Items in the checklist: (1-based band index, display label, checked).
    band_checklist_items: Vec<(usize, String, bool)>,
    /// Cursor row in the band checklist.
    band_checklist_cursor: usize,
    /// Last confirmed custom band selection (empty until user confirms once).
    custom_band_indices: Vec<usize>,
    /// Status message shown in the hints bar.
    export_status: Option<String>,
}

impl TuiState {
    fn new(band_preset_config: Option<&str>) -> Self {
        let app = AppState::default();
        let (band_presets, preset_status) = load_tui_band_presets(band_preset_config);
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
        let height_idx = STANDARD_ANTENNA_HEIGHTS_M
            .iter()
            .position(|&v| (v - app.config.antenna_height_m).abs() < 1e-9)
            .unwrap_or(1); // 10.0 m
        let ground_idx = GROUND_CLASS_PRESETS
            .iter()
            .position(|&g| g == app.config.ground_class)
            .unwrap_or(1); // Average
        let conductor_idx = CONDUCTOR_DIAMETER_PRESETS
            .iter()
            .position(|&v| (v - app.config.conductor_diameter_mm).abs() < 1e-9)
            .unwrap_or(2); // 2.0 mm
        let step_idx = STEP_PRESETS
            .iter()
            .position(|&v| (v - app.config.step_m).abs() < 1e-9)
            .unwrap_or(2); // 0.05 m
        Self {
            app,
            focus: Focus::Config,
            field_idx: 0,
            band_presets,
            band_preset_idx: 0,
            vf_idx,
            ratio_idx,
            wire_min_idx,
            wire_max_idx,
            height_idx,
            ground_idx,
            conductor_idx,
            step_idx,
            freq_idx: 0, // "Use bands" (empty list means revert to band selection)
            results_scroll: 0,
            show_info_popup: false,
            quit: false,
            show_band_checklist: false,
            band_checklist_items: Vec::new(),
            band_checklist_cursor: 0,
            custom_band_indices: Vec::new(),
            export_status: preset_status,
        }
    }

    fn current_band_preset(&self) -> &BandPresetChoice {
        &self.band_presets[self.band_preset_idx]
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
                        Some(AntennaModel::TrapDipole) => "Trap Dipole".into(),
                    },
                    ConfigField::ItuRegion => match c.itu_region {
                        ITURegion::Region1 => "1 (EU/AF/ME)".into(),
                        ITURegion::Region2 => "2 (Americas)".into(),
                        ITURegion::Region3 => "3 (Asia-Pac)".into(),
                    },
                    ConfigField::Bands => {
                        if self.current_band_preset().is_custom() {
                            // Custom sentinel
                            if self.custom_band_indices.is_empty() {
                                "Custom…".into()
                            } else {
                                format!("Custom ({} bands)", self.custom_band_indices.len())
                            }
                        } else {
                            self.current_band_preset().label.clone()
                        }
                    }
                    ConfigField::CustomFrequencies => FREQUENCY_PRESETS[self.freq_idx].0.into(),
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
                    ConfigField::AntennaHeight => {
                        format!("{:.0} m", STANDARD_ANTENNA_HEIGHTS_M[self.height_idx])
                    }
                    ConfigField::GroundClassField => match GROUND_CLASS_PRESETS[self.ground_idx] {
                        GroundClass::Poor => "Poor".into(),
                        GroundClass::Average => "Average".into(),
                        GroundClass::Good => "Good".into(),
                    },
                    ConfigField::ConductorDiameter => {
                        format!("{:.1} mm", CONDUCTOR_DIAMETER_PRESETS[self.conductor_idx])
                    }
                    ConfigField::StepSize => {
                        let s = STEP_PRESETS[self.step_idx];
                        if s < 0.1 {
                            format!("{:.2} m", s)
                        } else {
                            format!("{:.2} m", s)
                        }
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
        let current_band_indices = self.app.config.band_indices.clone();

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
                    Some(AntennaModel::TrapDipole),
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
                    self.band_preset_idx = (self.band_preset_idx + 1) % self.band_presets.len();
                } else {
                    self.band_preset_idx = self
                        .band_preset_idx
                        .checked_sub(1)
                        .unwrap_or(self.band_presets.len() - 1);
                }
                // Custom sentinel opens the checklist; keep the last confirmed
                // custom selection, or fall back to the current active bands.
                if self.current_band_preset().is_custom() {
                    let indices = if !self.custom_band_indices.is_empty() {
                        self.custom_band_indices.clone()
                    } else {
                        current_band_indices
                    };
                    AppAction::SetBandIndices(indices)
                } else if let Some(selection) = self.current_band_preset().selection.as_deref() {
                    match parse_band_selection(selection, region) {
                        Ok(indices) => AppAction::SetBandIndices(indices),
                        Err(err) => {
                            self.export_status = Some(format!(
                                "Preset '{}' is invalid for Region {}: {err}",
                                self.current_band_preset().label,
                                region.short_name()
                            ));
                            AppAction::SetBandIndices(current_band_indices)
                        }
                    }
                } else {
                    if !self.custom_band_indices.is_empty() {
                        AppAction::SetBandIndices(self.custom_band_indices.clone())
                    } else {
                        AppAction::SetBandIndices(current_band_indices)
                    }
                }
            }
            ConfigField::CustomFrequencies => {
                if forward {
                    self.freq_idx = (self.freq_idx + 1) % FREQUENCY_PRESETS.len();
                } else {
                    self.freq_idx = self
                        .freq_idx
                        .checked_sub(1)
                        .unwrap_or(FREQUENCY_PRESETS.len() - 1);
                }
                // First preset (index 0) has empty list — revert to band selection
                let freqs = FREQUENCY_PRESETS[self.freq_idx].1;
                if freqs.is_empty() {
                    AppAction::SetFreqList(Vec::new())
                } else {
                    AppAction::SetFreqList(freqs.to_vec())
                }
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
            ConfigField::AntennaHeight => {
                if forward {
                    self.height_idx =
                        (self.height_idx + 1).min(STANDARD_ANTENNA_HEIGHTS_M.len() - 1);
                } else if self.height_idx > 0 {
                    self.height_idx -= 1;
                }
                AppAction::SetAntennaHeight(STANDARD_ANTENNA_HEIGHTS_M[self.height_idx])
            }
            ConfigField::GroundClassField => {
                if forward {
                    self.ground_idx = (self.ground_idx + 1) % GROUND_CLASS_PRESETS.len();
                } else {
                    self.ground_idx = self
                        .ground_idx
                        .checked_sub(1)
                        .unwrap_or(GROUND_CLASS_PRESETS.len() - 1);
                }
                AppAction::SetGroundClass(GROUND_CLASS_PRESETS[self.ground_idx])
            }
            ConfigField::ConductorDiameter => {
                if forward {
                    self.conductor_idx =
                        (self.conductor_idx + 1).min(CONDUCTOR_DIAMETER_PRESETS.len() - 1);
                } else if self.conductor_idx > 0 {
                    self.conductor_idx -= 1;
                }
                AppAction::SetConductorDiameter(CONDUCTOR_DIAMETER_PRESETS[self.conductor_idx])
            }
            ConfigField::StepSize => {
                if forward {
                    self.step_idx = (self.step_idx + 1).min(STEP_PRESETS.len() - 1);
                } else if self.step_idx > 0 {
                    self.step_idx -= 1;
                }
                AppAction::SetStep(STEP_PRESETS[self.step_idx])
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

    /// Export the current results to a file.  Sets `export_status` with either
    /// a success message ("Exported to <file>") or an error message.
    fn try_export(&mut self, format: ExportFormat) {
        let Some(ref results) = self.app.results else {
            self.export_status = Some("No results to export — run a calculation first (r).".into());
            return;
        };
        let filename = default_output_name(format);
        match export_results(
            format,
            filename,
            &results.calculations,
            results.recommendation.as_ref(),
            results.config.units,
            results.config.wire_min_m,
            results.config.wire_max_m,
        ) {
            Ok(()) => {
                self.export_status = Some(format!("Exported → {filename}"));
            }
            Err(err) => {
                self.export_status = Some(format!("Export failed: {err}"));
            }
        }
    }

    /// Open the band-checklist overlay, initialising items from the current
    /// custom selection (or the active band indices when no custom exists yet).
    fn open_band_checklist(&mut self) {
        let region = self.app.config.itu_region;
        let active: std::collections::HashSet<usize> = if !self.custom_band_indices.is_empty() {
            self.custom_band_indices.iter().copied().collect()
        } else {
            self.app.config.band_indices.iter().copied().collect()
        };
        self.band_checklist_items = band_listing_view(region)
            .rows
            .into_iter()
            .map(|row| {
                let checked = active.contains(&row.index);
                (row.index, row.display, checked)
            })
            .collect();
        self.band_checklist_cursor = 0;
        self.show_band_checklist = true;
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        // Clear any previous export status on the next keypress.
        self.export_status = None;

        // Band-checklist overlay intercepts all keys.
        if self.show_band_checklist {
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.quit = true;
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.show_band_checklist = false;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.band_checklist_cursor > 0 {
                        self.band_checklist_cursor -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.band_checklist_cursor + 1 < self.band_checklist_items.len() {
                        self.band_checklist_cursor += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    if let Some(item) = self
                        .band_checklist_items
                        .get_mut(self.band_checklist_cursor)
                    {
                        item.2 = !item.2;
                    }
                }
                KeyCode::Enter => {
                    let indices: Vec<usize> = self
                        .band_checklist_items
                        .iter()
                        .filter(|(_, _, checked)| *checked)
                        .map(|(idx, _, _)| *idx)
                        .collect();
                    if !indices.is_empty() {
                        self.custom_band_indices = indices.clone();
                        self.dispatch(AppAction::SetBandIndices(indices));
                    }
                    self.show_band_checklist = false;
                }
                _ => {}
            }
            return;
        }

        // Global shortcuts — active regardless of focused panel.
        match key.code {
            KeyCode::Char('q') => {
                self.quit = true;
                return;
            }
            KeyCode::Esc => {
                if self.show_info_popup {
                    self.show_info_popup = false;
                } else {
                    self.quit = true;
                }
                return;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.quit = true;
                return;
            }
            KeyCode::Char('i') | KeyCode::Char('?') => {
                self.show_info_popup = !self.show_info_popup;
                return;
            }
            KeyCode::Char('r') => {
                self.run_calculation();
                return;
            }
            KeyCode::Char('e') => {
                self.try_export(ExportFormat::Csv);
                return;
            }
            KeyCode::Char('E') => {
                self.try_export(ExportFormat::Json);
                return;
            }
            KeyCode::Char('m') => {
                self.try_export(ExportFormat::Markdown);
                return;
            }
            KeyCode::Char('t') => {
                self.try_export(ExportFormat::Txt);
                return;
            }
            KeyCode::Enter => {
                // When the Bands field is on the Custom sentinel, Enter opens
                // the band-checklist instead of running a calculation.
                if self.focus == Focus::Config
                    && self.current_field() == ConfigField::Bands
                    && self.current_band_preset().is_custom()
                {
                    self.open_band_checklist();
                } else {
                    self.run_calculation();
                }
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

        if self.show_info_popup {
            return;
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

    if state.show_info_popup {
        render_info_popup(f, area);
    }

    if state.show_band_checklist {
        render_band_checklist(f, area, state);
    }
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
    if let Some(ref status) = state.export_status {
        let style = if status.starts_with("Export failed") || status.starts_with("No results") {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Green)
        };
        f.render_widget(Paragraph::new(status.as_str()).style(style), area);
        return;
    }

    let text = hint_text(state.focus, state.show_band_checklist);
    let para = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(para, area);
}

fn hint_text(focus: Focus, show_band_checklist: bool) -> &'static str {
    if show_band_checklist {
        return " ↑↓/jk:move  Space:toggle  Enter:confirm  Esc/q:cancel";
    }

    match focus {
        Focus::Config => {
            " ↑↓/jk:select  ←→/hl:change  r:run  e:csv  E:json  m:md  t:txt  i:info  Tab:→results  q:quit"
        }
        Focus::Results => {
            " ↑↓/jk:scroll  PgUp/Dn:page  r:run  e:csv  E:json  m:md  t:txt  i:info  Tab:→config   q:quit"
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn render_info_popup(f: &mut ratatui::Frame, area: Rect) {
    let popup_area = centered_rect(64, 42, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" About Rusty Wire ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let lines = info_popup_lines();

    let para = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .wrap(Wrap { trim: true });
    f.render_widget(para, popup_area);
}

fn info_popup_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(format!("Version: {}", env!("CARGO_PKG_VERSION"))),
        Line::from(format!("Author: {}", env!("CARGO_PKG_AUTHORS"))),
        Line::from(format!("GitHub: {PROJECT_URL}")),
        Line::from(format!("License: {}", env!("CARGO_PKG_LICENSE"))),
        Line::from(""),
        Line::from(Span::styled(
            "Press i, ?, or Esc to close.",
            Style::default().fg(Color::DarkGray),
        )),
    ]
}

// ---------------------------------------------------------------------------
// Band-checklist overlay
// ---------------------------------------------------------------------------

fn render_band_checklist(f: &mut ratatui::Frame, area: Rect, state: &TuiState) {
    let popup_area = centered_rect(72, 80, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Custom Band Selection  (Space:toggle  Enter:confirm  Esc/q:cancel) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let items: Vec<ListItem> = state
        .band_checklist_items
        .iter()
        .enumerate()
        .map(|(i, (_, display, checked))| {
            let checkbox = if *checked { "[x]" } else { "[ ]" };
            let is_cursor = i == state.band_checklist_cursor;
            let (prefix, style) = if is_cursor {
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
                Span::styled(format!("{checkbox} "), style),
                Span::styled(display.clone(), style),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, inner);
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
pub fn run(band_preset_config: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    // Panic hook: always restore the terminal before printing the panic message.
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let mut terminal = setup_terminal()?;
    let mut state = TuiState::new(band_preset_config);

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

fn load_tui_band_presets(
    band_preset_config: Option<&str>,
) -> (Vec<BandPresetChoice>, Option<String>) {
    let mut presets: Vec<BandPresetChoice> = BUILTIN_BAND_PRESETS
        .iter()
        .map(|(label, selection)| BandPresetChoice::named(*label, *selection))
        .collect();
    let mut status = None;
    let preset_path = band_preset_config.unwrap_or(DEFAULT_BAND_PRESET_CONFIG);
    let should_attempt_load = band_preset_config.is_some() || Path::new(preset_path).exists();

    if should_attempt_load {
        match load_named_presets(preset_path) {
            Ok(named) => {
                presets.extend(named.into_iter().map(|(name, selection)| {
                    BandPresetChoice::named(format!("Preset: {name}"), selection)
                }));
            }
            Err(err) => {
                status = Some(format!("Ignored {preset_path} preset file: {err}"));
            }
        }
    }

    presets.push(BandPresetChoice::custom());
    (presets, status)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn hint_text_for_config_focus_matches_documented_keybindings() {
        let text = hint_text(Focus::Config, false);

        assert!(text.contains("e:csv"));
        assert!(text.contains("E:json"));
        assert!(text.contains("m:md"));
        assert!(text.contains("t:txt"));
        assert!(text.contains("i:info"));
        assert!(text.contains("Tab:→results"));
    }

    #[test]
    fn hint_text_for_results_focus_mentions_scroll_and_tab_back() {
        let text = hint_text(Focus::Results, false);

        assert!(text.contains("↑↓/jk:scroll"));
        assert!(text.contains("PgUp/Dn:page"));
        assert!(text.contains("Tab:→config"));
    }

    #[test]
    fn hint_text_for_band_checklist_matches_overlay_controls() {
        let text = hint_text(Focus::Config, true);

        assert!(text.contains("Space:toggle"));
        assert!(text.contains("Enter:confirm"));
        assert!(text.contains("Esc/q:cancel"));
    }

    #[test]
    fn info_popup_lines_include_required_project_metadata() {
        let lines = info_popup_lines()
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<String>>();

        assert!(lines.iter().any(|line| line.starts_with("Version:")));
        assert!(lines.iter().any(|line| line.starts_with("Author:")));
        assert!(lines.iter().any(|line| line.starts_with("GitHub:")));
        assert!(lines.iter().any(|line| line.starts_with("License:")));
        assert!(lines
            .iter()
            .any(|line| line.contains("Press i, ?, or Esc to close.")));
    }

    #[test]
    fn handle_key_toggles_and_closes_info_popup() {
        let mut state = TuiState::new(None);

        state.handle_key(press(KeyCode::Char('i')));
        assert!(state.show_info_popup);
        assert!(!state.quit);

        state.handle_key(press(KeyCode::Esc));
        assert!(!state.show_info_popup);
        assert!(!state.quit);
    }

    #[test]
    fn tab_toggles_focus_between_config_and_results() {
        let mut state = TuiState::new(None);

        assert_eq!(state.focus, Focus::Config);
        state.handle_key(press(KeyCode::Tab));
        assert_eq!(state.focus, Focus::Results);

        state.handle_key(press(KeyCode::Tab));
        assert_eq!(state.focus, Focus::Config);
    }

    #[test]
    fn config_down_moves_to_next_field_and_wraps() {
        let mut state = TuiState::new(None);

        state.handle_key(press(KeyCode::Down));
        assert_eq!(state.field_idx, 1);

        state.field_idx = ConfigField::ALL.len() - 1;
        state.handle_key(press(KeyCode::Down));
        assert_eq!(state.field_idx, 0);
    }

    #[test]
    fn config_up_wraps_to_last_field() {
        let mut state = TuiState::new(None);

        state.handle_key(press(KeyCode::Up));

        assert_eq!(state.field_idx, ConfigField::ALL.len() - 1);
    }

    #[test]
    fn results_scroll_keys_update_scroll_with_saturation() {
        let mut state = TuiState::new(None);
        state.focus = Focus::Results;

        state.handle_key(press(KeyCode::Down));
        assert_eq!(state.results_scroll, 1);

        state.handle_key(press(KeyCode::Up));
        assert_eq!(state.results_scroll, 0);

        state.handle_key(press(KeyCode::Up));
        assert_eq!(state.results_scroll, 0);
    }

    #[test]
    fn results_page_keys_scroll_by_ten_with_saturation() {
        let mut state = TuiState::new(None);
        state.focus = Focus::Results;

        state.handle_key(press(KeyCode::PageDown));
        assert_eq!(state.results_scroll, 10);

        state.handle_key(press(KeyCode::PageUp));
        assert_eq!(state.results_scroll, 0);

        state.handle_key(press(KeyCode::PageUp));
        assert_eq!(state.results_scroll, 0);
    }

    #[test]
    fn open_band_checklist_prefers_existing_custom_selection() {
        let mut state = TuiState::new(None);
        state.custom_band_indices = vec![2, 5];

        state.open_band_checklist();

        assert!(state.show_band_checklist);
        let checked = state
            .band_checklist_items
            .iter()
            .filter(|(_, _, checked)| *checked)
            .map(|(idx, _, _)| *idx)
            .collect::<Vec<usize>>();
        assert_eq!(checked, vec![2, 5]);
    }

    #[test]
    fn checklist_enter_updates_custom_selection_and_app_band_indices() {
        let mut state = TuiState::new(None);
        state.show_band_checklist = true;
        state.band_checklist_items = vec![
            (1, "160m".to_string(), false),
            (2, "80m".to_string(), true),
            (3, "60m".to_string(), true),
        ];

        state.handle_key(press(KeyCode::Enter));

        assert!(!state.show_band_checklist);
        assert_eq!(state.custom_band_indices, vec![2, 3]);
        assert_eq!(state.app.config.band_indices, vec![2, 3]);
    }

    #[test]
    fn checklist_escape_closes_without_changing_existing_selection() {
        let mut state = TuiState::new(None);
        state.custom_band_indices = vec![4, 6];
        state.show_band_checklist = true;
        state.band_checklist_items = vec![
            (4, "40m".to_string(), true),
            (5, "30m".to_string(), false),
            (6, "20m".to_string(), true),
        ];

        state.handle_key(press(KeyCode::Esc));

        assert!(!state.show_band_checklist);
        assert_eq!(state.custom_band_indices, vec![4, 6]);
        assert_eq!(
            state.app.config.band_indices,
            crate::app::DEFAULT_BAND_SELECTION
        );
    }

    #[test]
    fn try_export_without_results_sets_warning_status() {
        let mut state = TuiState::new(None);

        state.try_export(ExportFormat::Csv);

        assert_eq!(
            state.export_status.as_deref(),
            Some("No results to export — run a calculation first (r).")
        );
    }

    #[test]
    fn next_keypress_clears_export_status_before_handling_action() {
        let mut state = TuiState::new(None);
        state.export_status = Some("Exported → rusty-wire-results.csv".to_string());

        state.handle_key(press(KeyCode::Char('i')));

        assert!(state.export_status.is_none());
        assert!(state.show_info_popup);
    }

    #[test]
    fn band_preset_cycle_to_named_preset_returns_parsed_band_indices() {
        let mut state = TuiState::new(None);
        state.field_idx = ConfigField::ALL
            .iter()
            .position(|field| *field == ConfigField::Bands)
            .expect("Bands field should exist");

        let action = state.compute_action(true);

        match action {
            AppAction::SetBandIndices(indices) => {
                let expected =
                    parse_band_selection(BUILTIN_BAND_PRESETS[1].1, state.app.config.itu_region)
                        .expect("built-in preset should parse");
                assert_eq!(indices, expected);
                assert_eq!(state.band_preset_idx, 1);
            }
            other => panic!("expected SetBandIndices, got {other:?}"),
        }
    }

    #[test]
    fn band_preset_cycle_to_custom_reuses_last_confirmed_custom_indices() {
        let mut state = TuiState::new(None);
        state.field_idx = ConfigField::ALL
            .iter()
            .position(|field| *field == ConfigField::Bands)
            .expect("Bands field should exist");
        state.band_preset_idx = state.band_presets.len() - 2;
        state.custom_band_indices = vec![4, 6, 8];

        let action = state.compute_action(true);

        assert!(state.current_band_preset().is_custom());
        match action {
            AppAction::SetBandIndices(indices) => assert_eq!(indices, vec![4, 6, 8]),
            other => panic!("expected SetBandIndices, got {other:?}"),
        }
    }

    #[test]
    fn frequency_preset_cycle_forward_sets_single_frequency_list() {
        let mut state = TuiState::new(None);
        state.field_idx = ConfigField::ALL
            .iter()
            .position(|field| *field == ConfigField::CustomFrequencies)
            .expect("CustomFrequencies field should exist");

        let action = state.compute_action(true);

        match action {
            AppAction::SetFreqList(freqs) => {
                assert_eq!(freqs, vec![3.5]);
                assert_eq!(state.freq_idx, 1);
            }
            other => panic!("expected SetFreqList, got {other:?}"),
        }
    }

    #[test]
    fn frequency_preset_cycle_backward_wraps_to_last_multi_frequency_set() {
        let mut state = TuiState::new(None);
        state.field_idx = ConfigField::ALL
            .iter()
            .position(|field| *field == ConfigField::CustomFrequencies)
            .expect("CustomFrequencies field should exist");

        let action = state.compute_action(false);

        match action {
            AppAction::SetFreqList(freqs) => {
                assert_eq!(freqs, vec![3.5, 7.0, 14.0]);
                assert_eq!(state.freq_idx, FREQUENCY_PRESETS.len() - 1);
            }
            other => panic!("expected SetFreqList, got {other:?}"),
        }
    }

    #[test]
    fn frequency_preset_use_bands_returns_empty_frequency_list() {
        let mut state = TuiState::new(None);
        state.field_idx = ConfigField::ALL
            .iter()
            .position(|field| *field == ConfigField::CustomFrequencies)
            .expect("CustomFrequencies field should exist");
        state.freq_idx = 1;

        let action = state.compute_action(false);

        match action {
            AppAction::SetFreqList(freqs) => {
                assert!(freqs.is_empty());
                assert_eq!(state.freq_idx, 0);
            }
            other => panic!("expected SetFreqList, got {other:?}"),
        }
    }

    #[test]
    fn load_tui_band_presets_always_keeps_custom_choice_last() {
        let (presets, status) = load_tui_band_presets(None);

        assert!(status.is_none());
        assert!(presets.last().is_some_and(BandPresetChoice::is_custom));
    }
}
