/// Wire length calculations for resonant dipoles and related measurements
use crate::bands::Band;
use clap::ValueEnum;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundClass {
    Poor,
    Average,
    Good,
}

impl GroundClass {
    pub fn as_label(self) -> &'static str {
        match self {
            GroundClass::Poor => "poor",
            GroundClass::Average => "average",
            GroundClass::Good => "good",
        }
    }
}

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

impl FromStr for TransformerRatio {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| {
            format!(
                "Invalid transformer ratio '{s}'. Must be one of: 1:1, 1:2, 1:4, 1:5, 1:6, 1:9, 1:16, 1:49, 1:56, 1:64"
            )
        })
    }
}

impl ValueEnum for TransformerRatio {
    fn value_variants<'a>() -> &'a [Self] {
        &[
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
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_label()))
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

    // First-batch derived antenna model lengths
    pub end_fed_half_wave_m: f64,
    pub end_fed_half_wave_ft: f64,
    pub inverted_v_total_m: f64,
    pub inverted_v_total_ft: f64,
    pub inverted_v_leg_m: f64,
    pub inverted_v_leg_ft: f64,
    pub inverted_v_span_90_m: f64,
    pub inverted_v_span_90_ft: f64,
    pub inverted_v_span_120_m: f64,
    pub inverted_v_span_120_ft: f64,
    pub full_wave_loop_circumference_m: f64,
    pub full_wave_loop_circumference_ft: f64,
    pub full_wave_loop_square_side_m: f64,
    pub full_wave_loop_square_side_ft: f64,
    pub ocfd_33_short_leg_m: f64,
    pub ocfd_33_short_leg_ft: f64,
    pub ocfd_33_long_leg_m: f64,
    pub ocfd_33_long_leg_ft: f64,
    pub ocfd_20_short_leg_m: f64,
    pub ocfd_20_short_leg_ft: f64,
    pub ocfd_20_long_leg_m: f64,
    pub ocfd_20_long_leg_ft: f64,
    pub trap_dipole_total_m: f64,
    pub trap_dipole_total_ft: f64,
    pub trap_dipole_leg_m: f64,
    pub trap_dipole_leg_ft: f64,

    // Skip distances
    pub skip_distance_min_km: f64,
    pub skip_distance_max_km: f64,
    pub skip_distance_avg_km: f64,

    // NEC-calibrated feedpoint resistance for dipole at deployment height and ground.
    // Derived from fnec-rust Hallén solver corpus data (corpus/reference-results.json).
    // Used for transformer-ratio length correction and surfaced in CLI output.
    pub dipole_feedpoint_r_ohm: f64,
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
pub struct OcfdSplitRecommendation {
    pub short_ratio: f64,
    pub long_ratio: f64,
    pub short_leg_m: f64,
    pub short_leg_ft: f64,
    pub long_leg_m: f64,
    pub long_leg_ft: f64,
    pub worst_leg_clearance_pct: f64,
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

pub const DEFAULT_CONDUCTOR_DIAMETER_MM: f64 = 2.0;
pub const MIN_CONDUCTOR_DIAMETER_MM: f64 = 1.0;
pub const MAX_CONDUCTOR_DIAMETER_MM: f64 = 4.0;
pub const CONDUCTOR_DIAMETER_LOG_COEFF: f64 = 0.011_542;
pub const CONDUCTOR_DIAMETER_MIN_FACTOR: f64 = 0.97;
pub const CONDUCTOR_DIAMETER_MAX_FACTOR: f64 = 1.03;

impl fmt::Display for WireCalculation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  Half-wave total: {:.2}m ({:.2}ft) [base: {:.2}m ({:.2}ft)]\n  Full-wave total: {:.2}m ({:.2}ft) [base: {:.2}m ({:.2}ft)]\n  Quarter-wave: {:.2}m ({:.2}ft) [base: {:.2}m ({:.2}ft)]\n  End-fed half-wave: {:.2}m ({:.2}ft)\n  Inverted-V total: {:.2}m ({:.2}ft)\n  Inverted-V each leg: {:.2}m ({:.2}ft)\n  Inverted-V span at 90 deg apex: {:.2}m ({:.2}ft)\n  Inverted-V span at 120 deg apex: {:.2}m ({:.2}ft)\n  Full-wave loop circumference: {:.2}m ({:.2}ft)\n  Full-wave loop square side: {:.2}m ({:.2}ft)\n  OCFD 33/67 legs: {:.2}m/{:.2}m ({:.2}ft/{:.2}ft)\n  OCFD 20/80 legs: {:.2}m/{:.2}m ({:.2}ft/{:.2}ft)\n  Trap dipole total: {:.2}m ({:.2}ft)\n  Trap dipole each element: {:.2}m ({:.2}ft)\n  Skip distance: {:.0}-{:.0}km (avg: {:.0}km)",
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
            self.end_fed_half_wave_m,
            self.end_fed_half_wave_ft,
            self.inverted_v_total_m,
            self.inverted_v_total_ft,
            self.inverted_v_leg_m,
            self.inverted_v_leg_ft,
            self.inverted_v_span_90_m,
            self.inverted_v_span_90_ft,
            self.inverted_v_span_120_m,
            self.inverted_v_span_120_ft,
            self.full_wave_loop_circumference_m,
            self.full_wave_loop_circumference_ft,
            self.full_wave_loop_square_side_m,
            self.full_wave_loop_square_side_ft,
            self.ocfd_33_short_leg_m,
            self.ocfd_33_long_leg_m,
            self.ocfd_33_short_leg_ft,
            self.ocfd_33_long_leg_ft,
            self.ocfd_20_short_leg_m,
            self.ocfd_20_long_leg_m,
            self.ocfd_20_short_leg_ft,
            self.ocfd_20_long_leg_ft,
            self.trap_dipole_total_m,
            self.trap_dipole_total_ft,
            self.trap_dipole_leg_m,
            self.trap_dipole_leg_ft,
            self.skip_distance_min_km,
            self.skip_distance_max_km,
            self.skip_distance_avg_km,
        )
    }
}

const METERS_TO_FEET: f64 = 3.28084;
const HALF_WAVE_COEFF_M: f64 = 142.646_544_192;
const FULL_WAVE_COEFF_M: f64 = 285.293_088_384;
const QUARTER_WAVE_COEFF_M: f64 = 71.323_272_096;
const FULL_LOOP_COEFF_M: f64 = 306.324_462_672;
const TRAP_DIPOLE_COEFF_M: f64 = 137.16;

/// Calculate resonant dipole wire lengths for a given frequency
///
/// Internal policy: compute lengths in meters first, then derive feet values only
/// for imperial output fields.
///
/// Using standard practical formulas in meters:
/// - Half-wave dipole (m): 142.6465 / frequency_MHz
/// - Full-wave dipole (m): 285.2931 / frequency_MHz
/// - Quarter-wave (m): 71.3233 / frequency_MHz
pub fn calculate_for_band_with_velocity(
    band: &Band,
    velocity_factor: f64,
    transformer: TransformerRatio,
    antenna_height_m: f64,
    ground_class: GroundClass,
) -> WireCalculation {
    calculate_for_band_with_environment(
        band,
        velocity_factor,
        transformer,
        antenna_height_m,
        ground_class,
        DEFAULT_CONDUCTOR_DIAMETER_MM,
    )
}

pub fn calculate_for_band_with_environment(
    band: &Band,
    velocity_factor: f64,
    transformer: TransformerRatio,
    antenna_height_m: f64,
    ground_class: GroundClass,
    conductor_diameter_mm: f64,
) -> WireCalculation {
    let freq = band.freq_center_mhz;

    // Metric-first core calculations.
    let half_wave_m = (HALF_WAVE_COEFF_M / freq) * velocity_factor;
    let full_wave_m = (FULL_WAVE_COEFF_M / freq) * velocity_factor;
    let quarter_wave_m = (QUARTER_WAVE_COEFF_M / freq) * velocity_factor;

    // Imperial output fields are derived from metric values.
    let half_wave_ft = half_wave_m * METERS_TO_FEET;
    let full_wave_ft = full_wave_m * METERS_TO_FEET;
    let quarter_wave_ft = quarter_wave_m * METERS_TO_FEET;

    // NEC-calibrated feedpoint resistance: use corpus reference data instead of the
    // classic textbook 73 Ω free-space value. Height and ground affect the radiation
    // resistance through image-method coupling; the NEC values are validated against
    // fnec-rust Hallén solver output (see corpus/reference-results.json).
    let nominal_feedpoint_r = nec_calibrated_dipole_r(antenna_height_m, ground_class);

    // Use the NEC-calibrated nominal for all transformer-ratio length corrections.
    let corrected_half_wave_m = impedance_corrected_length_m(
        half_wave_m,
        nominal_feedpoint_r,
        transformer,
        conductor_diameter_mm,
    );
    let corrected_full_wave_m = impedance_corrected_length_m(
        full_wave_m,
        nominal_feedpoint_r,
        transformer,
        conductor_diameter_mm,
    );
    let corrected_quarter_wave_m = impedance_corrected_length_m(
        quarter_wave_m,
        nominal_feedpoint_r,
        transformer,
        conductor_diameter_mm,
    );
    let corrected_half_wave_ft = corrected_half_wave_m * METERS_TO_FEET;
    let corrected_full_wave_ft = corrected_full_wave_m * METERS_TO_FEET;
    let corrected_quarter_wave_ft = corrected_quarter_wave_m * METERS_TO_FEET;

    let end_fed_half_wave_ft = corrected_half_wave_ft;
    let end_fed_half_wave_m = corrected_half_wave_m;

    // Inverted-V shortening: a drooping dipole resonates at a shorter total wire length
    // than a flat dipole due to capacitive coupling between the sloped legs.
    // ARRL Antenna Book empirical values:
    //   90° apex  → K_90  ≈ 0.97  (~3 % shorter than flat dipole)
    //   120° apex → K_120 ≈ 0.985 (~1.5 % shorter than flat dipole)
    const INV_V_SHORTENING_90: f64 = 0.97;
    const INV_V_SHORTENING_120: f64 = 0.985;
    let inverted_v_total_m = corrected_half_wave_m * INV_V_SHORTENING_90;
    let inverted_v_total_ft = inverted_v_total_m * METERS_TO_FEET;
    let inverted_v_leg_m = inverted_v_total_m / 2.0;
    let inverted_v_leg_ft = inverted_v_leg_m * METERS_TO_FEET;
    let inverted_v_span_90_ft = inverted_v_leg_ft * std::f64::consts::SQRT_2;
    let inverted_v_span_90_m = inverted_v_leg_m * std::f64::consts::SQRT_2;
    // 120° apex span uses its own shortened total so the geometry is consistent
    let inv_v_total_120_m = corrected_half_wave_m * INV_V_SHORTENING_120;
    let inv_v_leg_120_m = inv_v_total_120_m / 2.0;
    let inv_v_leg_120_ft = inv_v_leg_120_m * METERS_TO_FEET;
    let inverted_v_span_120_ft = inv_v_leg_120_ft * 3.0_f64.sqrt();
    let inverted_v_span_120_m = inv_v_leg_120_m * 3.0_f64.sqrt();

    // Full-wave loop: ARRL Antenna Book standard formula is 1005/f (feet).
    // This is ~7 % longer than 2 × half-wave dipole (936/f) because the "end effect"
    // for a closed resonant loop differs from that of open-ended dipole elements.
    let full_wave_loop_circumference_m = (FULL_LOOP_COEFF_M / freq) * velocity_factor;
    let full_wave_loop_circumference_ft = full_wave_loop_circumference_m * METERS_TO_FEET;
    let full_wave_loop_square_side_m = full_wave_loop_circumference_m / 4.0;
    let full_wave_loop_square_side_ft = full_wave_loop_square_side_m * METERS_TO_FEET;

    let ocfd_total_m = corrected_half_wave_m;
    let ocfd_33_short_leg_m = ocfd_total_m / 3.0;
    let ocfd_33_short_leg_ft = ocfd_33_short_leg_m * METERS_TO_FEET;
    let ocfd_33_long_leg_m = ocfd_total_m * 2.0 / 3.0;
    let ocfd_33_long_leg_ft = ocfd_33_long_leg_m * METERS_TO_FEET;
    let ocfd_20_short_leg_m = ocfd_total_m * 0.2;
    let ocfd_20_short_leg_ft = ocfd_20_short_leg_m * METERS_TO_FEET;
    let ocfd_20_long_leg_m = ocfd_total_m * 0.8;
    let ocfd_20_long_leg_ft = ocfd_20_long_leg_m * METERS_TO_FEET;

    let trap_dipole_total_m = (TRAP_DIPOLE_COEFF_M / freq) * velocity_factor;
    let trap_dipole_total_ft = trap_dipole_total_m * METERS_TO_FEET;
    let trap_dipole_leg_m = trap_dipole_total_m / 2.0;
    let trap_dipole_leg_ft = trap_dipole_leg_m * METERS_TO_FEET;

    // Height-aware first-order skip scaling for practical deployment heights.
    let skip_factor = height_skip_factor(antenna_height_m) * ground_skip_factor(ground_class);
    let skip_distance_min_km = band.typical_skip_km.0 * skip_factor;
    let skip_distance_max_km = band.typical_skip_km.1 * skip_factor;
    let skip_distance_avg_km = (skip_distance_min_km + skip_distance_max_km) / 2.0;

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
        end_fed_half_wave_m,
        end_fed_half_wave_ft,
        inverted_v_total_m,
        inverted_v_total_ft,
        inverted_v_leg_m,
        inverted_v_leg_ft,
        inverted_v_span_90_m,
        inverted_v_span_90_ft,
        inverted_v_span_120_m,
        inverted_v_span_120_ft,
        full_wave_loop_circumference_m,
        full_wave_loop_circumference_ft,
        full_wave_loop_square_side_m,
        full_wave_loop_square_side_ft,
        ocfd_33_short_leg_m,
        ocfd_33_short_leg_ft,
        ocfd_33_long_leg_m,
        ocfd_33_long_leg_ft,
        ocfd_20_short_leg_m,
        ocfd_20_short_leg_ft,
        ocfd_20_long_leg_m,
        ocfd_20_long_leg_ft,
        trap_dipole_total_m,
        trap_dipole_total_ft,
        trap_dipole_leg_m,
        trap_dipole_leg_ft,
        skip_distance_min_km,
        skip_distance_max_km,
        skip_distance_avg_km,
        dipole_feedpoint_r_ohm: nominal_feedpoint_r,
    }
}

/// Return the NEC-calibrated radiation resistance (real part of feedpoint impedance)
/// for a half-wave dipole at the given height and ground, derived from fnec-rust
/// Hallén solver corpus sweeps (corpus/reference-results.json, v2.9.0).
///
/// Reference data (7.1 MHz, 2 mm wire, 51 segments, 0.95 VF cut):
///   free space:          R ≈  62.94 Ω
///   7 m AGL, good:       R ≈  73.03 Ω
///   10 m AGL, poor:      R ≈  56.38 Ω
///   10 m AGL, average:   R ≈  54.35 Ω
///   10 m AGL, good:      R ≈  52.84 Ω
///   12 m AGL, good:      R ≈  45.56 Ω
///
/// For heights other than 7/10/12 m the value is linearly interpolated.
/// Ground-class offset at 10 m is applied as a calibrated delta.
pub fn nec_calibrated_dipole_r(antenna_height_m: f64, ground_class: GroundClass) -> f64 {
    // Anchor table: resistance at each height with "good" ground (from NEC corpus).
    const H7_GOOD: f64 = 73.03; // 7 m AGL, good soil
    const H10_GOOD: f64 = 52.84; // 10 m AGL, good soil
    const H12_GOOD: f64 = 45.56; // 12 m AGL, good soil

    // Ground-class correction deltas at 10 m AGL (NEC corpus deltas relative to "good"):
    //   average - good ≈ +1.51 Ω;  poor - good ≈ +3.54 Ω
    let ground_delta: f64 = match ground_class {
        GroundClass::Good => 0.0,
        GroundClass::Average => 1.51,
        GroundClass::Poor => 3.54,
    };

    // Interpolate the height-dependent "good" baseline.
    let h_good = if antenna_height_m <= 7.0 {
        H7_GOOD
    } else if antenna_height_m >= 12.0 {
        H12_GOOD
    } else if antenna_height_m <= 10.0 {
        let t = (antenna_height_m - 7.0) / (10.0 - 7.0);
        H7_GOOD + t * (H10_GOOD - H7_GOOD)
    } else {
        let t = (antenna_height_m - 10.0) / (12.0 - 10.0);
        H10_GOOD + t * (H12_GOOD - H10_GOOD)
    };

    (h_good + ground_delta).clamp(30.0, 90.0)
}

fn height_skip_factor(antenna_height_m: f64) -> f64 {
    // First practical step for "height vs. propagation" realism:
    // lower heights emphasize shorter skip, higher heights allow longer skip.
    // Standardized presets are 7 m, 10 m (baseline), and 12 m.
    if (antenna_height_m - 7.0).abs() < 1e-9 {
        0.78
    } else if (antenna_height_m - 10.0).abs() < 1e-9 {
        1.0
    } else if (antenna_height_m - 12.0).abs() < 1e-9 {
        1.12
    } else {
        // Keep a safe fallback if called outside presets.
        (1.0 + ((antenna_height_m - 10.0) * 0.035)).clamp(0.70, 1.20)
    }
}

fn ground_skip_factor(ground_class: GroundClass) -> f64 {
    match ground_class {
        GroundClass::Poor => 0.88,
        GroundClass::Average => 1.0,
        GroundClass::Good => 1.10,
    }
}

fn impedance_corrected_length_m(
    base_len_m: f64,
    nominal_feedpoint_ohm: f64,
    transformer: TransformerRatio,
    conductor_diameter_mm: f64,
) -> f64 {
    if transformer == TransformerRatio::R1To1 {
        return base_len_m * conductor_diameter_correction_factor(conductor_diameter_mm);
    }

    let target_antenna_ohm = 50.0 * transformer.impedance_ratio();
    let ratio = (target_antenna_ohm / nominal_feedpoint_ohm).max(0.01);

    // Heuristic correction: small logarithmic shift around resonance, bounded to practical limits.
    let correction = (1.0 + 0.03 * ratio.log10()).clamp(0.85, 1.15);
    base_len_m * correction * conductor_diameter_correction_factor(conductor_diameter_mm)
}

fn conductor_diameter_correction_factor(conductor_diameter_mm: f64) -> f64 {
    // First-order geometry term: thicker conductors generally resonate slightly
    // shorter, thinner conductors slightly longer, relative to a 2.0 mm baseline.
    let d_mm = conductor_diameter_mm
        .clamp(MIN_CONDUCTOR_DIAMETER_MM, MAX_CONDUCTOR_DIAMETER_MM)
        .max(0.1);
    (1.0 - CONDUCTOR_DIAMETER_LOG_COEFF * (d_mm / DEFAULT_CONDUCTOR_DIAMETER_MM).ln())
        .clamp(CONDUCTOR_DIAMETER_MIN_FACTOR, CONDUCTOR_DIAMETER_MAX_FACTOR)
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

    if config.min_len_m <= 0.0 || config.max_len_m <= config.min_len_m || config.step_m <= 0.0 {
        return Vec::new();
    }

    let min_len_m = config.min_len_m;
    let max_len_m = config.max_len_m;
    let step_m = config.step_m;

    let resonance_points_m =
        build_non_resonant_resonance_points(calculations, min_len_m, max_len_m);

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

/// Find local non-resonant optima inside the active search window.
///
/// Unlike `calculate_non_resonant_optima` (which keeps only equal global
/// winners), this returns all local maxima of resonance clearance so users can
/// inspect multiple practical candidates within the current window.
pub fn calculate_non_resonant_window_optima(
    calculations: &[WireCalculation],
    velocity_factor: f64,
    config: NonResonantSearchConfig,
) -> Vec<NonResonantRecommendation> {
    if calculations.is_empty() {
        return Vec::new();
    }

    if config.min_len_m <= 0.0 || config.max_len_m <= config.min_len_m || config.step_m <= 0.0 {
        return Vec::new();
    }

    let min_len_m = config.min_len_m;
    let max_len_m = config.max_len_m;
    let step_m = config.step_m;

    let resonance_points_m =
        build_non_resonant_resonance_points(calculations, min_len_m, max_len_m);

    // Keep API symmetry with non-resonant calculations.
    let _ = velocity_factor;

    if resonance_points_m.is_empty() {
        return Vec::new();
    }

    let mut samples: Vec<(f64, f64)> = Vec::new();
    let mut len = min_len_m;
    while len <= max_len_m + 1e-9 {
        let nearest = resonance_points_m
            .iter()
            .map(|r| (len - r).abs())
            .fold(f64::INFINITY, f64::min);
        samples.push((len, nearest));
        len += step_m;
    }

    if samples.is_empty() {
        return Vec::new();
    }

    let mut local_maxima: Vec<(f64, f64)> = Vec::new();

    if samples.len() == 1 {
        local_maxima.push(samples[0]);
    } else {
        if samples[0].1 >= samples[1].1 {
            local_maxima.push(samples[0]);
        }
        for i in 1..(samples.len() - 1) {
            let prev = samples[i - 1].1;
            let curr = samples[i].1;
            let next = samples[i + 1].1;
            if (curr >= prev && curr >= next) && (curr > prev || curr > next) {
                local_maxima.push(samples[i]);
            }
        }
        if samples[samples.len() - 1].1 >= samples[samples.len() - 2].1 {
            local_maxima.push(samples[samples.len() - 1]);
        }
    }

    if local_maxima.is_empty() {
        if let Some(global_best) = samples
            .iter()
            .cloned()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            local_maxima.push(global_best);
        }
    }

    local_maxima.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    if local_maxima.len() > 30 {
        local_maxima.truncate(30);
    }

    local_maxima
        .into_iter()
        .map(|(length_m, clearance_m)| NonResonantRecommendation {
            length_m,
            length_ft: length_m * METERS_TO_FEET,
            min_resonance_clearance_pct: (clearance_m / length_m) * 100.0,
        })
        .collect()
}

fn build_non_resonant_resonance_points(
    calculations: &[WireCalculation],
    min_len_m: f64,
    max_len_m: f64,
) -> Vec<f64> {
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
    resonance_points_m
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
    if config.min_len_m <= 0.0 || config.max_len_m <= config.min_len_m || config.step_m <= 0.0 {
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
        if let Some(global_best) = samples
            .iter()
            .cloned()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
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

pub fn optimize_ocfd_split_for_length(
    calculations: &[WireCalculation],
    total_len_m: f64,
) -> Option<OcfdSplitRecommendation> {
    if calculations.is_empty() || total_len_m <= 0.0 {
        return None;
    }

    let mut best: Option<OcfdSplitRecommendation> = None;

    for step in 20..=45 {
        let short_ratio = f64::from(step) / 100.0;
        let long_ratio = 1.0 - short_ratio;
        let short_leg_m = total_len_m * short_ratio;
        let long_leg_m = total_len_m * long_ratio;

        let mut worst_leg_clearance_pct = f64::INFINITY;
        for calc in calculations {
            let quarter_wave = calc.corrected_quarter_wave_m;
            if quarter_wave <= 0.0 {
                continue;
            }
            let short_clearance = nearest_resonance_clearance_pct(short_leg_m, quarter_wave);
            let long_clearance = nearest_resonance_clearance_pct(long_leg_m, quarter_wave);
            worst_leg_clearance_pct = worst_leg_clearance_pct
                .min(short_clearance)
                .min(long_clearance);
        }

        if !worst_leg_clearance_pct.is_finite() {
            continue;
        }

        let candidate = OcfdSplitRecommendation {
            short_ratio,
            long_ratio,
            short_leg_m,
            short_leg_ft: short_leg_m * METERS_TO_FEET,
            long_leg_m,
            long_leg_ft: long_leg_m * METERS_TO_FEET,
            worst_leg_clearance_pct,
        };

        best = match best {
            None => Some(candidate),
            Some(current) => {
                let better_clearance =
                    candidate.worst_leg_clearance_pct > current.worst_leg_clearance_pct + 1e-9;
                let tie_clearance =
                    (candidate.worst_leg_clearance_pct - current.worst_leg_clearance_pct).abs()
                        <= 1e-9;
                let candidate_balance = (candidate.short_ratio - (1.0 / 3.0)).abs();
                let current_balance = (current.short_ratio - (1.0 / 3.0)).abs();

                if better_clearance || (tie_clearance && candidate_balance < current_balance) {
                    Some(candidate)
                } else {
                    Some(current)
                }
            }
        };
    }

    best
}

fn nearest_resonance_clearance_pct(length_m: f64, quarter_wave_m: f64) -> f64 {
    if length_m <= 0.0 || quarter_wave_m <= 0.0 {
        return 0.0;
    }

    let harmonic = (length_m / quarter_wave_m).floor().max(1.0);
    let d1 = (length_m - (quarter_wave_m * harmonic)).abs();
    let d2 = (length_m - (quarter_wave_m * (harmonic + 1.0))).abs();
    let nearest = d1.min(d2);
    (nearest / length_m) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bands::Band;

    fn sample_band() -> Band {
        Band {
            name: "20m",
            band_type: crate::bands::BandType::HF,
            freq_low_mhz: 14.0,
            freq_high_mhz: 14.35,
            freq_center_mhz: 14.175,
            typical_skip_km: (150.0, 800.0),
            regions: &[crate::bands::ITURegion::Region1],
        }
    }

    #[test]
    fn transformer_ratio_impedance_values() {
        assert_eq!(TransformerRatio::R1To1.impedance_ratio(), 1.0);
        assert_eq!(TransformerRatio::R1To2.impedance_ratio(), 2.0);
        assert_eq!(TransformerRatio::R1To4.impedance_ratio(), 4.0);
        assert_eq!(TransformerRatio::R1To9.impedance_ratio(), 9.0);
        assert_eq!(TransformerRatio::R1To64.impedance_ratio(), 64.0);
    }

    #[test]
    fn transformer_ratio_labels() {
        assert_eq!(TransformerRatio::R1To1.as_label(), "1:1");
        assert_eq!(TransformerRatio::R1To2.as_label(), "1:2");
        assert_eq!(TransformerRatio::R1To16.as_label(), "1:16");
    }

    #[test]
    fn transformer_ratio_parse_colon_format() {
        assert_eq!(
            TransformerRatio::parse("1:1"),
            Some(TransformerRatio::R1To1)
        );
        assert_eq!(
            TransformerRatio::parse("1:2"),
            Some(TransformerRatio::R1To2)
        );
        assert_eq!(
            TransformerRatio::parse("1:64"),
            Some(TransformerRatio::R1To64)
        );
    }

    #[test]
    fn transformer_ratio_parse_numeric_format() {
        assert_eq!(TransformerRatio::parse("1"), Some(TransformerRatio::R1To1));
        assert_eq!(TransformerRatio::parse("4"), Some(TransformerRatio::R1To4));
        assert_eq!(
            TransformerRatio::parse("64"),
            Some(TransformerRatio::R1To64)
        );
    }

    #[test]
    fn transformer_ratio_parse_invalid() {
        assert_eq!(TransformerRatio::parse("invalid"), None);
        assert_eq!(TransformerRatio::parse("1:3"), None);
        assert_eq!(TransformerRatio::parse("99"), None);
    }

    #[test]
    fn calculate_for_band_basic_lengths() {
        let band = sample_band();
        let result = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );

        assert_eq!(result.band_name, "20m");
        assert_eq!(result.frequency_mhz, 14.175);
        assert!(result.half_wave_m > 0.0);
        assert!(result.full_wave_m > result.half_wave_m);
        assert!(result.quarter_wave_m > 0.0);
        assert!(result.quarter_wave_m < result.half_wave_m);
    }

    #[test]
    fn calculate_for_band_velocity_factor_effect() {
        let band = sample_band();
        let slow = calculate_for_band_with_velocity(
            &band,
            0.8,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        let fast = calculate_for_band_with_velocity(
            &band,
            1.0,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );

        assert!(slow.half_wave_m < fast.half_wave_m);
        assert!(slow.quarter_wave_m < fast.quarter_wave_m);
    }

    #[test]
    fn thicker_conductor_shortens_corrected_lengths() {
        let band = sample_band();
        let thin = calculate_for_band_with_environment(
            &band,
            0.95,
            TransformerRatio::R1To4,
            10.0,
            GroundClass::Average,
            1.0,
        );
        let thick = calculate_for_band_with_environment(
            &band,
            0.95,
            TransformerRatio::R1To4,
            10.0,
            GroundClass::Average,
            4.0,
        );

        assert!(thick.corrected_half_wave_m < thin.corrected_half_wave_m);
        assert!(thick.corrected_quarter_wave_m < thin.corrected_quarter_wave_m);
    }

    #[test]
    fn conductor_diameter_correction_matches_template_reference_points() {
        assert!((conductor_diameter_correction_factor(1.0) - 1.008).abs() < 1e-6);
        assert!((conductor_diameter_correction_factor(2.0) - 1.000).abs() < 1e-12);
        assert!((conductor_diameter_correction_factor(4.0) - 0.992).abs() < 1e-6);
    }

    #[test]
    fn conductor_diameter_correction_is_monotonic_over_supported_range() {
        let thin = conductor_diameter_correction_factor(1.0);
        let baseline = conductor_diameter_correction_factor(2.0);
        let thick = conductor_diameter_correction_factor(4.0);

        assert!(thin > baseline);
        assert!(baseline > thick);
    }

    #[test]
    fn conductor_diameter_runtime_clamp_is_broader_than_template_span() {
        let thin = conductor_diameter_correction_factor(MIN_CONDUCTOR_DIAMETER_MM);
        let thick = conductor_diameter_correction_factor(MAX_CONDUCTOR_DIAMETER_MM);

        assert!(thin >= 1.008);
        assert!(thick <= 0.992);
        assert_eq!(CONDUCTOR_DIAMETER_MIN_FACTOR, 0.97);
        assert_eq!(CONDUCTOR_DIAMETER_MAX_FACTOR, 1.03);
    }

    #[test]
    fn calculate_for_band_unit_conversion() {
        let band = sample_band();
        let result = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );

        let m_to_ft = 3.28084;
        assert!((result.half_wave_ft - result.half_wave_m * m_to_ft).abs() < 0.01);
        assert!((result.full_wave_ft - result.full_wave_m * m_to_ft).abs() < 0.01);
    }

    #[test]
    fn calculate_for_band_derived_antenna_models() {
        let band = sample_band();
        let result = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        let m_to_ft = 3.28084;

        assert!((result.end_fed_half_wave_m - result.corrected_half_wave_m).abs() < 1e-9);
        assert!((result.end_fed_half_wave_ft - result.corrected_half_wave_ft).abs() < 1e-9);
        assert!((result.inverted_v_total_ft - result.corrected_half_wave_ft * 0.97).abs() < 1e-9);
        assert!((result.inverted_v_total_m - (result.inverted_v_total_ft / m_to_ft)).abs() < 1e-9);
        assert!((result.inverted_v_leg_m * 2.0 - result.inverted_v_total_m).abs() < 1e-9);
        assert!((result.inverted_v_leg_ft * 2.0 - result.inverted_v_total_ft).abs() < 1e-9);
        assert!(result.inverted_v_span_120_m > result.inverted_v_span_90_m);
        let expected_span_120_ft = (result.corrected_half_wave_ft * 0.985 / 2.0) * 3.0_f64.sqrt();
        assert!((result.inverted_v_span_120_ft - expected_span_120_ft).abs() < 1e-9);
        let expected_loop_m = (FULL_LOOP_COEFF_M / band.freq_center_mhz) * 0.95;
        assert!((result.full_wave_loop_circumference_m - expected_loop_m).abs() < 1e-9);
        assert!(
            (result.full_wave_loop_circumference_m
                - result.full_wave_loop_circumference_ft / m_to_ft)
                .abs()
                < 1e-9
        );
        assert!(
            (result.full_wave_loop_square_side_m * 4.0 - result.full_wave_loop_circumference_m)
                .abs()
                < 1e-9
        );
        assert!(
            (result.full_wave_loop_square_side_ft * 4.0 - result.full_wave_loop_circumference_ft)
                .abs()
                < 1e-9
        );
        assert!(
            (result.ocfd_33_short_leg_m + result.ocfd_33_long_leg_m - result.end_fed_half_wave_m)
                .abs()
                < 1e-9
        );
        assert!(
            (result.ocfd_33_short_leg_ft + result.ocfd_33_long_leg_ft
                - result.end_fed_half_wave_ft)
                .abs()
                < 1e-9
        );
        assert!(
            (result.ocfd_20_short_leg_m + result.ocfd_20_long_leg_m - result.end_fed_half_wave_m)
                .abs()
                < 1e-9
        );
        assert!(
            (result.ocfd_20_short_leg_ft + result.ocfd_20_long_leg_ft
                - result.end_fed_half_wave_ft)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn optimize_ocfd_split_for_length_returns_valid_recommendation() {
        let band = sample_band();
        let calc = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        let total = calc.corrected_half_wave_m;

        let rec = optimize_ocfd_split_for_length(&[calc], total)
            .expect("expected OCFD split recommendation");

        assert!(rec.short_ratio >= 0.2 && rec.short_ratio <= 0.45);
        assert!((rec.short_ratio + rec.long_ratio - 1.0).abs() < 1e-9);
        assert!((rec.short_leg_m + rec.long_leg_m - total).abs() < 1e-9);
        assert!(rec.worst_leg_clearance_pct >= 0.0);
    }

    #[test]
    fn calculate_average_distances_empty() {
        let empty: Vec<WireCalculation> = Vec::new();
        assert_eq!(calculate_average_min_distance(&empty), 0.0);
        assert_eq!(calculate_average_max_distance(&empty), 0.0);
    }

    #[test]
    fn calculate_average_distances_single() {
        let band = sample_band();
        let calc = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );

        let avg_min = calculate_average_min_distance(&[calc.clone()]);
        let avg_max = calculate_average_max_distance(&[calc.clone()]);

        assert_eq!(avg_min, calc.skip_distance_min_km);
        assert_eq!(avg_max, calc.skip_distance_max_km);
    }

    #[test]
    fn calculate_average_distances_multiple() {
        let band1 = sample_band();
        let calc1 = calculate_for_band_with_velocity(
            &band1,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );

        let mut band2 = sample_band();
        band2.name = "10m";
        band2.typical_skip_km = (250.0, 1200.0);
        let calc2 = calculate_for_band_with_velocity(
            &band2,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );

        let avg_min = calculate_average_min_distance(&[calc1.clone(), calc2.clone()]);
        let avg_max = calculate_average_max_distance(&[calc1.clone(), calc2.clone()]);

        assert_eq!(
            avg_min,
            (calc1.skip_distance_min_km + calc2.skip_distance_min_km) / 2.0
        );
        assert_eq!(
            avg_max,
            (calc1.skip_distance_max_km + calc2.skip_distance_max_km) / 2.0
        );
    }

    #[test]
    fn calculate_non_resonant_optima_empty() {
        let empty: Vec<WireCalculation> = Vec::new();
        let config = NonResonantSearchConfig {
            min_len_m: 8.0,
            max_len_m: 35.0,
            step_m: 0.05,
            preferred_center_m: 20.0,
        };
        let result = calculate_non_resonant_optima(&empty, 0.95, config);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn calculate_non_resonant_optima_invalid_config() {
        let band = sample_band();
        let calc = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );

        // Invalid config: min > max
        let config = NonResonantSearchConfig {
            min_len_m: 35.0,
            max_len_m: 8.0,
            step_m: 0.05,
            preferred_center_m: 20.0,
        };
        let result = calculate_non_resonant_optima(&[calc], 0.95, config);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn calculate_non_resonant_optima_result_structure() {
        let band = sample_band();
        let calc = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );

        let config = NonResonantSearchConfig {
            min_len_m: 8.0,
            max_len_m: 35.0,
            step_m: 0.05,
            preferred_center_m: 20.0,
        };
        let result = calculate_non_resonant_optima(&[calc], 0.95, config);

        assert!(!result.is_empty());
        for rec in &result {
            assert!(rec.length_m >= config.min_len_m);
            assert!(rec.length_m <= config.max_len_m);
            assert!(rec.min_resonance_clearance_pct > 0.0);
        }
    }

    #[test]
    fn calculate_best_non_resonant_length_empty() {
        let empty: Vec<WireCalculation> = Vec::new();
        let config = NonResonantSearchConfig {
            min_len_m: 8.0,
            max_len_m: 35.0,
            step_m: 0.05,
            preferred_center_m: 20.0,
        };
        let result = calculate_best_non_resonant_length(&empty, 0.95, config);
        assert!(result.is_none());
    }

    #[test]
    fn calculate_best_non_resonant_length_from_optima() {
        let band = sample_band();
        let calc = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );

        let config = NonResonantSearchConfig {
            min_len_m: 8.0,
            max_len_m: 35.0,
            step_m: 0.1,
            preferred_center_m: 20.0,
        };
        let optima = calculate_non_resonant_optima(&[calc.clone()], 0.95, config);
        let best = calculate_best_non_resonant_length(&[calc], 0.95, config).unwrap();

        // Best recommendation should be one of the optima
        assert!(optima
            .iter()
            .any(|o| (o.length_m - best.length_m).abs() < 1e-6));
    }

    #[test]
    fn calculate_resonant_compromises_empty() {
        let empty: Vec<WireCalculation> = Vec::new();
        let config = NonResonantSearchConfig {
            min_len_m: 8.0,
            max_len_m: 35.0,
            step_m: 0.05,
            preferred_center_m: 20.0,
        };
        let result = calculate_resonant_compromises(&empty, config);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn calculate_resonant_compromises_result_structure() {
        let band = sample_band();
        let calc = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );

        let config = NonResonantSearchConfig {
            min_len_m: 8.0,
            max_len_m: 35.0,
            step_m: 0.2,
            preferred_center_m: 20.0,
        };
        let result = calculate_resonant_compromises(&[calc], config);

        assert!(!result.is_empty());
        for comp in &result {
            assert!(comp.length_m >= config.min_len_m);
            assert!(comp.length_m <= config.max_len_m);
            assert!(comp.worst_band_distance_m >= 0.0);
        }
    }

    // --- GroundClass ---

    #[test]
    fn ground_class_as_label() {
        assert_eq!(GroundClass::Poor.as_label(), "poor");
        assert_eq!(GroundClass::Average.as_label(), "average");
        assert_eq!(GroundClass::Good.as_label(), "good");
    }

    // --- TransformerRatio: missing variants ---

    #[test]
    fn transformer_ratio_all_impedance_values() {
        assert_eq!(TransformerRatio::R1To5.impedance_ratio(), 5.0);
        assert_eq!(TransformerRatio::R1To6.impedance_ratio(), 6.0);
        assert_eq!(TransformerRatio::R1To16.impedance_ratio(), 16.0);
        assert_eq!(TransformerRatio::R1To49.impedance_ratio(), 49.0);
        assert_eq!(TransformerRatio::R1To56.impedance_ratio(), 56.0);
    }

    #[test]
    fn transformer_ratio_all_labels() {
        assert_eq!(TransformerRatio::R1To4.as_label(), "1:4");
        assert_eq!(TransformerRatio::R1To5.as_label(), "1:5");
        assert_eq!(TransformerRatio::R1To6.as_label(), "1:6");
        assert_eq!(TransformerRatio::R1To9.as_label(), "1:9");
        assert_eq!(TransformerRatio::R1To49.as_label(), "1:49");
        assert_eq!(TransformerRatio::R1To56.as_label(), "1:56");
        assert_eq!(TransformerRatio::R1To64.as_label(), "1:64");
    }

    #[test]
    fn transformer_ratio_parse_all_colon_formats() {
        assert_eq!(
            TransformerRatio::parse("1:5"),
            Some(TransformerRatio::R1To5)
        );
        assert_eq!(
            TransformerRatio::parse("1:6"),
            Some(TransformerRatio::R1To6)
        );
        assert_eq!(
            TransformerRatio::parse("1:9"),
            Some(TransformerRatio::R1To9)
        );
        assert_eq!(
            TransformerRatio::parse("1:16"),
            Some(TransformerRatio::R1To16)
        );
        assert_eq!(
            TransformerRatio::parse("1:49"),
            Some(TransformerRatio::R1To49)
        );
        assert_eq!(
            TransformerRatio::parse("1:56"),
            Some(TransformerRatio::R1To56)
        );
    }

    #[test]
    fn transformer_ratio_parse_all_numeric_formats() {
        assert_eq!(TransformerRatio::parse("5"), Some(TransformerRatio::R1To5));
        assert_eq!(TransformerRatio::parse("6"), Some(TransformerRatio::R1To6));
        assert_eq!(TransformerRatio::parse("9"), Some(TransformerRatio::R1To9));
        assert_eq!(
            TransformerRatio::parse("16"),
            Some(TransformerRatio::R1To16)
        );
        assert_eq!(
            TransformerRatio::parse("49"),
            Some(TransformerRatio::R1To49)
        );
        assert_eq!(
            TransformerRatio::parse("56"),
            Some(TransformerRatio::R1To56)
        );
    }

    #[test]
    fn transformer_ratio_from_str_error() {
        let err = "bad".parse::<TransformerRatio>();
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("Invalid transformer ratio"));
    }

    // --- height_skip_factor and ground_skip_factor ---

    #[test]
    fn skip_scaling_at_height_7m_reduces_distance() {
        let band = sample_band();
        let h7 = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            7.0,
            GroundClass::Average,
        );
        let h10 = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        assert!(h7.skip_distance_min_km < h10.skip_distance_min_km);
        assert!(h7.skip_distance_max_km < h10.skip_distance_max_km);
    }

    #[test]
    fn skip_scaling_at_height_12m_increases_distance() {
        let band = sample_band();
        let h10 = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        let h12 = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            12.0,
            GroundClass::Average,
        );
        assert!(h12.skip_distance_min_km > h10.skip_distance_min_km);
        assert!(h12.skip_distance_max_km > h10.skip_distance_max_km);
    }

    #[test]
    fn skip_scaling_fallback_height_is_interpolated() {
        let band = sample_band();
        // 15m is outside presets — exercises the fallback branch
        let h15 = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            15.0,
            GroundClass::Average,
        );
        let h10 = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        // Factor at 15m = (1 + (15-10)*0.035).clamp(0.70, 1.20) = 1.175
        assert!(h15.skip_distance_max_km > h10.skip_distance_max_km);
        assert!(h15.skip_distance_max_km <= h10.skip_distance_max_km * 1.21);
    }

    #[test]
    fn skip_scaling_fallback_height_clamps_at_minimum() {
        let band = sample_band();
        // Very low height should clamp to 0.70 factor
        let h1 = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            1.0,
            GroundClass::Average,
        );
        let h10 = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        // Factor = (1 + (1-10)*0.035).clamp(0.70, 1.20) = max(0.685, 0.70) = 0.70
        let ratio = h1.skip_distance_max_km / h10.skip_distance_max_km;
        assert!(
            (ratio - 0.70).abs() < 1e-9,
            "expected clamp to 0.70, got ratio {ratio}"
        );
    }

    #[test]
    fn ground_class_poor_reduces_skip() {
        let band = sample_band();
        let avg = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        let poor = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Poor,
        );
        assert!(poor.skip_distance_min_km < avg.skip_distance_min_km);
        assert!(poor.skip_distance_max_km < avg.skip_distance_max_km);
    }

    #[test]
    fn ground_class_good_increases_skip() {
        let band = sample_band();
        let avg = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        let good = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Good,
        );
        assert!(good.skip_distance_min_km > avg.skip_distance_min_km);
        assert!(good.skip_distance_max_km > avg.skip_distance_max_km);
    }

    // --- WireCalculation Display ---

    #[test]
    fn wire_calculation_display_contains_key_fields() {
        let band = sample_band();
        let calc = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        let display = format!("{calc}");
        assert!(display.contains("20m"), "band name missing from display");
        assert!(display.contains("14.175"), "frequency missing from display");
        assert!(
            display.contains("1:1"),
            "transformer ratio missing from display"
        );
        assert!(display.contains("Half-wave"), "half-wave label missing");
        assert!(display.contains("Skip distance"), "skip distance missing");
    }

    // --- optimize_ocfd_split_for_length edge cases ---

    #[test]
    fn optimize_ocfd_split_zero_length_returns_none() {
        let band = sample_band();
        let calc = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        assert!(optimize_ocfd_split_for_length(&[calc], 0.0).is_none());
    }

    #[test]
    fn optimize_ocfd_split_empty_calcs_returns_none() {
        assert!(optimize_ocfd_split_for_length(&[], 20.0).is_none());
    }

    // --- calculate_resonant_compromises invalid config ---

    #[test]
    fn calculate_resonant_compromises_zero_step_returns_empty() {
        let band = sample_band();
        let calc = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        let config = NonResonantSearchConfig {
            min_len_m: 8.0,
            max_len_m: 35.0,
            step_m: 0.0,
            preferred_center_m: 20.0,
        };
        let result = calculate_resonant_compromises(&[calc], config);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn calculate_non_resonant_optima_zero_step_returns_empty() {
        let band = sample_band();
        let calc = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        let config = NonResonantSearchConfig {
            min_len_m: 8.0,
            max_len_m: 35.0,
            step_m: 0.0,
            preferred_center_m: 20.0,
        };
        let result = calculate_non_resonant_optima(&[calc], 0.95, config);
        assert_eq!(result.len(), 0);
    }

    // --- nearest_resonance_clearance_pct edge cases ---

    #[test]
    fn nearest_resonance_clearance_zero_inputs_return_zero() {
        assert_eq!(nearest_resonance_clearance_pct(0.0, 5.0), 0.0);
        assert_eq!(nearest_resonance_clearance_pct(10.0, 0.0), 0.0);
        assert_eq!(nearest_resonance_clearance_pct(0.0, 0.0), 0.0);
    }

    #[test]
    fn nearest_resonance_clearance_at_resonance_is_zero() {
        // Exactly at a harmonic → clearance should be 0 (or very near 0)
        let quarter_wave = 10.0_f64;
        let at_resonance = quarter_wave * 4.0; // 4th harmonic
        assert!(nearest_resonance_clearance_pct(at_resonance, quarter_wave) < 1e-9);
    }

    // --- impedance_corrected_length_m via non-1:1 transformers ---

    #[test]
    fn non_unity_transformer_changes_corrected_length() {
        let band = sample_band();
        let r1to1 = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To1,
            10.0,
            GroundClass::Average,
        );
        let r1to49 = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To49,
            10.0,
            GroundClass::Average,
        );
        let r1to64 = calculate_for_band_with_velocity(
            &band,
            0.95,
            TransformerRatio::R1To64,
            10.0,
            GroundClass::Average,
        );
        // All transformers produce positive lengths
        assert!(r1to49.corrected_half_wave_m > 0.0);
        assert!(r1to64.corrected_half_wave_m > 0.0);
        // Higher ratio → larger target impedance → different correction
        assert!((r1to49.corrected_half_wave_m - r1to1.corrected_half_wave_m).abs() > 1e-9);
        assert!((r1to64.corrected_half_wave_m - r1to1.corrected_half_wave_m).abs() > 1e-9);
    }
}
