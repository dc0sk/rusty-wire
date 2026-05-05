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
//! | `s` | Save current settings as persistent preferences (`~/.config/rusty-wire/config.toml`) |
//! | `a` | Toggle balun/unun advise panel |
//! | `i` / `?` | Toggle project info popup |
//! | `Tab` | Toggle focus between config and results panels |
//! | `q` / `Esc` | Quit |
//! | `Ctrl-C` | Quit |
//! | `PgUp` / `PgDn` | Scroll results (results panel focused) |

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Stdout};
use std::panic;
use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::{CrosstermBackend, TestBackend};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Terminal;

use crate::app::{
    apply_action, band_listing_view, build_advise_candidates, execute_request_checked,
    parse_band_selection, results_display_document, AdviseView, AntennaModel, AppAction, AppConfig,
    AppRequest, AppState, CalcMode, ExportFormat, UnitSystem, STANDARD_ANTENNA_HEIGHTS_M,
};
use crate::band_presets::load_named_presets;
use crate::bands::ITURegion;
use crate::calculations::{GroundClass, TransformerRatio};
use crate::export::{
    default_advise_output_name, default_output_name, export_advise, export_results,
    export_results_nec, to_advise_csv, to_advise_html, to_advise_json, to_advise_markdown,
    to_advise_txt, to_advise_yaml, to_csv, to_html, to_json, to_markdown, to_txt, to_yaml,
};

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
/// Some labels include the current region's 80m/40m allocations so the user
/// can immediately see when a preset will map to the wider Region 2/3 edges.
const BUILTIN_BAND_PRESET_TEMPLATES: &[(&str, &str, bool)] = &[
    ("40m–10m (7 bands)", "40m,30m,20m,17m,15m,12m,10m", true),
    ("80m–10m (8 bands)", "80m,40m,30m,20m,17m,15m,12m,10m", true),
    (
        "160m–10m + 60m (10 bands)",
        "160m,80m,60m,40m,30m,20m,17m,15m,12m,10m",
        true,
    ),
    ("20m–10m (5 bands)", "20m,17m,15m,12m,10m", false),
    ("Contest 80/40/20/15/10", "80m,40m,20m,15m,10m", true),
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
    /// Optional explicit bands.toml path passed into the TUI.
    band_preset_config: Option<String>,
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
    /// Whether to show ranked wire + balun/unun advise candidates in results.
    show_advise_panel: bool,
    /// Cached advise candidates for the current configuration.
    advise_view: Option<AdviseView>,
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
    /// When set, an auto-recalculation is pending after this instant + debounce.
    pending_recalc: Option<std::time::Instant>,
    /// Active export preview: (format, is_advise, content).
    export_preview: Option<(ExportFormat, bool, String)>,
    /// Vertical scroll offset for the export preview overlay.
    preview_scroll: u16,
    /// Set of band titles whose detail lines are collapsed in the results panel.
    collapsed_bands: std::collections::HashSet<String>,
    /// Index of the band currently selected for toggle in the results panel.
    results_band_cursor: usize,
    /// Whether the session-name input overlay is open (save flow).
    show_session_save: bool,
    /// Text being typed in the session-name input overlay.
    session_name_input: String,
    /// Whether the session picker/load overlay is open.
    show_session_picker: bool,
    /// Names of sessions loaded for the picker.
    session_picker_items: Vec<String>,
    /// Cursor row in the session picker.
    session_picker_cursor: usize,
}

impl TuiState {
    fn new(band_preset_config: Option<&str>) -> Self {
        let mut app = AppState::default();

        // Apply persistent user preferences over the compiled-in defaults so
        // the TUI starts with the user's preferred region, mode, velocity, etc.
        let prefs = crate::prefs::UserPrefs::load();
        prefs.apply_to_config(&mut app.config);

        let band_preset_config = band_preset_config.map(str::to_string);
        let (band_presets, preset_status) =
            load_tui_band_presets(band_preset_config.as_deref(), app.config.itu_region);
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
            band_preset_config,
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
            show_advise_panel: false,
            advise_view: None,
            quit: false,
            show_band_checklist: false,
            band_checklist_items: Vec::new(),
            band_checklist_cursor: 0,
            custom_band_indices: Vec::new(),
            export_status: preset_status,
            pending_recalc: None,
            export_preview: None,
            preview_scroll: 0,
            collapsed_bands: std::collections::HashSet::new(),
            results_band_cursor: 0,
            show_session_save: false,
            session_name_input: String::new(),
            show_session_picker: false,
            session_picker_items: Vec::new(),
            session_picker_cursor: 0,
        }
    }

    fn current_band_preset(&self) -> &BandPresetChoice {
        &self.band_presets[self.band_preset_idx]
    }

    fn current_field(&self) -> ConfigField {
        ConfigField::ALL[self.field_idx]
    }

    fn refresh_band_presets_for_region(&mut self) {
        let current_selection = self.current_band_preset().selection.clone();
        let was_custom = self.current_band_preset().is_custom();
        let (band_presets, preset_status) = load_tui_band_presets(
            self.band_preset_config.as_deref(),
            self.app.config.itu_region,
        );

        self.band_presets = band_presets;
        self.band_preset_idx = if was_custom {
            self.band_presets.len() - 1
        } else if let Some(selection) = current_selection.as_deref() {
            self.band_presets
                .iter()
                .position(|preset| preset.selection.as_deref() == Some(selection))
                .unwrap_or(0)
        } else {
            0
        };

        if let Some(status) = preset_status {
            self.export_status = Some(status);
        }
    }

    fn refresh_band_checklist_for_region(&mut self) {
        if !self.show_band_checklist {
            return;
        }

        let checked: std::collections::HashSet<usize> = self
            .band_checklist_items
            .iter()
            .filter(|(_, _, is_checked)| *is_checked)
            .map(|(idx, _, _)| *idx)
            .collect();

        self.band_checklist_items = band_listing_view(self.app.config.itu_region)
            .rows
            .into_iter()
            .map(|row| {
                let is_checked = checked.contains(&row.index);
                (row.index, row.display, is_checked)
            })
            .collect();

        if self.band_checklist_items.is_empty() {
            self.band_checklist_cursor = 0;
        } else {
            self.band_checklist_cursor = self
                .band_checklist_cursor
                .min(self.band_checklist_items.len() - 1);
        }
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
                        format!("{:.2} m", s)
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
        // Schedule a debounced auto-recalculation for any config-change action.
        // RunCalculation itself must NOT set this or we'd loop.
        let is_config_change = !matches!(
            action,
            AppAction::RunCalculation | AppAction::ClearResults | AppAction::ClearError
        );
        let region_change = match &action {
            AppAction::SetItuRegion(region) => Some((
                *region,
                self.current_band_preset().selection.clone(),
                self.current_band_preset().label.clone(),
                self.current_band_preset().is_custom(),
                self.app.config.band_indices.clone(),
            )),
            _ => None,
        };

        self.app = apply_action(self.app.clone(), action);
        self.show_advise_panel = false;
        self.advise_view = None;

        if let Some((region, selection, label, was_custom, previous_indices)) = region_change {
            self.refresh_band_presets_for_region();
            self.refresh_band_checklist_for_region();

            if was_custom {
                if !self.custom_band_indices.is_empty() {
                    self.app = apply_action(
                        self.app.clone(),
                        AppAction::SetBandIndices(self.custom_band_indices.clone()),
                    );
                }
                return;
            }

            if let Some(selection) = selection.as_deref() {
                match parse_band_selection(selection, region) {
                    Ok(indices) => {
                        self.app =
                            apply_action(self.app.clone(), AppAction::SetBandIndices(indices));
                    }
                    Err(err) => {
                        self.export_status = Some(format!(
                            "Preset '{label}' is invalid for Region {}: {err}",
                            region.short_name()
                        ));
                        self.app = apply_action(
                            self.app.clone(),
                            AppAction::SetBandIndices(previous_indices),
                        );
                    }
                }
            }
        }
        if is_config_change {
            self.pending_recalc = Some(std::time::Instant::now());
        }
    }

    fn toggle_advise_panel(&mut self) {
        if self.show_advise_panel {
            self.show_advise_panel = false;
            return;
        }

        if let Err(err) = execute_request_checked(AppRequest::new(self.app.config.clone())) {
            self.export_status = Some(format!("Cannot build advise candidates: {err}"));
            return;
        }

        let view = build_advise_candidates(&self.app.config, 5);
        if view.candidates.is_empty() {
            self.export_status = Some("No advise candidates available for current setup.".into());
            return;
        }

        self.results_scroll = 0;
        self.advise_view = Some(view);
        self.show_advise_panel = true;
    }

    fn run_calculation(&mut self) {
        self.results_scroll = 0;
        self.pending_recalc = None;
        self.collapsed_bands.clear();
        self.results_band_cursor = 0;
        self.dispatch(AppAction::RunCalculation);
    }

    /// Automatic recalculation triggered by the debounce timer.
    /// Does NOT reset the scroll position so the user can read results
    /// while tweaking config fields.
    fn auto_recalculate(&mut self) {
        self.pending_recalc = None;
        self.dispatch(AppAction::RunCalculation);
    }

    /// Number of band sections in the current results (0 if none).
    fn band_count(&self) -> usize {
        self.app
            .results
            .as_ref()
            .map(|r| {
                let doc = results_display_document(r);
                doc.band_views.len()
            })
            .unwrap_or(0)
    }

    /// Toggle collapse of the band at `results_band_cursor`.
    fn toggle_band_at_cursor(&mut self) {
        if self.show_advise_panel {
            return;
        }
        let Some(ref results) = self.app.results else {
            return;
        };
        let doc = results_display_document(results);
        if doc.band_views.is_empty() {
            return;
        }
        let idx = self.results_band_cursor.min(doc.band_views.len() - 1);
        let title = doc.band_views[idx].title.clone();
        if self.collapsed_bands.contains(&title) {
            self.collapsed_bands.remove(&title);
        } else {
            self.collapsed_bands.insert(title);
        }
    }

    /// Move the band cursor forward (next=true) or backward.
    fn move_band_cursor(&mut self, next: bool) {
        if self.show_advise_panel {
            return;
        }
        let count = self.band_count();
        if count == 0 {
            return;
        }
        if next {
            self.results_band_cursor = (self.results_band_cursor + 1).min(count - 1);
        } else {
            self.results_band_cursor = self.results_band_cursor.saturating_sub(1);
        }
    }

    /// Build a preview string for the given format and open the preview overlay.
    /// When `show_advise_panel` is active and an advise view exists, previews the
    /// advise export; otherwise previews results.
    fn try_export(&mut self, format: ExportFormat) {
        if self.show_advise_panel {
            let Some(ref view) = self.advise_view else {
                self.export_status = Some("No advise results — run advise first (a).".into());
                return;
            };
            let content = match format {
                ExportFormat::Csv => to_advise_csv(view.assumed_feedpoint_ohm, &view.candidates),
                ExportFormat::Html => to_advise_html(view.assumed_feedpoint_ohm, &view.candidates),
                ExportFormat::Json => to_advise_json(view.assumed_feedpoint_ohm, &view.candidates),
                ExportFormat::Markdown => {
                    to_advise_markdown(view.assumed_feedpoint_ohm, &view.candidates)
                }
                // NEC export has no advise variant; fall back to plain text.
                ExportFormat::Nec => to_advise_txt(view.assumed_feedpoint_ohm, &view.candidates),
                ExportFormat::Txt => to_advise_txt(view.assumed_feedpoint_ohm, &view.candidates),
                ExportFormat::Yaml => to_advise_yaml(view.assumed_feedpoint_ohm, &view.candidates),
            };
            self.export_preview = Some((format, true, content));
            self.preview_scroll = 0;
        } else {
            let Some(ref results) = self.app.results else {
                self.export_status =
                    Some("No results to export — run a calculation first (r).".into());
                return;
            };
            let content = match format {
                ExportFormat::Csv => to_csv(
                    &results.calculations,
                    results.recommendation.as_ref(),
                    results.config.units,
                    results.config.wire_min_m,
                    results.config.wire_max_m,
                ),
                ExportFormat::Html => to_html(
                    &results.calculations,
                    results.recommendation.as_ref(),
                    results.config.units,
                    results.config.wire_min_m,
                    results.config.wire_max_m,
                ),
                ExportFormat::Json => to_json(
                    &results.calculations,
                    results.recommendation.as_ref(),
                    results.config.units,
                    results.config.wire_min_m,
                    results.config.wire_max_m,
                ),
                ExportFormat::Markdown => to_markdown(
                    &results.calculations,
                    results.recommendation.as_ref(),
                    results.config.units,
                    results.config.wire_min_m,
                    results.config.wire_max_m,
                ),
                ExportFormat::Nec => crate::nec_export::to_nec(
                    &results.calculations,
                    &results.config,
                    env!("CARGO_PKG_VERSION"),
                ),
                ExportFormat::Txt => to_txt(
                    &results.calculations,
                    results.recommendation.as_ref(),
                    results.config.units,
                    results.config.wire_min_m,
                    results.config.wire_max_m,
                ),
                ExportFormat::Yaml => to_yaml(
                    &results.calculations,
                    results.recommendation.as_ref(),
                    results.config.units,
                    results.config.wire_min_m,
                    results.config.wire_max_m,
                ),
            };
            self.export_preview = Some((format, false, content));
            self.preview_scroll = 0;
        }
    }

    /// Commit the active export preview to disk.
    fn confirm_export(&mut self) {
        let Some((format, is_advise, ref content)) = self.export_preview.take() else {
            return;
        };
        let filename = if is_advise {
            default_advise_output_name(format)
        } else {
            default_output_name(format)
        };
        let result = if is_advise {
            let Some(ref view) = self.advise_view else {
                self.export_status = Some("Advise data lost — cannot write.".into());
                return;
            };
            export_advise(
                format,
                filename,
                view.assumed_feedpoint_ohm,
                &view.candidates,
            )
        } else {
            let Some(ref results) = self.app.results else {
                self.export_status = Some("Results lost — cannot write.".into());
                return;
            };
            if format == ExportFormat::Nec {
                export_results_nec(filename, &results.calculations, &results.config)
            } else {
                export_results(
                    format,
                    filename,
                    &results.calculations,
                    results.recommendation.as_ref(),
                    results.config.units,
                    results.config.wire_min_m,
                    results.config.wire_max_m,
                )
            }
        };
        match result {
            Ok(()) => self.export_status = Some(format!("Exported → {filename}")),
            Err(err) => self.export_status = Some(format!("Export failed: {err}")),
        }
        // content was already taken, preview is closed
        let _ = content; // silence unused warning
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

    /// Resync all TUI preset-index fields to match a newly loaded `AppConfig`.
    /// Called after loading a session so the config panel reflects the session values.
    fn sync_indices_from_config(&mut self, config: &AppConfig) {
        if let Some(idx) = VF_PRESETS
            .iter()
            .position(|&v| (v - config.velocity_factor).abs() < 1e-9)
        {
            self.vf_idx = idx;
        }
        if let Some(idx) = TRANSFORMER_RATIOS
            .iter()
            .position(|&r| r == config.transformer_ratio)
        {
            self.ratio_idx = idx;
        }
        if let Some(idx) = WIRE_MIN_PRESETS
            .iter()
            .position(|&v| (v - config.wire_min_m).abs() < 1e-9)
        {
            self.wire_min_idx = idx;
        }
        if let Some(idx) = WIRE_MAX_PRESETS
            .iter()
            .position(|&v| (v - config.wire_max_m).abs() < 1e-9)
        {
            self.wire_max_idx = idx;
        }
        if let Some(idx) = STANDARD_ANTENNA_HEIGHTS_M
            .iter()
            .position(|&v| (v - config.antenna_height_m).abs() < 1e-9)
        {
            self.height_idx = idx;
        }
        if let Some(idx) = GROUND_CLASS_PRESETS
            .iter()
            .position(|&g| g == config.ground_class)
        {
            self.ground_idx = idx;
        }
        if let Some(idx) = CONDUCTOR_DIAMETER_PRESETS
            .iter()
            .position(|&v| (v - config.conductor_diameter_mm).abs() < 1e-9)
        {
            self.conductor_idx = idx;
        }
        self.custom_band_indices = config.band_indices.clone();
        if let Some(idx) = STEP_PRESETS
            .iter()
            .position(|&v| (v - config.step_m).abs() < 1e-9)
        {
            self.step_idx = idx;
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        // Clear any previous export status on the next keypress.
        self.export_status = None;

        // Export preview overlay intercepts all keys.
        if self.export_preview.is_some() {
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.quit = true;
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.export_preview = None;
                }
                KeyCode::Enter => {
                    self.confirm_export();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.preview_scroll = self.preview_scroll.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.preview_scroll = self.preview_scroll.saturating_add(1);
                }
                KeyCode::PageUp => {
                    self.preview_scroll = self.preview_scroll.saturating_sub(20);
                }
                KeyCode::PageDown => {
                    self.preview_scroll = self.preview_scroll.saturating_add(20);
                }
                _ => {}
            }
            return;
        }

        // Session-name input overlay (save flow).
        if self.show_session_save {
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.quit = true;
                }
                KeyCode::Esc => {
                    self.show_session_save = false;
                    self.session_name_input.clear();
                }
                KeyCode::Enter => {
                    let name = self.session_name_input.trim().to_string();
                    if !name.is_empty() {
                        match crate::sessions::SessionStore::save(&name, &self.app.config) {
                            Ok(()) => {
                                self.export_status = Some(format!("Session \"{name}\" saved."));
                            }
                            Err(err) => {
                                self.export_status = Some(format!("Session save failed: {err}"));
                            }
                        }
                    }
                    self.show_session_save = false;
                    self.session_name_input.clear();
                }
                KeyCode::Backspace => {
                    self.session_name_input.pop();
                }
                KeyCode::Char(c)
                    // Reject control characters; allow printable.
                    if !c.is_control() => {
                        self.session_name_input.push(c);
                    }
                _ => {}
            }
            return;
        }

        // Session picker overlay (load/delete flow).
        if self.show_session_picker {
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.quit = true;
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.show_session_picker = false;
                }
                KeyCode::Up | KeyCode::Char('k') if self.session_picker_cursor > 0 => {
                    self.session_picker_cursor -= 1;
                }
                KeyCode::Down | KeyCode::Char('j')
                    if self.session_picker_cursor + 1 < self.session_picker_items.len() =>
                {
                    self.session_picker_cursor += 1;
                }
                KeyCode::Enter => {
                    if let Some(name) = self
                        .session_picker_items
                        .get(self.session_picker_cursor)
                        .cloned()
                    {
                        if let Some(config) = crate::sessions::SessionStore::load_config(&name) {
                            self.app.config = config.clone();
                            // Sync TUI preset indices to the loaded config.
                            self.sync_indices_from_config(&config);
                            self.export_status = Some(format!("Session \"{name}\" loaded."));
                        }
                    }
                    self.show_session_picker = false;
                }
                KeyCode::Char('d') => {
                    if let Some(name) = self
                        .session_picker_items
                        .get(self.session_picker_cursor)
                        .cloned()
                    {
                        match crate::sessions::SessionStore::delete(&name) {
                            Ok(_) => {
                                self.export_status = Some(format!("Session \"{name}\" deleted."));
                                // Refresh list.
                                self.session_picker_items = crate::sessions::SessionStore::list();
                                self.session_picker_cursor = self
                                    .session_picker_cursor
                                    .min(self.session_picker_items.len().saturating_sub(1));
                                if self.session_picker_items.is_empty() {
                                    self.show_session_picker = false;
                                }
                            }
                            Err(err) => {
                                self.export_status = Some(format!("Delete failed: {err}"));
                            }
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // Band-checklist overlay intercepts all keys.
        if self.show_band_checklist {
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.quit = true;
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.show_band_checklist = false;
                }
                KeyCode::Up | KeyCode::Char('k') if self.band_checklist_cursor > 0 => {
                    self.band_checklist_cursor -= 1;
                }
                KeyCode::Down | KeyCode::Char('j')
                    if self.band_checklist_cursor + 1 < self.band_checklist_items.len() =>
                {
                    self.band_checklist_cursor += 1;
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
            KeyCode::Char('y') => {
                self.try_export(ExportFormat::Yaml);
                return;
            }
            KeyCode::Char('H') => {
                self.try_export(ExportFormat::Html);
                return;
            }
            KeyCode::Char('N') => {
                self.try_export(ExportFormat::Nec);
                return;
            }
            KeyCode::Char('s') => {
                let prefs = crate::prefs::UserPrefs::from_config(&self.app.config);
                match prefs.save() {
                    Ok(()) => {
                        self.export_status = Some(format!(
                            "Preferences saved → {}",
                            crate::prefs::UserPrefs::prefs_path_display()
                        ));
                    }
                    Err(err) => {
                        self.export_status = Some(format!("Preferences save failed: {err}"));
                    }
                }
                return;
            }
            KeyCode::Char('S') => {
                self.show_session_save = true;
                self.session_name_input.clear();
                return;
            }
            KeyCode::Char('O') => {
                self.session_picker_items = crate::sessions::SessionStore::list();
                if self.session_picker_items.is_empty() {
                    self.export_status = Some("No saved sessions yet. Press S to save one.".into());
                } else {
                    self.session_picker_cursor = 0;
                    self.show_session_picker = true;
                }
                return;
            }
            KeyCode::Char('a') => {
                self.toggle_advise_panel();
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
                KeyCode::Char(' ') | KeyCode::Enter => {
                    self.toggle_band_at_cursor();
                }
                KeyCode::Char(']') | KeyCode::Char('n') => {
                    self.move_band_cursor(true);
                }
                KeyCode::Char('[') | KeyCode::Char('p') => {
                    self.move_band_cursor(false);
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

    if state.export_preview.is_some() {
        render_export_preview(f, area, state);
    }

    if state.show_session_save {
        render_session_name_input(f, area, state);
    }

    if state.show_session_picker {
        render_session_picker(f, area, state);
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

    // Determine the best-recommended transformer ratio (from advise view, if cached).
    let recommended_ratio = state
        .advise_view
        .as_ref()
        .and_then(|v| v.candidates.first())
        .map(|c| c.ratio);

    // Count skipped bands from the last completed results.
    let skipped_count = state
        .app
        .results
        .as_ref()
        .map(|r| r.skipped_band_indices.len())
        .unwrap_or(0);

    let items: Vec<ListItem> = state
        .all_field_values()
        .into_iter()
        .enumerate()
        .map(|(i, (label, value, selected))| {
            let is_transformer_field = ConfigField::ALL[i] == ConfigField::TransformerRatio;
            let is_bands_field = ConfigField::ALL[i] == ConfigField::Bands;

            // Annotate the Bands value when some bands were skipped.
            let display_value = if is_bands_field && skipped_count > 0 {
                format!("{value}  ⚠ {skipped_count} skipped")
            } else {
                value
            };

            let (prefix, style) = if selected {
                (
                    "► ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_transformer_field
                && recommended_ratio.is_some_and(|r| r == state.app.config.transformer_ratio)
            {
                // Current transformer ratio is the recommended one → green highlight.
                (
                    "✓ ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_bands_field && skipped_count > 0 {
                ("  ", Style::default().fg(Color::Yellow))
            } else {
                ("  ", Style::default().fg(Color::White))
            };
            let line = Line::from(vec![
                Span::styled(prefix.to_string(), style),
                Span::styled(format!("{:<12}", label), style),
                Span::styled(display_value, style),
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

    let title = if state.show_advise_panel {
        " Advise  (↑↓/PgUp/Dn scroll) "
    } else {
        " Results  (↑↓/PgUp/Dn scroll) "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line<'static>> = if state.show_advise_panel {
        if let Some(ref view) = state.advise_view {
            render_advise_lines(view)
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No advise data yet. Press a to generate candidates.",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
    } else if let Some(ref err) = state.app.error {
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
        let band_count = doc.band_views.len();
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

        for (i, band_view) in doc.band_views.iter().enumerate() {
            let is_cursor = state.focus == Focus::Results
                && i == state.results_band_cursor.min(band_count.saturating_sub(1));
            let is_collapsed = state.collapsed_bands.contains(&band_view.title);
            let indicator = if is_collapsed { "▶ " } else { "▼ " };
            let title_text = format!("{}{}", indicator, band_view.title);
            let header_style = if is_cursor {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            };
            out.push(Line::from(Span::styled(title_text, header_style)));
            if !is_collapsed {
                for l in &band_view.lines {
                    out.push(Line::from(l.clone()));
                }
                out.push(Line::from(""));
            }
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

fn render_advise_lines(view: &AdviseView) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = Vec::new();

    out.push(Line::from(Span::styled(
        "Advise candidates",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    out.push(Line::from(format!(
        "Assumed feedpoint impedance: {:.0} ohm",
        view.assumed_feedpoint_ohm
    )));

    if let Some(ref cmp) = view.efhw_comparison {
        out.push(Line::from(""));
        out.push(Line::from(Span::styled(
            format!(
                "EFHW transformer comparison (feedpoint R: {:.0} \u{03a9}):",
                cmp.feedpoint_r_ohm
            ),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        out.push(Line::from(format!(
            "  {:<5}  {:<8}  {:<6}  {:<11}  {}",
            "Ratio", "Target Z", "SWR", "Efficiency", "Loss"
        )));
        for entry in &cmp.entries {
            let marker = if entry.is_best {
                "  \u{2190} recommended"
            } else {
                ""
            };
            let style = if entry.is_best {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            out.push(Line::from(Span::styled(
                format!(
                    "  {:<5}  {:>5.0} \u{03a9}  {:>4.2}:1  {:>9.2}%  {:.3} dB{}",
                    entry.ratio.as_label(),
                    entry.target_z_ohm,
                    entry.swr,
                    entry.efficiency_pct,
                    entry.mismatch_loss_db,
                    marker
                ),
                style,
            )));
        }
    }

    out.push(Line::from(""));

    for (idx, candidate) in view.candidates.iter().enumerate() {
        let title_style = match candidate.validation_status {
            Some(crate::fnec_validation::ValidationStatus::Rejected) => {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            }
            Some(crate::fnec_validation::ValidationStatus::Warning) => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            _ => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        };
        let badge = match candidate.validation_status {
            Some(crate::fnec_validation::ValidationStatus::Passed) => "  [PASSED]",
            Some(crate::fnec_validation::ValidationStatus::Warning) => "  [WARNING]",
            Some(crate::fnec_validation::ValidationStatus::Rejected) => "  [REJECTED]",
            _ => "",
        };
        out.push(Line::from(Span::styled(
            format!(
                "{:2}. ratio {}  wire {:.2} m ({:.2} ft){}",
                idx + 1,
                candidate.ratio.as_label(),
                candidate.recommended_length_m,
                candidate.recommended_length_ft,
                badge
            ),
            title_style,
        )));
        out.push(Line::from(format!(
            "    efficiency {:.2}%  mismatch loss {:.3} dB  clearance {:.2}%",
            candidate.estimated_efficiency_pct,
            candidate.mismatch_loss_db,
            candidate.min_resonance_clearance_pct
        )));
        out.push(Line::from(format!(
            "    score {:.2}  correction shift {:.2}%",
            candidate.score, candidate.average_length_shift_pct
        )));
        out.push(Line::from(Span::styled(
            format!("    note: {}", candidate.tradeoff_note),
            Style::default().fg(Color::Yellow),
        )));
        if let Some(note) = &candidate.validation_note {
            let status = candidate
                .validation_status
                .map(|value| value.as_str())
                .unwrap_or(if candidate.validated {
                    "validated"
                } else {
                    "not-validated"
                });
            let fnec_style = match candidate.validation_status {
                Some(crate::fnec_validation::ValidationStatus::Passed) => {
                    Style::default().fg(Color::Green)
                }
                Some(crate::fnec_validation::ValidationStatus::Warning) => {
                    Style::default().fg(Color::Yellow)
                }
                Some(crate::fnec_validation::ValidationStatus::Rejected) => {
                    Style::default().fg(Color::Red)
                }
                _ => Style::default().fg(Color::DarkGray),
            };
            out.push(Line::from(Span::styled(
                format!("    fnec: {status} — {note}"),
                fnec_style,
            )));
        }
        out.push(Line::from(""));
    }

    out.push(Line::from(Span::styled(
        "Note: efficiency and score are model-based estimates for ranking, not lab measurements.",
        Style::default().fg(Color::DarkGray),
    )));

    out
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

    let text = hint_text(
        state.focus,
        state.show_band_checklist,
        state.export_preview.is_some(),
        state.show_session_save,
        state.show_session_picker,
    );
    let para = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(para, area);
}

fn hint_text(
    focus: Focus,
    show_band_checklist: bool,
    show_preview: bool,
    show_session_save: bool,
    show_session_picker: bool,
) -> &'static str {
    if show_session_save {
        return " Type a name  Enter:save  Esc:cancel";
    }
    if show_session_picker {
        return " ↑↓/jk:move  Enter:load  d:delete  Esc/q:cancel";
    }
    if show_preview {
        return " ↑↓/jk:scroll  PgUp/Dn:page  Enter:write  Esc/q:cancel";
    }
    if show_band_checklist {
        return " ↑↓/jk:move  Space:toggle  Enter:confirm  Esc/q:cancel";
    }

    match focus {
        Focus::Config => {
            " ↑↓/jk:select  ←→/hl:change  r:run  a:advise  e:csv  E:json  m:md  t:txt  y:yaml  H:html  s:prefs  S:save-session  O:sessions  i:info  Tab:→results  q:quit"
        }
        Focus::Results => {
            " ↑↓/jk:scroll  PgUp/Dn:page  [/]:band  Space:collapse  r:run  a:advise  e:csv  E:json  m:md  t:txt  y:yaml  H:html  S:save-session  O:sessions  Tab:→config  q:quit"
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
        Line::from(format!(
            "Platform: {}/{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )),
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

fn render_export_preview(f: &mut ratatui::Frame, area: Rect, state: &TuiState) {
    let Some((format, is_advise, ref content)) = state.export_preview else {
        return;
    };
    let filename = if is_advise {
        default_advise_output_name(format)
    } else {
        default_output_name(format)
    };

    let popup_area = centered_rect(90, 85, area);
    f.render_widget(Clear, popup_area);

    let title = format!(" Export preview → {filename}  (Enter:write  Esc:cancel) ");
    let block = Block::default()
        .title(title.as_str())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let lines: Vec<Line<'static>> = content.lines().map(|l| Line::from(l.to_string())).collect();

    let para = Paragraph::new(lines)
        .scroll((state.preview_scroll, 0))
        .style(Style::default().fg(Color::White));
    f.render_widget(para, inner);
}

// ── Session-name input overlay ─────────────────────────────────────────────

fn render_session_name_input(f: &mut ratatui::Frame, area: Rect, state: &TuiState) {
    let popup_area = centered_rect(60, 20, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Save Session ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Split inner: label row / input row / hint row.
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // label
            Constraint::Length(1), // input field
            Constraint::Length(1), // hint
        ])
        .split(inner);

    let label = Paragraph::new("Session name:");
    f.render_widget(label, rows[0]);

    // Show typed text + a blinking-cursor indicator.
    let input_text = format!("{}▏", state.session_name_input);
    let input = Paragraph::new(input_text).style(Style::default().fg(Color::White));
    f.render_widget(input, rows[1]);

    let hint =
        Paragraph::new("Enter: save   Esc: cancel").style(Style::default().fg(Color::DarkGray));
    f.render_widget(hint, rows[2]);
}

// ── Session picker overlay ──────────────────────────────────────────────────

fn render_session_picker(f: &mut ratatui::Frame, area: Rect, state: &TuiState) {
    let popup_area = centered_rect(72, 80, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Load Session ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(0),    // list
            Constraint::Length(1), // hint
        ])
        .split(inner);

    let list_items: Vec<ratatui::widgets::ListItem> = state
        .session_picker_items
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let prefix = if i == state.session_picker_cursor {
                "► "
            } else {
                "  "
            };
            let style = if i == state.session_picker_cursor {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ratatui::widgets::ListItem::new(format!("{prefix}{name}")).style(style)
        })
        .collect();

    let list = ratatui::widgets::List::new(list_items);
    f.render_widget(list, rows[0]);

    let hint = Paragraph::new("↑↓/jk: move   Enter: load   d: delete   Esc/q: cancel")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hint, rows[1]);
}

// ── Band checklist overlay ──────────────────────────────────────────────────

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

struct DocSnapshot {
    id: &'static str,
    title: &'static str,
    html: String,
}

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

    const RECALC_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(300);

    loop {
        // Fire auto-recalculation once the debounce window has elapsed.
        if let Some(pending) = state.pending_recalc {
            if pending.elapsed() >= RECALC_DEBOUNCE {
                state.auto_recalculate();
            }
        }

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

/// Render the canonical TUI documentation snapshots to a single HTML gallery.
///
/// The generated page is intended for deterministic browser-based PNG capture.
/// Each snapshot is wrapped in a `<section>` whose `id` matches the canonical
/// filename stem from `docs/tui-screenshots.md`.
pub fn write_doc_snapshots_html(output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let snapshots = vec![
        DocSnapshot {
            id: "01-default-layout",
            title: "Default Layout",
            html: render_state_html(&build_default_snapshot_state())?,
        },
        DocSnapshot {
            id: "02-trap-dipole-results",
            title: "Trap Dipole Results",
            html: render_state_html(&build_trap_dipole_snapshot_state())?,
        },
        DocSnapshot {
            id: "03-non-resonant-window",
            title: "Non-resonant Window",
            html: render_state_html(&build_non_resonant_snapshot_state())?,
        },
        DocSnapshot {
            id: "04-about-popup",
            title: "About Popup",
            html: render_state_html(&build_about_popup_snapshot_state())?,
        },
        DocSnapshot {
            id: "05-results-scroll",
            title: "Results Scroll",
            html: render_state_html(&build_scrolled_results_snapshot_state())?,
        },
    ];

    let mut page = String::new();
    page.push_str(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>Rusty Wire TUI Doc Snapshots</title>\n<style>\n:root { color-scheme: light; }\nbody { margin: 0; padding: 32px; background: linear-gradient(180deg, #f6f4ef 0%, #ece7dc 100%); font-family: \"IBM Plex Sans\", \"Segoe UI\", sans-serif; color: #1f2328; }\nh1 { margin: 0 0 8px; font-size: 28px; }\np { margin: 0 0 24px; max-width: 72ch; line-height: 1.5; }\n.gallery { display: grid; gap: 28px; }\n.snapshot-card { padding: 20px; background: rgba(255,255,255,0.82); border: 1px solid rgba(15,23,42,0.08); border-radius: 18px; box-shadow: 0 16px 40px rgba(15,23,42,0.10); width: fit-content; }\n.snapshot-card h2 { margin: 0 0 12px; font-size: 16px; font-weight: 600; letter-spacing: 0.02em; }\n.terminal { display: inline-block; background: #1f1f1f; border-radius: 10px; box-shadow: 0 16px 32px rgba(0,0,0,0.24); overflow: hidden; }\n.screen { margin: 0; background: #1f1f1f; color: #d4d4d4; font: 20px/24px \"DejaVu Sans Mono\", \"Liberation Mono\", monospace; white-space: pre; }\n.line { display: block; height: 24px; }\n</style>\n</head>\n<body>\n<h1>Rusty Wire TUI Documentation Snapshots</h1>\n<p>Open this file in the integrated browser and capture each section by id to refresh the canonical PNG assets under <code>docs/images/tui</code>.</p>\n<div class=\"gallery\">\n",
    );

    for snapshot in snapshots {
        writeln!(
            page,
            "<section class=\"snapshot-card\" id=\"{}\">\n<h2>{}</h2>\n<div class=\"terminal\">\n<pre class=\"screen\">{}</pre>\n</div>\n</section>",
            snapshot.id, snapshot.title, snapshot.html
        )?;
    }

    page.push_str("</div>\n</body>\n</html>\n");

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, page)?;
    Ok(())
}

fn build_default_snapshot_state() -> TuiState {
    let mut state = TuiState::new(None);
    state.run_calculation();
    state
}

fn build_trap_dipole_snapshot_state() -> TuiState {
    let mut state = TuiState::new(None);
    state.dispatch(AppAction::SetAntennaModel(Some(AntennaModel::TrapDipole)));
    state.run_calculation();
    state
}

fn build_non_resonant_snapshot_state() -> TuiState {
    let mut state = TuiState::new(None);
    state.band_preset_idx = 3;
    let indices = parse_band_selection(
        BUILTIN_BAND_PRESET_TEMPLATES[3].1,
        state.app.config.itu_region,
    )
    .expect("built-in band preset should parse");
    state.dispatch(AppAction::SetBandIndices(indices));
    state.dispatch(AppAction::SetAntennaModel(Some(
        AntennaModel::EndFedHalfWave,
    )));
    state.dispatch(AppAction::SetMode(CalcMode::NonResonant));
    state.run_calculation();
    state.results_scroll = 28;
    state
}

fn build_about_popup_snapshot_state() -> TuiState {
    let mut state = TuiState::new(None);
    state.show_info_popup = true;
    state
}

fn build_scrolled_results_snapshot_state() -> TuiState {
    let mut state = build_default_snapshot_state();
    state.focus = Focus::Results;
    state.results_scroll = 12;
    state
}

fn render_state_html(state: &TuiState) -> Result<String, Box<dyn std::error::Error>> {
    let width = 120;
    let height = 30;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| render(frame, state))?;
    Ok(buffer_to_html(terminal.backend().buffer(), width, height))
}

fn buffer_to_html(buffer: &Buffer, width: u16, height: u16) -> String {
    let mut out = String::new();

    for y in 0..height {
        out.push_str("<span class=\"line\">");
        for x in 0..width {
            let cell = &buffer[(x, y)];
            let style = cell.style();
            let mut css = format!("color:{};", fg_color_to_css(style.fg));
            if let Some(bg) = bg_color_to_css(style.bg) {
                write!(css, "background:{bg};").expect("css rendering should not fail");
            }
            if style.add_modifier.contains(Modifier::BOLD) {
                css.push_str("font-weight:700;");
            }
            if style.add_modifier.contains(Modifier::ITALIC) {
                css.push_str("font-style:italic;");
            }
            if style.add_modifier.contains(Modifier::UNDERLINED) {
                css.push_str("text-decoration:underline;");
            }

            write!(
                out,
                "<span style=\"{}\">{}</span>",
                css,
                escape_html(cell.symbol())
            )
            .expect("html rendering should not fail");
        }
        out.push_str("</span>\n");
    }

    out
}

fn fg_color_to_css(color: Option<Color>) -> String {
    match color.unwrap_or(Color::Reset) {
        Color::Reset => "#d4d4d4".to_string(),
        Color::Black => "#1f1f1f".to_string(),
        Color::Red => "#f87171".to_string(),
        Color::Green => "#6ee7b7".to_string(),
        Color::Yellow => "#facc15".to_string(),
        Color::Blue => "#60a5fa".to_string(),
        Color::Magenta => "#f472b6".to_string(),
        Color::Cyan => "#67e8f9".to_string(),
        Color::Gray => "#9ca3af".to_string(),
        Color::DarkGray => "#6b7280".to_string(),
        Color::LightRed => "#fca5a5".to_string(),
        Color::LightGreen => "#a7f3d0".to_string(),
        Color::LightYellow => "#fde68a".to_string(),
        Color::LightBlue => "#93c5fd".to_string(),
        Color::LightMagenta => "#f9a8d4".to_string(),
        Color::LightCyan => "#a5f3fc".to_string(),
        Color::White => "#f3f4f6".to_string(),
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Indexed(_) => "#d4d4d4".to_string(),
    }
}

fn bg_color_to_css(color: Option<Color>) -> Option<String> {
    match color.unwrap_or(Color::Reset) {
        Color::Reset => None,
        other => Some(match other {
            Color::Reset => unreachable!(),
            Color::Black => "#1f1f1f".to_string(),
            Color::Red => "#7f1d1d".to_string(),
            Color::Green => "#14532d".to_string(),
            Color::Yellow => "#713f12".to_string(),
            Color::Blue => "#1d4ed8".to_string(),
            Color::Magenta => "#831843".to_string(),
            Color::Cyan => "#155e75".to_string(),
            Color::Gray => "#4b5563".to_string(),
            Color::DarkGray => "#374151".to_string(),
            Color::LightRed => "#991b1b".to_string(),
            Color::LightGreen => "#166534".to_string(),
            Color::LightYellow => "#854d0e".to_string(),
            Color::LightBlue => "#1d4ed8".to_string(),
            Color::LightMagenta => "#9d174d".to_string(),
            Color::LightCyan => "#0f766e".to_string(),
            Color::White => "#6b7280".to_string(),
            Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
            Color::Indexed(_) => "#1f1f1f".to_string(),
        }),
    }
}

fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn user_bands_config_path() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let path = std::path::PathBuf::from(home)
        .join(".config")
        .join("rusty-wire")
        .join("bands.toml");
    if path.exists() {
        Some(path.display().to_string())
    } else {
        None
    }
}

fn region_band_edges_label(region: ITURegion) -> &'static str {
    match region {
        ITURegion::Region1 => " [R1 80m 3.5–3.8 / 40m 7.0–7.2]",
        ITURegion::Region2 => " [R2 80m 3.5–4.0 / 40m 7.0–7.3]",
        ITURegion::Region3 => " [R3 80m 3.5–3.9 / 40m 7.0–7.2]",
    }
}

fn builtin_band_presets(region: ITURegion) -> Vec<BandPresetChoice> {
    BUILTIN_BAND_PRESET_TEMPLATES
        .iter()
        .map(|(label, selection, region_specific)| {
            let label = if *region_specific {
                format!("{label}{}", region_band_edges_label(region))
            } else {
                (*label).to_string()
            };
            BandPresetChoice::named(label, *selection)
        })
        .collect()
}

fn load_tui_band_presets(
    band_preset_config: Option<&str>,
    region: ITURegion,
) -> (Vec<BandPresetChoice>, Option<String>) {
    let mut presets = builtin_band_presets(region);
    let mut status = None;

    // Resolve the preset file path.  Priority:
    //   1. Explicit --bands-config argument
    //   2. ~/.config/rusty-wire/bands.toml (if it exists)
    //   3. ./bands.toml in the current directory (if it exists)
    let resolved_path: Option<String> = if let Some(explicit) = band_preset_config {
        Some(explicit.to_string())
    } else if let Some(user_path) = user_bands_config_path() {
        Some(user_path)
    } else if Path::new(DEFAULT_BAND_PRESET_CONFIG).exists() {
        Some(DEFAULT_BAND_PRESET_CONFIG.to_string())
    } else {
        None
    };

    if let Some(ref preset_path) = resolved_path {
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

    fn press_with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn key_event_with_kind(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    #[test]
    fn hint_text_for_config_focus_matches_documented_keybindings() {
        let text = hint_text(Focus::Config, false, false, false, false);
        assert!(text.contains("e:csv"));
        assert!(text.contains("E:json"));
        assert!(text.contains("m:md"));
        assert!(text.contains("t:txt"));
        assert!(text.contains("i:info"));
        assert!(text.contains("Tab:→results"));
    }

    #[test]
    fn hint_text_for_results_focus_mentions_scroll_and_tab_back() {
        let text = hint_text(Focus::Results, false, false, false, false);

        assert!(text.contains("a:advise"));
        assert!(text.contains("↑↓/jk:scroll"));
        assert!(text.contains("PgUp/Dn:page"));
        assert!(text.contains("Tab:→config"));
    }

    #[test]
    fn hint_text_for_band_checklist_matches_overlay_controls() {
        let text = hint_text(Focus::Config, true, false, false, false);

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
        assert!(lines.iter().any(|line| line.starts_with("Platform:")));
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
    fn enter_on_custom_bands_opens_checklist_instead_of_running() {
        let mut state = TuiState::new(None);
        state.focus = Focus::Config;
        state.field_idx = ConfigField::ALL
            .iter()
            .position(|field| *field == ConfigField::Bands)
            .expect("Bands field should exist");
        state.band_preset_idx = state.band_presets.len() - 1;
        state.results_scroll = 7;

        state.handle_key(press(KeyCode::Enter));

        assert!(state.show_band_checklist);
        assert_eq!(state.results_scroll, 7);
    }

    #[test]
    fn enter_on_non_custom_field_runs_calculation_and_resets_scroll() {
        let mut state = TuiState::new(None);
        state.focus = Focus::Config;
        state.field_idx = ConfigField::ALL
            .iter()
            .position(|field| *field == ConfigField::Mode)
            .expect("Mode field should exist");
        state.results_scroll = 9;

        state.handle_key(press(KeyCode::Enter));

        assert!(!state.show_band_checklist);
        assert_eq!(state.results_scroll, 0);
        assert!(state.app.results.is_some());
    }

    #[test]
    fn enter_in_results_focus_runs_calculation_even_if_custom_bands_selected() {
        let mut state = TuiState::new(None);
        state.focus = Focus::Results;
        state.field_idx = ConfigField::ALL
            .iter()
            .position(|field| *field == ConfigField::Bands)
            .expect("Bands field should exist");
        state.band_preset_idx = state.band_presets.len() - 1;
        state.results_scroll = 6;

        state.handle_key(press(KeyCode::Enter));

        assert!(!state.show_band_checklist);
        assert_eq!(state.results_scroll, 0);
        assert!(state.app.results.is_some());
    }

    #[test]
    fn r_key_runs_calculation_and_resets_scroll() {
        let mut state = TuiState::new(None);
        state.results_scroll = 11;

        state.handle_key(press(KeyCode::Char('r')));

        assert_eq!(state.results_scroll, 0);
        assert!(state.app.results.is_some());
        assert!(
            state.pending_recalc.is_none(),
            "explicit r should not leave a pending recalc"
        );
    }

    #[test]
    fn config_change_schedules_pending_recalc() {
        let mut state = TuiState::new(None);
        assert!(state.pending_recalc.is_none());

        state.handle_key(press(KeyCode::Right)); // change a config field

        assert!(
            state.pending_recalc.is_some(),
            "config change should schedule auto-recalc"
        );
    }

    #[test]
    fn auto_recalculate_does_not_reset_scroll() {
        let mut state = TuiState::new(None);
        state.run_calculation();
        state.results_scroll = 5;

        state.auto_recalculate();

        assert_eq!(
            state.results_scroll, 5,
            "auto-recalc should preserve scroll position"
        );
        assert!(
            state.pending_recalc.is_none(),
            "auto_recalculate should clear the pending flag"
        );
        assert!(state.app.results.is_some());
    }

    #[test]
    fn a_key_toggles_advise_panel_and_populates_candidates() {
        let mut state = TuiState::new(None);

        state.handle_key(press(KeyCode::Char('a')));
        assert!(state.show_advise_panel);
        assert!(state
            .advise_view
            .as_ref()
            .is_some_and(|view| !view.candidates.is_empty()));

        state.handle_key(press(KeyCode::Char('a')));
        assert!(!state.show_advise_panel);
    }

    #[test]
    fn changing_configuration_hides_stale_advise_panel() {
        let mut state = TuiState::new(None);
        state.handle_key(press(KeyCode::Char('a')));
        assert!(state.show_advise_panel);

        state.handle_key(press(KeyCode::Right));

        assert!(!state.show_advise_panel);
        assert!(state.advise_view.is_none());
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
    fn checklist_j_and_k_move_cursor_with_bounds() {
        let mut state = TuiState::new(None);
        state.show_band_checklist = true;
        state.band_checklist_items = vec![
            (1, "160m".to_string(), false),
            (2, "80m".to_string(), false),
            (3, "60m".to_string(), false),
        ];

        state.handle_key(press(KeyCode::Char('j')));
        assert_eq!(state.band_checklist_cursor, 1);

        state.handle_key(press(KeyCode::Char('j')));
        assert_eq!(state.band_checklist_cursor, 2);

        state.handle_key(press(KeyCode::Char('j')));
        assert_eq!(state.band_checklist_cursor, 2);

        state.handle_key(press(KeyCode::Char('k')));
        assert_eq!(state.band_checklist_cursor, 1);

        state.handle_key(press(KeyCode::Char('k')));
        assert_eq!(state.band_checklist_cursor, 0);

        state.handle_key(press(KeyCode::Char('k')));
        assert_eq!(state.band_checklist_cursor, 0);
    }

    #[test]
    fn checklist_space_toggles_current_item_checked_state() {
        let mut state = TuiState::new(None);
        state.show_band_checklist = true;
        state.band_checklist_items =
            vec![(1, "160m".to_string(), false), (2, "80m".to_string(), true)];
        state.band_checklist_cursor = 0;

        state.handle_key(press(KeyCode::Char(' ')));
        assert!(state.band_checklist_items[0].2);

        state.handle_key(press(KeyCode::Char(' ')));
        assert!(!state.band_checklist_items[0].2);
    }

    #[test]
    fn checklist_q_closes_overlay_without_quitting() {
        let mut state = TuiState::new(None);
        state.show_band_checklist = true;
        state.band_checklist_items = vec![(1, "160m".to_string(), false)];

        state.handle_key(press(KeyCode::Char('q')));

        assert!(!state.show_band_checklist);
        assert!(!state.quit);
    }

    #[test]
    fn checklist_ctrl_c_sets_quit_flag() {
        let mut state = TuiState::new(None);
        state.show_band_checklist = true;
        state.band_checklist_items = vec![(1, "160m".to_string(), false)];

        state.handle_key(press_with_modifiers(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        ));

        assert!(state.quit);
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
                let expected = parse_band_selection(
                    BUILTIN_BAND_PRESET_TEMPLATES[1].1,
                    state.app.config.itu_region,
                )
                .expect("built-in preset should parse");
                assert_eq!(indices, expected);
                assert_eq!(state.band_preset_idx, 1);
            }
            other => panic!("expected SetBandIndices, got {other:?}"),
        }
    }

    #[test]
    fn region_change_refreshes_band_preset_labels_and_active_selection() {
        let mut state = TuiState::new(None);
        state.band_preset_idx = 1;
        state.app.config.band_indices =
            parse_band_selection(BUILTIN_BAND_PRESET_TEMPLATES[1].1, ITURegion::Region1)
                .expect("region 1 preset should parse");

        state.dispatch(AppAction::SetItuRegion(ITURegion::Region2));

        assert_eq!(state.app.config.itu_region, ITURegion::Region2);
        assert!(state
            .current_band_preset()
            .label
            .contains("R2 80m 3.5–4.0 / 40m 7.0–7.3"));
        assert_eq!(
            state.app.config.band_indices,
            parse_band_selection(BUILTIN_BAND_PRESET_TEMPLATES[1].1, ITURegion::Region2)
                .expect("region 2 preset should parse")
        );
    }

    #[test]
    fn region_change_refreshes_open_band_checklist_items() {
        let mut state = TuiState::new(None);
        state.custom_band_indices = vec![2, 4];
        state.open_band_checklist();

        state.dispatch(AppAction::SetItuRegion(ITURegion::Region2));

        assert!(state.show_band_checklist);
        assert!(state
            .band_checklist_items
            .iter()
            .any(|(_, display, checked)| *checked && display.contains("80m [HF] (3.5-4 MHz)")));
        assert!(state
            .band_checklist_items
            .iter()
            .any(|(_, display, checked)| *checked && display.contains("40m [HF] (7-7.3 MHz)")));
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
        let (presets, status) = load_tui_band_presets(None, ITURegion::Region1);

        assert!(status.is_none());
        assert!(presets.last().is_some_and(BandPresetChoice::is_custom));
    }

    #[test]
    fn non_press_key_events_are_ignored() {
        let mut state = TuiState::new(None);
        state.export_status = Some("Previous status".to_string());

        // Release event should not clear export status or change state
        state.handle_key(key_event_with_kind(
            KeyCode::Char('q'),
            KeyEventKind::Release,
        ));

        assert_eq!(
            state.export_status.as_deref(),
            Some("Previous status"),
            "Release events should not clear export_status"
        );
        assert!(!state.quit, "Release events should not trigger quit");
    }

    #[test]
    fn esc_without_info_popup_sets_quit_flag() {
        let mut state = TuiState::new(None);

        // Esc with no popup should quit
        state.handle_key(press(KeyCode::Esc));

        assert!(state.quit);
        assert!(!state.show_info_popup);
    }

    #[test]
    fn toggle_band_at_cursor_collapses_and_expands() {
        use crate::app::{run_calculation, AppConfig};
        let mut state = TuiState::new(None);
        let results = run_calculation(AppConfig {
            band_indices: vec![4, 6], // 40m + 20m
            ..Default::default()
        });
        state.app.results = Some(results);
        state.focus = Focus::Results;
        state.results_band_cursor = 0;

        // First toggle collapses band 0.
        state.toggle_band_at_cursor();
        let doc = results_display_document(state.app.results.as_ref().unwrap());
        let title = &doc.band_views[0].title;
        assert!(
            state.collapsed_bands.contains(title),
            "band 0 should be collapsed"
        );

        // Second toggle expands it again.
        state.toggle_band_at_cursor();
        assert!(
            !state.collapsed_bands.contains(title),
            "band 0 should be expanded again"
        );
    }

    #[test]
    fn move_band_cursor_clamps_at_boundaries() {
        use crate::app::{run_calculation, AppConfig};
        let mut state = TuiState::new(None);
        let results = run_calculation(AppConfig {
            band_indices: vec![4, 6],
            ..Default::default()
        });
        state.app.results = Some(results);
        state.focus = Focus::Results;
        state.results_band_cursor = 0;

        // Move backward from 0 stays at 0.
        state.move_band_cursor(false);
        assert_eq!(state.results_band_cursor, 0);

        // Move forward twice reaches the last band (idx 1 for 2 bands).
        state.move_band_cursor(true);
        assert_eq!(state.results_band_cursor, 1);
        state.move_band_cursor(true);
        assert_eq!(state.results_band_cursor, 1, "should clamp at last band");
    }

    #[test]
    fn run_calculation_resets_collapsed_bands_and_cursor() {
        let mut state = TuiState::new(None);
        state.collapsed_bands.insert("40m".to_string());
        state.results_band_cursor = 3;

        state.run_calculation();

        assert!(state.collapsed_bands.is_empty());
        assert_eq!(state.results_band_cursor, 0);
    }

    #[test]
    fn results_focus_space_key_toggles_band() {
        use crate::app::{run_calculation, AppConfig};
        let mut state = TuiState::new(None);
        let results = run_calculation(AppConfig {
            band_indices: vec![4, 6],
            ..Default::default()
        });
        state.app.results = Some(results);
        state.focus = Focus::Results;
        state.results_band_cursor = 0;

        state.handle_key(press(KeyCode::Char(' ')));

        let doc = results_display_document(state.app.results.as_ref().unwrap());
        assert!(state.collapsed_bands.contains(&doc.band_views[0].title));
    }

    #[test]
    fn results_focus_bracket_keys_move_band_cursor() {
        use crate::app::{run_calculation, AppConfig};
        let mut state = TuiState::new(None);
        let results = run_calculation(AppConfig {
            band_indices: vec![4, 6],
            ..Default::default()
        });
        state.app.results = Some(results);
        state.focus = Focus::Results;
        state.results_band_cursor = 0;

        state.handle_key(press(KeyCode::Char(']')));
        assert_eq!(state.results_band_cursor, 1);

        state.handle_key(press(KeyCode::Char('[')));
        assert_eq!(state.results_band_cursor, 0);
    }

    // ── Session-save overlay ────────────────────────────────────────────────

    #[test]
    fn shift_s_opens_session_save_overlay() {
        let mut state = TuiState::new(None);
        state.handle_key(press_with_modifiers(
            KeyCode::Char('S'),
            KeyModifiers::SHIFT,
        ));
        assert!(state.show_session_save);
        assert!(state.session_name_input.is_empty());
    }

    #[test]
    fn session_save_typing_fills_input_buffer() {
        let mut state = TuiState::new(None);
        state.show_session_save = true;
        state.handle_key(press(KeyCode::Char('m')));
        state.handle_key(press(KeyCode::Char('y')));
        assert_eq!(state.session_name_input, "my");
    }

    #[test]
    fn session_save_backspace_removes_last_char() {
        let mut state = TuiState::new(None);
        state.show_session_save = true;
        state.session_name_input = "abc".to_string();
        state.handle_key(press(KeyCode::Backspace));
        assert_eq!(state.session_name_input, "ab");
    }

    #[test]
    fn session_save_esc_closes_overlay_and_clears_input() {
        let mut state = TuiState::new(None);
        state.show_session_save = true;
        state.session_name_input = "draft".to_string();
        state.handle_key(press(KeyCode::Esc));
        assert!(!state.show_session_save);
        assert!(state.session_name_input.is_empty());
    }

    #[test]
    fn session_save_enter_with_empty_name_closes_without_saving() {
        let mut state = TuiState::new(None);
        state.show_session_save = true;
        state.session_name_input = String::new();
        state.handle_key(press(KeyCode::Enter));
        assert!(!state.show_session_save);
        assert!(state.export_status.is_none());
    }

    #[test]
    fn session_save_enter_with_name_writes_and_shows_confirmation() {
        crate::test_env::with_temp_home(|| {
            let mut state = TuiState::new(None);
            state.show_session_save = true;
            state.session_name_input = "overlay-test".to_string();
            state.handle_key(press(KeyCode::Enter));
            assert!(!state.show_session_save);
            assert!(state.session_name_input.is_empty());
            assert_eq!(
                state.export_status.as_deref(),
                Some("Session \"overlay-test\" saved.")
            );
            let names = crate::sessions::SessionStore::list();
            assert!(names.contains(&"overlay-test".to_string()));
        });
    }

    // ── Session-picker overlay ──────────────────────────────────────────────

    #[test]
    fn session_picker_esc_closes_overlay() {
        let mut state = TuiState::new(None);
        state.show_session_picker = true;
        state.session_picker_items = vec!["alpha".to_string(), "beta".to_string()];
        state.handle_key(press(KeyCode::Esc));
        assert!(!state.show_session_picker);
    }

    #[test]
    fn session_picker_q_closes_overlay() {
        let mut state = TuiState::new(None);
        state.show_session_picker = true;
        state.session_picker_items = vec!["alpha".to_string()];
        state.handle_key(press(KeyCode::Char('q')));
        assert!(!state.show_session_picker);
    }

    #[test]
    fn session_picker_down_moves_cursor() {
        let mut state = TuiState::new(None);
        state.show_session_picker = true;
        state.session_picker_items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        state.session_picker_cursor = 0;
        state.handle_key(press(KeyCode::Down));
        assert_eq!(state.session_picker_cursor, 1);
        state.handle_key(press(KeyCode::Down));
        assert_eq!(state.session_picker_cursor, 2);
        state.handle_key(press(KeyCode::Down));
        assert_eq!(
            state.session_picker_cursor, 2,
            "should not exceed last item"
        );
    }

    #[test]
    fn session_picker_up_moves_cursor() {
        let mut state = TuiState::new(None);
        state.show_session_picker = true;
        state.session_picker_items = vec!["a".to_string(), "b".to_string()];
        state.session_picker_cursor = 1;
        state.handle_key(press(KeyCode::Up));
        assert_eq!(state.session_picker_cursor, 0);
        state.handle_key(press(KeyCode::Up));
        assert_eq!(state.session_picker_cursor, 0, "should not go below 0");
    }

    // ── Export-preview overlay ──────────────────────────────────────────────

    #[test]
    fn export_preview_esc_dismisses_overlay() {
        let mut state = TuiState::new(None);
        state.export_preview = Some((ExportFormat::Csv, false, "content".to_string()));
        state.handle_key(press(KeyCode::Esc));
        assert!(state.export_preview.is_none());
    }

    #[test]
    fn export_preview_q_dismisses_overlay() {
        let mut state = TuiState::new(None);
        state.export_preview = Some((ExportFormat::Json, false, "{}".to_string()));
        state.handle_key(press(KeyCode::Char('q')));
        assert!(state.export_preview.is_none());
    }

    #[test]
    fn export_preview_j_increments_scroll() {
        let mut state = TuiState::new(None);
        state.export_preview = Some((ExportFormat::Txt, false, "data".to_string()));
        state.preview_scroll = 0;
        state.handle_key(press(KeyCode::Down));
        assert_eq!(state.preview_scroll, 1);
        state.handle_key(press(KeyCode::Char('j')));
        assert_eq!(state.preview_scroll, 2);
    }

    #[test]
    fn export_preview_k_decrements_scroll_with_floor() {
        let mut state = TuiState::new(None);
        state.export_preview = Some((ExportFormat::Txt, false, "data".to_string()));
        state.preview_scroll = 3;
        state.handle_key(press(KeyCode::Up));
        assert_eq!(state.preview_scroll, 2);
        state.handle_key(press(KeyCode::Char('k')));
        assert_eq!(state.preview_scroll, 1);
        state.preview_scroll = 0;
        state.handle_key(press(KeyCode::Char('k')));
        assert_eq!(state.preview_scroll, 0, "should not underflow");
    }

    #[test]
    fn export_preview_intercepts_all_keys_before_other_handlers() {
        let mut state = TuiState::new(None);
        state.export_preview = Some((ExportFormat::Csv, false, "data".to_string()));
        // 'i' would normally open the info popup — with preview open it should not.
        state.handle_key(press(KeyCode::Char('i')));
        assert!(
            state.export_preview.is_some(),
            "preview should still be open"
        );
        assert!(
            !state.show_info_popup,
            "info popup must NOT open while preview is active"
        );
    }
}
