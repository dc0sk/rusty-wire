/// Wire length calculations for resonant dipoles and related measurements
use crate::bands::Band;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformerRatio {
    R1To1,
    R1To2,
    R1To4,
    R1To5,
    R1To6,
    R1To9,
    R1To16,
    R1To49,
    R1To56,
    R1To64,
}

impl TransformerRatio {
    pub fn as_label(self) -> &'static str {
        match self {
            TransformerRatio::R1To1 => "1:1",
            TransformerRatio::R1To2 => "1:2",
            TransformerRatio::R1To4 => "1:4",
            TransformerRatio::R1To5 => "1:5",
            TransformerRatio::R1To6 => "1:6",
            TransformerRatio::R1To9 => "1:9",
            TransformerRatio::R1To16 => "1:16",
            TransformerRatio::R1To49 => "1:49",
            TransformerRatio::R1To56 => "1:56",
            TransformerRatio::R1To64 => "1:64",
        }
    }

    pub fn impedance_ratio(self) -> f64 {
        match self {
            TransformerRatio::R1To1 => 1.0,
            TransformerRatio::R1To2 => 2.0,
            TransformerRatio::R1To4 => 4.0,
            TransformerRatio::R1To5 => 5.0,
            TransformerRatio::R1To6 => 6.0,
            TransformerRatio::R1To9 => 9.0,
            TransformerRatio::R1To16 => 16.0,
            TransformerRatio::R1To49 => 49.0,
            TransformerRatio::R1To56 => 56.0,
            TransformerRatio::R1To64 => 64.0,
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "1:1" | "1" => Some(TransformerRatio::R1To1),
            "1:2" | "2" => Some(TransformerRatio::R1To2),
            "1:4" | "4" => Some(TransformerRatio::R1To4),
            "1:5" | "5" => Some(TransformerRatio::R1To5),
            "1:6" | "6" => Some(TransformerRatio::R1To6),
            "1:9" | "9" => Some(TransformerRatio::R1To9),
            "1:16" | "16" => Some(TransformerRatio::R1To16),
            "1:49" | "49" => Some(TransformerRatio::R1To49),
            "1:56" | "56" => Some(TransformerRatio::R1To56),
            "1:64" | "64" => Some(TransformerRatio::R1To64),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WireCalculation {
    pub band_name: String,
    pub frequency_mhz: f64,
    pub transformer_ratio_label: &'static str,
    
    // Dipole lengths (in meters)
    pub half_wave_m: f64,
    pub full_wave_m: f64,
    pub quarter_wave_m: f64,
    
    // Dipole lengths (in feet)
    pub half_wave_ft: f64,
    pub full_wave_ft: f64,
    pub quarter_wave_ft: f64,

    // Impedance-corrected lengths for selected transformer ratio
    pub corrected_half_wave_m: f64,
    pub corrected_full_wave_m: f64,
    pub corrected_quarter_wave_m: f64,
    pub corrected_half_wave_ft: f64,
    pub corrected_full_wave_ft: f64,
    pub corrected_quarter_wave_ft: f64,
    
    // Skip distances
    pub skip_distance_min_km: f64,
    pub skip_distance_max_km: f64,
    pub skip_distance_avg_km: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct NonResonantRecommendation {
    pub length_m: f64,
    pub length_ft: f64,
    // Minimum distance from nearest quarter-wave harmonic resonance, in percent.
    pub min_resonance_clearance_pct: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct ResonantCompromise {
    pub length_m: f64,
    pub length_ft: f64,
    // Worst-case distance to nearest resonant point among selected bands.
    pub worst_band_distance_m: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct NonResonantSearchConfig {
    pub min_len_m: f64,
    pub max_len_m: f64,
    pub step_m: f64,
    pub preferred_center_m: f64,
}

pub const DEFAULT_NON_RESONANT_CONFIG: NonResonantSearchConfig = NonResonantSearchConfig {
    min_len_m: 8.0,
    max_len_m: 35.0,
    step_m: 0.05,
    preferred_center_m: 20.0,
};

impl fmt::Display for WireCalculation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  Half-wave total: {:.2}m ({:.2}ft) [base: {:.2}m ({:.2}ft)]\n  Full-wave total: {:.2}m ({:.2}ft) [base: {:.2}m ({:.2}ft)]\n  Quarter-wave: {:.2}m ({:.2}ft) [base: {:.2}m ({:.2}ft)]\n  Skip distance: {:.0}-{:.0}km (avg: {:.0}km)",
            self.band_name,
            self.frequency_mhz,
            self.transformer_ratio_label,
            self.corrected_half_wave_m,
            self.corrected_half_wave_ft,
            self.half_wave_m,
            self.half_wave_ft,
            self.corrected_full_wave_m,
            self.corrected_full_wave_ft,
            self.full_wave_m,
            self.full_wave_ft,
            self.corrected_quarter_wave_m,
            self.corrected_quarter_wave_ft,
            self.quarter_wave_m,
            self.quarter_wave_ft,
            self.skip_distance_min_km,
            self.skip_distance_max_km,
            self.skip_distance_avg_km,
        )
    }
}

const METERS_TO_FEET: f64 = 3.28084;

/// Calculate resonant dipole wire lengths for a given frequency
/// 
/// Using the standard formulas:
/// - Half-wave dipole (feet): 468 / frequency_MHz
/// - Full-wave dipole (feet): 936 / frequency_MHz
/// - Quarter-wave (feet): 234 / frequency_MHz
pub fn calculate_for_band_with_velocity(
    band: &Band,
    velocity_factor: f64,
    transformer: TransformerRatio,
) -> WireCalculation {
    let freq = band.freq_center_mhz;

    // The constants are in feet; apply velocity factor in feet, then convert to meters.
    let half_wave_ft = (468.0 / freq) * velocity_factor;
    let full_wave_ft = (936.0 / freq) * velocity_factor;
    let quarter_wave_ft = (234.0 / freq) * velocity_factor;

    let half_wave_m = half_wave_ft / METERS_TO_FEET;
    let full_wave_m = full_wave_ft / METERS_TO_FEET;
    let quarter_wave_m = quarter_wave_ft / METERS_TO_FEET;

    // Use a shared nominal feedpoint reference so transformer selection has a
    // consistent impact across resonant families and optimization behavior.
    let corrected_half_wave_ft = impedance_corrected_length_ft(half_wave_ft, 73.0, transformer);
    let corrected_full_wave_ft = impedance_corrected_length_ft(full_wave_ft, 73.0, transformer);
    let corrected_quarter_wave_ft = impedance_corrected_length_ft(quarter_wave_ft, 73.0, transformer);
    let corrected_half_wave_m = corrected_half_wave_ft / METERS_TO_FEET;
    let corrected_full_wave_m = corrected_full_wave_ft / METERS_TO_FEET;
    let corrected_quarter_wave_m = corrected_quarter_wave_ft / METERS_TO_FEET;
    
    // Calculate skip distance average
    let skip_distance_avg_km =
        (band.typical_skip_km.0 + band.typical_skip_km.1) / 2.0;
    
    WireCalculation {
        band_name: band.name.to_string(),
        frequency_mhz: freq,
        transformer_ratio_label: transformer.as_label(),
        half_wave_m,
        full_wave_m,
        quarter_wave_m,
        half_wave_ft,
        full_wave_ft,
        quarter_wave_ft,
        corrected_half_wave_m,
        corrected_full_wave_m,
        corrected_quarter_wave_m,
        corrected_half_wave_ft,
        corrected_full_wave_ft,
        corrected_quarter_wave_ft,
        skip_distance_min_km: band.typical_skip_km.0,
        skip_distance_max_km: band.typical_skip_km.1,
        skip_distance_avg_km,
    }
}

fn impedance_corrected_length_ft(base_len_ft: f64, nominal_feedpoint_ohm: f64, transformer: TransformerRatio) -> f64 {
    if transformer == TransformerRatio::R1To1 {
        return base_len_ft;
    }

    let target_antenna_ohm = 50.0 * transformer.impedance_ratio();
    let ratio = (target_antenna_ohm / nominal_feedpoint_ohm).max(0.01);

    // Heuristic correction: small logarithmic shift around resonance, bounded to practical limits.
    let correction = (1.0 + 0.03 * ratio.log10()).clamp(0.85, 1.15);
    base_len_ft * correction
}

/// Calculate the most distant reachable distance by averaging skip distances
pub fn calculate_average_max_distance(calculations: &[WireCalculation]) -> f64 {
    if calculations.is_empty() {
        return 0.0;
    }
    
    let sum: f64 = calculations.iter().map(|c| c.skip_distance_max_km).sum();
    sum / calculations.len() as f64
}

/// Calculate the minimum reachable distance by averaging
pub fn calculate_average_min_distance(calculations: &[WireCalculation]) -> f64 {
    if calculations.is_empty() {
        return 0.0;
    }
    
    let sum: f64 = calculations.iter().map(|c| c.skip_distance_min_km).sum();
    sum / calculations.len() as f64
}

/// Find a practical non-resonant random-wire length for the given selected bands.
///
/// Method:
/// - Build quarter-wave harmonic resonance points for each selected band's center frequency
/// - Search candidate wire lengths and maximize the minimum distance to any resonance point
pub fn calculate_non_resonant_optima(
    calculations: &[WireCalculation],
    velocity_factor: f64,
    config: NonResonantSearchConfig,
) -> Vec<NonResonantRecommendation> {
    if calculations.is_empty() {
        return Vec::new();
    }

    if config.min_len_m <= 0.0
        || config.max_len_m <= config.min_len_m
        || config.step_m <= 0.0
    {
        return Vec::new();
    }

    let min_len_m = config.min_len_m;
    let max_len_m = config.max_len_m;
    let step_m = config.step_m;

    let mut resonance_points_m = Vec::new();
    for c in calculations {
        // Use transformer-corrected quarter-wave as the base resonance point so
        // optimum common wire length reflects the selected Unun/Balun ratio.
        let quarter_wave_m = c.corrected_quarter_wave_m;

        let mut harmonic = 1_u32;
        loop {
            let resonant_len_m = quarter_wave_m * f64::from(harmonic);
            if resonant_len_m > max_len_m + 1.0 {
                break;
            }
            if resonant_len_m >= min_len_m - 1.0 {
                resonance_points_m.push(resonant_len_m);
            }
            harmonic += 1;
        }
    }

    // Keep API stability while calculations now consume corrected per-band values.
    let _ = velocity_factor;

    if resonance_points_m.is_empty() {
        return Vec::new();
    }

    let mut candidates: Vec<(f64, f64)> = Vec::new();
    let mut best_clearance_m = -1.0_f64;
    let mut len = min_len_m;

    while len <= max_len_m {
        let nearest = resonance_points_m
            .iter()
            .map(|r| (len - r).abs())
            .fold(f64::INFINITY, f64::min);

        if nearest > best_clearance_m + 1e-9 {
            best_clearance_m = nearest;
            candidates.clear();
            candidates.push((len, nearest));
        } else if (nearest - best_clearance_m).abs() < 1e-9 {
            candidates.push((len, nearest));
        }

        len += step_m;
    }

    candidates
        .into_iter()
        .map(|(best_len_m, clearance_m)| NonResonantRecommendation {
            length_m: best_len_m,
            length_ft: best_len_m * METERS_TO_FEET,
            min_resonance_clearance_pct: (clearance_m / best_len_m) * 100.0,
        })
        .collect()
}

pub fn calculate_best_non_resonant_length(
    calculations: &[WireCalculation],
    velocity_factor: f64,
    config: NonResonantSearchConfig,
) -> Option<NonResonantRecommendation> {
    let preferred_center_m = config.preferred_center_m;
    let optima = calculate_non_resonant_optima(calculations, velocity_factor, config);
    if optima.is_empty() {
        return None;
    }

    optima.into_iter().min_by(|a, b| {
        let ad = (a.length_m - preferred_center_m).abs();
        let bd = (b.length_m - preferred_center_m).abs();
        ad.partial_cmp(&bd).unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// Find compromise wire lengths that are as close as possible to resonant points
/// across all selected bands within the active search window.
///
/// Objective:
/// - For each candidate length in the window, compute distance to nearest
///   in-window resonant point per band.
/// - Minimize the worst per-band distance.
/// - Return all equal optima (within tolerance) in ascending order.
pub fn calculate_resonant_compromises(
    calculations: &[WireCalculation],
    config: NonResonantSearchConfig,
) -> Vec<ResonantCompromise> {
    if calculations.is_empty() {
        return Vec::new();
    }
    if config.min_len_m <= 0.0
        || config.max_len_m <= config.min_len_m
        || config.step_m <= 0.0
    {
        return Vec::new();
    }

    let min_len_m = config.min_len_m;
    let max_len_m = config.max_len_m;
    let step_m = config.step_m;

    let mut band_points: Vec<Vec<f64>> = Vec::new();
    for calc in calculations {
        let quarter_wave_m = calc.corrected_quarter_wave_m;
        if quarter_wave_m <= 0.0 {
            continue;
        }

        let mut points = Vec::new();
        let mut harmonic = 1_u32;
        loop {
            let resonant_len_m = quarter_wave_m * f64::from(harmonic);
            if resonant_len_m > max_len_m + 1e-9 {
                break;
            }
            if resonant_len_m >= min_len_m - 1e-9 {
                points.push(resonant_len_m);
            }
            harmonic += 1;
        }

        if !points.is_empty() {
            band_points.push(points);
        }
    }

    if band_points.is_empty() {
        return Vec::new();
    }

    let mut samples: Vec<(f64, f64)> = Vec::new();
    let mut len = min_len_m;
    while len <= max_len_m + 1e-9 {
        let mut worst_distance = 0.0_f64;
        for points in &band_points {
            let nearest = points
                .iter()
                .map(|p| (len - p).abs())
                .fold(f64::INFINITY, f64::min);
            if nearest > worst_distance {
                worst_distance = nearest;
            }
        }
        samples.push((len, worst_distance));
        len += step_m;
    }

    if samples.is_empty() {
        return Vec::new();
    }

    let mut local_minima: Vec<(f64, f64)> = Vec::new();

    if samples.len() == 1 {
        local_minima.push(samples[0]);
    } else {
        if samples[0].1 <= samples[1].1 {
            local_minima.push(samples[0]);
        }
        for i in 1..(samples.len() - 1) {
            let prev = samples[i - 1].1;
            let curr = samples[i].1;
            let next = samples[i + 1].1;
            if (curr <= prev && curr <= next) && (curr < prev || curr < next) {
                local_minima.push(samples[i]);
            }
        }
        if samples[samples.len() - 1].1 <= samples[samples.len() - 2].1 {
            local_minima.push(samples[samples.len() - 1]);
        }
    }

    if local_minima.is_empty() {
        if let Some(global_best) = samples.iter().cloned().min_by(|a, b| {
            a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            local_minima.push(global_best);
        }
    }

    let best_worst_distance_m = local_minima
        .iter()
        .map(|(_, d)| *d)
        .fold(f64::INFINITY, f64::min);

    // Keep nearby local minima so users can see practical alternates
    // around repeated resonant alignment points (e.g. ~10/20/30m).
    let keep_threshold = (best_worst_distance_m * 3.0).max(best_worst_distance_m + 0.05);

    let mut winners: Vec<(f64, f64)> = local_minima
        .into_iter()
        .filter(|(_, d)| *d <= keep_threshold + 1e-9)
        .collect();

    winners.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    if winners.len() > 20 {
        winners.truncate(20);
    }

    winners
        .into_iter()
        .map(|(length_m, worst_band_distance_m)| ResonantCompromise {
            length_m,
            length_ft: length_m * METERS_TO_FEET,
            worst_band_distance_m,
        })
        .collect()
}
