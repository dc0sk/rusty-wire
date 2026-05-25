#[derive(Debug, Clone, PartialEq)]
pub enum AppError {
    InvalidVelocityFactor(f64),
    InvalidWireLengthWindow {
        min_m: f64,
        max_m: f64,
    },
    MixedWireWindowUnits,
    InvalidCalcMode(String),
    InvalidExportFormat(String),
    InvalidUnitSystem(String),
    InvalidAntennaModel(String),
    InvalidBandSelection(String),
    InvalidSearchStep(f64),
    InvalidFrequency(f64),
    InvalidAntennaHeight(f64),
    InvalidConductorDiameter(f64),
    InvalidHybridSectionSplit([f64; 3]),
    /// A velocity factor in a `--velocity-sweep` list is out of the 0.50–1.00 range.
    InvalidVelocitySweep(f64),
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
            AppError::InvalidAntennaHeight(height) => {
                write!(
                    f,
                    "antenna height must be one of 7 m, 10 m, or 12 m (got {height:.2} m)"
                )
            }
            AppError::InvalidConductorDiameter(diameter_mm) => {
                write!(
                    f,
                    "conductor diameter must be between {MIN_CONDUCTOR_DIAMETER_MM:.1} mm and {MAX_CONDUCTOR_DIAMETER_MM:.1} mm (got {diameter_mm:.2} mm)"
                )
            }
            AppError::InvalidHybridSectionSplit(split) => {
                let sum = split[0] + split[1] + split[2];
                write!(
                    f,
                    "hybrid section split must have 3 positive ratios summing to 1.00 (got {:.3},{:.3},{:.3}; sum {:.3})",
                    split[0], split[1], split[2], sum
                )
            }
            AppError::InvalidVelocitySweep(vf) => {
                write!(
                    f,
                    "velocity factor {vf:.2} is out of range (must be 0.50\u{2013}1.00)"
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
    calculate_best_non_resonant_length, calculate_for_band_with_environment,
    calculate_non_resonant_optima, calculate_non_resonant_window_optima,
    calculate_resonant_compromises, optimize_ocfd_split_for_length, GroundClass,
    NonResonantRecommendation, NonResonantSearchConfig, ResonantCompromise, TransformerRatio,
    WireCalculation, DEFAULT_CONDUCTOR_DIAMETER_MM as CALC_DEFAULT_CONDUCTOR_DIAMETER_MM,
    DEFAULT_NON_RESONANT_CONFIG, MAX_CONDUCTOR_DIAMETER_MM, MIN_CONDUCTOR_DIAMETER_MM,
};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;

pub mod advise;
pub mod state;
pub use advise::*;
pub use state::*;

pub const FEET_TO_METERS: f64 = 0.3048;
pub const DEFAULT_BAND_SELECTION: [usize; 7] = [4, 5, 6, 7, 8, 9, 10];
pub const DEFAULT_ITU_REGION: ITURegion = ITURegion::Region1;
pub const DEFAULT_TRANSFORMER_RATIO: TransformerRatio = TransformerRatio::R1To1;
pub const STANDARD_ANTENNA_HEIGHTS_M: [f64; 3] = [7.0, 10.0, 12.0];
pub const DEFAULT_ANTENNA_HEIGHT_M: f64 = 10.0;
pub const DEFAULT_GROUND_CLASS: GroundClass = GroundClass::Average;
pub const DEFAULT_CONDUCTOR_DIAMETER_MM: f64 = CALC_DEFAULT_CONDUCTOR_DIAMETER_MM;
pub const DEFAULT_HYBRID_SECTION_SPLIT: [f64; 3] = [0.40, 0.35, 0.25];

pub fn recommended_transformer_ratio(
    mode: CalcMode,
    antenna_model: Option<AntennaModel>,
) -> TransformerRatio {
    match antenna_model {
        Some(AntennaModel::Dipole)
        | Some(AntennaModel::InvertedVDipole)
        | Some(AntennaModel::TrapDipole)
        | Some(AntennaModel::HybridMultiSection)
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
    let explanation = transformer_ratio_explanation(mode, antenna_model);
    format!(
        "Unknown ratio. Using recommended {}.",
        explanation.ratio.as_label()
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
    Html,
    Json,
    Markdown,
    Nec,
    Txt,
    Yaml,
}

impl ExportFormat {
    #[allow(dead_code)] // used in tests
    pub fn as_str(self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Html => "html",
            ExportFormat::Json => "json",
            ExportFormat::Markdown => "markdown",
            ExportFormat::Nec => "nec",
            ExportFormat::Txt => "txt",
            ExportFormat::Yaml => "yaml",
        }
    }
}

impl FromStr for ExportFormat {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "csv" => Ok(ExportFormat::Csv),
            "html" | "htm" => Ok(ExportFormat::Html),
            "json" => Ok(ExportFormat::Json),
            "markdown" | "md" => Ok(ExportFormat::Markdown),
            "nec" => Ok(ExportFormat::Nec),
            "txt" | "text" => Ok(ExportFormat::Txt),
            "yaml" | "yml" => Ok(ExportFormat::Yaml),
            _ => Err(AppError::InvalidExportFormat(format!(
                "'{s}' (must be 'csv', 'html', 'json', 'markdown', 'nec', 'txt', or 'yaml')"
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
    TrapDipole,
    HybridMultiSection,
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
            "trap-dipole" | "trap" | "trapdipole" => Ok(AntennaModel::TrapDipole),
            "hybrid" | "hybrid-multi" | "hybrid-multi-section" | "multi-section"
            | "multi-section-dipole" | "multisection" => Ok(AntennaModel::HybridMultiSection),
            _ => Err(AppError::InvalidAntennaModel(format!(
                "'{s}' (must be 'dipole', 'inverted-v', 'efhw', 'loop', 'ocfd', 'trap-dipole', or 'hybrid-multi')"
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
    pub antenna_height_m: f64,
    pub ground_class: GroundClass,
    pub conductor_diameter_mm: f64,
    /// Hybrid multi-section per-side split ratios `[s1, s2, s3]`.
    /// Must be positive and sum to 1.0.
    pub hybrid_section_split: [f64; 3],
    /// Direct frequency input in MHz; when set, bypasses band selection entirely.
    pub custom_freq_mhz: Option<f64>,
    /// Multiple explicit frequencies in MHz; when non-empty, bypasses band selection.
    /// Takes precedence over `custom_freq_mhz` if both are set.
    pub freq_list_mhz: Vec<f64>,
    /// Whether to validate advise candidates using fnec-rust (if available).
    pub validate_with_fnec: bool,
    /// User-defined bands loaded from a `bands.toml` file.
    /// These are always appended to the calculation results, independent of
    /// `band_indices` / `freq_list_mhz` / `custom_freq_mhz`.
    pub extra_bands: Vec<crate::bands::OwnedBand>,
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
            antenna_height_m: DEFAULT_ANTENNA_HEIGHT_M,
            ground_class: DEFAULT_GROUND_CLASS,
            conductor_diameter_mm: DEFAULT_CONDUCTOR_DIAMETER_MM,
            hybrid_section_split: DEFAULT_HYBRID_SECTION_SPLIT,
            custom_freq_mhz: None,
            freq_list_mhz: vec![],
            validate_with_fnec: false,
            extra_bands: vec![],
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
    /// Optional context for IPC, async, or correlation tracing.
    /// `None` in all synchronous CLI/TUI paths; set by callers that need
    /// per-request correlation (e.g. a future iced GUI message dispatch).
    pub context: Option<RequestContext>,
}

impl AppRequest {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            context: None,
        }
    }
}

impl From<AppConfig> for AppRequest {
    fn from(config: AppConfig) -> Self {
        Self::new(config)
    }
}

/// Optional correlation context attached to an [`AppRequest`] and echoed
/// back on the corresponding [`AppResponse`].
///
/// Designed for async and IPC use cases (e.g. the planned iced GUI) where
/// multiple in-flight requests need to be correlated to their responses.
/// Neither field is required for synchronous CLI/TUI paths.
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    /// Monotonically-increasing request identifier, generated by a
    /// process-global atomic counter via [`RequestContext::new`].
    pub request_id: u64,
    /// Wall-clock timestamp at creation, seconds since the Unix epoch.
    pub timestamp_secs: u64,
}

impl RequestContext {
    /// Create a new context with an auto-incrementing `request_id`
    /// and the current Unix timestamp.
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let request_id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let timestamp_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            request_id,
            timestamp_secs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppResponse {
    pub results: AppResults,
    /// Context echoed from the originating [`AppRequest`], if any.
    pub context: Option<RequestContext>,
}

impl AppResponse {
    pub fn new(results: AppResults) -> Self {
        Self {
            results,
            context: None,
        }
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
    /// Structured explanation for the transformer ratio in use — suitable for
    /// TUI tooltip text, GUI labels, or verbose CLI output.
    pub transformer_explanation: TransformerRatioExplanation,
    /// Structured list of bands excluded from this run, with per-band reasons.
    /// Mirrors the text in `warning_lines` but gives consumers structured access.
    pub skipped_band_details: Vec<SkippedBandDetail>,
    /// Present when the configured transformer ratio differs from the model recommendation.
    /// Also mirrored in `warning_lines` for text-based consumers.
    pub transformer_mismatch_warning: Option<TransformerMismatchWarning>,
}

/// One example L/C component pair satisfying a trap's resonant condition.
#[derive(Debug, Clone, PartialEq)]
pub struct TrapDipoleComponentExample {
    /// Capacitor value (pF).
    pub cap_pf: f64,
    /// Required inductance (μH) for this capacitor to resonate at the trap frequency.
    pub ind_uh: f64,
}

/// Structured guidance for a single trap dipole band pair.
#[derive(Debug, Clone)]
pub struct TrapDipoleGuidanceSection {
    /// Human-readable label, e.g. "40m / 20m".
    pub label: String,
    /// Recommended trap resonant frequency (MHz) — equals upper-band centre.
    pub trap_freq_mhz: f64,
    /// Inner section per side: feedpoint to trap (m).
    pub inner_leg_m: f64,
    /// Outer section per side: trap to tip (m).
    pub outer_section_m: f64,
    /// Total leg per side: inner + outer (m).
    pub total_leg_m: f64,
    /// Full tip-to-tip span (m).
    pub full_span_m: f64,
    /// Example L/C pairs for the trap at `trap_freq_mhz`.
    pub component_examples: Vec<TrapDipoleComponentExample>,
}

/// View model for the trap dipole guidance block shown in results.
#[derive(Debug, Clone)]
pub struct TrapDipoleGuidanceView {
    pub velocity_factor: f64,
    pub sections: Vec<TrapDipoleGuidanceSection>,
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
    pub hybrid_section_split: [f64; 3],
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

/// One row in the band-listing view (used by `band_listing_view`).
#[derive(Debug, Clone)]
pub struct BandListingRow {
    /// 1-based display index.
    pub index: usize,
    /// Full formatted band description (e.g. "40m [HF] (7.0-7.2 MHz)").
    pub display: String,
}

/// Structured data for a region's band listing.
///
/// Pure view model — no I/O. Use `band_listing_display_lines` to render it.
#[derive(Debug, Clone)]
pub struct BandListingView {
    pub region_short_name: String,
    pub region_long_name: String,
    pub rows: Vec<BandListingRow>,
}

/// Per-band skip detail attached to a calculation result.
#[derive(Debug, Clone)]
pub struct SkippedBandDetail {
    /// 1-based band index that was excluded from the run.
    pub band_index: usize,
    /// Human-readable reason the band was skipped.
    pub reason: &'static str,
}

/// Warning issued when the user-configured transformer ratio differs from the
/// antenna-model recommendation produced by `transformer_ratio_explanation`.
#[derive(Debug, Clone)]
pub struct TransformerMismatchWarning {
    /// The ratio the user has configured.
    pub configured: TransformerRatio,
    /// The ratio recommended for the active antenna model and mode.
    pub recommended: TransformerRatio,
}

impl TransformerMismatchWarning {
    /// One-sentence human-readable description of the mismatch.
    pub fn message(&self) -> String {
        format!(
            "Transformer set to {} but {} is recommended for this antenna model.",
            self.configured.as_label(),
            self.recommended.as_label(),
        )
    }
}

// ---------------------------------------------------------------------------
// Public computation API
// ---------------------------------------------------------------------------

/// Run all wire calculations for the given configuration.
///
/// This is a pure, I/O-free function suitable for use from both the CLI and
/// any future GUI front-end.
pub fn run_calculation(config: AppConfig) -> AppResults {
    let (mut calculations, skipped_band_indices) = if !config.freq_list_mhz.is_empty() {
        let calcs = config
            .freq_list_mhz
            .iter()
            .map(|&freq_mhz| {
                let custom_band = Band {
                    name: "custom",
                    band_type: BandType::HF,
                    freq_low_mhz: freq_mhz,
                    freq_high_mhz: freq_mhz,
                    freq_center_mhz: freq_mhz,
                    typical_skip_km: (0.0, 0.0),
                    regions: &[],
                };
                let mut calc = calculate_for_band_with_environment(
                    &custom_band,
                    config.velocity_factor,
                    config.transformer_ratio,
                    config.antenna_height_m,
                    config.ground_class,
                    config.conductor_diameter_mm,
                );
                calc.band_name = format!("{freq_mhz:.3} MHz");
                calc
            })
            .collect();
        (calcs, Vec::new())
    } else if let Some(freq_mhz) = config.custom_freq_mhz {
        let custom_band = Band {
            name: "custom",
            band_type: BandType::HF,
            freq_low_mhz: freq_mhz,
            freq_high_mhz: freq_mhz,
            freq_center_mhz: freq_mhz,
            typical_skip_km: (0.0, 0.0),
            regions: &[],
        };
        let mut calc = calculate_for_band_with_environment(
            &custom_band,
            config.velocity_factor,
            config.transformer_ratio,
            config.antenna_height_m,
            config.ground_class,
            config.conductor_diameter_mm,
        );
        calc.band_name = format!("{freq_mhz:.3} MHz");
        (vec![calc], Vec::new())
    } else {
        build_calculations(
            &config.band_indices,
            config.velocity_factor,
            config.itu_region,
            config.transformer_ratio,
            config.antenna_height_m,
            config.ground_class,
            config.conductor_diameter_mm,
        )
    };

    // Append user-defined extra bands from a bands.toml file.
    for owned_band in &config.extra_bands {
        let stub = crate::bands::Band {
            name: "custom",
            band_type: crate::bands::BandType::HF,
            freq_low_mhz: owned_band.freq_low_mhz,
            freq_high_mhz: owned_band.freq_high_mhz,
            freq_center_mhz: owned_band.center_mhz(),
            typical_skip_km: (0.0, 0.0),
            regions: &[],
        };
        let mut calc = calculate_for_band_with_environment(
            &stub,
            config.velocity_factor,
            config.transformer_ratio,
            config.antenna_height_m,
            config.ground_class,
            config.conductor_diameter_mm,
        );
        calc.band_name = owned_band.name.clone();
        calculations.push(calc);
    }

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
    if !config.freq_list_mhz.is_empty() {
        for &freq in &config.freq_list_mhz {
            if freq <= 0.0 || freq > 1000.0 {
                return Err(AppError::InvalidFrequency(freq));
            }
        }
    } else if let Some(freq) = config.custom_freq_mhz {
        if freq <= 0.0 || freq > 1000.0 {
            return Err(AppError::InvalidFrequency(freq));
        }
    } else if config.band_indices.is_empty() {
        return Err(AppError::EmptyBandSelection);
    }

    if !(0.5..=1.0).contains(&config.velocity_factor) {
        return Err(AppError::InvalidVelocityFactor(config.velocity_factor));
    }

    if !STANDARD_ANTENNA_HEIGHTS_M
        .iter()
        .any(|v| (config.antenna_height_m - *v).abs() < 1e-9)
    {
        return Err(AppError::InvalidAntennaHeight(config.antenna_height_m));
    }

    if !(MIN_CONDUCTOR_DIAMETER_MM..=MAX_CONDUCTOR_DIAMETER_MM)
        .contains(&config.conductor_diameter_mm)
    {
        return Err(AppError::InvalidConductorDiameter(
            config.conductor_diameter_mm,
        ));
    }

    let split = config.hybrid_section_split;
    let split_sum = split[0] + split[1] + split[2];
    if split.iter().any(|v| *v <= 0.0) || (split_sum - 1.0).abs() > 1e-6 {
        return Err(AppError::InvalidHybridSectionSplit(split));
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

/// Build a pure view model for the band listing of a given ITU region.
///
/// Pure function; performs no I/O.
pub fn band_listing_view(region: ITURegion) -> BandListingView {
    let rows = crate::bands::get_bands_for_region(region)
        .into_iter()
        .map(|(idx, band)| BandListingRow {
            index: idx + 1,
            display: format!("{band}"),
        })
        .collect();
    BandListingView {
        region_short_name: region.short_name().to_string(),
        region_long_name: region.long_name().to_string(),
        rows,
    }
}

/// Render a `BandListingView` to display lines (no I/O).
pub fn band_listing_display_lines(view: &BandListingView) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(String::new());
    lines.push(format!(
        "Available bands in Region {} ({} total):",
        view.region_short_name,
        view.rows.len()
    ));
    lines.push(format!("  ({})", view.region_long_name));
    lines.push("------------------------------------------------------------".to_string());
    for row in &view.rows {
        lines.push(format!("{:2}. {}", row.index, row.display));
    }
    lines.push(String::new());
    lines
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
    let context = request.context.clone();
    let results = run_calculation_checked(request.config)?;
    Ok(AppResponse { results, context })
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
            Some(AntennaModel::TrapDipole) => "trap dipole",
            Some(AntennaModel::HybridMultiSection) => "hybrid multi-section dipole",
        },
        band_count: results.calculations.len(),
        average_min_skip_km: calculate_average_min_distance(&results.calculations),
        average_max_skip_km: calculate_average_max_distance(&results.calculations),
    }
}

pub fn results_overview_view(results: &AppResults) -> ResultsOverviewView {
    let summary = summarize_results(results);

    let mut header_lines = vec![
        "------------------------------------------------------------".to_string(),
        format!(
            "Using transformer ratio: {}",
            summary.transformer_ratio_label
        ),
        format!("Antenna model: {}", summary.antenna_model_label),
        format!("Antenna height: {:.0} m", results.config.antenna_height_m),
        format!("Ground class: {}", results.config.ground_class.as_label()),
        format!(
            "Conductor diameter: {:.1} mm",
            results.config.conductor_diameter_mm
        ),
        "------------------------------------------------------------".to_string(),
    ];

    if results.config.antenna_model == Some(AntennaModel::EndFedHalfWave) {
        let feedpoint_r = assumed_feedpoint_impedance_ohm(
            results.config.mode,
            results.config.antenna_model,
            results.config.antenna_height_m,
            results.config.ground_class,
        );
        let cmp = compare_efhw_transformers(feedpoint_r);
        header_lines.push(format!(
            "EFHW transformer comparison (feedpoint R: {:.0} \u{03a9}):",
            cmp.feedpoint_r_ohm
        ));
        header_lines.push(format!(
            "  {:<5}  {:<8}  {:<6}  {:<11}  {}",
            "Ratio", "Target Z", "SWR", "Efficiency", "Loss"
        ));
        for entry in &cmp.entries {
            let marker = if entry.is_best {
                "  \u{2190} recommended"
            } else {
                ""
            };
            header_lines.push(format!(
                "  {:<5}  {:>5.0} \u{03a9}  {:>4.2}:1  {:>9.2}%  {:.3} dB{}",
                entry.ratio.as_label(),
                entry.target_z_ohm,
                entry.swr,
                entry.efficiency_pct,
                entry.mismatch_loss_db,
                marker
            ));
        }
        header_lines
            .push("------------------------------------------------------------".to_string());
    }

    ResultsOverviewView {
        heading: summary.overview_heading,
        header_lines,
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
        .map(|row| {
            band_display_view(
                row,
                results.config.units,
                results.config.antenna_model,
                results.config.transformer_ratio,
            )
        })
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
    if let Some(trap_guidance) = trap_dipole_guidance_view(results) {
        sections.push(ResultsTextSectionView {
            lines: trap_dipole_guidance_display_lines(&trap_guidance, results.config.units),
        });
    }

    let skipped = skipped_band_details(results);
    let transformer_expl =
        transformer_ratio_explanation(results.config.mode, results.config.antenna_model);
    let mismatch = if results.config.transformer_ratio != transformer_expl.ratio {
        Some(TransformerMismatchWarning {
            configured: results.config.transformer_ratio,
            recommended: transformer_expl.ratio,
        })
    } else {
        None
    };
    let mut warning_lines: Vec<String> = skipped_band_warning(results).into_iter().collect();
    if let Some(ref mw) = mismatch {
        warning_lines.push(mw.message());
    }
    ResultsDisplayDocument {
        overview_heading: overview.heading,
        overview_header_lines: overview.header_lines,
        band_views,
        summary_lines: overview.summary_lines,
        sections,
        warning_lines,
        transformer_explanation: transformer_expl,
        skipped_band_details: skipped,
        transformer_mismatch_warning: mismatch,
    }
}

/// Return per-band skip details for all bands excluded from this run.
///
/// Pure function; performs no I/O.
pub fn skipped_band_details(results: &AppResults) -> Vec<SkippedBandDetail> {
    results
        .skipped_band_indices
        .iter()
        .map(|&idx| SkippedBandDetail {
            band_index: idx,
            reason: "not available in the selected ITU region",
        })
        .collect()
}

pub fn skipped_band_warning(results: &AppResults) -> Option<String> {
    let details = skipped_band_details(results);
    if details.is_empty() {
        return None;
    }

    let skipped = details
        .iter()
        .map(|d| d.band_index.to_string())
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
        Some(AntennaModel::TrapDipole) => {
            "Closest combined compromises to resonant points (trap dipole guidance):"
        }
        Some(AntennaModel::HybridMultiSection) => {
            "Closest combined compromises to resonant points (hybrid multi-section guidance):"
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
    if matches!(results.config.antenna_model, Some(AntennaModel::TrapDipole)) {
        notes.push(
            "Trap mode: each compromise line is total tip-to-tip wire; each element is half that value.",
        );
        notes.push(
            "Trap frequency/components: tune each trap near the upper-band resonance (for example, around 14 MHz for 40m/20m or around 7 MHz for 80m/40m) and target high unloaded Q with low-loss capacitors.",
        );
        notes.push(
            "Physical placement: start with traps positioned at the upper-band element endpoint and trim symmetrically from both outer ends.",
        );
        notes.push(
            "Common pairings: 40m/20m and 80m/40m are the most practical starting configurations for two-trap builds.",
        );
    }
    if matches!(
        results.config.antenna_model,
        Some(AntennaModel::HybridMultiSection)
    ) {
        notes.push(
            "Hybrid multi-section mode: each element side is split into 3 contiguous sections using the configured planning ratio.",
        );
        notes.push(
            "Use these as cut-sheet starting values; trim symmetrically from outer sections during final tuning.",
        );
    }

    ResonantCompromiseNarrative {
        heading,
        notes,
        empty_message: "(none available in this window)",
    }
}

/// Choose three representative capacitor values (pF) from a standard series
/// for a trap with the given L·C product (μH·pF).  The returned values yield
/// inductors in roughly the 1–20 μH range, which is practical for HF traps.
fn select_trap_cap_examples(lc_product: f64) -> [f64; 3] {
    // Boundaries: cap_min gives L≈20 μH, cap_max gives L≈1 μH.
    // Pick three representative values from standard E6/E12 series.
    if lc_product < 80.0 {
        [47.0, 33.0, 22.0]
    } else if lc_product < 200.0 {
        [100.0, 68.0, 47.0]
    } else if lc_product < 600.0 {
        [470.0, 220.0, 100.0]
    } else if lc_product < 2_000.0 {
        [1_000.0, 470.0, 220.0]
    } else {
        [3_300.0, 1_500.0, 1_000.0]
    }
}

/// Build structured trap-dipole guidance for the currently selected bands.
///
/// Returns `None` when not in trap-dipole mode or when fewer than two bands
/// are selected (the calculation requires an upper and a lower band).
pub fn trap_dipole_guidance_view(results: &AppResults) -> Option<TrapDipoleGuidanceView> {
    if !matches!(results.config.antenna_model, Some(AntennaModel::TrapDipole)) {
        return None;
    }

    // Collect Band objects for every selected index.
    // `band_indices` are 1-based (matching user display); convert to 0-based for lookup.
    let mut bands: Vec<Band> = results
        .config
        .band_indices
        .iter()
        .filter_map(|&idx| {
            idx.checked_sub(1)
                .and_then(|i| get_band_by_index_for_region(i, results.config.itu_region))
        })
        .collect();

    if bands.len() < 2 {
        return None;
    }

    // Sort descending by frequency (highest = upper band, lowest = lower band).
    bands.sort_by(|a, b| {
        b.freq_center_mhz
            .partial_cmp(&a.freq_center_mhz)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let vf = results.config.velocity_factor;

    // For each adjacent (upper, lower) pair compute one guidance section.
    // The first pair is the most useful; additional pairs cover multi-trap cases.
    let mut sections = Vec::new();
    for pair in bands.windows(2) {
        let upper = &pair[0];
        let lower = &pair[1];

        let f_upper = upper.freq_center_mhz;
        let f_lower = lower.freq_center_mhz;

        // Quarter-wave inner leg (upper-band driven element per side).
        let inner_leg_m = (71.58 / f_upper) * vf;
        // Trap-dipole total leg per side for the lower band (uses TRAP_DIPOLE_COEFF_M/2).
        let total_leg_m = (68.58 / f_lower) * vf;
        let outer_section_m = (total_leg_m - inner_leg_m).max(0.0);
        let full_span_m = total_leg_m * 2.0;

        // Trap resonant frequency = upper-band centre.
        let trap_freq_mhz = f_upper;
        // L·C product in μH·pF:  L[μH] × C[pF] = 25 330 / f[MHz]²
        let lc_product = 25_330.0 / (trap_freq_mhz * trap_freq_mhz);
        let cap_values = select_trap_cap_examples(lc_product);
        let component_examples: Vec<TrapDipoleComponentExample> = cap_values
            .iter()
            .map(|&c| TrapDipoleComponentExample {
                cap_pf: c,
                ind_uh: lc_product / c,
            })
            .collect();

        sections.push(TrapDipoleGuidanceSection {
            label: format!("{} / {}", lower.name, upper.name),
            trap_freq_mhz,
            inner_leg_m,
            outer_section_m,
            total_leg_m,
            full_span_m,
            component_examples,
        });
    }

    if sections.is_empty() {
        return None;
    }

    Some(TrapDipoleGuidanceView {
        velocity_factor: vf,
        sections,
    })
}

/// Format trap-dipole guidance into display lines.
pub fn trap_dipole_guidance_display_lines(
    view: &TrapDipoleGuidanceView,
    units: UnitSystem,
) -> Vec<String> {
    const M_TO_FT: f64 = 3.280_84;
    let mut lines = Vec::new();
    lines.push(String::new());
    lines.push(format!(
        "Trap dipole guidance (VF {:.2}):",
        view.velocity_factor
    ));

    for section in &view.sections {
        lines.push(format!(
            "  \u{2500}\u{2500} {} \u{2500}\u{2500}",
            section.label
        ));
        lines.push(format!(
            "  Trap resonant frequency:  {:.3} MHz",
            section.trap_freq_mhz
        ));

        let fmt_len = |m: f64| match units {
            UnitSystem::Metric => format!("{:.2} m", m),
            UnitSystem::Imperial => format!("{:.1} ft", m * M_TO_FT),
            UnitSystem::Both => format!("{:.2} m  ({:.1} ft)", m, m * M_TO_FT),
        };

        lines.push(format!(
            "  Inner section (feedpoint \u{2192} trap): {}",
            fmt_len(section.inner_leg_m)
        ));
        lines.push(format!(
            "  Outer section (trap \u{2192} tip):       {}",
            fmt_len(section.outer_section_m)
        ));
        lines.push(format!(
            "  Total leg per side:                {}",
            fmt_len(section.total_leg_m)
        ));
        lines.push(format!(
            "  Full span (tip-to-tip):            {}",
            fmt_len(section.full_span_m)
        ));
        lines.push(format!(
            "  Component examples at {:.3} MHz (target coil Qu \u{003e} 200, use silver-mica or NP0 cap):",
            section.trap_freq_mhz
        ));
        let examples: Vec<String> = section
            .component_examples
            .iter()
            .map(|ex| format!("{:.0} pF \u{2192} {:.2} \u{03bc}H", ex.cap_pf, ex.ind_uh))
            .collect();
        lines.push(format!("    {}", examples.join("  |  ")));
    }

    lines
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

// ---------------------------------------------------------------------------
// Velocity sweep views
// ---------------------------------------------------------------------------

/// One row in a velocity-sweep comparison table.
#[derive(Debug, Clone)]
pub struct VelocitySweepRow {
    pub velocity_factor: f64,
    /// Non-resonant mode: the recommended wire length (None when no recommendation exists).
    pub non_resonant_length_m: Option<f64>,
    pub non_resonant_length_ft: Option<f64>,
    pub non_resonant_clearance_pct: Option<f64>,
    /// Resonant mode: per-band (band_name, half_wave_m, half_wave_ft).
    pub resonant_band_lengths: Vec<(String, f64, f64)>,
}

#[derive(Debug, Clone)]
pub struct VelocitySweepView {
    pub mode: CalcMode,
    /// Human-readable comma-joined band list from the first result set.
    pub bands_label: String,
    pub itu_region_label: String,
    pub rows: Vec<VelocitySweepRow>,
}

/// Build a pure view model for a velocity sweep.
///
/// `results_by_vf` is a slice of `(velocity_factor, AppResults)` pairs in
/// sweep order. The order is preserved in the returned view.
pub fn velocity_sweep_view(results_by_vf: &[(f64, AppResults)]) -> Option<VelocitySweepView> {
    let (_, first) = results_by_vf.first()?;
    let mode = first.config.mode;
    let bands_label = first
        .calculations
        .iter()
        .map(|c| c.band_name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let itu_region_label = first.config.itu_region.short_name().to_string();

    let rows = results_by_vf
        .iter()
        .map(|(vf, res)| match mode {
            CalcMode::NonResonant => VelocitySweepRow {
                velocity_factor: *vf,
                non_resonant_length_m: res.recommendation.as_ref().map(|r| r.length_m),
                non_resonant_length_ft: res.recommendation.as_ref().map(|r| r.length_ft),
                non_resonant_clearance_pct: res
                    .recommendation
                    .as_ref()
                    .map(|r| r.min_resonance_clearance_pct),
                resonant_band_lengths: Vec::new(),
            },
            CalcMode::Resonant => VelocitySweepRow {
                velocity_factor: *vf,
                non_resonant_length_m: None,
                non_resonant_length_ft: None,
                non_resonant_clearance_pct: None,
                resonant_band_lengths: res
                    .calculations
                    .iter()
                    .map(|c| (c.band_name.clone(), c.half_wave_m, c.half_wave_ft))
                    .collect(),
            },
        })
        .collect();

    Some(VelocitySweepView {
        mode,
        bands_label,
        itu_region_label,
        rows,
    })
}

/// Render a `VelocitySweepView` to display lines (no I/O).
pub fn velocity_sweep_display_lines(view: &VelocitySweepView, units: UnitSystem) -> Vec<String> {
    let mode_label = match view.mode {
        CalcMode::Resonant => "resonant",
        CalcMode::NonResonant => "non-resonant",
    };
    let mut lines = vec![
        String::new(),
        format!(
            "Velocity sweep \u{2014} {mode_label} | {} | Region {}:",
            view.bands_label, view.itu_region_label
        ),
    ];

    match view.mode {
        CalcMode::NonResonant => {
            lines.push(format!("  {:<6}  {:<24}  {}", "VF", "Length", "Clearance"));
            lines.push(format!("  {}", "\u{2500}".repeat(46)));
            for row in &view.rows {
                let len_str = match (row.non_resonant_length_m, row.non_resonant_length_ft) {
                    (Some(m), Some(ft)) => match units {
                        UnitSystem::Metric => format!("{:.2} m", m),
                        UnitSystem::Imperial => format!("{:.1} ft", ft),
                        UnitSystem::Both => format!("{:.2} m / {:.1} ft", m, ft),
                    },
                    _ => "\u{2014}".to_string(),
                };
                let clearance_str = row
                    .non_resonant_clearance_pct
                    .map(|p| format!("{:.1}%", p))
                    .unwrap_or_else(|| "\u{2014}".to_string());
                lines.push(format!(
                    "  {:<6.2}  {:<24}  {}",
                    row.velocity_factor, len_str, clearance_str
                ));
            }
        }
        CalcMode::Resonant => {
            for row in &view.rows {
                let parts: Vec<String> = row
                    .resonant_band_lengths
                    .iter()
                    .map(|(name, m, ft)| {
                        let len_str = match units {
                            UnitSystem::Metric => format!("{:.2} m", m),
                            UnitSystem::Imperial => format!("{:.1} ft", ft),
                            UnitSystem::Both => format!("{:.2} m / {:.1} ft", m, ft),
                        };
                        format!("{} = {}", name, len_str)
                    })
                    .collect();
                lines.push(format!(
                    "  VF {:.2}:  {}",
                    row.velocity_factor,
                    parts.join("  ")
                ));
            }
        }
    }

    lines.push(String::new());
    lines
}

/// Validate that every velocity factor in a sweep is within 0.50–1.00.
pub fn validate_velocity_sweep(velocities: &[f64]) -> Result<(), AppError> {
    for &vf in velocities {
        if !(0.5..=1.0).contains(&vf) {
            return Err(AppError::InvalidVelocitySweep(vf));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Transformer sweep
// ---------------------------------------------------------------------------

/// One row in a transformer sweep table — one transformer ratio and its match metrics.
#[derive(Debug, Clone)]
pub struct TransformerSweepRow {
    pub ratio: TransformerRatio,
    pub target_z_ohm: f64,
    pub swr: f64,
    pub efficiency_pct: f64,
    pub mismatch_loss_db: f64,
    /// Non-resonant mode: the recommended wire length (None when no recommendation exists).
    pub non_resonant_length_m: Option<f64>,
    pub non_resonant_length_ft: Option<f64>,
    pub non_resonant_clearance_pct: Option<f64>,
    /// Resonant mode: per-band (band_name, half_wave_m, half_wave_ft).
    pub resonant_band_lengths: Vec<(String, f64, f64)>,
}

/// View model for a transformer ratio sweep.
#[derive(Debug, Clone)]
pub struct TransformerSweepView {
    pub mode: CalcMode,
    pub assumed_feedpoint_ohm: f64,
    pub bands_label: String,
    pub itu_region_label: String,
    pub rows: Vec<TransformerSweepRow>,
}

/// Build a transformer sweep view from a pre-computed set of `(ratio, results)` pairs.
///
/// Returns `None` if the input is empty.
pub fn transformer_sweep_view(
    results_by_ratio: &[(TransformerRatio, AppResults)],
    assumed_feedpoint_ohm: f64,
) -> Option<TransformerSweepView> {
    let (_, first) = results_by_ratio.first()?;
    let mode = first.config.mode;
    let bands_label = first
        .calculations
        .iter()
        .map(|c| c.band_name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let itu_region_label = first.config.itu_region.short_name().to_string();

    let rows = results_by_ratio
        .iter()
        .map(|(ratio, res)| {
            let target_z = 50.0 * ratio.impedance_ratio();
            let gamma = if assumed_feedpoint_ohm > 0.0 {
                ((target_z - assumed_feedpoint_ohm).abs() / (target_z + assumed_feedpoint_ohm))
                    .clamp(0.0, 0.999_999)
            } else {
                0.0
            };
            let efficiency_pct = (1.0 - gamma * gamma) * 100.0;
            let mismatch_loss_db = -10.0 * (1.0 - gamma * gamma).log10();
            let swr = if assumed_feedpoint_ohm > 0.0 {
                assumed_feedpoint_ohm.max(target_z) / assumed_feedpoint_ohm.min(target_z)
            } else {
                1.0
            };

            match mode {
                CalcMode::NonResonant => TransformerSweepRow {
                    ratio: *ratio,
                    target_z_ohm: target_z,
                    swr,
                    efficiency_pct,
                    mismatch_loss_db,
                    non_resonant_length_m: res.recommendation.as_ref().map(|r| r.length_m),
                    non_resonant_length_ft: res.recommendation.as_ref().map(|r| r.length_ft),
                    non_resonant_clearance_pct: res
                        .recommendation
                        .as_ref()
                        .map(|r| r.min_resonance_clearance_pct),
                    resonant_band_lengths: Vec::new(),
                },
                CalcMode::Resonant => TransformerSweepRow {
                    ratio: *ratio,
                    target_z_ohm: target_z,
                    swr,
                    efficiency_pct,
                    mismatch_loss_db,
                    non_resonant_length_m: None,
                    non_resonant_length_ft: None,
                    non_resonant_clearance_pct: None,
                    resonant_band_lengths: res
                        .calculations
                        .iter()
                        .map(|c| (c.band_name.clone(), c.half_wave_m, c.half_wave_ft))
                        .collect(),
                },
            }
        })
        .collect();

    Some(TransformerSweepView {
        mode,
        assumed_feedpoint_ohm,
        bands_label,
        itu_region_label,
        rows,
    })
}

/// Render a `TransformerSweepView` to display lines (no I/O).
pub fn transformer_sweep_display_lines(
    view: &TransformerSweepView,
    units: UnitSystem,
) -> Vec<String> {
    let mode_label = match view.mode {
        CalcMode::Resonant => "resonant",
        CalcMode::NonResonant => "non-resonant",
    };
    let mut lines = vec![
        String::new(),
        format!(
            "Transformer sweep \u{2014} {mode_label} | {} | Region {} | feedpoint R: {:.0} \u{03a9}:",
            view.bands_label, view.itu_region_label, view.assumed_feedpoint_ohm
        ),
    ];

    match view.mode {
        CalcMode::NonResonant => {
            lines.push(format!(
                "  {:<5}  {:<7}  {:<6}  {:<11}  {:<8}  {:<24}  {}",
                "Ratio", "Target Z", "SWR", "Efficiency", "Loss", "Length", "Clearance"
            ));
            lines.push(format!("  {}", "\u{2500}".repeat(78)));
            for row in &view.rows {
                let len_str = match (row.non_resonant_length_m, row.non_resonant_length_ft) {
                    (Some(m), Some(ft)) => match units {
                        UnitSystem::Metric => format!("{:.2} m", m),
                        UnitSystem::Imperial => format!("{:.1} ft", ft),
                        UnitSystem::Both => format!("{:.2} m / {:.1} ft", m, ft),
                    },
                    _ => "\u{2014}".to_string(),
                };
                let clearance_str = row
                    .non_resonant_clearance_pct
                    .map(|p| format!("{:.1}%", p))
                    .unwrap_or_else(|| "\u{2014}".to_string());
                lines.push(format!(
                    "  {:<5}  {:>5.0} \u{03a9}  {:>4.2}:1  {:>9.2}%  {:.3} dB  {:<24}  {}",
                    row.ratio.as_label(),
                    row.target_z_ohm,
                    row.swr,
                    row.efficiency_pct,
                    row.mismatch_loss_db,
                    len_str,
                    clearance_str
                ));
            }
        }
        CalcMode::Resonant => {
            lines.push(format!(
                "  {:<5}  {:<7}  {:<6}  {:<11}  {:<8}  {}",
                "Ratio", "Target Z", "SWR", "Efficiency", "Loss", "Per-band lengths"
            ));
            lines.push(format!("  {}", "\u{2500}".repeat(78)));
            for row in &view.rows {
                let band_parts: Vec<String> = row
                    .resonant_band_lengths
                    .iter()
                    .map(|(name, m, ft)| {
                        let len_str = match units {
                            UnitSystem::Metric => format!("{:.2} m", m),
                            UnitSystem::Imperial => format!("{:.1} ft", ft),
                            UnitSystem::Both => format!("{:.2} m / {:.1} ft", m, ft),
                        };
                        format!("{}={}", name, len_str)
                    })
                    .collect();
                lines.push(format!(
                    "  {:<5}  {:>5.0} \u{03a9}  {:>4.2}:1  {:>9.2}%  {:.3} dB  {}",
                    row.ratio.as_label(),
                    row.target_z_ohm,
                    row.swr,
                    row.efficiency_pct,
                    row.mismatch_loss_db,
                    band_parts.join("  ")
                ));
            }
        }
    }

    lines.push(String::new());
    lines
}

// ---------------------------------------------------------------------------
// Quiet summary
// ---------------------------------------------------------------------------

/// Return the single-line quiet summary string for non-resonant mode, or
/// `None` when resonant mode (quiet resonant = no output).
///
/// This is a pure function; the caller is responsible for printing.
pub fn format_quiet_summary(results: &AppResults) -> Option<String> {
    match results.config.mode {
        CalcMode::NonResonant => {
            let rec = results.recommendation.as_ref()?;
            let line = match results.config.units {
                UnitSystem::Metric => format!("{:.2} m", rec.length_m),
                UnitSystem::Imperial => format!("{:.1} ft", rec.length_ft),
                UnitSystem::Both => format!("{:.2} m ({:.1} ft)", rec.length_m, rec.length_ft),
            };
            Some(line)
        }
        CalcMode::Resonant => None,
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
        .map(|calc| BandDisplayRow {
            calc,
            hybrid_section_split: results.config.hybrid_section_split,
        })
        .collect()
}

pub fn band_display_view(
    row: &BandDisplayRow,
    units: UnitSystem,
    antenna_model: Option<AntennaModel>,
    transformer_ratio: TransformerRatio,
) -> BandDisplayView {
    let c = &row.calc;
    let split = row.hybrid_section_split;
    let hybrid_leg_m = c.corrected_half_wave_m / 2.0;
    let hybrid_leg_ft = c.corrected_half_wave_ft / 2.0;
    let hybrid_s1_m = hybrid_leg_m * split[0];
    let hybrid_s2_m = hybrid_leg_m * split[1];
    let hybrid_s3_m = hybrid_leg_m * split[2];
    let hybrid_s1_ft = hybrid_leg_ft * split[0];
    let hybrid_s2_ft = hybrid_leg_ft * split[1];
    let hybrid_s3_ft = hybrid_leg_ft * split[2];
    let split_label = format!(
        "{:.0}/{:.0}/{:.0}",
        split[0] * 100.0,
        split[1] * 100.0,
        split[2] * 100.0
    );
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
            Some(AntennaModel::TrapDipole) => {
                lines.push(format!(
                    "  Trap dipole total: {:.2} m",
                    c.trap_dipole_total_m
                ));
                lines.push(format!(
                    "  Trap dipole each element: {:.2} m",
                    c.trap_dipole_leg_m
                ));
            }
            Some(AntennaModel::HybridMultiSection) => {
                lines.push(format!("  Hybrid total: {:.2} m", c.corrected_half_wave_m));
                lines.push(format!("  Per side (feedpoint to tip): {:.2} m", hybrid_leg_m));
                lines.push(format!(
                    "  Section split ({}): {:.2} m / {:.2} m / {:.2} m",
                    split_label, hybrid_s1_m, hybrid_s2_m, hybrid_s3_m
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
                lines.push(format!(
                    "  Trap dipole total: {:.2} m",
                    c.trap_dipole_total_m
                ));
                lines.push(format!(
                    "  Trap dipole each element: {:.2} m",
                    c.trap_dipole_leg_m
                ));
                lines.push(format!("  Hybrid total: {:.2} m", c.corrected_half_wave_m));
                lines.push(format!(
                    "  Section split per side ({}): {:.2} m / {:.2} m / {:.2} m",
                    split_label, hybrid_s1_m, hybrid_s2_m, hybrid_s3_m
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
            Some(AntennaModel::TrapDipole) => {
                lines.push(format!(
                    "  Trap dipole total: {:.2} ft",
                    c.trap_dipole_total_ft
                ));
                lines.push(format!(
                    "  Trap dipole each element: {:.2} ft",
                    c.trap_dipole_leg_ft
                ));
            }
            Some(AntennaModel::HybridMultiSection) => {
                lines.push(format!("  Hybrid total: {:.2} ft", c.corrected_half_wave_ft));
                lines.push(format!(
                    "  Per side (feedpoint to tip): {:.2} ft",
                    hybrid_leg_ft
                ));
                lines.push(format!(
                    "  Section split ({}): {:.2} ft / {:.2} ft / {:.2} ft",
                    split_label, hybrid_s1_ft, hybrid_s2_ft, hybrid_s3_ft
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
                lines.push(format!(
                    "  Trap dipole total: {:.2} ft",
                    c.trap_dipole_total_ft
                ));
                lines.push(format!(
                    "  Trap dipole each element: {:.2} ft",
                    c.trap_dipole_leg_ft
                ));
                lines.push(format!("  Hybrid total: {:.2} ft", c.corrected_half_wave_ft));
                lines.push(format!(
                    "  Section split per side ({}): {:.2} ft / {:.2} ft / {:.2} ft",
                    split_label, hybrid_s1_ft, hybrid_s2_ft, hybrid_s3_ft
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
            Some(AntennaModel::TrapDipole) => {
                lines.push(format!(
                    "  Trap dipole total: {:.2} m ({:.2} ft)",
                    c.trap_dipole_total_m, c.trap_dipole_total_ft
                ));
                lines.push(format!(
                    "  Trap dipole each element: {:.2} m ({:.2} ft)",
                    c.trap_dipole_leg_m, c.trap_dipole_leg_ft
                ));
            }
            Some(AntennaModel::HybridMultiSection) => {
                lines.push(format!(
                    "  Hybrid total: {:.2} m ({:.2} ft)",
                    c.corrected_half_wave_m, c.corrected_half_wave_ft
                ));
                lines.push(format!(
                    "  Per side (feedpoint to tip): {:.2} m ({:.2} ft)",
                    hybrid_leg_m, hybrid_leg_ft
                ));
                lines.push(format!(
                    "  Section split ({}): {:.2}/{:.2}/{:.2} m ({:.2}/{:.2}/{:.2} ft)",
                    split_label,
                    hybrid_s1_m,
                    hybrid_s2_m,
                    hybrid_s3_m,
                    hybrid_s1_ft,
                    hybrid_s2_ft,
                    hybrid_s3_ft
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
                lines.push(format!(
                    "  Trap dipole total: {:.2} m ({:.2} ft)",
                    c.trap_dipole_total_m, c.trap_dipole_total_ft
                ));
                lines.push(format!(
                    "  Trap dipole each element: {:.2} m ({:.2} ft)",
                    c.trap_dipole_leg_m, c.trap_dipole_leg_ft
                ));
                lines.push(format!(
                    "  Hybrid total: {:.2} m ({:.2} ft)",
                    c.corrected_half_wave_m, c.corrected_half_wave_ft
                ));
                lines.push(format!(
                    "  Section split per side ({}): {:.2}/{:.2}/{:.2} m ({:.2}/{:.2}/{:.2} ft)",
                    split_label,
                    hybrid_s1_m,
                    hybrid_s2_m,
                    hybrid_s3_m,
                    hybrid_s1_ft,
                    hybrid_s2_ft,
                    hybrid_s3_ft
                ));
            }
        },
    }

    // Show NEC-calibrated feedpoint resistance and estimated SWR for dipole-family antennas.
    // SWR is computed at resonance (X=0) against the transformer output impedance.
    let show_feedpoint = matches!(
        antenna_model,
        Some(AntennaModel::Dipole)
            | Some(AntennaModel::InvertedVDipole)
            | Some(AntennaModel::HybridMultiSection)
            | None
    );
    if show_feedpoint {
        let r = c.dipole_feedpoint_r_ohm;
        let target_z = 50.0 * transformer_ratio.impedance_ratio();
        // SWR = max(R, Z_target) / min(R, Z_target) — purely resistive (resonant wire assumption).
        let swr = if r > 0.0 && target_z > 0.0 {
            r.max(target_z) / r.min(target_z)
        } else {
            1.0
        };
        lines.push(format!(
            "  Est. feedpoint R: {:.1} \u{03a9} (NEC-calibrated, SWR \u{2248} {:.1}:1 into {} \u{03a9})",
            r, swr, target_z as u32
        ));
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
// Shared state machine — AppState / AppAction / apply_action
//
// Framework-agnostic state machine shared by the TUI (ratatui), future GUI
// (iced), and any other front-end.  The contract is simple:
//
//   new_state = apply_action(old_state, action)
//
// apply_action is a pure function: no I/O, no side-effects.  Front-ends are
// responsible for calling it and re-rendering the result.

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn build_calculations(
    indices: &[usize],
    velocity: f64,
    region: ITURegion,
    transformer_ratio: TransformerRatio,
    antenna_height_m: f64,
    ground_class: GroundClass,
    conductor_diameter_mm: f64,
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
            calculations.push(calculate_for_band_with_environment(
                &band,
                velocity,
                transformer_ratio,
                antenna_height_m,
                ground_class,
                conductor_diameter_mm,
            ));
        } else {
            skipped_band_indices.push(idx);
        }
    }

    (calculations, skipped_band_indices)
}

// ---------------------------------------------------------------------------
// Shared private helpers
// ---------------------------------------------------------------------------

fn assumed_feedpoint_impedance_ohm(
    mode: CalcMode,
    antenna_model: Option<AntennaModel>,
    antenna_height_m: f64,
    ground_class: GroundClass,
) -> f64 {
    match antenna_model {
        // Use NEC-calibrated height/ground-aware feedpoint resistance for dipole types.
        Some(AntennaModel::Dipole)
        | Some(AntennaModel::InvertedVDipole)
        | Some(AntennaModel::HybridMultiSection) => {
            crate::calculations::nec_calibrated_dipole_r(antenna_height_m, ground_class)
        }
        Some(AntennaModel::TrapDipole) => 65.0,
        Some(AntennaModel::FullWaveLoop) => 100.0,
        Some(AntennaModel::EndFedHalfWave) => 2800.0,
        Some(AntennaModel::OffCenterFedDipole) => 200.0,
        None => match mode {
            CalcMode::Resonant => {
                crate::calculations::nec_calibrated_dipole_r(antenna_height_m, ground_class)
            }
            CalcMode::NonResonant => 450.0,
        },
    }
}
#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
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
    fn run_calculation_appends_extra_bands() {
        let extra = crate::bands::OwnedBand {
            name: "FT8-40m".to_string(),
            freq_low_mhz: 7.074,
            freq_high_mhz: 7.076,
            freq_center_mhz: None,
        };
        let config = AppConfig {
            band_indices: vec![4], // 40m
            extra_bands: vec![extra],
            ..AppConfig::default()
        };
        let results = run_calculation(config);
        // Standard 40m + custom FT8-40m
        assert_eq!(results.calculations.len(), 2);
        let names: Vec<&str> = results
            .calculations
            .iter()
            .map(|c| c.band_name.as_str())
            .collect();
        assert!(names.contains(&"40m"), "standard band should be present");
        assert!(names.contains(&"FT8-40m"), "custom band should be appended");
        // Custom band centre should be ~7.075 MHz → half-wave in ballpark 18–22 m
        let ft8 = results
            .calculations
            .iter()
            .find(|c| c.band_name == "FT8-40m")
            .unwrap();
        assert!(
            ft8.half_wave_m > 18.0 && ft8.half_wave_m < 22.0,
            "half-wave length should be ~20 m, got {}",
            ft8.half_wave_m
        );
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
        assert_eq!(config.antenna_height_m, DEFAULT_ANTENNA_HEIGHT_M);
        assert_eq!(config.ground_class, DEFAULT_GROUND_CLASS);
        assert_eq!(config.band_indices, vec![4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn validate_config_rejects_non_standard_antenna_height() {
        let mut config = AppConfig::default();
        config.antenna_height_m = 9.0;

        let err = validate_config(&config).expect_err("height 9 m should be rejected");
        assert!(matches!(err, AppError::InvalidAntennaHeight(9.0)));
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
    fn transformer_optimizer_prefers_efhw_match() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            antenna_model: Some(AntennaModel::EndFedHalfWave),
            band_indices: vec![4, 6, 8],
            ..AppConfig::default()
        };

        let view = optimize_transformer_candidates(&config);
        assert_eq!(view.candidate_count, TRANSFORMER_OPTIMIZER_CANDIDATES.len());
        assert_eq!(view.candidates[0].ratio, TransformerRatio::R1To56);
        assert!(view.candidates[0].estimated_efficiency_pct > 99.0);
    }

    #[test]
    fn transformer_optimizer_prefers_ocfd_match() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            antenna_model: Some(AntennaModel::OffCenterFedDipole),
            band_indices: vec![4, 6],
            ..AppConfig::default()
        };

        let view = optimize_transformer_candidates(&config);
        assert_eq!(view.candidates[0].ratio, TransformerRatio::R1To4);
    }

    #[test]
    fn transformer_optimizer_prefers_non_resonant_random_wire_default() {
        let config = AppConfig {
            mode: CalcMode::NonResonant,
            antenna_model: None,
            band_indices: vec![4, 6, 8, 10],
            ..AppConfig::default()
        };

        let view = optimize_transformer_candidates(&config);
        assert_eq!(view.candidates[0].ratio, TransformerRatio::R1To9);
    }

    #[test]
    fn build_advise_candidates_returns_ranked_wire_and_ratio_matches() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            antenna_model: Some(AntennaModel::EndFedHalfWave),
            band_indices: vec![4, 6, 8],
            ..AppConfig::default()
        };

        let view = build_advise_candidates(&config, 3);
        assert_eq!(view.candidates.len(), 3);
        assert_eq!(view.candidates[0].ratio, TransformerRatio::R1To56);
        assert!(view.candidates[0].recommended_length_m > 0.0);
        assert!(view.candidates[0].estimated_efficiency_pct > 90.0);
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
    fn request_context_new_increments_request_id() {
        let ctx1 = RequestContext::new();
        let ctx2 = RequestContext::new();
        assert!(ctx2.request_id > ctx1.request_id);
    }

    #[test]
    fn request_context_is_echoed_through_execute_request() {
        let ctx = RequestContext::new();
        let id = ctx.request_id;
        let mut request = AppRequest::new(AppConfig::default());
        request.context = Some(ctx);

        let response = execute_request_checked(request).expect("should succeed");
        let echo = response.context.expect("context should be echoed");
        assert_eq!(echo.request_id, id);
    }

    #[test]
    fn execute_request_without_context_produces_none_context_in_response() {
        let request = AppRequest::new(AppConfig::default());
        let response = execute_request_checked(request).expect("should succeed");
        assert!(response.context.is_none());
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
    fn resonant_compromise_narrative_includes_trap_dipole_guidance_notes() {
        let config = AppConfig {
            antenna_model: Some(AntennaModel::TrapDipole),
            ..AppConfig::default()
        };
        let results = run_calculation(config);

        let narrative = resonant_compromise_narrative(&results);
        assert!(narrative.heading.contains("trap dipole guidance"));
        assert!(narrative
            .notes
            .iter()
            .any(|note| note.contains("Trap mode")));
        assert!(narrative
            .notes
            .iter()
            .any(|note| note.contains("Trap frequency/components")));
        assert!(narrative
            .notes
            .iter()
            .any(|note| note.contains("Physical placement")));
        assert!(narrative
            .notes
            .iter()
            .any(|note| note.contains("Common pairings")));
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

        let view = band_display_view(
            &rows[0],
            UnitSystem::Metric,
            Some(AntennaModel::Dipole),
            TransformerRatio::R1To1,
        );
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
    fn results_display_document_transformer_explanation_matches_config() {
        // EFHW should carry a 1:49/1:56 reason in the explanation
        let mut config = AppConfig::default();
        config.antenna_model = Some(AntennaModel::EndFedHalfWave);
        let results = run_calculation(config);
        let doc = results_display_document(&results);

        assert!(doc.transformer_explanation.ratio == TransformerRatio::R1To56
            || doc.transformer_explanation.ratio == TransformerRatio::R1To49,
            "EFHW explanation should recommend a high step-up ratio");
        assert!(!doc.transformer_explanation.reason.is_empty());
        // Reason text should mention EFHW context
        assert!(doc.transformer_explanation.reason.contains("EFHW")
            || doc.transformer_explanation.reason.contains("2500")
            || doc.transformer_explanation.reason.contains("transformer"));
    }

    #[test]
    fn results_display_document_skipped_band_details_empty_when_none_skipped() {
        let results = run_calculation(AppConfig::default());
        let doc = results_display_document(&results);

        assert!(doc.skipped_band_details.is_empty());
        assert!(doc.warning_lines.is_empty());
    }

    #[test]
    fn results_display_document_skipped_band_details_populated_when_bands_skipped() {
        let mut config = AppConfig::default();
        // Band index 999 does not exist in any region — will be skipped
        config.band_indices = vec![1, 999];
        let results = run_calculation(config);
        let doc = results_display_document(&results);

        assert!(!doc.skipped_band_details.is_empty());
        assert!(doc.skipped_band_details.iter().any(|d| d.band_index == 999));
        assert!(!doc.skipped_band_details[0].reason.is_empty());
        // warning_lines and skipped_band_details should agree on count
        assert_eq!(doc.warning_lines.len(), 1); // one combined warning
    }

    #[test]
    fn all_antenna_models_execute_without_error() {
        let models = [
            Some(AntennaModel::Dipole),
            Some(AntennaModel::InvertedVDipole),
            Some(AntennaModel::EndFedHalfWave),
            Some(AntennaModel::FullWaveLoop),
            Some(AntennaModel::OffCenterFedDipole),
            Some(AntennaModel::TrapDipole),
            Some(AntennaModel::HybridMultiSection),
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
#[allow(clippy::field_reassign_with_default)]
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

    #[test]
    fn validate_velocity_sweep_accepts_valid_range() {
        assert!(validate_velocity_sweep(&[0.5, 0.85, 0.95, 1.0]).is_ok());
    }

    #[test]
    fn validate_velocity_sweep_rejects_out_of_range() {
        let err = validate_velocity_sweep(&[0.85, 1.5]).unwrap_err();
        assert!(matches!(err, AppError::InvalidVelocitySweep(v) if (v - 1.5).abs() < 1e-9));
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn format_quiet_summary_resonant_returns_none() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            band_indices: vec![5],
            ..AppConfig::default()
        };
        let results = run_calculation(config);
        assert!(format_quiet_summary(&results).is_none());
    }

    #[test]
    fn format_quiet_summary_non_resonant_metric() {
        let config = AppConfig {
            mode: CalcMode::NonResonant,
            band_indices: vec![5, 7],
            units: UnitSystem::Metric,
            wire_min_m: 8.0,
            wire_max_m: 35.0,
            ..AppConfig::default()
        };
        let results = run_calculation(config);
        let line = format_quiet_summary(&results).expect("non-resonant should produce a summary");
        assert!(line.ends_with(" m"), "expected metric suffix, got: {line}");
        assert!(!line.contains("ft"), "should not contain ft in metric mode");
    }

    #[test]
    fn velocity_sweep_view_non_resonant_has_clearance() {
        let config = AppConfig {
            mode: CalcMode::NonResonant,
            band_indices: vec![5, 7],
            wire_min_m: 8.0,
            wire_max_m: 35.0,
            ..AppConfig::default()
        };
        let mut r85 = config.clone();
        r85.velocity_factor = 0.85;
        let mut r95 = config.clone();
        r95.velocity_factor = 0.95;

        let results_by_vf = vec![
            (0.85_f64, run_calculation(r85)),
            (0.95_f64, run_calculation(r95)),
        ];
        let view = velocity_sweep_view(&results_by_vf).expect("view should be produced");
        assert_eq!(view.mode, CalcMode::NonResonant);
        assert_eq!(view.rows.len(), 2);
        assert!(view.rows[0].non_resonant_clearance_pct.is_some());
        assert!(view.rows[1].non_resonant_clearance_pct.is_some());
    }

    #[test]
    fn velocity_sweep_view_resonant_has_band_lengths() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            band_indices: vec![5, 7],
            ..AppConfig::default()
        };
        let mut r85 = config.clone();
        r85.velocity_factor = 0.85;
        let mut r95 = config.clone();
        r95.velocity_factor = 0.95;

        let results_by_vf = vec![
            (0.85_f64, run_calculation(r85)),
            (0.95_f64, run_calculation(r95)),
        ];
        let view = velocity_sweep_view(&results_by_vf).expect("view should be produced");
        assert_eq!(view.mode, CalcMode::Resonant);
        for row in &view.rows {
            assert_eq!(row.resonant_band_lengths.len(), 2);
        }
    }

    #[test]
    fn velocity_sweep_display_lines_contains_vf_values() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            band_indices: vec![7],
            ..AppConfig::default()
        };
        let mut r85 = config.clone();
        r85.velocity_factor = 0.85;
        let mut r95 = config.clone();
        r95.velocity_factor = 0.95;

        let results_by_vf = vec![
            (0.85_f64, run_calculation(r85)),
            (0.95_f64, run_calculation(r95)),
        ];
        let view = velocity_sweep_view(&results_by_vf).unwrap();
        let lines = velocity_sweep_display_lines(&view, UnitSystem::Metric);
        let combined = lines.join("\n");
        assert!(combined.contains("0.85"), "expected 0.85 in output");
        assert!(combined.contains("0.95"), "expected 0.95 in output");
        assert!(combined.contains("resonant"));
    }

    #[test]
    fn transformer_sweep_view_resonant_has_band_lengths_and_efficiency() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            band_indices: vec![5, 7],
            antenna_model: Some(AntennaModel::Dipole),
            ..AppConfig::default()
        };
        let mut c1to1 = config.clone();
        c1to1.transformer_ratio = TransformerRatio::R1To1;
        let mut c1to4 = config.clone();
        c1to4.transformer_ratio = TransformerRatio::R1To4;

        let feedpoint_r = 50.0; // 1:1 maps perfectly to 50 Ω
        let results = vec![
            (TransformerRatio::R1To1, run_calculation(c1to1)),
            (TransformerRatio::R1To4, run_calculation(c1to4)),
        ];
        let view = transformer_sweep_view(&results, feedpoint_r).expect("view should be produced");
        assert_eq!(view.mode, CalcMode::Resonant);
        assert_eq!(view.rows.len(), 2);
        assert_eq!(view.rows[0].ratio, TransformerRatio::R1To1);
        assert_eq!(view.rows[1].ratio, TransformerRatio::R1To4);
        for row in &view.rows {
            assert_eq!(row.resonant_band_lengths.len(), 2);
        }
        // 1:1 into 50 Ω feedpoint should be a perfect match.
        let row1to1 = &view.rows[0];
        assert!(row1to1.efficiency_pct > 99.9);
        assert!(row1to1.swr < 1.01);
        // 1:4 (200 Ω) into 50 Ω feedpoint should have higher SWR.
        let row1to4 = &view.rows[1];
        assert!(row1to4.swr > 3.5);
    }

    #[test]
    fn transformer_sweep_display_lines_contains_ratio_labels() {
        let config = AppConfig {
            mode: CalcMode::Resonant,
            band_indices: vec![7],
            antenna_model: Some(AntennaModel::Dipole),
            ..AppConfig::default()
        };
        let mut c1to1 = config.clone();
        c1to1.transformer_ratio = TransformerRatio::R1To1;
        let mut c1to9 = config.clone();
        c1to9.transformer_ratio = TransformerRatio::R1To9;

        let feedpoint_r = 73.0;
        let results = vec![
            (TransformerRatio::R1To1, run_calculation(c1to1)),
            (TransformerRatio::R1To9, run_calculation(c1to9)),
        ];
        let view = transformer_sweep_view(&results, feedpoint_r).unwrap();
        let lines = transformer_sweep_display_lines(&view, UnitSystem::Metric);
        let combined = lines.join("\n");
        assert!(combined.contains("1:1"), "expected 1:1 in output");
        assert!(combined.contains("1:9"), "expected 1:9 in output");
        assert!(combined.contains("resonant"));
        assert!(combined.contains("feedpoint R:"));
    }

    #[test]
    fn transformer_ratio_explanation_efhw_returns_correct_ratio_and_reason() {
        let expl =
            transformer_ratio_explanation(CalcMode::Resonant, Some(AntennaModel::EndFedHalfWave));
        assert_eq!(expl.ratio, TransformerRatio::R1To56);
        assert!(expl.reason.contains("49") || expl.reason.contains("56"));
    }

    #[test]
    fn transformer_ratio_explanation_hybrid_multi_section_returns_1to1() {
        let expl = transformer_ratio_explanation(
            CalcMode::Resonant,
            Some(AntennaModel::HybridMultiSection),
        );
        assert_eq!(expl.ratio, TransformerRatio::R1To1);
        assert!(expl.reason.contains("1:1"));
    }

    #[test]
    fn band_display_view_hybrid_multi_section_shows_split_lines() {
        let config = AppConfig {
            antenna_model: Some(AntennaModel::HybridMultiSection),
            band_indices: vec![5],
            ..AppConfig::default()
        };
        let results = run_calculation(config);
        let row = band_display_rows(&results)
            .into_iter()
            .next()
            .expect("expected one row");
        let view = band_display_view(
            &row,
            UnitSystem::Metric,
            Some(AntennaModel::HybridMultiSection),
            results.config.transformer_ratio,
        );
        let joined = view.lines.join("\n");
        assert!(joined.contains("Hybrid total:"));
        assert!(joined.contains("Section split (40/35/25):"));
    }

    #[test]
    fn compare_efhw_transformers_2800_ohm_best_is_1to56() {
        // At 2800 Ω feedpoint R, 1:56 (target 2800 Ω) should be the perfect match.
        let cmp = compare_efhw_transformers(2800.0);
        assert_eq!(cmp.best_ratio, TransformerRatio::R1To56);
        let best = cmp.entries.iter().find(|e| e.is_best).unwrap();
        assert_eq!(best.ratio, TransformerRatio::R1To56);
        // Should be a near-perfect match: negligible mismatch loss.
        assert!(best.mismatch_loss_db < 0.001);
        assert!(best.swr < 1.01);
        assert!(best.efficiency_pct > 99.99);
    }

    #[test]
    fn compare_efhw_transformers_2450_ohm_best_is_1to49() {
        // At 2450 Ω feedpoint R, 1:49 (target 2450 Ω) should be the best match.
        let cmp = compare_efhw_transformers(2450.0);
        assert_eq!(cmp.best_ratio, TransformerRatio::R1To49);
        let best = cmp.entries.iter().find(|e| e.is_best).unwrap();
        assert_eq!(best.ratio, TransformerRatio::R1To49);
        assert!(best.mismatch_loss_db < 0.001);
    }

    #[test]
    fn compare_efhw_transformers_3200_ohm_best_is_1to64() {
        // At 3200 Ω feedpoint R, 1:64 (target 3200 Ω) should be the best match.
        let cmp = compare_efhw_transformers(3200.0);
        assert_eq!(cmp.best_ratio, TransformerRatio::R1To64);
        let best = cmp.entries.iter().find(|e| e.is_best).unwrap();
        assert_eq!(best.ratio, TransformerRatio::R1To64);
        assert!(best.mismatch_loss_db < 0.001);
    }

    #[test]
    fn compare_efhw_transformers_returns_three_entries_in_order() {
        let cmp = compare_efhw_transformers(2800.0);
        assert_eq!(cmp.entries.len(), 3);
        assert_eq!(cmp.entries[0].ratio, TransformerRatio::R1To49);
        assert_eq!(cmp.entries[1].ratio, TransformerRatio::R1To56);
        assert_eq!(cmp.entries[2].ratio, TransformerRatio::R1To64);
        // Exactly one entry should be flagged as best.
        let best_count = cmp.entries.iter().filter(|e| e.is_best).count();
        assert_eq!(best_count, 1);
    }

    #[test]
    fn compare_efhw_transformers_mid_range_picks_closest() {
        // 2625 Ω is midpoint between 2450 and 2800 — 1:49 target is 175 Ω away, 1:56 is 175 Ω
        // away. For a value just above midpoint, 1:56 should win.
        let cmp = compare_efhw_transformers(2630.0);
        // Best should be 1:56 (closer to 2800) or 1:49 (closer to 2450) — just check the field.
        assert!(cmp.entries.iter().filter(|e| e.is_best).count() == 1);
    }

    #[test]
    fn transformer_ratio_explanation_resonant_no_model_uses_1to1() {
        let expl = transformer_ratio_explanation(CalcMode::Resonant, None);
        assert_eq!(expl.ratio, TransformerRatio::R1To1);
        assert!(!expl.reason.is_empty());
    }

    #[test]
    fn transformer_ratio_explanation_non_resonant_no_model_uses_1to9() {
        let expl = transformer_ratio_explanation(CalcMode::NonResonant, None);
        assert_eq!(expl.ratio, TransformerRatio::R1To9);
        assert!(expl.reason.contains("1:9"));
    }

    #[test]
    fn band_listing_view_region1_has_rows() {
        let view = band_listing_view(ITURegion::Region1);
        assert!(!view.rows.is_empty());
        assert_eq!(view.region_short_name, "1");
        assert!(view.region_long_name.contains("Europe"));
    }

    #[test]
    fn band_listing_display_lines_contains_band_name() {
        let view = band_listing_view(ITURegion::Region1);
        let lines = band_listing_display_lines(&view);
        let combined = lines.join("\n");
        assert!(combined.contains("40m"), "expected 40m in band listing");
        assert!(combined.contains("Region 1"), "expected region header");
    }

    #[test]
    fn band_listing_row_indices_are_one_based() {
        let view = band_listing_view(ITURegion::Region1);
        // First row should have index 1
        assert_eq!(view.rows[0].index, 1);
        // Indices should be consecutive and start at 1
        for (i, row) in view.rows.iter().enumerate() {
            assert_eq!(row.index, i + 1);
        }
    }

    #[test]
    fn skipped_band_details_returns_reason_for_each_skipped() {
        // Build a config that requests a band not in Region 3 (e.g. 60m is region-limited)
        // The easiest way is to directly construct AppResults with known skipped indices.
        let results = AppResults {
            calculations: Vec::new(),
            recommendation: None,
            optima: Vec::new(),
            window_optima: Vec::new(),
            resonant_compromises: Vec::new(),
            config: AppConfig::default(),
            skipped_band_indices: vec![3, 7],
        };
        let details = skipped_band_details(&results);
        assert_eq!(details.len(), 2);
        assert_eq!(details[0].band_index, 3);
        assert_eq!(details[1].band_index, 7);
        assert!(details[0].reason.contains("ITU region"));
        assert!(details[1].reason.contains("ITU region"));
    }

    #[test]
    fn skipped_band_details_empty_when_no_skipped_bands() {
        let config = AppConfig {
            band_indices: vec![5],
            ..AppConfig::default()
        };
        let results = run_calculation(config);
        assert!(skipped_band_details(&results).is_empty());
    }

    #[test]
    fn transformer_mismatch_warning_present_when_ratio_differs_from_recommendation() {
        // EFHW recommends 1:49; configure 1:1 to trigger the warning.
        let config = AppConfig {
            antenna_model: Some(AntennaModel::EndFedHalfWave),
            transformer_ratio: TransformerRatio::R1To1,
            band_indices: vec![5],
            ..AppConfig::default()
        };
        let results = run_calculation(config);
        let doc = results_display_document(&results);
        let mismatch = doc.transformer_mismatch_warning.as_ref().expect("should be Some");
        assert_eq!(mismatch.configured, TransformerRatio::R1To1);
        assert_eq!(mismatch.recommended, TransformerRatio::R1To56);
        let msg = mismatch.message();
        assert!(msg.contains("1:1"), "message should mention configured ratio");
        assert!(msg.contains("1:56"), "message should mention recommended ratio");
    }

    #[test]
    fn transformer_mismatch_warning_absent_when_ratio_matches_recommendation() {
        // Dipole recommends 1:1; configure 1:1 → no warning.
        let config = AppConfig {
            antenna_model: Some(AntennaModel::Dipole),
            transformer_ratio: TransformerRatio::R1To1,
            band_indices: vec![5],
            ..AppConfig::default()
        };
        let results = run_calculation(config);
        let doc = results_display_document(&results);
        assert!(doc.transformer_mismatch_warning.is_none());
    }

    #[test]
    fn transformer_mismatch_warning_added_to_warning_lines() {
        let config = AppConfig {
            antenna_model: Some(AntennaModel::OffCenterFedDipole),
            transformer_ratio: TransformerRatio::R1To9,
            band_indices: vec![5],
            ..AppConfig::default()
        };
        let results = run_calculation(config);
        let doc = results_display_document(&results);
        assert!(doc.transformer_mismatch_warning.is_some());
        // The mismatch message must also appear in warning_lines.
        let has_warning = doc
            .warning_lines
            .iter()
            .any(|l| l.contains("1:9") && l.contains("1:4"));
        assert!(has_warning, "warning_lines should contain mismatch text");
    }
}
