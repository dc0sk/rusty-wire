/// Wire length calculations for resonant dipoles and related measurements
use crate::bands::Band;
use std::fmt;

#[derive(Debug, Clone)]
pub struct WireCalculation {
    pub band_name: String,
    pub frequency_mhz: f64,
    
    // Dipole lengths (in meters)
    pub half_wave_m: f64,
    pub full_wave_m: f64,
    pub quarter_wave_m: f64,
    
    // Dipole lengths (in feet)
    pub half_wave_ft: f64,
    pub full_wave_ft: f64,
    pub quarter_wave_ft: f64,
    
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
            "{}\n  Frequency: {:.3} MHz\n  Half-wave total: {:.2}m ({:.2}ft)\n  Full-wave total: {:.2}m ({:.2}ft)\n  Quarter-wave: {:.2}m ({:.2}ft)\n  Skip distance: {:.0}-{:.0}km (avg: {:.0}km)",
            self.band_name,
            self.frequency_mhz,
            self.half_wave_m,
            self.half_wave_ft,
            self.full_wave_m,
            self.full_wave_ft,
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
) -> WireCalculation {
    let freq = band.freq_center_mhz;

    // The constants are in feet; apply velocity factor in feet, then convert to meters.
    let half_wave_ft = (468.0 / freq) * velocity_factor;
    let full_wave_ft = (936.0 / freq) * velocity_factor;
    let quarter_wave_ft = (234.0 / freq) * velocity_factor;

    let half_wave_m = half_wave_ft / METERS_TO_FEET;
    let full_wave_m = full_wave_ft / METERS_TO_FEET;
    let quarter_wave_m = quarter_wave_ft / METERS_TO_FEET;
    
    // Calculate skip distance average
    let skip_distance_avg_km =
        (band.typical_skip_km.0 + band.typical_skip_km.1) / 2.0;
    
    WireCalculation {
        band_name: band.name.to_string(),
        frequency_mhz: freq,
        half_wave_m,
        full_wave_m,
        quarter_wave_m,
        half_wave_ft,
        full_wave_ft,
        quarter_wave_ft,
        skip_distance_min_km: band.typical_skip_km.0,
        skip_distance_max_km: band.typical_skip_km.1,
        skip_distance_avg_km,
    }
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
        // Wavelength in meters from frequency in MHz.
        let wavelength_m = 300.0 / c.frequency_mhz;
        let quarter_wave_m = (wavelength_m / 4.0) * velocity_factor;

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
