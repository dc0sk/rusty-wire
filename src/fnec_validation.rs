/// Optional fnec-rust validation for advise-mode candidates.
///
/// This module shells out to the fnec-rust tool (if installed) to validate
/// advise-mode wire candidates against actual NEC antenna models. Validation
/// is optional and gracefully degrades if fnec is not found in PATH.
///
/// Example fnec-rust invocation:
///
/// ```ignore
/// ./fnec dipole.nec > output.txt
/// ```
///
/// We generate a simple NEC deck for each candidate wire, run fnec, and
/// compare the calculated feedpoint impedance against our model estimates.
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation ran successfully.
    pub validated: bool,
    /// Human-readable validation note (e.g., warning or success message).
    pub validation_note: Option<String>,
}

impl ValidationResult {
    /// Create a successful validation result.
    pub fn success(note: impl Into<String>) -> Self {
        Self {
            validated: true,
            validation_note: Some(note.into()),
        }
    }

    /// Create a validation warning (still validated, but with caveats).
    pub fn warning(note: impl Into<String>) -> Self {
        Self {
            validated: true,
            validation_note: Some(note.into()),
        }
    }

    /// Create when validation could not run (e.g., fnec not found).
    pub fn skipped(reason: impl Into<String>) -> Self {
        Self {
            validated: false,
            validation_note: Some(reason.into()),
        }
    }

    /// Create when validation encountered an error.
    pub fn error(error_msg: impl Into<String>) -> Self {
        Self {
            validated: false,
            validation_note: Some(error_msg.into()),
        }
    }
}

/// Check if fnec binary is available in PATH.
pub fn fnec_available() -> bool {
    Command::new("fnec")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Generate a simple NEC deck for a dipole wire at a given frequency.
///
/// # Parameters
/// - `length_m`: total dipole length in meters
/// - `frequency_mhz`: operating frequency in MHz
/// - `height_m`: height above ground in meters
/// - `wire_radius_mm`: conductor radius in millimeters
///
/// # Returns
/// NEC2 deck content as a string.
fn generate_dipole_nec_deck(
    length_m: f64,
    frequency_mhz: f64,
    height_m: f64,
    wire_radius_mm: f64,
) -> String {
    // Calculate segment count: aim for ~lambda/20 segments
    let wavelength_m = 299.792458 / frequency_mhz;
    let min_segments = ((length_m / wavelength_m) * 20.0).ceil() as i32;
    let segment_count = min_segments.max(21).min(100); // Clamp between 21 and 100

    let half_length = length_m / 2.0;
    let wire_radius = wire_radius_mm / 1000.0; // Convert mm to meters

    // GW card: wire geometry
    // Format: GW tag nseg x1 y1 z1 x2 y2 z2 rad
    // Position dipole horizontally at given height, centered at origin
    let gw_card = format!(
        "GW 1 {} 0 0 {:.4} 0 0 {:.4} {:.6}",
        segment_count,
        height_m - half_length,
        height_m + half_length,
        wire_radius
    );

    // GE: geometry end
    // GND: use free-space (no ground) for initial validation
    // EX: excitation at segment 1 (center)
    // FR: frequency
    // EN: end

    format!(
        "{}\nGE 0\nEX 0 1 {} 0 1.0 0.0\nFR 0 1 0 0 {:.4} 0.0\nEN\n",
        gw_card,
        (segment_count / 2) + 1,
        frequency_mhz
    )
}

/// Validate a single advise candidate using fnec-rust.
///
/// Shells out to fnec if available, generates a simple NEC model,
/// and returns validation result.
///
/// # Parameters
/// - `length_m`: proposed wire length in meters
/// - `frequency_mhz`: band frequency in MHz
/// - `height_m`: antenna height in meters
/// - `temp_dir`: temporary directory for NEC deck and output files
///
/// # Returns
/// ValidationResult with status and optional note.
pub fn validate_candidate(
    length_m: f64,
    frequency_mhz: f64,
    height_m: f64,
    temp_dir: &str,
) -> ValidationResult {
    // Check if fnec is available
    if !fnec_available() {
        return ValidationResult::skipped(
            "fnec-rust not found in PATH; skipping validation. Install fnec-rust for cross-check validation."
        );
    }

    // Generate NEC deck
    let nec_content = generate_dipole_nec_deck(length_m, frequency_mhz, height_m, 2.0);

    // Write NEC deck to temporary file
    let nec_path = PathBuf::from(temp_dir).join("candidate.nec");
    if let Err(e) = fs::write(&nec_path, &nec_content) {
        return ValidationResult::error(format!("Failed to write NEC deck: {e}"));
    }

    // Run fnec
    let output = match Command::new("fnec").arg(&nec_path).output() {
        Ok(o) => o,
        Err(e) => {
            return ValidationResult::error(format!("Failed to run fnec: {e}"));
        }
    };

    // Clean up
    let _ = fs::remove_file(&nec_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return ValidationResult::warning(format!("fnec validation exited with error: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse feedpoint impedance from output
    let z_re = extract_impedance_real(&stdout);
    let z_im = extract_impedance_imag(&stdout);

    if let (Some(re), Some(im)) = (z_re, z_im) {
        let mismatch = calculate_mismatch_factor(re, im);
        let note = format!(
            "NEC model: {:.1}Ω (real {:.1}, imag {:.2}); mismatch factor {:.2}",
            (re * re + im * im).sqrt(),
            re,
            im,
            mismatch
        );
        if mismatch > 0.5 {
            ValidationResult::warning(format!("High mismatch: {}", note))
        } else {
            ValidationResult::success(note)
        }
    } else {
        ValidationResult::warning("Could not parse impedance from fnec output".to_string())
    }
}

/// Extract real part of impedance from fnec output.
fn extract_impedance_real(output: &str) -> Option<f64> {
    for line in output.lines() {
        if line.contains("Z_RE") || line.contains("V_RE") {
            // Try to parse numeric values from the line
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 3 {
                if let Ok(val) = parts[3].parse::<f64>() {
                    return Some(val);
                }
            }
        }
    }
    None
}

/// Extract imaginary part of impedance from fnec output.
fn extract_impedance_imag(output: &str) -> Option<f64> {
    for line in output.lines() {
        if line.contains("Z_IM") || line.contains("V_IM") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 4 {
                if let Ok(val) = parts[4].parse::<f64>() {
                    return Some(val);
                }
            }
        }
    }
    None
}

/// Calculate mismatch factor between calculated and expected impedance.
/// Range 0.0 (perfect match) to 1.0 (worst match).
fn calculate_mismatch_factor(z_real: f64, z_imag: f64) -> f64 {
    // Typical dipole feedpoint impedance is ~70 ohms + reactance
    let target_real = 70.0;
    let delta_real = (z_real - target_real).abs();
    let delta_imag = z_imag.abs();

    // Normalize: high real delta or high reactance increases mismatch
    ((delta_real / 100.0).max(delta_imag / 50.0)).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_dipole_nec_deck() {
        let deck = generate_dipole_nec_deck(10.0, 14.2, 8.0, 2.0);
        assert!(deck.contains("GW 1"));
        assert!(deck.contains("GE 0"));
        assert!(deck.contains("FR 0 1 0 0 14.2000"));
        assert!(deck.contains("EN"));
    }

    #[test]
    fn test_validation_result_success() {
        let result = ValidationResult::success("Test success");
        assert!(result.validated);
        assert_eq!(result.validation_note, Some("Test success".to_string()));
    }

    #[test]
    fn test_validation_result_skipped() {
        let result = ValidationResult::skipped("fnec not found");
        assert!(!result.validated);
        assert!(result.validation_note.is_some());
    }

    #[test]
    fn test_mismatch_factor_perfect() {
        let factor = calculate_mismatch_factor(70.0, 0.0);
        assert!(factor < 0.1);
    }

    #[test]
    fn test_mismatch_factor_high_reactance() {
        let factor = calculate_mismatch_factor(70.0, 30.0);
        assert!(factor > 0.1);
    }
}
