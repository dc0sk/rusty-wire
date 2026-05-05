/// NEC2 card deck export.
///
/// Generates a plain-text NEC2 input file (`.nec`) from the current
/// `AppConfig` + per-band `WireCalculation` results.  No NEC2 runtime
/// dependency is required – the output is intended for use in 4NEC2,
/// EZNEC, `nec2c`, or any other NEC2-compatible solver.
///
/// One NEC2 deck is emitted per band, each fully self-contained.
///
/// NEC2 card format reference: https://nec2.org/part_3/cards/
use crate::app::{AntennaModel, AppConfig};
use crate::calculations::{GroundClass, WireCalculation};

const NEC2_SEGMENTS: u32 = 21; // odd number, feed at segment (N+1)/2
const SPEED_OF_LIGHT_MS: f64 = 299_792_458.0;

/// Conductivity (S/m) and relative permittivity for standard ground classes.
/// Values from ITU-R P.527 / NEC2 ground table.
fn ground_params(g: GroundClass) -> (f64, f64) {
    match g {
        GroundClass::Poor => (0.001, 5.0),
        GroundClass::Average => (0.005, 13.0),
        GroundClass::Good => (0.030, 20.0),
    }
}

/// Wire radius in metres from conductor diameter in mm.
fn radius_m(conductor_diameter_mm: f64) -> f64 {
    (conductor_diameter_mm / 2.0) / 1000.0
}

/// NEC2 wire length for the antenna model at this band result.
/// Returns `(wire_half_m, is_center_fed)`.
/// For loop antennas the full circumference is used as a single wire
/// approximation; for OCFD the long-leg length drives the deck.
fn wire_params(config: &AppConfig, calc: &WireCalculation) -> (f64, bool) {
    match config.antenna_model {
        Some(AntennaModel::Dipole) | None => (calc.corrected_half_wave_m, true),
        Some(AntennaModel::InvertedVDipole) => (calc.inverted_v_leg_m, true),
        Some(AntennaModel::EndFedHalfWave) => (calc.end_fed_half_wave_m, false),
        Some(AntennaModel::FullWaveLoop) => (calc.full_wave_loop_circumference_m, false),
        Some(AntennaModel::OffCenterFedDipole) => (calc.ocfd_33_long_leg_m, false),
        Some(AntennaModel::TrapDipole) => (calc.trap_dipole_leg_m, true),
    }
}

/// Build a single NEC2 deck for one band.
fn deck_for_band(config: &AppConfig, calc: &WireCalculation, version: &str) -> String {
    let mut out = String::with_capacity(1024);
    let freq_mhz = calc.frequency_mhz;
    let radius = radius_m(config.conductor_diameter_mm);
    let antenna_label = config
        .antenna_model
        .map(|m| format!("{m:?}"))
        .unwrap_or_else(|| "Dipole".into());

    // CM – comment block
    out.push_str(&format!("CM rusty-wire v{version}\n"));
    out.push_str(&format!(
        "CM Band: {}  Freq: {:.4} MHz  Antenna: {}\n",
        calc.band_name, freq_mhz, antenna_label
    ));
    out.push_str(&format!(
        "CM Height: {:.1} m  Ground: {}  Conductor: {:.2} mm dia\n",
        config.antenna_height_m,
        config.ground_class.as_label(),
        config.conductor_diameter_mm
    ));
    out.push_str("CE\n");

    let (wire_half, center_fed) = wire_params(config, calc);
    let height = config.antenna_height_m;
    let segs = NEC2_SEGMENTS;
    let half = wire_half;

    // GW – wire geometry
    // For center-fed antennas: wire spans -half .. +half along X at height h.
    // For end-fed: wire spans 0 .. +wire_half along X at height h.
    let (x1, x2) = if center_fed {
        (-half, half)
    } else {
        (0.0, half)
    };
    out.push_str(&format!(
        "GW  1 {:3}  {:10.5}  {:10.5}  {:10.5}  {:10.5}  {:10.5}  {:10.5}  {:10.5}\n",
        segs, x1, 0.0_f64, height, x2, 0.0_f64, height, radius
    ));

    // GE – geometry end  (ground plane flag: 1 if ground present, 0 for free space)
    let ground_flag = 1_i32;
    out.push_str(&format!("GE  {ground_flag}\n"));

    // GN – ground card
    let (sigma, epsr) = ground_params(config.ground_class);
    // type 2 = finite ground (Sommerfeld-Norton, most accurate in NEC2)
    out.push_str(&format!(
        "GN  2  0  0  0  {:8.4}  {:8.6}\n",
        epsr, sigma
    ));

    // EX – excitation: voltage source at feed segment
    let feed_seg = if center_fed { (segs + 1) / 2 } else { 1 };
    // EX 0 = voltage source; tag=1, segment=feed_seg, real=1 V, imag=0
    out.push_str(&format!(
        "EX  0  1 {:3}  0  1.0  0.0\n",
        feed_seg
    ));

    // FR – frequency card (single frequency)
    out.push_str(&format!(
        "FR  0  1  0  0  {:10.6}  0.0\n",
        freq_mhz
    ));

    // RP – radiation pattern: azimuth sweep at 10° elevation, 1° step
    // RP 0 = normal mode; 37 theta × 72 phi points at 5° increments
    out.push_str("RP  0  37  72  1000  0.0  0.0  5.0  5.0  1.0E3\n");

    // EN – end of input
    out.push_str("EN\n");

    out
}

/// Generate a full NEC2 export string: one deck per band, separated by a
/// comment banner.  `version` should be `env!("CARGO_PKG_VERSION")`.
pub fn to_nec(
    calculations: &[WireCalculation],
    config: &AppConfig,
    version: &str,
) -> String {
    // Warn about models where NEC2 deck is approximate
    let mut decks = String::new();
    for calc in calculations {
        if !decks.is_empty() {
            decks.push('\n');
        }
        decks.push_str(&deck_for_band(config, calc, version));
    }
    decks
}

/// Wavelength in metres for a given frequency in MHz (for segment-length checks).
#[allow(dead_code)]
fn wavelength_m(freq_mhz: f64) -> f64 {
    SPEED_OF_LIGHT_MS / (freq_mhz * 1_000_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AntennaModel, AppConfig};
    use crate::calculations::GroundClass;

    fn base_config() -> AppConfig {
        AppConfig {
            antenna_model: Some(AntennaModel::Dipole),
            antenna_height_m: 10.0,
            ground_class: GroundClass::Average,
            conductor_diameter_mm: 2.0,
            ..Default::default()
        }
    }

    fn mock_calc(band: &str, freq: f64, half_wave: f64) -> WireCalculation {
        WireCalculation {
            band_name: band.to_string(),
            frequency_mhz: freq,
            transformer_ratio_label: "1:1",
            half_wave_m: half_wave,
            full_wave_m: half_wave * 2.0,
            quarter_wave_m: half_wave / 2.0,
            half_wave_ft: half_wave * 3.28084,
            full_wave_ft: half_wave * 2.0 * 3.28084,
            quarter_wave_ft: half_wave / 2.0 * 3.28084,
            corrected_half_wave_m: half_wave,
            corrected_full_wave_m: half_wave * 2.0,
            corrected_quarter_wave_m: half_wave / 2.0,
            corrected_half_wave_ft: half_wave * 3.28084,
            corrected_full_wave_ft: half_wave * 2.0 * 3.28084,
            corrected_quarter_wave_ft: half_wave / 2.0 * 3.28084,
            end_fed_half_wave_m: half_wave,
            end_fed_half_wave_ft: half_wave * 3.28084,
            inverted_v_total_m: half_wave * 0.97,
            inverted_v_total_ft: half_wave * 0.97 * 3.28084,
            inverted_v_leg_m: half_wave * 0.97 / 2.0,
            inverted_v_leg_ft: half_wave * 0.97 / 2.0 * 3.28084,
            inverted_v_span_90_m: 0.0,
            inverted_v_span_90_ft: 0.0,
            inverted_v_span_120_m: 0.0,
            inverted_v_span_120_ft: 0.0,
            full_wave_loop_circumference_m: half_wave * 2.0,
            full_wave_loop_circumference_ft: half_wave * 2.0 * 3.28084,
            full_wave_loop_square_side_m: half_wave / 2.0,
            full_wave_loop_square_side_ft: half_wave / 2.0 * 3.28084,
            ocfd_33_short_leg_m: half_wave * 0.33,
            ocfd_33_short_leg_ft: half_wave * 0.33 * 3.28084,
            ocfd_33_long_leg_m: half_wave * 0.67,
            ocfd_33_long_leg_ft: half_wave * 0.67 * 3.28084,
            ocfd_20_short_leg_m: half_wave * 0.20,
            ocfd_20_short_leg_ft: half_wave * 0.20 * 3.28084,
            ocfd_20_long_leg_m: half_wave * 0.80,
            ocfd_20_long_leg_ft: half_wave * 0.80 * 3.28084,
            trap_dipole_total_m: half_wave,
            trap_dipole_total_ft: half_wave * 3.28084,
            trap_dipole_leg_m: half_wave / 2.0,
            trap_dipole_leg_ft: half_wave / 2.0 * 3.28084,
            skip_distance_min_km: 0.0,
            skip_distance_max_km: 0.0,
            skip_distance_avg_km: 0.0,
            dipole_feedpoint_r_ohm: 73.0,
        }
    }

    #[test]
    fn nec_deck_contains_required_cards() {
        let config = base_config();
        let calc = mock_calc("40m", 7.1, 20.18);
        let deck = deck_for_band(&config, &calc, "2.17.1");

        assert!(deck.contains("CM "), "missing CM card");
        assert!(deck.contains("CE\n"), "missing CE card");
        assert!(deck.contains("GW "), "missing GW card");
        assert!(deck.contains("GE "), "missing GE card");
        assert!(deck.contains("GN "), "missing GN card");
        assert!(deck.contains("EX "), "missing EX card");
        assert!(deck.contains("FR "), "missing FR card");
        assert!(deck.contains("RP "), "missing RP card");
        assert!(deck.contains("EN\n"), "missing EN card");
    }

    #[test]
    fn nec_deck_contains_frequency() {
        let config = base_config();
        let calc = mock_calc("40m", 7.074, 20.18);
        let deck = deck_for_band(&config, &calc, "2.17.1");
        assert!(deck.contains("7.074000"), "FR card frequency mismatch");
    }

    #[test]
    fn nec_deck_center_fed_symmetric() {
        let config = base_config();
        let calc = mock_calc("40m", 7.1, 20.18);
        let deck = deck_for_band(&config, &calc, "2.17.1");
        // Dipole: GW wire should have negative X start
        let gw_line = deck.lines().find(|l| l.starts_with("GW")).unwrap();
        // First coordinate field is negative (half-wave span from -L/2 to +L/2)
        let parts: Vec<&str> = gw_line.split_whitespace().collect();
        let x1: f64 = parts[3].parse().unwrap();
        assert!(x1 < 0.0, "dipole GW x1 should be negative, got {x1}");
    }

    #[test]
    fn nec_multi_band_output() {
        let config = base_config();
        let calcs = vec![
            mock_calc("40m", 7.1, 20.18),
            mock_calc("20m", 14.2, 10.09),
        ];
        let out = to_nec(&calcs, &config, "2.17.1");
        assert_eq!(out.matches("EN\n").count(), 2, "expected two EN cards");
    }

    #[test]
    fn ground_params_values() {
        let (sigma, epsr) = ground_params(GroundClass::Average);
        assert!((sigma - 0.005).abs() < 1e-9);
        assert!((epsr - 13.0).abs() < 1e-9);
    }
}
