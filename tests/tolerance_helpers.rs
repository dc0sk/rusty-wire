/// Tolerance checking utilities for numeric output validation.
///
/// See docs/requirements.md for the tolerance matrix and policy.

/// Check if a measured value falls within the tolerance bounds.
///
/// Uses the wider of relative or absolute tolerance:
/// - relative_error = abs(measured - expected) / abs(expected)
/// - absolute_error = abs(measured - expected)
/// - passes if: relative_error <= rel_tol OR absolute_error <= abs_tol
///
/// # Arguments
///
/// * `measured` - The actual value computed
/// * `expected` - The reference value (from golden reference)
/// * `rel_tol` - Relative tolerance as a decimal (e.g., 0.01 for ±1%)
/// * `abs_tol` - Absolute tolerance in metric units (meters, km, etc.)
///
/// # Returns
///
/// `Ok(())` if within tolerance, `Err(message)` if outside tolerance
pub fn check_tolerance(
    measured: f64,
    expected: f64,
    rel_tol: f64,
    abs_tol: f64,
) -> Result<(), String> {
    if expected == 0.0 {
        return Err("expected value cannot be zero".to_string());
    }

    let absolute_error = (measured - expected).abs();
    let relative_error = absolute_error / expected.abs();

    let within_relative = relative_error <= rel_tol;
    let within_absolute = absolute_error <= abs_tol;

    if within_relative || within_absolute {
        Ok(())
    } else {
        Err(format!(
            "Tolerance breach: measured={:.6}, expected={:.6}, rel_error={:.4} (limit ≤{:.4}), abs_error={:.6} (limit ≤{:.6})",
            measured, expected, relative_error, rel_tol, absolute_error, abs_tol
        ))
    }
}

/// Check multiple metric tolerances at once.
///
/// # Arguments
///
/// * `metrics` - Vec of (name, measured, expected, rel_tol, abs_tol) tuples
///
/// # Returns
///
/// List of failures (empty if all pass)
pub fn check_tolerance_batch(
    metrics: Vec<(&str, f64, f64, f64, f64)>,
) -> Vec<(String, String)> {
    metrics
        .iter()
        .filter_map(|(name, measured, expected, rel_tol, abs_tol)| {
            check_tolerance(*measured, *expected, *rel_tol, *abs_tol)
                .err()
                .map(|err| (name.to_string(), err))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerance_check_within_relative() {
        assert!(check_tolerance(20.1, 20.0, 0.01, 0.05).is_ok());
    }

    #[test]
    fn tolerance_check_within_absolute() {
        assert!(check_tolerance(20.03, 20.0, 0.001, 0.05).is_ok());
    }

    #[test]
    fn tolerance_check_outside() {
        assert!(check_tolerance(21.0, 20.0, 0.01, 0.05).is_err());
    }

    #[test]
    fn tolerance_check_boundary_relative() {
        // Exactly at the relative tolerance limit
        assert!(check_tolerance(20.2, 20.0, 0.01, 0.05).is_ok());
    }

    #[test]
    fn tolerance_check_batch_mixed() {
        let failures = check_tolerance_batch(vec![
            ("metric_a", 10.0, 10.0, 0.01, 0.1),
            ("metric_b", 20.5, 20.0, 0.01, 0.1),
            ("metric_c", 30.1, 30.0, 0.001, 0.05),
        ]);
        assert_eq!(failures.len(), 0, "all should pass");
    }
}
