//! Velocity- and transformer-sweep view models and display formatting.
//!
//! Pure view/formatting helpers over pre-computed `AppResults` sets; extracted
//! from `app/mod.rs` to keep that module focused on the config/orchestration core.

use super::*;

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
