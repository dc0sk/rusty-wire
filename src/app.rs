/// Core application types, configuration, and computation entry point.
///
/// This module is the primary API surface for both the CLI front-end and any
/// future GUI (e.g. iced).  It is deliberately free of I/O; nothing here reads
/// from stdin or writes to stdout/stderr except for the `eprintln!` that reports
/// skipped invalid band indices.
use crate::bands::{get_band_by_index_for_region, ITURegion};
use crate::calculations::{
    calculate_best_non_resonant_length, calculate_for_band_with_velocity,
    calculate_non_resonant_optima, calculate_non_resonant_window_optima,
    calculate_resonant_compromises, NonResonantRecommendation, NonResonantSearchConfig,
    ResonantCompromise, TransformerRatio, WireCalculation, DEFAULT_NON_RESONANT_CONFIG,
};
use clap::ValueEnum;
use std::str::FromStr;

pub const FEET_TO_METERS: f64 = 0.3048;
pub const DEFAULT_BAND_SELECTION: [usize; 7] = [4, 5, 6, 7, 8, 9, 10];
pub const DEFAULT_ITU_REGION: ITURegion = ITURegion::Region1;
pub const DEFAULT_TRANSFORMER_RATIO: TransformerRatio = TransformerRatio::R1To1;

// ---------------------------------------------------------------------------
// Shared enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcMode {
    Resonant,
    NonResonant,
}

impl FromStr for CalcMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "resonant" => Ok(CalcMode::Resonant),
            "non-resonant" | "nonresonant" | "non_resonant" => Ok(CalcMode::NonResonant),
            _ => Err(format!(
                "Invalid calculation mode '{}'. Must be 'resonant' or 'non-resonant'.",
                s
            )),
        }
    }
}

impl ValueEnum for CalcMode {
    fn value_variants<'a>() -> &'a [Self] {
        &[CalcMode::Resonant, CalcMode::NonResonant]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            CalcMode::Resonant => Some(
                clap::builder::PossibleValue::new("resonant")
                    .help("Calculate resonant wire lengths"),
            ),
            CalcMode::NonResonant => Some(
                clap::builder::PossibleValue::new("non-resonant")
                    .help("Find optimal non-resonant wire length within constraints"),
            ),
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
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "csv" => Ok(ExportFormat::Csv),
            "json" => Ok(ExportFormat::Json),
            "markdown" | "md" => Ok(ExportFormat::Markdown),
            "txt" | "text" => Ok(ExportFormat::Txt),
            _ => Err(format!(
                "Invalid export format '{}'. Must be 'csv', 'json', 'markdown', or 'txt'.",
                s
            )),
        }
    }
}

impl ValueEnum for ExportFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            ExportFormat::Csv,
            ExportFormat::Json,
            ExportFormat::Markdown,
            ExportFormat::Txt,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            ExportFormat::Csv => Some(clap::builder::PossibleValue::new("csv").help("CSV format")),
            ExportFormat::Json => {
                Some(clap::builder::PossibleValue::new("json").help("JSON format"))
            }
            ExportFormat::Markdown => {
                Some(clap::builder::PossibleValue::new("markdown").help("Markdown format"))
            }
            ExportFormat::Txt => {
                Some(clap::builder::PossibleValue::new("txt").help("Plain text format"))
            }
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
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "m" | "metric" => Ok(UnitSystem::Metric),
            "ft" | "imperial" => Ok(UnitSystem::Imperial),
            "both" => Ok(UnitSystem::Both),
            _ => Err(format!(
                "Invalid unit system '{}'. Must be 'm', 'ft', or 'both'.",
                s
            )),
        }
    }
}

impl ValueEnum for UnitSystem {
    fn value_variants<'a>() -> &'a [Self] {
        &[UnitSystem::Metric, UnitSystem::Imperial, UnitSystem::Both]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            UnitSystem::Metric => Some(clap::builder::PossibleValue::new("m").help("Meters")),
            UnitSystem::Imperial => Some(clap::builder::PossibleValue::new("ft").help("Feet")),
            UnitSystem::Both => {
                Some(clap::builder::PossibleValue::new("both").help("Both meters and feet"))
            }
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
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "dipole" => Ok(AntennaModel::Dipole),
            "inverted-v" | "inv-v" | "invertedv" | "invv" => Ok(AntennaModel::InvertedVDipole),
            "efhw" | "end-fed" | "end-fed-half-wave" => Ok(AntennaModel::EndFedHalfWave),
            "loop" | "full-wave-loop" => Ok(AntennaModel::FullWaveLoop),
            "ocfd" | "off-center-fed" | "off-center-fed-dipole" | "windom" => {
                Ok(AntennaModel::OffCenterFedDipole)
            }
            _ => Err(format!(
                "Invalid antenna model '{}'. Must be 'dipole', 'inverted-v', 'efhw', 'loop', or 'ocfd'.",
                s
            )),
        }
    }
}

impl ValueEnum for AntennaModel {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            AntennaModel::Dipole,
            AntennaModel::InvertedVDipole,
            AntennaModel::EndFedHalfWave,
            AntennaModel::FullWaveLoop,
            AntennaModel::OffCenterFedDipole,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            AntennaModel::Dipole => {
                Some(clap::builder::PossibleValue::new("dipole").help("Center-fed dipole model"))
            }
            AntennaModel::InvertedVDipole => Some(
                clap::builder::PossibleValue::new("inverted-v").help("Inverted-V dipole model"),
            ),
            AntennaModel::EndFedHalfWave => {
                Some(clap::builder::PossibleValue::new("efhw").help("End-fed half-wave model"))
            }
            AntennaModel::FullWaveLoop => {
                Some(clap::builder::PossibleValue::new("loop").help("Full-wave loop model"))
            }
            AntennaModel::OffCenterFedDipole => Some(
                clap::builder::PossibleValue::new("ocfd")
                    .help("Off-center-fed dipole (Windom) model"),
            ),
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
    pub units: UnitSystem,
    pub itu_region: ITURegion,
    pub transformer_ratio: TransformerRatio,
    pub antenna_model: Option<AntennaModel>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            band_indices: DEFAULT_BAND_SELECTION.to_vec(),
            velocity_factor: 0.95,
            mode: CalcMode::Resonant,
            wire_min_m: DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            wire_max_m: DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            units: UnitSystem::Both,
            itu_region: DEFAULT_ITU_REGION,
            transformer_ratio: DEFAULT_TRANSFORMER_RATIO,
            antenna_model: None,
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

// ---------------------------------------------------------------------------
// Public computation API
// ---------------------------------------------------------------------------

/// Run all wire calculations for the given configuration.
///
/// This is a pure, I/O-free function suitable for use from both the CLI and
/// any future GUI front-end.
pub fn run_calculation(config: AppConfig) -> AppResults {
    let (calculations, skipped_band_indices) = build_calculations(
        &config.band_indices,
        config.velocity_factor,
        config.itu_region,
        config.transformer_ratio,
    );

    // For resonant mode use the default search window; for non-resonant use the
    // user-supplied window.  Optima (tied candidates) are only relevant in
    // non-resonant mode.
    let non_res_cfg = NonResonantSearchConfig {
        min_len_m: config.wire_min_m,
        max_len_m: config.wire_max_m,
        step_m: DEFAULT_NON_RESONANT_CONFIG.step_m,
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
        let mut config = AppConfig::default();
        config.band_indices = vec![0, 1, 100];
        config.mode = CalcMode::Resonant;

        let results = run_calculation(config);

        assert_eq!(results.calculations.len(), 1);
        assert_eq!(results.calculations[0].band_name, "160m");
        assert_eq!(results.skipped_band_indices, vec![0, 100]);
    }

    #[test]
    fn run_calculation_resonant_mode() {
        let mut config = AppConfig::default();
        config.mode = CalcMode::Resonant;
        config.band_indices = vec![1, 2];

        let results = run_calculation(config);

        assert_eq!(results.calculations.len(), 2);
        assert!(results.window_optima.is_empty());
        assert!(results.optima.is_empty());
        assert!(!results.resonant_compromises.is_empty());
    }

    #[test]
    fn run_calculation_non_resonant_mode() {
        let mut config = AppConfig::default();
        config.mode = CalcMode::NonResonant;
        config.band_indices = vec![1, 2];
        config.wire_min_m = 8.0;
        config.wire_max_m = 35.0;

        let results = run_calculation(config);

        assert_eq!(results.calculations.len(), 2);
        assert!(!results.window_optima.is_empty());
        assert!(!results.optima.is_empty());
        assert!(results.resonant_compromises.is_empty());
    }

    #[test]
    fn run_calculation_stores_config() {
        let mut config = AppConfig::default();
        config.velocity_factor = 0.85;
        config.mode = CalcMode::Resonant;

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
            let mut config = AppConfig::default();
            config.itu_region = *region;
            config.band_indices = vec![1, 2, 3];

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
            let mut config = AppConfig::default();
            config.transformer_ratio = *ratio;
            config.band_indices = vec![1];

            let results = run_calculation(config);
            assert_eq!(
                results.calculations[0].transformer_ratio_label,
                ratio.as_label()
            );
        }
    }

    #[test]
    fn run_calculation_velocity_factor_range() {
        for vf in &[0.5, 0.75, 0.95, 1.0] {
            let mut config = AppConfig::default();
            config.velocity_factor = *vf;

            let results = run_calculation(config);
            assert_eq!(results.config.velocity_factor, *vf);
        }
    }
}
