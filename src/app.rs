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
    calculate_resonant_compromises,
    NonResonantRecommendation, NonResonantSearchConfig, ResonantCompromise,
    TransformerRatio, WireCalculation, DEFAULT_NON_RESONANT_CONFIG,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Json,
    Markdown,
    Txt,
}

impl ExportFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            ExportFormat::Markdown => "markdown",
            ExportFormat::Txt => "txt",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitSystem {
    Metric,
    Imperial,
    Both,
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
}
