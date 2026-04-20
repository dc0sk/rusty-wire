#[derive(Debug, Clone, PartialEq)]
pub enum AppError {
    InvalidVelocityFactor(f64),
    InvalidWireLengthWindow { min_m: f64, max_m: f64 },
    MixedWireWindowUnits,
    InvalidCalcMode(String),
    InvalidExportFormat(String),
    InvalidUnitSystem(String),
    InvalidAntennaModel(String),
    InvalidBandSelection(String),
    InvalidSearchStep(f64),
    InvalidFrequency(f64),
    EmptyBandSelection,
    AllBandsSkipped,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::InvalidVelocityFactor(value) => {
                write!(
                    f,
                    "velocity factor must be between 0.50 and 1.00 (got {value:.3})"
                )
            }
            AppError::InvalidWireLengthWindow { min_m, max_m } => {
                write!(
                    f,
                    "invalid wire length window in meters ({min_m:.3}..{max_m:.3})"
                )
            }
            AppError::MixedWireWindowUnits => {
                write!(
                    f,
                    "cannot mix meter and feet constraints; choose one unit system"
                )
            }
            AppError::InvalidCalcMode(s) => write!(f, "Invalid calculation mode: {s}"),
            AppError::InvalidExportFormat(s) => write!(f, "Invalid export format: {s}"),
            AppError::InvalidUnitSystem(s) => write!(f, "Invalid unit system: {s}"),
            AppError::InvalidAntennaModel(s) => write!(f, "Invalid antenna model: {s}"),
            AppError::InvalidBandSelection(s) => write!(f, "Invalid band selection: {s}"),
            AppError::InvalidSearchStep(step) => {
                write!(
                    f,
                    "search step must be greater than 0 and less than the wire length window (got {step:.4} m)"
                )
            }
            AppError::InvalidFrequency(freq) => {
                write!(
                    f,
                    "frequency must be greater than 0 and at most 1000 MHz (got {freq:.3} MHz)"
                )
            }
            AppError::EmptyBandSelection => {
                write!(f, "empty selection; provide at least one band name.")
            }
            AppError::AllBandsSkipped => {
                write!(f, "no valid bands for the selected ITU region")
            }
        }
    }
}

impl std::error::Error for AppError {}
/// Core application types, configuration, and computation entry point.
///
/// This module is the primary API surface for both the CLI front-end and any
/// future GUI (e.g. iced). It is deliberately free of I/O.
use crate::bands::{get_band_by_index_for_region, Band, BandType, ITURegion};
use crate::calculations::{
    calculate_average_max_distance, calculate_average_min_distance,
    calculate_best_non_resonant_length, calculate_for_band_with_velocity,
    calculate_non_resonant_optima, calculate_non_resonant_window_optima,
    calculate_resonant_compromises, optimize_ocfd_split_for_length, NonResonantRecommendation,
    NonResonantSearchConfig, ResonantCompromise, TransformerRatio, WireCalculation,
    DEFAULT_NON_RESONANT_CONFIG,
};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;

pub const FEET_TO_METERS: f64 = 0.3048;
pub const DEFAULT_BAND_SELECTION: [usize; 7] = [4, 5, 6, 7, 8, 9, 10];
pub const DEFAULT_ITU_REGION: ITURegion = ITURegion::Region1;
pub const DEFAULT_TRANSFORMER_RATIO: TransformerRatio = TransformerRatio::R1To1;

pub fn recommended_transformer_ratio(
    mode: CalcMode,
    antenna_model: Option<AntennaModel>,
) -> TransformerRatio {
    match antenna_model {
        Some(AntennaModel::Dipole)
        | Some(AntennaModel::InvertedVDipole)
        | Some(AntennaModel::FullWaveLoop) => TransformerRatio::R1To1,
        Some(AntennaModel::EndFedHalfWave) => TransformerRatio::R1To56,
        Some(AntennaModel::OffCenterFedDipole) => TransformerRatio::R1To4,
        None => match mode {
            CalcMode::Resonant => TransformerRatio::R1To1,
            CalcMode::NonResonant => TransformerRatio::R1To9,
        },
    }
}

pub fn recommended_transformer_ratio_fallback_message(
    mode: CalcMode,
    antenna_model: Option<AntennaModel>,
) -> String {
    let recommended = recommended_transformer_ratio(mode, antenna_model);
    format!(
        "Unknown ratio. Using recommended {}.",
        recommended.as_label()
    )
}

// ---------------------------------------------------------------------------
// Shared enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcMode {
    Resonant,
    NonResonant,
}

impl FromStr for CalcMode {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "resonant" => Ok(CalcMode::Resonant),
            "non-resonant" | "nonresonant" | "non_resonant" => Ok(CalcMode::NonResonant),
            _ => Err(AppError::InvalidCalcMode(format!(
                "'{s}' (must be 'resonant' or 'non-resonant')"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Json,
    Markdown,
    Txt,
}

impl ExportFormat {
    #[allow(dead_code)] // used in tests
    pub fn as_str(self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            ExportFormat::Markdown => "markdown",
            ExportFormat::Txt => "txt",
        }
    }
}

impl FromStr for ExportFormat {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "csv" => Ok(ExportFormat::Csv),
            "json" => Ok(ExportFormat::Json),
            "markdown" | "md" => Ok(ExportFormat::Markdown),
            "txt" | "text" => Ok(ExportFormat::Txt),
            _ => Err(AppError::InvalidExportFormat(format!(
                "'{s}' (must be 'csv', 'json', 'markdown', or 'txt')"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitSystem {
    Metric,
    Imperial,
    Both,
}

impl FromStr for UnitSystem {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "m" | "metric" => Ok(UnitSystem::Metric),
            "ft" | "imperial" => Ok(UnitSystem::Imperial),
            "both" => Ok(UnitSystem::Both),
            _ => Err(AppError::InvalidUnitSystem(format!(
                "'{s}' (must be 'm', 'ft', or 'both')"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntennaModel {
    Dipole,
    InvertedVDipole,
    EndFedHalfWave,
    FullWaveLoop,
    OffCenterFedDipole,
}

impl FromStr for AntennaModel {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "dipole" => Ok(AntennaModel::Dipole),
            "inverted-v" | "inv-v" | "invertedv" | "invv" => Ok(AntennaModel::InvertedVDipole),
            "efhw" | "end-fed" | "end-fed-half-wave" => Ok(AntennaModel::EndFedHalfWave),
            "loop" | "full-wave-loop" => Ok(AntennaModel::FullWaveLoop),
            "ocfd" | "off-center-fed" | "off-center-fed-dipole" => {
                Ok(AntennaModel::OffCenterFedDipole)
            }
            _ => Err(AppError::InvalidAntennaModel(format!(
                "'{s}' (must be 'dipole', 'inverted-v', 'efhw', 'loop', or 'ocfd')"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// AppConfig – all inputs needed for a single calculation run
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub band_indices: Vec<usize>,
    pub velocity_factor: f64,
    pub mode: CalcMode,
    pub wire_min_m: f64,
    pub wire_max_m: f64,
    pub step_m: f64,
    pub units: UnitSystem,
    pub itu_region: ITURegion,
    pub transformer_ratio: TransformerRatio,
    pub antenna_model: Option<AntennaModel>,
    /// Direct frequency input in MHz; when set, bypasses band selection entirely.
    pub custom_freq_mhz: Option<f64>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            band_indices: DEFAULT_BAND_SELECTION.to_vec(),
            velocity_factor: 0.95,
            mode: CalcMode::Resonant,
            wire_min_m: DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            wire_max_m: DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            step_m: DEFAULT_NON_RESONANT_CONFIG.step_m,
            units: UnitSystem::Both,
            itu_region: DEFAULT_ITU_REGION,
            transformer_ratio: DEFAULT_TRANSFORMER_RATIO,
            antenna_model: None,
            custom_freq_mhz: None,
        }
    }
}

// ---------------------------------------------------------------------------
// AppResults – all outputs produced by run_calculation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AppResults {
    pub calculations: Vec<WireCalculation>,
    pub recommendation: Option<NonResonantRecommendation>,
    /// All equally-optimal wire lengths in ascending order.
    pub optima: Vec<NonResonantRecommendation>,
    /// In non-resonant mode: all local optima (clearance maxima) within the
    /// active search window, in ascending order.
    pub window_optima: Vec<NonResonantRecommendation>,
    /// In resonant mode: all compromise lengths that minimize worst-band
    /// distance to in-window resonant points.
    pub resonant_compromises: Vec<ResonantCompromise>,
    /// The configuration that produced these results.
    pub config: AppConfig,
    /// Band indices that were invalid for the selected ITU region and skipped.
    pub skipped_band_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct AppRequest {
    pub config: AppConfig,
}

impl AppRequest {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }
}

impl From<AppConfig> for AppRequest {
    fn from(config: AppConfig) -> Self {
        Self::new(config)
    }
}

#[derive(Debug, Clone)]
pub struct AppResponse {
    pub results: AppResults,
}

impl AppResponse {
    pub fn new(results: AppResults) -> Self {
        Self { results }
    }
}

#[derive(Debug, Clone)]
pub struct RunSummary {
    pub overview_heading: &'static str,
    pub transformer_ratio_label: &'static str,
    pub antenna_model_label: &'static str,
    pub band_count: usize,
    pub average_min_skip_km: f64,
    pub average_max_skip_km: f64,
}

#[derive(Debug, Clone)]
pub struct ResultsOverviewView {
    pub heading: &'static str,
    pub header_lines: Vec<String>,
    pub summary_lines: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultsSectionLayout {
    pub show_resonant_points: bool,
    pub show_resonant_compromises: bool,
    pub show_non_resonant_recommendation: bool,
}

#[derive(Debug, Clone)]
pub struct ResultsTextSectionView {
    pub lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResultsDisplayDocument {
    pub overview_heading: &'static str,
    pub overview_header_lines: Vec<String>,
    pub band_views: Vec<BandDisplayView>,
    pub summary_lines: Vec<String>,
    pub sections: Vec<ResultsTextSectionView>,
    pub warning_lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResonantCompromiseNarrative {
    pub heading: &'static str,
    pub notes: Vec<&'static str>,
    pub empty_message: &'static str,
}

#[derive(Debug, Clone)]
pub struct ResonantPoint {
    pub length_m: f64,
    pub band_name: String,
    pub harmonic: u32,
}

#[derive(Debug, Clone)]
pub struct ResonantPointsView {
    pub heading: &'static str,
    pub window_line: String,
    pub point_lines: Vec<String>,
    pub empty_message: &'static str,
}

#[derive(Debug, Clone)]
pub struct NonResonantRecommendationRow {
    pub length_m: f64,
    pub length_ft: f64,
    pub min_resonance_clearance_pct: f64,
    pub is_recommended: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NonResonantRecommendationView {
    pub heading: &'static str,
    pub unavailable_message: &'static str,
    pub search_window_min_m: f64,
    pub search_window_max_m: f64,
    pub search_window_min_ft: f64,
    pub search_window_max_ft: f64,
    pub recommended: Option<NonResonantRecommendationRow>,
    pub equal_optima: Vec<NonResonantRecommendationRow>,
    pub local_optima: Vec<NonResonantRecommendationRow>,
    pub window_line: String,
    pub recommended_line: Option<String>,
    pub equal_optima_heading: Option<&'static str>,
    pub equal_optima_lines: Vec<String>,
    pub local_optima_heading: Option<&'static str>,
    pub local_optima_lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct InvertedVCompromiseDetails {
    pub leg_m: f64,
    pub leg_ft: f64,
    pub span_90_m: f64,
    pub span_90_ft: f64,
    pub span_120_m: f64,
    pub span_120_ft: f64,
}

#[derive(Debug, Clone)]
pub struct OcfdLegSplit {
    pub short_m: f64,
    pub short_ft: f64,
    pub long_m: f64,
    pub long_ft: f64,
}

#[derive(Debug, Clone)]
pub struct OptimizedOcfdSplitView {
    pub short_ratio_pct: f64,
    pub long_ratio_pct: f64,
    pub short_leg_m: f64,
    pub short_leg_ft: f64,
    pub long_leg_m: f64,
    pub long_leg_ft: f64,
    pub worst_leg_clearance_pct: f64,
}

#[derive(Debug, Clone)]
pub struct OcfdCompromiseDetails {
    pub split_33_67: OcfdLegSplit,
    pub split_20_80: OcfdLegSplit,
    pub optimized: Option<OptimizedOcfdSplitView>,
}

#[derive(Debug, Clone)]
pub struct ResonantCompromiseRow {
    pub length_m: f64,
    pub length_ft: f64,
    pub worst_band_distance_m: f64,
    pub worst_band_distance_ft: f64,
    pub inverted_v: Option<InvertedVCompromiseDetails>,
    pub ocfd: Option<OcfdCompromiseDetails>,
}

#[derive(Debug, Clone)]
pub struct ResonantCompromiseView {
    pub heading: &'static str,
    pub notes: Vec<&'static str>,
    pub empty_message: &'static str,
    pub rows: Vec<ResonantCompromiseRow>,
}

#[derive(Debug, Clone)]
pub struct ResonantCompromiseDisplayView {
    pub heading: &'static str,
    pub notes: Vec<&'static str>,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BandDisplayRow {
    pub calc: WireCalculation,
}

#[derive(Debug, Clone)]
pub struct BandDisplayView {
    pub title: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireWindowInputUnit {
    Metric,
    Imperial,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedWireWindow {
    pub min_m: f64,
    pub max_m: f64,
    pub input_unit: WireWindowInputUnit,
    pub inferred_display_units: UnitSystem,
}

// ---------------------------------------------------------------------------
// Public computation API
// ---------------------------------------------------------------------------

/// Run all wire calculations for the given configuration.
///
/// This is a pure, I/O-free function suitable for use from both the CLI and
/// any future GUI front-end.
pub fn run_calculation(config: AppConfig) -> AppResults {
    let (calculations, skipped_band_indices) = if let Some(freq_mhz) = config.custom_freq_mhz {
        let custom_band = Band {
            name: "custom",
            band_type: BandType::HF,
            freq_low_mhz: freq_mhz,
            freq_high_mhz: freq_mhz,
            freq_center_mhz: freq_mhz,
            typical_skip_km: (0.0, 0.0),
            regions: &[],
        };
        let mut calc = calculate_for_band_with_velocity(
            &custom_band,
            config.velocity_factor,
            config.transformer_ratio,
        );
        calc.band_name = format!("{freq_mhz:.3} MHz");
        (vec![calc], Vec::new())
    } else {
        build_calculations(
            &config.band_indices,
            config.velocity_factor,
            config.itu_region,
            config.transformer_ratio,
        )
    };

    // For resonant mode use the default search window; for non-resonant use the
    // user-supplied window.  Optima (tied candidates) are only relevant in
    // non-resonant mode.
    let non_res_cfg = NonResonantSearchConfig {
        min_len_m: config.wire_min_m,
        max_len_m: config.wire_max_m,
        step_m: config.step_m,
        preferred_center_m: (config.wire_min_m + config.wire_max_m) / 2.0,
    };
    let recommendation =
        calculate_best_non_resonant_length(&calculations, config.velocity_factor, non_res_cfg);
    let optima = if config.mode == CalcMode::NonResonant {
        calculate_non_resonant_optima(&calculations, config.velocity_factor, non_res_cfg)
    } else {
        Vec::new()
    };
    let window_optima = if config.mode == CalcMode::NonResonant {
        calculate_non_resonant_window_optima(&calculations, config.velocity_factor, non_res_cfg)
    } else {
        Vec::new()
    };
    let resonant_compromises = if config.mode == CalcMode::Resonant {
        calculate_resonant_compromises(&calculations, non_res_cfg)
    } else {
        Vec::new()
    };

    AppResults {
        calculations,
        recommendation,
        optima,
        window_optima,
        resonant_compromises,
        config,
        skipped_band_indices,
    }
}

pub fn validate_config(config: &AppConfig) -> Result<(), AppError> {
    if let Some(freq) = config.custom_freq_mhz {
        if freq <= 0.0 || freq > 1000.0 {
            return Err(AppError::InvalidFrequency(freq));
        }
    } else if config.band_indices.is_empty() {
        return Err(AppError::EmptyBandSelection);
    }

    if !(0.5..=1.0).contains(&config.velocity_factor) {
        return Err(AppError::InvalidVelocityFactor(config.velocity_factor));
    }

    if config.wire_min_m <= 0.0 || config.wire_max_m <= config.wire_min_m {
        return Err(AppError::InvalidWireLengthWindow {
            min_m: config.wire_min_m,
            max_m: config.wire_max_m,
        });
    }

    let window = config.wire_max_m - config.wire_min_m;
    if config.step_m <= 0.0 || config.step_m >= window {
        return Err(AppError::InvalidSearchStep(config.step_m));
    }

    Ok(())
}

pub fn resolve_wire_window_inputs(
    wire_min_m: Option<f64>,
    wire_max_m: Option<f64>,
    wire_min_ft: Option<f64>,
    wire_max_ft: Option<f64>,
) -> Result<ResolvedWireWindow, AppError> {
    let using_ft = wire_min_ft.is_some() || wire_max_ft.is_some();
    let using_m = wire_min_m.is_some() || wire_max_m.is_some();

    if using_ft && using_m {
        return Err(AppError::MixedWireWindowUnits);
    }

    let resolved = if using_ft {
        let min_ft = wire_min_ft.unwrap_or(DEFAULT_NON_RESONANT_CONFIG.min_len_m / FEET_TO_METERS);
        let max_ft = wire_max_ft.unwrap_or(DEFAULT_NON_RESONANT_CONFIG.max_len_m / FEET_TO_METERS);

        ResolvedWireWindow {
            min_m: min_ft * FEET_TO_METERS,
            max_m: max_ft * FEET_TO_METERS,
            input_unit: WireWindowInputUnit::Imperial,
            inferred_display_units: UnitSystem::Imperial,
        }
    } else {
        ResolvedWireWindow {
            min_m: wire_min_m.unwrap_or(DEFAULT_NON_RESONANT_CONFIG.min_len_m),
            max_m: wire_max_m.unwrap_or(DEFAULT_NON_RESONANT_CONFIG.max_len_m),
            input_unit: WireWindowInputUnit::Metric,
            inferred_display_units: UnitSystem::Metric,
        }
    };

    if resolved.min_m <= 0.0 || resolved.max_m <= resolved.min_m {
        return Err(AppError::InvalidWireLengthWindow {
            min_m: resolved.min_m,
            max_m: resolved.max_m,
        });
    }

    Ok(resolved)
}

pub fn parse_band_selection(selection: &str, region: ITURegion) -> Result<Vec<usize>, AppError> {
    let mut parsed = Vec::new();
    let mut seen = HashSet::new();

    for token in selection.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }

        if let Some((start, end)) = token.split_once('-') {
            let start_idx = parse_single_band_token(start.trim(), region)?;
            let end_idx = parse_single_band_token(end.trim(), region)?;

            let ordered = ordered_band_indices_for_region(region);
            let start_pos = ordered
                .iter()
                .position(|idx| *idx == start_idx)
                .ok_or_else(|| {
                    AppError::InvalidBandSelection(format!(
                        "unknown range start '{}'.",
                        start.trim()
                    ))
                })?;
            let end_pos = ordered
                .iter()
                .position(|idx| *idx == end_idx)
                .ok_or_else(|| {
                    AppError::InvalidBandSelection(format!("unknown range end '{}'.", end.trim()))
                })?;

            if start_pos <= end_pos {
                for idx in &ordered[start_pos..=end_pos] {
                    if seen.insert(*idx) {
                        parsed.push(*idx);
                    }
                }
            } else {
                for idx in ordered[end_pos..=start_pos].iter().rev() {
                    if seen.insert(*idx) {
                        parsed.push(*idx);
                    }
                }
            }

            continue;
        }

        let idx = parse_single_band_token(token, region)?;
        if seen.insert(idx) {
            parsed.push(idx);
        }
    }

    if parsed.is_empty() {
        return Err(AppError::EmptyBandSelection);
    }

    Ok(parsed)
}

pub fn parse_single_band_token(token: &str, region: ITURegion) -> Result<usize, AppError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(AppError::InvalidBandSelection(
            "empty band token".to_string(),
        ));
    }

    let aliases = band_alias_to_index(region);
    let key = token.to_ascii_lowercase();
    aliases
        .get(&key)
        .copied()
        .ok_or_else(|| AppError::InvalidBandSelection(format!("unknown band '{token}'.")))
}

pub fn band_label_for_index(index: usize, region: ITURegion) -> String {
    let zero_based = match index.checked_sub(1) {
        Some(v) => v,
        None => return index.to_string(),
    };

    for (idx, band) in crate::bands::get_bands_for_region(region) {
        if idx == zero_based {
            return band
                .name
                .split_whitespace()
                .next()
                .unwrap_or(band.name)
                .to_string();
        }
    }

    index.to_string()
}

fn ordered_band_indices_for_region(region: ITURegion) -> Vec<usize> {
    crate::bands::get_bands_for_region(region)
        .into_iter()
        .map(|(idx, _)| idx + 1)
        .collect()
}

fn band_alias_to_index(region: ITURegion) -> HashMap<String, usize> {
    let mut aliases = HashMap::new();

    for (idx, band) in crate::bands::get_bands_for_region(region) {
        let one_based = idx + 1;
        let full_name = band.name.to_ascii_lowercase();
        aliases.insert(full_name.clone(), one_based);

        if let Some(short_name) = full_name.split_whitespace().next() {
            aliases.insert(short_name.to_string(), one_based);
        }
    }

    aliases
}

/// Validate and execute a calculation run.
///
/// This is the preferred API for front-ends that need structured error
/// handling before rendering output.
pub fn run_calculation_checked(config: AppConfig) -> Result<AppResults, AppError> {
    validate_config(&config)?;
    let results = run_calculation(config);
    if results.calculations.is_empty() {
        return Err(AppError::AllBandsSkipped);
    }
    Ok(results)
}

/// Validate and execute a full app-layer request.
pub fn execute_request_checked(request: AppRequest) -> Result<AppResponse, AppError> {
    Ok(AppResponse::new(run_calculation_checked(request.config)?))
}

pub fn summarize_results(results: &AppResults) -> RunSummary {
    RunSummary {
        overview_heading: if results.config.mode == CalcMode::Resonant {
            "Resonant Overview:"
        } else {
            "Non-resonant Overview (band context):"
        },
        transformer_ratio_label: results.config.transformer_ratio.as_label(),
        antenna_model_label: match results.config.antenna_model {
            None => "all",
            Some(AntennaModel::Dipole) => "dipole",
            Some(AntennaModel::InvertedVDipole) => "inverted-v dipole",
            Some(AntennaModel::EndFedHalfWave) => "end-fed half-wave",
            Some(AntennaModel::FullWaveLoop) => "full-wave loop",
            Some(AntennaModel::OffCenterFedDipole) => "off-center-fed dipole",
        },
        band_count: results.calculations.len(),
        average_min_skip_km: calculate_average_min_distance(&results.calculations),
        average_max_skip_km: calculate_average_max_distance(&results.calculations),
    }
}

pub fn results_overview_view(results: &AppResults) -> ResultsOverviewView {
    let summary = summarize_results(results);

    ResultsOverviewView {
        heading: summary.overview_heading,
        header_lines: vec![
            "------------------------------------------------------------".to_string(),
            format!(
                "Using transformer ratio: {}",
                summary.transformer_ratio_label
            ),
            format!("Antenna model: {}", summary.antenna_model_label),
            "------------------------------------------------------------".to_string(),
        ],
        summary_lines: vec![
            format!("Summary for {} band(s):", summary.band_count),
            format!(
                "  Average minimum skip distance: {:.0} km",
                summary.average_min_skip_km
            ),
            format!(
                "  Average maximum skip distance: {:.0} km",
                summary.average_max_skip_km
            ),
        ],
    }
}

pub fn results_section_layout(results: &AppResults) -> ResultsSectionLayout {
    let show_resonant_points = matches!(
        (results.config.mode, results.config.antenna_model),
        (
            CalcMode::Resonant,
            None | Some(AntennaModel::Dipole) | Some(AntennaModel::InvertedVDipole)
        )
    );

    ResultsSectionLayout {
        show_resonant_points,
        show_resonant_compromises: results.config.mode == CalcMode::Resonant,
        show_non_resonant_recommendation: results.config.mode == CalcMode::NonResonant,
    }
}

pub fn results_display_document(results: &AppResults) -> ResultsDisplayDocument {
    let overview = results_overview_view(results);
    let layout = results_section_layout(results);
    let band_views = band_display_rows(results)
        .iter()
        .map(|row| band_display_view(row, results.config.units, results.config.antenna_model))
        .collect();

    let mut sections = Vec::new();
    if layout.show_resonant_points {
        sections.push(ResultsTextSectionView {
            lines: resonant_points_display_lines(results),
        });
    }
    if layout.show_resonant_compromises {
        let compromise_view = resonant_compromise_display_view(results);
        let mut lines = Vec::new();
        lines.push(compromise_view.heading.to_string());
        lines.extend(compromise_view.notes.iter().map(|note| format!("  {note}")));
        lines.extend(compromise_view.lines);
        sections.push(ResultsTextSectionView { lines });
    }
    if layout.show_non_resonant_recommendation {
        sections.push(ResultsTextSectionView {
            lines: non_resonant_recommendation_display_lines(results),
        });
    }

    ResultsDisplayDocument {
        overview_heading: overview.heading,
        overview_header_lines: overview.header_lines,
        band_views,
        summary_lines: overview.summary_lines,
        sections,
        warning_lines: skipped_band_warning(results).into_iter().collect(),
    }
}

pub fn skipped_band_warning(results: &AppResults) -> Option<String> {
    if results.skipped_band_indices.is_empty() {
        return None;
    }

    let skipped = results
        .skipped_band_indices
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Some(format!(
        "Warning: the following band selections were invalid and skipped: {skipped}"
    ))
}

pub fn non_resonant_recommendation_heading() -> &'static str {
    "Best non-resonant wire length for selected bands:"
}

pub fn non_resonant_recommendation_unavailable_message() -> &'static str {
    "No non-resonant recommendation available for the current selection."
}

pub fn non_resonant_recommendation_view(results: &AppResults) -> NonResonantRecommendationView {
    let rec = results.recommendation.as_ref();
    let units = results.config.units;
    let to_row = |candidate: &NonResonantRecommendation| NonResonantRecommendationRow {
        length_m: candidate.length_m,
        length_ft: candidate.length_ft,
        min_resonance_clearance_pct: candidate.min_resonance_clearance_pct,
        is_recommended: rec
            .map(|r| (candidate.length_m - r.length_m).abs() < 1e-6)
            .unwrap_or(false),
    };

    let equal_optima: Vec<NonResonantRecommendationRow> =
        results.optima.iter().map(to_row).collect();
    let local_optima: Vec<NonResonantRecommendationRow> =
        results.window_optima.iter().map(to_row).collect();

    let window_line = match units {
        UnitSystem::Metric => format!(
            "  Search window: {:.2}-{:.2} m",
            results.config.wire_min_m, results.config.wire_max_m
        ),
        UnitSystem::Imperial => format!(
            "  Search window: {:.2}-{:.2} ft",
            results.config.wire_min_m / FEET_TO_METERS,
            results.config.wire_max_m / FEET_TO_METERS
        ),
        UnitSystem::Both => format!(
            "  Search window: {:.2}-{:.2} m ({:.2}-{:.2} ft)",
            results.config.wire_min_m,
            results.config.wire_max_m,
            results.config.wire_min_m / FEET_TO_METERS,
            results.config.wire_max_m / FEET_TO_METERS
        ),
    };

    let recommended_line = rec.map(|r| match units {
        UnitSystem::Metric => format!(
            "  {:.2} m, resonance clearance: {:.2}%",
            r.length_m, r.min_resonance_clearance_pct
        ),
        UnitSystem::Imperial => format!(
            "  {:.2} ft, resonance clearance: {:.2}%",
            r.length_ft, r.min_resonance_clearance_pct
        ),
        UnitSystem::Both => format!(
            "  {:.2} m ({:.2} ft), resonance clearance: {:.2}%",
            r.length_m, r.length_ft, r.min_resonance_clearance_pct
        ),
    });

    let equal_optima_heading = if equal_optima.len() > 1 {
        Some("  Additional equal optima in range (ascending):")
    } else {
        None
    };
    let equal_optima_lines = if equal_optima.len() > 1 {
        equal_optima
            .iter()
            .enumerate()
            .map(|(idx, o)| match units {
                UnitSystem::Metric => format!(
                    "    {:2}. {:.2} m (clearance: {:.2}%)",
                    idx + 1,
                    o.length_m,
                    o.min_resonance_clearance_pct
                ),
                UnitSystem::Imperial => format!(
                    "    {:2}. {:.2} ft (clearance: {:.2}%)",
                    idx + 1,
                    o.length_ft,
                    o.min_resonance_clearance_pct
                ),
                UnitSystem::Both => format!(
                    "    {:2}. {:.2} m ({:.2} ft, clearance: {:.2}%{})",
                    idx + 1,
                    o.length_m,
                    o.length_ft,
                    o.min_resonance_clearance_pct,
                    if o.is_recommended {
                        ", recommended"
                    } else {
                        ""
                    }
                ),
            })
            .collect()
    } else {
        Vec::new()
    };

    let local_optima_heading = if local_optima.len() > 1 {
        Some("  Local optima in search window (ascending):")
    } else {
        None
    };
    let local_optima_lines = if local_optima.len() > 1 {
        local_optima
            .iter()
            .enumerate()
            .map(|(idx, o)| match units {
                UnitSystem::Metric => format!(
                    "    {:2}. {:.2} m (clearance: {:.2}%{})",
                    idx + 1,
                    o.length_m,
                    o.min_resonance_clearance_pct,
                    if o.is_recommended {
                        ", recommended"
                    } else {
                        ""
                    }
                ),
                UnitSystem::Imperial => format!(
                    "    {:2}. {:.2} ft (clearance: {:.2}%{})",
                    idx + 1,
                    o.length_ft,
                    o.min_resonance_clearance_pct,
                    if o.is_recommended {
                        ", recommended"
                    } else {
                        ""
                    }
                ),
                UnitSystem::Both => format!(
                    "    {:2}. {:.2} m ({:.2} ft, clearance: {:.2}%{})",
                    idx + 1,
                    o.length_m,
                    o.length_ft,
                    o.min_resonance_clearance_pct,
                    if o.is_recommended {
                        ", recommended"
                    } else {
                        ""
                    }
                ),
            })
            .collect()
    } else {
        Vec::new()
    };

    NonResonantRecommendationView {
        heading: non_resonant_recommendation_heading(),
        unavailable_message: non_resonant_recommendation_unavailable_message(),
        search_window_min_m: results.config.wire_min_m,
        search_window_max_m: results.config.wire_max_m,
        search_window_min_ft: results.config.wire_min_m / FEET_TO_METERS,
        search_window_max_ft: results.config.wire_max_m / FEET_TO_METERS,
        recommended: rec.map(to_row),
        equal_optima,
        local_optima,
        window_line,
        recommended_line,
        equal_optima_heading,
        equal_optima_lines,
        local_optima_heading,
        local_optima_lines,
    }
}

pub fn non_resonant_recommendation_display_lines(results: &AppResults) -> Vec<String> {
    let view = non_resonant_recommendation_view(results);
    let rec_line = match view.recommended_line {
        Some(line) => line,
        None => return vec![view.unavailable_message.to_string()],
    };

    let mut lines = vec![view.heading.to_string(), view.window_line, rec_line];

    if let Some(heading) = view.equal_optima_heading {
        lines.push(heading.to_string());
        lines.extend(view.equal_optima_lines);
    }

    if let Some(heading) = view.local_optima_heading {
        lines.push(heading.to_string());
        lines.extend(view.local_optima_lines);
    }

    lines
}

pub fn resonant_compromise_narrative(results: &AppResults) -> ResonantCompromiseNarrative {
    let heading = match results.config.antenna_model {
        Some(AntennaModel::InvertedVDipole) => {
            "Closest combined compromises to resonant points (inverted-V guidance):"
        }
        Some(AntennaModel::EndFedHalfWave) => {
            "Closest combined compromises to resonant points (tuner-assisted EFHW guidance):"
        }
        Some(AntennaModel::FullWaveLoop) => {
            "Closest combined compromises to resonant points (tuner-assisted loop guidance):"
        }
        Some(AntennaModel::OffCenterFedDipole) => {
            "Closest combined compromises to resonant points (tuner-assisted OCFD guidance):"
        }
        _ => "Closest combined compromises to resonant points:",
    };

    let mut notes = Vec::new();
    if matches!(
        results.config.antenna_model,
        Some(AntennaModel::EndFedHalfWave)
            | Some(AntennaModel::FullWaveLoop)
            | Some(AntennaModel::OffCenterFedDipole)
    ) {
        notes.push(
            "Note: These are dipole-derived compromise lengths shown as tuner-assisted starting points.",
        );
    }
    if matches!(
        results.config.antenna_model,
        Some(AntennaModel::InvertedVDipole)
    ) {
        notes.push(
            "Inverted-V mode: each compromise line shows a total wire length; per-leg and span estimates are listed directly below.",
        );
    }
    if matches!(
        results.config.antenna_model,
        Some(AntennaModel::OffCenterFedDipole)
    ) {
        notes.push(
            "OCFD mode: each compromise line shows a total wire length; leg splits are listed directly below.",
        );
    }

    ResonantCompromiseNarrative {
        heading,
        notes,
        empty_message: "(none available in this window)",
    }
}

pub fn resonant_compromise_view(results: &AppResults) -> ResonantCompromiseView {
    let narrative = resonant_compromise_narrative(results);
    let is_inverted_v = matches!(
        results.config.antenna_model,
        Some(AntennaModel::InvertedVDipole)
    );
    let is_ocfd = matches!(
        results.config.antenna_model,
        Some(AntennaModel::OffCenterFedDipole)
    );

    let rows = results
        .resonant_compromises
        .iter()
        .map(|c| {
            let inverted_v = if is_inverted_v {
                let leg_m = c.length_m / 2.0;
                let leg_ft = leg_m / FEET_TO_METERS;
                let span_90_m = leg_m * std::f64::consts::SQRT_2;
                let span_90_ft = span_90_m / FEET_TO_METERS;
                let span_120_m = leg_m * 3.0_f64.sqrt();
                let span_120_ft = span_120_m / FEET_TO_METERS;
                Some(InvertedVCompromiseDetails {
                    leg_m,
                    leg_ft,
                    span_90_m,
                    span_90_ft,
                    span_120_m,
                    span_120_ft,
                })
            } else {
                None
            };

            let ocfd = if is_ocfd {
                let split_33_short_m = c.length_m / 3.0;
                let split_33_long_m = c.length_m * 2.0 / 3.0;
                let split_20_short_m = c.length_m * 0.2;
                let split_20_long_m = c.length_m * 0.8;
                let split_33_67 = OcfdLegSplit {
                    short_m: split_33_short_m,
                    short_ft: split_33_short_m / FEET_TO_METERS,
                    long_m: split_33_long_m,
                    long_ft: split_33_long_m / FEET_TO_METERS,
                };
                let split_20_80 = OcfdLegSplit {
                    short_m: split_20_short_m,
                    short_ft: split_20_short_m / FEET_TO_METERS,
                    long_m: split_20_long_m,
                    long_ft: split_20_long_m / FEET_TO_METERS,
                };
                let optimized = optimize_ocfd_split_for_length(&results.calculations, c.length_m)
                    .map(|best| OptimizedOcfdSplitView {
                        short_ratio_pct: best.short_ratio * 100.0,
                        long_ratio_pct: best.long_ratio * 100.0,
                        short_leg_m: best.short_leg_m,
                        short_leg_ft: best.short_leg_ft,
                        long_leg_m: best.long_leg_m,
                        long_leg_ft: best.long_leg_ft,
                        worst_leg_clearance_pct: best.worst_leg_clearance_pct,
                    });

                Some(OcfdCompromiseDetails {
                    split_33_67,
                    split_20_80,
                    optimized,
                })
            } else {
                None
            };

            ResonantCompromiseRow {
                length_m: c.length_m,
                length_ft: c.length_ft,
                worst_band_distance_m: c.worst_band_distance_m,
                worst_band_distance_ft: c.worst_band_distance_m / FEET_TO_METERS,
                inverted_v,
                ocfd,
            }
        })
        .collect();

    ResonantCompromiseView {
        heading: narrative.heading,
        notes: narrative.notes,
        empty_message: narrative.empty_message,
        rows,
    }
}

pub fn resonant_compromise_display_view(results: &AppResults) -> ResonantCompromiseDisplayView {
    let view = resonant_compromise_view(results);
    if view.rows.is_empty() {
        return ResonantCompromiseDisplayView {
            heading: view.heading,
            notes: view.notes,
            lines: vec![format!("  {}", view.empty_message)],
        };
    }

    let units = results.config.units;
    let mut lines = Vec::new();

    for (idx, row) in view.rows.iter().take(10).enumerate() {
        match units {
            UnitSystem::Metric => lines.push(format!(
                "  {:2}. {:.2} m (worst-band delta: {:.2} m)",
                idx + 1,
                row.length_m,
                row.worst_band_distance_m
            )),
            UnitSystem::Imperial => lines.push(format!(
                "  {:2}. {:.2} ft (worst-band delta: {:.2} ft)",
                idx + 1,
                row.length_ft,
                row.worst_band_distance_ft
            )),
            UnitSystem::Both => lines.push(format!(
                "  {:2}. {:.2} m ({:.2} ft), worst-band delta: {:.2} m ({:.2} ft)",
                idx + 1,
                row.length_m,
                row.length_ft,
                row.worst_band_distance_m,
                row.worst_band_distance_ft
            )),
        }

        if let Some(inverted_v) = row.inverted_v.as_ref() {
            match units {
                UnitSystem::Metric => {
                    lines.push(format!("      each leg: {:.2} m", inverted_v.leg_m));
                    lines.push(format!(
                        "      span at 90 deg apex: {:.2} m",
                        inverted_v.span_90_m
                    ));
                    lines.push(format!(
                        "      span at 120 deg apex: {:.2} m",
                        inverted_v.span_120_m
                    ));
                }
                UnitSystem::Imperial => {
                    lines.push(format!("      each leg: {:.2} ft", inverted_v.leg_ft));
                    lines.push(format!(
                        "      span at 90 deg apex: {:.2} ft",
                        inverted_v.span_90_ft
                    ));
                    lines.push(format!(
                        "      span at 120 deg apex: {:.2} ft",
                        inverted_v.span_120_ft
                    ));
                }
                UnitSystem::Both => {
                    lines.push(format!(
                        "      each leg: {:.2} m ({:.2} ft)",
                        inverted_v.leg_m, inverted_v.leg_ft
                    ));
                    lines.push(format!(
                        "      span at 90 deg apex: {:.2} m ({:.2} ft)",
                        inverted_v.span_90_m, inverted_v.span_90_ft
                    ));
                    lines.push(format!(
                        "      span at 120 deg apex: {:.2} m ({:.2} ft)",
                        inverted_v.span_120_m, inverted_v.span_120_ft
                    ));
                }
            }
        }

        if let Some(ocfd) = row.ocfd.as_ref() {
            match units {
                UnitSystem::Metric => {
                    lines.push(format!(
                        "      33/67 legs: {:.2} m / {:.2} m",
                        ocfd.split_33_67.short_m, ocfd.split_33_67.long_m
                    ));
                    lines.push(format!(
                        "      20/80 legs: {:.2} m / {:.2} m",
                        ocfd.split_20_80.short_m, ocfd.split_20_80.long_m
                    ));
                }
                UnitSystem::Imperial => {
                    lines.push(format!(
                        "      33/67 legs: {:.2} ft / {:.2} ft",
                        ocfd.split_33_67.short_ft, ocfd.split_33_67.long_ft
                    ));
                    lines.push(format!(
                        "      20/80 legs: {:.2} ft / {:.2} ft",
                        ocfd.split_20_80.short_ft, ocfd.split_20_80.long_ft
                    ));
                }
                UnitSystem::Both => {
                    lines.push(format!(
                        "      33/67 legs: {:.2} m / {:.2} m ({:.2} ft / {:.2} ft)",
                        ocfd.split_33_67.short_m,
                        ocfd.split_33_67.long_m,
                        ocfd.split_33_67.short_ft,
                        ocfd.split_33_67.long_ft
                    ));
                    lines.push(format!(
                        "      20/80 legs: {:.2} m / {:.2} m ({:.2} ft / {:.2} ft)",
                        ocfd.split_20_80.short_m,
                        ocfd.split_20_80.long_m,
                        ocfd.split_20_80.short_ft,
                        ocfd.split_20_80.long_ft
                    ));
                }
            }

            if let Some(best) = ocfd.optimized.as_ref() {
                match units {
                    UnitSystem::Metric => lines.push(format!(
                        "      Optimized split: {:.0}/{:.0} -> {:.2} m / {:.2} m (worst-leg clearance: {:.2}%)",
                        best.short_ratio_pct,
                        best.long_ratio_pct,
                        best.short_leg_m,
                        best.long_leg_m,
                        best.worst_leg_clearance_pct
                    )),
                    UnitSystem::Imperial => lines.push(format!(
                        "      Optimized split: {:.0}/{:.0} -> {:.2} ft / {:.2} ft (worst-leg clearance: {:.2}%)",
                        best.short_ratio_pct,
                        best.long_ratio_pct,
                        best.short_leg_ft,
                        best.long_leg_ft,
                        best.worst_leg_clearance_pct
                    )),
                    UnitSystem::Both => lines.push(format!(
                        "      Optimized split: {:.0}/{:.0} -> {:.2} m / {:.2} m ({:.2} ft / {:.2} ft), worst-leg clearance: {:.2}%",
                        best.short_ratio_pct,
                        best.long_ratio_pct,
                        best.short_leg_m,
                        best.long_leg_m,
                        best.short_leg_ft,
                        best.long_leg_ft,
                        best.worst_leg_clearance_pct
                    )),
                }
            }
        }
    }

    if view.rows.len() > 10 {
        lines.push(format!(
            "  ... and {} more equal compromises",
            view.rows.len() - 10
        ));
    }

    ResonantCompromiseDisplayView {
        heading: view.heading,
        notes: view.notes,
        lines,
    }
}

pub fn resonant_points_in_window(results: &AppResults) -> Vec<ResonantPoint> {
    let min_m = results.config.wire_min_m;
    let max_m = results.config.wire_max_m;
    let mut points = Vec::new();

    for calc in &results.calculations {
        let quarter_wave_m = calc.corrected_quarter_wave_m;
        if quarter_wave_m <= 0.0 {
            continue;
        }

        let mut harmonic = 1_u32;
        loop {
            let resonant_len_m = quarter_wave_m * f64::from(harmonic);
            if resonant_len_m > max_m + 1e-9 {
                break;
            }
            if resonant_len_m >= min_m - 1e-9 {
                points.push(ResonantPoint {
                    length_m: resonant_len_m,
                    band_name: calc.band_name.clone(),
                    harmonic,
                });
            }
            harmonic += 1;
        }
    }

    points.sort_by(|a, b| {
        a.length_m
            .partial_cmp(&b.length_m)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    points
}

pub fn resonant_points_view(results: &AppResults) -> ResonantPointsView {
    let points = resonant_points_in_window(results);
    let min_m = results.config.wire_min_m;
    let max_m = results.config.wire_max_m;
    let min_ft = min_m / FEET_TO_METERS;
    let max_ft = max_m / FEET_TO_METERS;

    let window_line = match results.config.units {
        UnitSystem::Metric => format!("  Search window: {min_m:.2}-{max_m:.2} m"),
        UnitSystem::Imperial => format!("  Search window: {min_ft:.2}-{max_ft:.2} ft"),
        UnitSystem::Both => {
            format!("  Search window: {min_m:.2}-{max_m:.2} m ({min_ft:.2}-{max_ft:.2} ft)")
        }
    };

    let point_lines = points
        .into_iter()
        .map(|point| match results.config.units {
            UnitSystem::Metric => format!(
                "  - {}: {}x quarter-wave = {:.2} m",
                point.band_name, point.harmonic, point.length_m
            ),
            UnitSystem::Imperial => format!(
                "  - {}: {}x quarter-wave = {:.2} ft",
                point.band_name,
                point.harmonic,
                point.length_m / FEET_TO_METERS
            ),
            UnitSystem::Both => format!(
                "  - {}: {}x quarter-wave = {:.2} m ({:.2} ft)",
                point.band_name,
                point.harmonic,
                point.length_m,
                point.length_m / FEET_TO_METERS
            ),
        })
        .collect();

    ResonantPointsView {
        heading: "Resonant points within search window:",
        window_line,
        point_lines,
        empty_message: "  (no resonant points fall within this window)",
    }
}

pub fn resonant_points_display_lines(results: &AppResults) -> Vec<String> {
    let view = resonant_points_view(results);
    let mut lines = vec![view.heading.to_string(), view.window_line];

    if view.point_lines.is_empty() {
        lines.push(view.empty_message.to_string());
    } else {
        lines.extend(view.point_lines);
    }

    lines
}

pub fn band_display_rows(results: &AppResults) -> Vec<BandDisplayRow> {
    results
        .calculations
        .iter()
        .cloned()
        .map(|calc| BandDisplayRow { calc })
        .collect()
}

pub fn band_display_view(
    row: &BandDisplayRow,
    units: UnitSystem,
    antenna_model: Option<AntennaModel>,
) -> BandDisplayView {
    let c = &row.calc;
    let mut lines = vec![
        format!("  Frequency: {:.3} MHz", c.frequency_mhz),
        format!("  Transformer ratio: {}", c.transformer_ratio_label),
    ];

    match units {
        UnitSystem::Metric => match antenna_model {
            Some(AntennaModel::Dipole) => {
                lines.push(format!(
                    "  Half-wave: {:.2} m (base: {:.2} m)",
                    c.corrected_half_wave_m, c.half_wave_m
                ));
                lines.push(format!(
                    "  Full-wave: {:.2} m (base: {:.2} m)",
                    c.corrected_full_wave_m, c.full_wave_m
                ));
                lines.push(format!(
                    "  Quarter-wave: {:.2} m (base: {:.2} m)",
                    c.corrected_quarter_wave_m, c.quarter_wave_m
                ));
            }
            Some(AntennaModel::EndFedHalfWave) => {
                lines.push(format!(
                    "  End-fed half-wave: {:.2} m",
                    c.end_fed_half_wave_m
                ));
            }
            Some(AntennaModel::InvertedVDipole) => {
                lines.push(format!("  Inverted-V total: {:.2} m", c.inverted_v_total_m));
                lines.push(format!(
                    "  Inverted-V each leg: {:.2} m",
                    c.inverted_v_leg_m
                ));
                lines.push(format!(
                    "  Inverted-V span at 90 deg apex: {:.2} m",
                    c.inverted_v_span_90_m
                ));
                lines.push(format!(
                    "  Inverted-V span at 120 deg apex: {:.2} m",
                    c.inverted_v_span_120_m
                ));
            }
            Some(AntennaModel::FullWaveLoop) => {
                lines.push(format!(
                    "  Full-wave loop circumference: {:.2} m",
                    c.full_wave_loop_circumference_m
                ));
                lines.push(format!(
                    "  Full-wave loop square side: {:.2} m",
                    c.full_wave_loop_square_side_m
                ));
            }
            Some(AntennaModel::OffCenterFedDipole) => {
                lines.push(format!(
                    "  OCFD 33/67 legs: {:.2} m / {:.2} m",
                    c.ocfd_33_short_leg_m, c.ocfd_33_long_leg_m
                ));
                lines.push(format!(
                    "  OCFD 20/80 legs: {:.2} m / {:.2} m",
                    c.ocfd_20_short_leg_m, c.ocfd_20_long_leg_m
                ));
            }
            None => {
                lines.push(format!(
                    "  Half-wave: {:.2} m (base: {:.2} m)",
                    c.corrected_half_wave_m, c.half_wave_m
                ));
                lines.push(format!(
                    "  Full-wave: {:.2} m (base: {:.2} m)",
                    c.corrected_full_wave_m, c.full_wave_m
                ));
                lines.push(format!(
                    "  Quarter-wave: {:.2} m (base: {:.2} m)",
                    c.corrected_quarter_wave_m, c.quarter_wave_m
                ));
                lines.push(format!(
                    "  End-fed half-wave: {:.2} m",
                    c.end_fed_half_wave_m
                ));
                lines.push(format!("  Inverted-V total: {:.2} m", c.inverted_v_total_m));
                lines.push(format!(
                    "  Inverted-V each leg: {:.2} m",
                    c.inverted_v_leg_m
                ));
                lines.push(format!(
                    "  Inverted-V span at 90 deg apex: {:.2} m",
                    c.inverted_v_span_90_m
                ));
                lines.push(format!(
                    "  Inverted-V span at 120 deg apex: {:.2} m",
                    c.inverted_v_span_120_m
                ));
                lines.push(format!(
                    "  Full-wave loop circumference: {:.2} m",
                    c.full_wave_loop_circumference_m
                ));
                lines.push(format!(
                    "  Full-wave loop square side: {:.2} m",
                    c.full_wave_loop_square_side_m
                ));
                lines.push(format!(
                    "  OCFD 33/67 legs: {:.2} m / {:.2} m",
                    c.ocfd_33_short_leg_m, c.ocfd_33_long_leg_m
                ));
                lines.push(format!(
                    "  OCFD 20/80 legs: {:.2} m / {:.2} m",
                    c.ocfd_20_short_leg_m, c.ocfd_20_long_leg_m
                ));
            }
        },
        UnitSystem::Imperial => match antenna_model {
            Some(AntennaModel::Dipole) => {
                lines.push(format!(
                    "  Half-wave: {:.2} ft (base: {:.2} ft)",
                    c.corrected_half_wave_ft, c.half_wave_ft
                ));
                lines.push(format!(
                    "  Full-wave: {:.2} ft (base: {:.2} ft)",
                    c.corrected_full_wave_ft, c.full_wave_ft
                ));
                lines.push(format!(
                    "  Quarter-wave: {:.2} ft (base: {:.2} ft)",
                    c.corrected_quarter_wave_ft, c.quarter_wave_ft
                ));
            }
            Some(AntennaModel::EndFedHalfWave) => {
                lines.push(format!(
                    "  End-fed half-wave: {:.2} ft",
                    c.end_fed_half_wave_ft
                ));
            }
            Some(AntennaModel::InvertedVDipole) => {
                lines.push(format!(
                    "  Inverted-V total: {:.2} ft",
                    c.inverted_v_total_ft
                ));
                lines.push(format!(
                    "  Inverted-V each leg: {:.2} ft",
                    c.inverted_v_leg_ft
                ));
                lines.push(format!(
                    "  Inverted-V span at 90 deg apex: {:.2} ft",
                    c.inverted_v_span_90_ft
                ));
                lines.push(format!(
                    "  Inverted-V span at 120 deg apex: {:.2} ft",
                    c.inverted_v_span_120_ft
                ));
            }
            Some(AntennaModel::FullWaveLoop) => {
                lines.push(format!(
                    "  Full-wave loop circumference: {:.2} ft",
                    c.full_wave_loop_circumference_ft
                ));
                lines.push(format!(
                    "  Full-wave loop square side: {:.2} ft",
                    c.full_wave_loop_square_side_ft
                ));
            }
            Some(AntennaModel::OffCenterFedDipole) => {
                lines.push(format!(
                    "  OCFD 33/67 legs: {:.2} ft / {:.2} ft",
                    c.ocfd_33_short_leg_ft, c.ocfd_33_long_leg_ft
                ));
                lines.push(format!(
                    "  OCFD 20/80 legs: {:.2} ft / {:.2} ft",
                    c.ocfd_20_short_leg_ft, c.ocfd_20_long_leg_ft
                ));
            }
            None => {
                lines.push(format!(
                    "  Half-wave: {:.2} ft (base: {:.2} ft)",
                    c.corrected_half_wave_ft, c.half_wave_ft
                ));
                lines.push(format!(
                    "  Full-wave: {:.2} ft (base: {:.2} ft)",
                    c.corrected_full_wave_ft, c.full_wave_ft
                ));
                lines.push(format!(
                    "  Quarter-wave: {:.2} ft (base: {:.2} ft)",
                    c.corrected_quarter_wave_ft, c.quarter_wave_ft
                ));
                lines.push(format!(
                    "  End-fed half-wave: {:.2} ft",
                    c.end_fed_half_wave_ft
                ));
                lines.push(format!(
                    "  Inverted-V total: {:.2} ft",
                    c.inverted_v_total_ft
                ));
                lines.push(format!(
                    "  Inverted-V each leg: {:.2} ft",
                    c.inverted_v_leg_ft
                ));
                lines.push(format!(
                    "  Inverted-V span at 90 deg apex: {:.2} ft",
                    c.inverted_v_span_90_ft
                ));
                lines.push(format!(
                    "  Inverted-V span at 120 deg apex: {:.2} ft",
                    c.inverted_v_span_120_ft
                ));
                lines.push(format!(
                    "  Full-wave loop circumference: {:.2} ft",
                    c.full_wave_loop_circumference_ft
                ));
                lines.push(format!(
                    "  Full-wave loop square side: {:.2} ft",
                    c.full_wave_loop_square_side_ft
                ));
                lines.push(format!(
                    "  OCFD 33/67 legs: {:.2} ft / {:.2} ft",
                    c.ocfd_33_short_leg_ft, c.ocfd_33_long_leg_ft
                ));
                lines.push(format!(
                    "  OCFD 20/80 legs: {:.2} ft / {:.2} ft",
                    c.ocfd_20_short_leg_ft, c.ocfd_20_long_leg_ft
                ));
            }
        },
        UnitSystem::Both => match antenna_model {
            Some(AntennaModel::Dipole) => {
                lines.push(format!(
                    "  Half-wave: {:.2} m ({:.2} ft, base: {:.2} m/{:.2} ft)",
                    c.corrected_half_wave_m,
                    c.corrected_half_wave_ft,
                    c.half_wave_m,
                    c.half_wave_ft
                ));
                lines.push(format!(
                    "  Full-wave: {:.2} m ({:.2} ft, base: {:.2} m/{:.2} ft)",
                    c.corrected_full_wave_m,
                    c.corrected_full_wave_ft,
                    c.full_wave_m,
                    c.full_wave_ft
                ));
                lines.push(format!(
                    "  Quarter-wave: {:.2} m ({:.2} ft, base: {:.2} m/{:.2} ft)",
                    c.corrected_quarter_wave_m,
                    c.corrected_quarter_wave_ft,
                    c.quarter_wave_m,
                    c.quarter_wave_ft
                ));
            }
            Some(AntennaModel::EndFedHalfWave) => {
                lines.push(format!(
                    "  End-fed half-wave: {:.2} m ({:.2} ft)",
                    c.end_fed_half_wave_m, c.end_fed_half_wave_ft
                ));
            }
            Some(AntennaModel::InvertedVDipole) => {
                lines.push(format!(
                    "  Inverted-V total: {:.2} m ({:.2} ft)",
                    c.inverted_v_total_m, c.inverted_v_total_ft
                ));
                lines.push(format!(
                    "  Inverted-V each leg: {:.2} m ({:.2} ft)",
                    c.inverted_v_leg_m, c.inverted_v_leg_ft
                ));
                lines.push(format!(
                    "  Inverted-V span at 90 deg apex: {:.2} m ({:.2} ft)",
                    c.inverted_v_span_90_m, c.inverted_v_span_90_ft
                ));
                lines.push(format!(
                    "  Inverted-V span at 120 deg apex: {:.2} m ({:.2} ft)",
                    c.inverted_v_span_120_m, c.inverted_v_span_120_ft
                ));
            }
            Some(AntennaModel::FullWaveLoop) => {
                lines.push(format!(
                    "  Full-wave loop circumference: {:.2} m ({:.2} ft)",
                    c.full_wave_loop_circumference_m, c.full_wave_loop_circumference_ft
                ));
                lines.push(format!(
                    "  Full-wave loop square side: {:.2} m ({:.2} ft)",
                    c.full_wave_loop_square_side_m, c.full_wave_loop_square_side_ft
                ));
            }
            Some(AntennaModel::OffCenterFedDipole) => {
                lines.push(format!(
                    "  OCFD 33/67 legs: {:.2} m / {:.2} m ({:.2} ft / {:.2} ft)",
                    c.ocfd_33_short_leg_m,
                    c.ocfd_33_long_leg_m,
                    c.ocfd_33_short_leg_ft,
                    c.ocfd_33_long_leg_ft
                ));
                lines.push(format!(
                    "  OCFD 20/80 legs: {:.2} m / {:.2} m ({:.2} ft / {:.2} ft)",
                    c.ocfd_20_short_leg_m,
                    c.ocfd_20_long_leg_m,
                    c.ocfd_20_short_leg_ft,
                    c.ocfd_20_long_leg_ft
                ));
            }
            None => {
                lines.push(format!(
                    "  Half-wave: {:.2} m ({:.2} ft, base: {:.2} m/{:.2} ft)",
                    c.corrected_half_wave_m,
                    c.corrected_half_wave_ft,
                    c.half_wave_m,
                    c.half_wave_ft
                ));
                lines.push(format!(
                    "  Full-wave: {:.2} m ({:.2} ft, base: {:.2} m/{:.2} ft)",
                    c.corrected_full_wave_m,
                    c.corrected_full_wave_ft,
                    c.full_wave_m,
                    c.full_wave_ft
                ));
                lines.push(format!(
                    "  Quarter-wave: {:.2} m ({:.2} ft, base: {:.2} m/{:.2} ft)",
                    c.corrected_quarter_wave_m,
                    c.corrected_quarter_wave_ft,
                    c.quarter_wave_m,
                    c.quarter_wave_ft
                ));
                lines.push(format!(
                    "  End-fed half-wave: {:.2} m ({:.2} ft)",
                    c.end_fed_half_wave_m, c.end_fed_half_wave_ft
                ));
                lines.push(format!(
                    "  Inverted-V total: {:.2} m ({:.2} ft)",
                    c.inverted_v_total_m, c.inverted_v_total_ft
                ));
                lines.push(format!(
                    "  Inverted-V each leg: {:.2} m ({:.2} ft)",
                    c.inverted_v_leg_m, c.inverted_v_leg_ft
                ));
                lines.push(format!(
                    "  Inverted-V span at 90 deg apex: {:.2} m ({:.2} ft)",
                    c.inverted_v_span_90_m, c.inverted_v_span_90_ft
                ));
                lines.push(format!(
                    "  Inverted-V span at 120 deg apex: {:.2} m ({:.2} ft)",
                    c.inverted_v_span_120_m, c.inverted_v_span_120_ft
                ));
                lines.push(format!(
                    "  Full-wave loop circumference: {:.2} m ({:.2} ft)",
                    c.full_wave_loop_circumference_m, c.full_wave_loop_circumference_ft
                ));
                lines.push(format!(
                    "  Full-wave loop square side: {:.2} m ({:.2} ft)",
                    c.full_wave_loop_square_side_m, c.full_wave_loop_square_side_ft
                ));
                lines.push(format!(
                    "  OCFD 33/67 legs: {:.2} m / {:.2} m ({:.2} ft / {:.2} ft)",
                    c.ocfd_33_short_leg_m,
                    c.ocfd_33_long_leg_m,
                    c.ocfd_33_short_leg_ft,
                    c.ocfd_33_long_leg_ft
                ));
                lines.push(format!(
                    "  OCFD 20/80 legs: {:.2} m / {:.2} m ({:.2} ft / {:.2} ft)",
                    c.ocfd_20_short_leg_m,
                    c.ocfd_20_long_leg_m,
                    c.ocfd_20_short_leg_ft,
                    c.ocfd_20_long_leg_ft
                ));
            }
        },
    }

    lines.push(format!(
        "  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
        c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km
    ));

    BandDisplayView {
        title: c.band_name.clone(),
        lines,
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn build_calculations(
    indices: &[usize],
    velocity: f64,
    region: ITURegion,
    transformer_ratio: TransformerRatio,
) -> (Vec<WireCalculation>, Vec<usize>) {
    let mut calculations = Vec::new();
    let mut skipped_band_indices = Vec::new();

    for &idx in indices {
        if idx == 0 {
            skipped_band_indices.push(idx);
            continue;
        }

        let band_index = idx - 1;
        if let Some(band) = get_band_by_index_for_region(band_index, region) {
            calculations.push(calculate_for_band_with_velocity(
                &band,
                velocity,
                transformer_ratio,
            ));
        } else {
            skipped_band_indices.push(idx);
        }
    }

    (calculations, skipped_band_indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_calculation_skips_invalid_band_indices() {
        let config = AppConfig {
            band_indices: vec![0, 1, 100],
            mode: CalcMode::Resonant,
            ..AppConfig::default()
        };

        let results = run_calculation(config);

        assert_eq!(results.calculations.len(), 1);
        assert_eq!(results.calculations[0].band_name, "160m");
        assert_eq!(results.skipped_band_indices, vec![0, 100]);
    }

    #[test]
    fn run_calculation_resonant_mode() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            band_indices: vec![1, 2],
            ..AppConfig::default()
        };

        let results = run_calculation(config);

        assert_eq!(results.calculations.len(), 2);
        assert!(results.window_optima.is_empty());
        assert!(results.optima.is_empty());
        assert!(!results.resonant_compromises.is_empty());
    }

    #[test]
    fn run_calculation_non_resonant_mode() {
        let config = AppConfig {
            mode: CalcMode::NonResonant,
            band_indices: vec![1, 2],
            wire_min_m: 8.0,
            wire_max_m: 35.0,
            ..AppConfig::default()
        };

        let results = run_calculation(config);

        assert_eq!(results.calculations.len(), 2);
        assert!(!results.window_optima.is_empty());
        assert!(!results.optima.is_empty());
        assert!(results.resonant_compromises.is_empty());
    }

    #[test]
    fn run_calculation_stores_config() {
        let config = AppConfig {
            velocity_factor: 0.85,
            mode: CalcMode::Resonant,
            ..AppConfig::default()
        };

        let results = run_calculation(config);

        assert_eq!(results.config.velocity_factor, 0.85);
        assert_eq!(results.config.mode, CalcMode::Resonant);
    }

    #[test]
    fn app_config_default() {
        let config = AppConfig::default();

        assert_eq!(config.mode, CalcMode::Resonant);
        assert_eq!(config.velocity_factor, 0.95);
        assert_eq!(config.itu_region, ITURegion::Region1);
        assert_eq!(config.transformer_ratio, TransformerRatio::R1To1);
        assert_eq!(config.antenna_model, None);
        assert_eq!(config.band_indices, vec![4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn calc_mode_enum_values() {
        let resonant = CalcMode::Resonant;
        let non_resonant = CalcMode::NonResonant;

        assert!(resonant == CalcMode::Resonant);
        assert!(non_resonant == CalcMode::NonResonant);
        assert!(resonant != non_resonant);
    }

    #[test]
    fn export_format_as_str() {
        assert_eq!(ExportFormat::Csv.as_str(), "csv");
        assert_eq!(ExportFormat::Json.as_str(), "json");
        assert_eq!(ExportFormat::Markdown.as_str(), "markdown");
        assert_eq!(ExportFormat::Txt.as_str(), "txt");
    }

    #[test]
    fn unit_system_enum_values() {
        let metric = UnitSystem::Metric;
        let imperial = UnitSystem::Imperial;
        let both = UnitSystem::Both;

        assert!(metric == UnitSystem::Metric);
        assert!(imperial == UnitSystem::Imperial);
        assert!(both == UnitSystem::Both);
    }

    #[test]
    fn run_calculation_multiple_regions() {
        for region in &[ITURegion::Region1, ITURegion::Region2, ITURegion::Region3] {
            let config = AppConfig {
                itu_region: *region,
                band_indices: vec![1, 2, 3],
                ..AppConfig::default()
            };

            let results = run_calculation(config);
            assert!(!results.calculations.is_empty());
        }
    }

    #[test]
    fn run_calculation_all_transformer_ratios() {
        for ratio in &[
            TransformerRatio::R1To1,
            TransformerRatio::R1To2,
            TransformerRatio::R1To4,
            TransformerRatio::R1To9,
            TransformerRatio::R1To64,
        ] {
            let config = AppConfig {
                transformer_ratio: *ratio,
                band_indices: vec![1],
                ..AppConfig::default()
            };

            let results = run_calculation(config);
            assert_eq!(
                results.calculations[0].transformer_ratio_label,
                ratio.as_label()
            );
        }
    }

    #[test]
    fn recommended_transformer_ratio_defaults_by_mode() {
        assert_eq!(
            recommended_transformer_ratio(CalcMode::Resonant, None),
            TransformerRatio::R1To1
        );
        assert_eq!(
            recommended_transformer_ratio(CalcMode::NonResonant, None),
            TransformerRatio::R1To9
        );
    }

    #[test]
    fn recommended_transformer_ratio_matches_antenna_model() {
        assert_eq!(
            recommended_transformer_ratio(CalcMode::Resonant, Some(AntennaModel::Dipole)),
            TransformerRatio::R1To1
        );
        assert_eq!(
            recommended_transformer_ratio(CalcMode::Resonant, Some(AntennaModel::InvertedVDipole)),
            TransformerRatio::R1To1
        );
        assert_eq!(
            recommended_transformer_ratio(CalcMode::Resonant, Some(AntennaModel::FullWaveLoop)),
            TransformerRatio::R1To1
        );
        assert_eq!(
            recommended_transformer_ratio(CalcMode::Resonant, Some(AntennaModel::EndFedHalfWave)),
            TransformerRatio::R1To56
        );
        assert_eq!(
            recommended_transformer_ratio(
                CalcMode::Resonant,
                Some(AntennaModel::OffCenterFedDipole)
            ),
            TransformerRatio::R1To4
        );
    }

    #[test]
    fn recommended_transformer_ratio_fallback_message_is_stable() {
        let msg = recommended_transformer_ratio_fallback_message(
            CalcMode::Resonant,
            Some(AntennaModel::EndFedHalfWave),
        );
        assert_eq!(msg, "Unknown ratio. Using recommended 1:56.");
    }

    #[test]
    fn run_calculation_velocity_factor_range() {
        for vf in &[0.5, 0.75, 0.95, 1.0] {
            let config = AppConfig {
                velocity_factor: *vf,
                ..AppConfig::default()
            };

            let results = run_calculation(config);
            assert_eq!(results.config.velocity_factor, *vf);
        }
    }

    #[test]
    fn validate_config_rejects_invalid_velocity() {
        let config = AppConfig {
            velocity_factor: 1.1,
            ..AppConfig::default()
        };

        let err = validate_config(&config).expect_err("expected invalid velocity error");
        assert_eq!(err, AppError::InvalidVelocityFactor(1.1));
    }

    #[test]
    fn validate_config_rejects_invalid_window() {
        let config = AppConfig {
            wire_min_m: 12.0,
            wire_max_m: 12.0,
            ..AppConfig::default()
        };

        let err = validate_config(&config).expect_err("expected invalid window error");
        assert_eq!(
            err,
            AppError::InvalidWireLengthWindow {
                min_m: 12.0,
                max_m: 12.0,
            }
        );
    }

    #[test]
    fn validate_config_rejects_empty_band_selection() {
        let mut config = AppConfig::default();
        config.band_indices = vec![];

        let err = validate_config(&config).expect_err("expected empty band selection error");
        assert_eq!(err, AppError::EmptyBandSelection);
    }

    #[test]
    fn validate_config_rejects_zero_step() {
        let mut config = AppConfig::default();
        config.step_m = 0.0;

        let err = validate_config(&config).expect_err("expected invalid step error");
        assert_eq!(err, AppError::InvalidSearchStep(0.0));
    }

    #[test]
    fn validate_config_rejects_step_exceeding_window() {
        let mut config = AppConfig::default();
        config.wire_min_m = 8.0;
        config.wire_max_m = 10.0;
        config.step_m = 5.0;

        let err = validate_config(&config).expect_err("expected invalid step error");
        assert_eq!(err, AppError::InvalidSearchStep(5.0));
    }

    #[test]
    fn validate_config_accepts_custom_step_within_window() {
        let mut config = AppConfig::default();
        config.wire_min_m = 8.0;
        config.wire_max_m = 35.0;
        config.step_m = 0.01;

        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn run_calculation_checked_returns_all_bands_skipped_for_invalid_region_indices() {
        let mut config = AppConfig::default();
        // Use an index well beyond any real band to guarantee all calculations are skipped.
        config.band_indices = vec![9999];

        let err = run_calculation_checked(config).expect_err("expected all-bands-skipped error");
        assert_eq!(err, AppError::AllBandsSkipped);
    }

    #[test]
    fn resolve_wire_window_inputs_uses_metric_defaults() {
        let resolved = resolve_wire_window_inputs(None, None, None, None)
            .expect("expected default wire window to resolve");

        assert_eq!(resolved.min_m, DEFAULT_NON_RESONANT_CONFIG.min_len_m);
        assert_eq!(resolved.max_m, DEFAULT_NON_RESONANT_CONFIG.max_len_m);
        assert_eq!(resolved.input_unit, WireWindowInputUnit::Metric);
        assert_eq!(resolved.inferred_display_units, UnitSystem::Metric);
    }

    #[test]
    fn resolve_wire_window_inputs_converts_feet_to_meters() {
        let resolved = resolve_wire_window_inputs(None, None, Some(30.0), Some(90.0))
            .expect("expected imperial wire window to resolve");

        assert!((resolved.min_m - (30.0 * FEET_TO_METERS)).abs() < 1e-9);
        assert!((resolved.max_m - (90.0 * FEET_TO_METERS)).abs() < 1e-9);
        assert_eq!(resolved.input_unit, WireWindowInputUnit::Imperial);
        assert_eq!(resolved.inferred_display_units, UnitSystem::Imperial);
    }

    #[test]
    fn resolve_wire_window_inputs_rejects_mixed_units() {
        let err = resolve_wire_window_inputs(Some(8.0), None, Some(30.0), None)
            .expect_err("expected mixed-unit wire window to fail");

        assert_eq!(err, AppError::MixedWireWindowUnits);
    }

    #[test]
    fn resolve_wire_window_inputs_rejects_invalid_range() {
        let err = resolve_wire_window_inputs(Some(12.0), Some(12.0), None, None)
            .expect_err("expected invalid wire window to fail");

        assert_eq!(
            err,
            AppError::InvalidWireLengthWindow {
                min_m: 12.0,
                max_m: 12.0,
            }
        );
    }

    #[test]
    fn parse_band_selection_supports_names_and_ranges() {
        let parsed = parse_band_selection("40m,20m,17m-12m", ITURegion::Region1)
            .expect("expected band selection to parse");

        assert_eq!(parsed, vec![4, 6, 7, 8, 9]);
    }

    #[test]
    fn parse_band_selection_rejects_unknown_names() {
        let err = parse_band_selection("40m,foobar", ITURegion::Region1)
            .expect_err("expected unknown band selection to fail");

        assert!(err.to_string().contains("unknown band 'foobar'"));
    }

    #[test]
    fn parse_single_band_token_rejects_empty_input() {
        let err = parse_single_band_token("", ITURegion::Region1)
            .expect_err("expected empty token to fail");

        assert!(matches!(err, AppError::InvalidBandSelection(s) if s == "empty band token"));
    }

    #[test]
    fn band_label_for_index_returns_short_name() {
        assert_eq!(band_label_for_index(4, ITURegion::Region1), "40m");
        assert_eq!(band_label_for_index(11, ITURegion::Region1), "120m");
    }

    #[test]
    fn run_calculation_checked_validates_before_execution() {
        let config = AppConfig {
            velocity_factor: 0.4,
            ..AppConfig::default()
        };

        let err = run_calculation_checked(config).expect_err("expected validation failure");
        assert_eq!(err, AppError::InvalidVelocityFactor(0.4));
    }

    #[test]
    fn execute_request_checked_returns_response_wrapper() {
        let request = AppRequest::new(AppConfig::default());

        let response = execute_request_checked(request).expect("expected successful execution");
        assert!(!response.results.calculations.is_empty());
    }

    #[test]
    fn summarize_results_includes_core_overview_metrics() {
        let results = run_calculation(AppConfig::default());

        let summary = summarize_results(&results);
        assert_eq!(summary.overview_heading, "Resonant Overview:");
        assert_eq!(summary.transformer_ratio_label, "1:1");
        assert_eq!(summary.antenna_model_label, "all");
        assert_eq!(summary.band_count, results.calculations.len());
        assert!(summary.average_min_skip_km > 0.0);
        assert!(summary.average_max_skip_km > 0.0);
    }

    #[test]
    fn results_overview_view_formats_expected_lines() {
        let results = run_calculation(AppConfig::default());

        let view = results_overview_view(&results);
        assert_eq!(view.heading, "Resonant Overview:");
        assert!(view
            .header_lines
            .iter()
            .any(|line| line.contains("Using transformer ratio:")));
        assert!(view
            .summary_lines
            .iter()
            .any(|line| line.contains("Summary for")));
        assert!(view
            .summary_lines
            .iter()
            .any(|line| line.contains("Average minimum skip distance:")));
    }

    #[test]
    fn results_section_layout_reflects_resonant_default_mode() {
        let results = run_calculation(AppConfig::default());

        let layout = results_section_layout(&results);
        assert!(layout.show_resonant_points);
        assert!(layout.show_resonant_compromises);
        assert!(!layout.show_non_resonant_recommendation);
    }

    #[test]
    fn results_section_layout_reflects_non_resonant_mode() {
        let config = AppConfig {
            mode: CalcMode::NonResonant,
            ..AppConfig::default()
        };
        let results = run_calculation(config);

        let layout = results_section_layout(&results);
        assert!(!layout.show_resonant_points);
        assert!(!layout.show_resonant_compromises);
        assert!(layout.show_non_resonant_recommendation);
    }

    #[test]
    fn results_display_document_includes_resonant_sections() {
        let results = run_calculation(AppConfig::default());

        let doc = results_display_document(&results);
        assert_eq!(doc.overview_heading, "Resonant Overview:");
        assert!(!doc.band_views.is_empty());
        assert_eq!(doc.sections.len(), 2);
        assert!(doc.sections[0]
            .lines
            .iter()
            .any(|line| line.contains("Resonant points within search window")));
        assert!(doc.sections[1]
            .lines
            .iter()
            .any(|line| line.contains("Closest combined compromises")));
    }

    #[test]
    fn results_display_document_includes_non_resonant_section() {
        let config = AppConfig {
            mode: CalcMode::NonResonant,
            ..AppConfig::default()
        };
        let results = run_calculation(config);

        let doc = results_display_document(&results);
        assert_eq!(
            doc.overview_heading,
            "Non-resonant Overview (band context):"
        );
        assert_eq!(doc.sections.len(), 1);
        assert!(doc.sections[0]
            .lines
            .iter()
            .any(|line| line.contains("Best non-resonant wire length for selected bands:")));
    }

    #[test]
    fn results_display_document_has_no_warning_lines_by_default() {
        let results = run_calculation(AppConfig::default());

        let doc = results_display_document(&results);
        assert!(doc.warning_lines.is_empty());
    }

    #[test]
    fn results_display_document_includes_skipped_band_warning_lines() {
        let mut results = run_calculation(AppConfig::default());
        results.skipped_band_indices = vec![0, 99];

        let doc = results_display_document(&results);
        assert_eq!(doc.warning_lines.len(), 1);
        assert!(doc.warning_lines[0].contains("0, 99"));
    }

    #[test]
    fn skipped_band_warning_formats_values() {
        let mut results = run_calculation(AppConfig::default());
        results.skipped_band_indices = vec![0, 99];

        let warning = skipped_band_warning(&results).expect("expected warning message");
        assert!(warning.contains("0, 99"));
    }

    #[test]
    fn resonant_compromise_narrative_reflects_antenna_model() {
        let config = AppConfig {
            antenna_model: Some(AntennaModel::OffCenterFedDipole),
            ..AppConfig::default()
        };
        let results = run_calculation(config);

        let narrative = resonant_compromise_narrative(&results);
        assert!(narrative.heading.contains("OCFD guidance"));
        assert!(narrative
            .notes
            .iter()
            .any(|note| note.contains("OCFD mode")));
    }

    #[test]
    fn non_resonant_recommendation_messages_are_stable() {
        assert_eq!(
            non_resonant_recommendation_heading(),
            "Best non-resonant wire length for selected bands:"
        );
        assert_eq!(
            non_resonant_recommendation_unavailable_message(),
            "No non-resonant recommendation available for the current selection."
        );
    }

    #[test]
    fn non_resonant_recommendation_view_marks_recommended_rows() {
        let config = AppConfig {
            mode: CalcMode::NonResonant,
            band_indices: vec![4, 5, 6],
            ..AppConfig::default()
        };
        let results = run_calculation(config);

        let view = non_resonant_recommendation_view(&results);
        assert!(view.recommended.is_some());
        assert!(view.local_optima.iter().any(|row| row.is_recommended));
    }

    #[test]
    fn non_resonant_recommendation_view_marks_recommended_rows_in_both_units_text() {
        let config = AppConfig {
            mode: CalcMode::NonResonant,
            units: UnitSystem::Both,
            band_indices: vec![4, 5, 6],
            ..AppConfig::default()
        };
        let mut results = run_calculation(config);
        let recommended = results
            .recommendation
            .expect("expected non-resonant recommendation");
        results.window_optima = vec![
            recommended,
            NonResonantRecommendation {
                length_m: recommended.length_m + 1.0,
                length_ft: recommended.length_ft + (1.0 / FEET_TO_METERS),
                min_resonance_clearance_pct: recommended.min_resonance_clearance_pct - 1.0,
            },
        ];

        let view = non_resonant_recommendation_view(&results);
        assert!(view
            .local_optima_lines
            .iter()
            .any(|line| line.contains(", recommended")));
    }

    #[test]
    fn non_resonant_recommendation_view_handles_missing_recommendation() {
        let mut results = run_calculation(AppConfig::default());
        results.recommendation = None;
        results.optima.clear();
        results.window_optima.clear();

        let view = non_resonant_recommendation_view(&results);
        assert!(view.recommended.is_none());
        assert!(view.equal_optima.is_empty());
        assert!(view.local_optima.is_empty());
        assert_eq!(
            view.unavailable_message,
            "No non-resonant recommendation available for the current selection."
        );
    }

    #[test]
    fn non_resonant_recommendation_display_lines_handles_missing_recommendation() {
        let mut results = run_calculation(AppConfig::default());
        results.recommendation = None;
        results.optima.clear();
        results.window_optima.clear();

        let lines = non_resonant_recommendation_display_lines(&results);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("No non-resonant recommendation available"));
    }

    #[test]
    fn resonant_compromise_view_contains_ocfd_details() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            antenna_model: Some(AntennaModel::OffCenterFedDipole),
            ..AppConfig::default()
        };
        let results = run_calculation(config);

        let view = resonant_compromise_view(&results);
        assert!(view.heading.contains("OCFD guidance"));
        if let Some(first) = view.rows.first() {
            assert!(first.ocfd.is_some());
            assert!(first.inverted_v.is_none());
        }
    }

    #[test]
    fn resonant_compromise_view_contains_inverted_v_details() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            antenna_model: Some(AntennaModel::InvertedVDipole),
            ..AppConfig::default()
        };
        let results = run_calculation(config);

        let view = resonant_compromise_view(&results);
        if let Some(first) = view.rows.first() {
            assert!(first.inverted_v.is_some());
            assert!(first.ocfd.is_none());
        }
    }

    #[test]
    fn resonant_compromise_display_view_formats_lines() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            antenna_model: Some(AntennaModel::OffCenterFedDipole),
            ..AppConfig::default()
        };
        let results = run_calculation(config);

        let view = resonant_compromise_display_view(&results);
        assert!(view.heading.contains("OCFD guidance"));
        assert!(view
            .lines
            .iter()
            .any(|line| line.contains("worst-band delta")));
        assert!(view.lines.iter().any(|line| line.contains("33/67 legs")));
    }

    #[test]
    fn band_display_rows_match_calculation_count() {
        let results = run_calculation(AppConfig::default());

        let rows = band_display_rows(&results);
        assert_eq!(rows.len(), results.calculations.len());
        assert_eq!(rows[0].calc.band_name, results.calculations[0].band_name);
    }

    #[test]
    fn band_display_view_splits_title_and_lines() {
        let results = run_calculation(AppConfig::default());
        let rows = band_display_rows(&results);

        let view = band_display_view(&rows[0], UnitSystem::Metric, Some(AntennaModel::Dipole));
        assert!(!view.title.is_empty());
        assert!(!view.lines.is_empty());
        assert!(view.lines[0].starts_with("  Frequency:"));
    }

    #[test]
    fn resonant_points_in_window_returns_sorted_points() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            band_indices: vec![4, 5, 6],
            ..AppConfig::default()
        };
        let results = run_calculation(config);

        let points = resonant_points_in_window(&results);
        assert!(!points.is_empty());
        assert!(points
            .windows(2)
            .all(|pair| pair[0].length_m <= pair[1].length_m));
    }

    #[test]
    fn resonant_points_in_window_can_be_empty() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            wire_min_m: 0.1,
            wire_max_m: 0.2,
            ..AppConfig::default()
        };
        let results = run_calculation(config);

        let points = resonant_points_in_window(&results);
        assert!(points.is_empty());
    }

    #[test]
    fn resonant_points_view_formats_search_window() {
        let config = AppConfig {
            units: UnitSystem::Both,
            ..AppConfig::default()
        };
        let results = run_calculation(config);

        let view = resonant_points_view(&results);
        assert_eq!(view.heading, "Resonant points within search window:");
        assert!(view.window_line.contains("Search window:"));
    }

    #[test]
    fn resonant_points_view_can_render_empty_message() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            wire_min_m: 0.1,
            wire_max_m: 0.2,
            ..AppConfig::default()
        };
        let results = run_calculation(config);

        let view = resonant_points_view(&results);
        assert!(view.point_lines.is_empty());
        assert!(view.empty_message.contains("no resonant points"));
    }

    #[test]
    fn resonant_points_display_lines_include_empty_message() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            wire_min_m: 0.1,
            wire_max_m: 0.2,
            ..AppConfig::default()
        };
        let results = run_calculation(config);

        let lines = resonant_points_display_lines(&results);
        assert!(lines
            .iter()
            .any(|line| line.contains("Resonant points within search window")));
        assert!(lines
            .iter()
            .any(|line| line.contains("no resonant points fall within this window")));
    }

    // --- App API contract tests (guard the stable GUI-facing boundary) ---

    #[test]
    fn app_request_from_config_round_trips() {
        let config = AppConfig::default();
        let request = AppRequest::from(config.clone());
        assert_eq!(request.config.velocity_factor, config.velocity_factor);
        assert_eq!(request.config.mode, config.mode);
        assert_eq!(request.config.band_indices, config.band_indices);
    }

    #[test]
    fn execute_request_checked_response_contains_results() {
        let response = execute_request_checked(AppRequest::new(AppConfig::default()))
            .expect("default config should succeed");
        assert!(!response.results.calculations.is_empty());
        assert_eq!(response.results.config.mode, CalcMode::Resonant);
    }

    #[test]
    fn results_display_document_is_fully_populated_for_resonant_default() {
        let results = run_calculation(AppConfig::default());
        let doc = results_display_document(&results);

        assert!(!doc.overview_heading.is_empty());
        assert!(!doc.overview_header_lines.is_empty());
        assert!(!doc.band_views.is_empty());
        assert!(!doc.summary_lines.is_empty());
        assert!(!doc.sections.is_empty());
    }

    #[test]
    fn results_display_document_is_fully_populated_for_non_resonant() {
        let mut config = AppConfig::default();
        config.mode = CalcMode::NonResonant;
        let results = run_calculation(config);
        let doc = results_display_document(&results);

        assert!(!doc.overview_heading.is_empty());
        assert!(!doc.band_views.is_empty());
        assert!(!doc.sections.is_empty());
    }

    #[test]
    fn all_antenna_models_execute_without_error() {
        let models = [
            Some(AntennaModel::Dipole),
            Some(AntennaModel::InvertedVDipole),
            Some(AntennaModel::EndFedHalfWave),
            Some(AntennaModel::FullWaveLoop),
            Some(AntennaModel::OffCenterFedDipole),
            None,
        ];
        for model in &models {
            let mut config = AppConfig::default();
            config.antenna_model = *model;
            execute_request_checked(AppRequest::new(config))
                .expect("all antenna models should succeed with default config");
        }
    }

    #[test]
    fn all_calc_modes_execute_without_error() {
        for mode in &[CalcMode::Resonant, CalcMode::NonResonant] {
            let mut config = AppConfig::default();
            config.mode = *mode;
            execute_request_checked(AppRequest::new(config))
                .expect("both calc modes should succeed with default config");
        }
    }
}

#[cfg(test)]
mod app_error_tests {
    use super::*;

    #[test]
    fn test_invalid_velocity_factor() {
        let config = AppConfig {
            velocity_factor: 1.5,
            ..Default::default()
        };
        let err = validate_config(&config).unwrap_err();
        assert!(matches!(err, AppError::InvalidVelocityFactor(_)));
        assert!(err.to_string().contains("velocity factor must be between"));
    }

    #[test]
    fn test_invalid_wire_length_window() {
        let config = AppConfig {
            wire_min_m: 10.0,
            wire_max_m: 5.0,
            ..Default::default()
        };
        let err = validate_config(&config).unwrap_err();
        assert!(matches!(err, AppError::InvalidWireLengthWindow { .. }));
        assert!(err.to_string().contains("invalid wire length window"));
    }

    #[test]
    fn test_mixed_wire_window_units() {
        let err = resolve_wire_window_inputs(Some(10.0), None, Some(20.0), None).unwrap_err();
        assert!(matches!(err, AppError::MixedWireWindowUnits));
        assert!(err.to_string().contains("cannot mix meter and feet"));
    }

    #[test]
    fn test_invalid_calc_mode() {
        let err = <CalcMode as std::str::FromStr>::from_str("foo").unwrap_err();
        assert!(matches!(err, AppError::InvalidCalcMode(_)));
        assert!(err.to_string().contains("Invalid calculation mode"));
    }

    #[test]
    fn test_invalid_export_format() {
        let err = <ExportFormat as std::str::FromStr>::from_str("foo").unwrap_err();
        assert!(matches!(err, AppError::InvalidExportFormat(_)));
        assert!(err.to_string().contains("Invalid export format"));
    }

    #[test]
    fn test_invalid_unit_system() {
        let err = <UnitSystem as std::str::FromStr>::from_str("foo").unwrap_err();
        assert!(matches!(err, AppError::InvalidUnitSystem(_)));
        assert!(err.to_string().contains("Invalid unit system"));
    }

    #[test]
    fn test_invalid_antenna_model() {
        let err = <AntennaModel as std::str::FromStr>::from_str("foo").unwrap_err();
        assert!(matches!(err, AppError::InvalidAntennaModel(_)));
        assert!(err.to_string().contains("Invalid antenna model"));
    }

    #[test]
    fn test_invalid_band_selection() {
        let err = parse_band_selection("foo", ITURegion::Region1).unwrap_err();
        assert!(matches!(err, AppError::InvalidBandSelection(_)));
        assert!(err.to_string().contains("Invalid band selection"));
    }

    #[test]
    fn test_empty_band_selection() {
        let err = parse_band_selection("", ITURegion::Region1).unwrap_err();
        assert!(matches!(err, AppError::EmptyBandSelection));
        assert!(err.to_string().contains("empty selection"));
    }
}
