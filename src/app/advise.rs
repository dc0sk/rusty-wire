use super::*;
use crate::bands::{get_band_by_index_for_region, Band, BandType};
use crate::calculations::{
    calculate_best_non_resonant_length, calculate_for_band_with_environment,
    NonResonantSearchConfig, TransformerRatio, WireCalculation,
};

pub const TRANSFORMER_OPTIMIZER_CANDIDATES: &[TransformerRatio] = &[
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
];

/// The three standard EFHW transformer ratios to compare.
pub const EFHW_TRANSFORMER_CANDIDATES: &[TransformerRatio] = &[
    TransformerRatio::R1To49,
    TransformerRatio::R1To56,
    TransformerRatio::R1To64,
];

/// One entry in the EFHW transformer side-by-side comparison.
#[derive(Debug, Clone)]
pub struct EfhwTransformerEntry {
    pub ratio: TransformerRatio,
    pub target_z_ohm: f64,
    pub swr: f64,
    pub efficiency_pct: f64,
    pub mismatch_loss_db: f64,
    /// True for the single entry with the lowest mismatch loss.
    pub is_best: bool,
}

/// Side-by-side comparison of 1:49, 1:56, and 1:64 for a given EFHW feedpoint R.
#[derive(Debug, Clone)]
pub struct EfhwTransformerComparison {
    pub feedpoint_r_ohm: f64,
    pub best_ratio: TransformerRatio,
    pub entries: Vec<EfhwTransformerEntry>,
}

/// Compare 1:49, 1:56, and 1:64 transformers against an EFHW feedpoint impedance.
///
/// The best entry (lowest mismatch loss) is flagged with `is_best = true`.
/// Input `feedpoint_r_ohm` is typically obtained from `assumed_feedpoint_impedance_ohm()`.
pub fn compare_efhw_transformers(feedpoint_r_ohm: f64) -> EfhwTransformerComparison {
    let mut entries: Vec<EfhwTransformerEntry> = EFHW_TRANSFORMER_CANDIDATES
        .iter()
        .copied()
        .map(|ratio| {
            let target_z = 50.0 * ratio.impedance_ratio();
            let gamma = ((target_z - feedpoint_r_ohm).abs() / (target_z + feedpoint_r_ohm))
                .clamp(0.0, 0.999_999);
            let efficiency_pct = (1.0 - gamma * gamma) * 100.0;
            let mismatch_loss_db = -10.0 * (1.0 - gamma * gamma).log10();
            let swr = if feedpoint_r_ohm > 0.0 {
                feedpoint_r_ohm.max(target_z) / feedpoint_r_ohm.min(target_z)
            } else {
                1.0
            };
            EfhwTransformerEntry {
                ratio,
                target_z_ohm: target_z,
                swr,
                efficiency_pct,
                mismatch_loss_db,
                is_best: false,
            }
        })
        .collect();

    let best_idx = entries
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.mismatch_loss_db
                .partial_cmp(&b.mismatch_loss_db)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(1); // default to 1:56 (index 1) if all equal

    let best_ratio = entries[best_idx].ratio;
    entries[best_idx].is_best = true;

    EfhwTransformerComparison {
        feedpoint_r_ohm,
        best_ratio,
        entries,
    }
}

#[derive(Debug, Clone)]
pub struct TransformerOptimizerCandidate {
    pub ratio: TransformerRatio,
    pub target_impedance_ohm: f64,
    pub mismatch_gamma: f64,
    pub estimated_efficiency_pct: f64,
    pub mismatch_loss_db: f64,
    pub average_length_shift_pct: f64,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct TransformerOptimizerView {
    pub assumed_feedpoint_ohm: f64,
    pub candidate_count: usize,
    pub candidates: Vec<TransformerOptimizerCandidate>,
}

#[derive(Debug, Clone)]
pub struct AdviseCandidate {
    pub ratio: TransformerRatio,
    pub recommended_length_m: f64,
    pub recommended_length_ft: f64,
    pub min_resonance_clearance_pct: f64,
    pub estimated_efficiency_pct: f64,
    pub mismatch_loss_db: f64,
    pub average_length_shift_pct: f64,
    pub score: f64,
    /// One-sentence tradeoff note explaining the key advantage / limitation of this candidate.
    pub tradeoff_note: String,
    /// Whether fnec-rust validation was performed (if available).
    pub validated: bool,
    /// Structured validation status for pass/warn/reject handling.
    pub validation_status: Option<crate::fnec_validation::ValidationStatus>,
    /// Optional validation note (e.g., cross-check result or reason skipped).
    pub validation_note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AdviseView {
    pub assumed_feedpoint_ohm: f64,
    pub candidates: Vec<AdviseCandidate>,
    /// For EFHW antennas, a side-by-side 1:49/1:56/1:64 comparison.
    pub efhw_comparison: Option<EfhwTransformerComparison>,
}

pub fn optimize_transformer_candidates(config: &AppConfig) -> TransformerOptimizerView {
    let source_impedance = super::assumed_feedpoint_impedance_ohm(
        config.mode,
        config.antenna_model,
        config.antenna_height_m,
        config.ground_class,
    );

    let baseline = build_optimizer_calculations(config, TransformerRatio::R1To1);
    let baseline_avg_half_wave = mean_half_wave_m(&baseline);

    let mut candidates: Vec<TransformerOptimizerCandidate> = TRANSFORMER_OPTIMIZER_CANDIDATES
        .iter()
        .copied()
        .map(|ratio| {
            let target_impedance = 50.0 * ratio.impedance_ratio();
            let gamma = if source_impedance <= 0.0 {
                1.0
            } else {
                ((target_impedance - source_impedance).abs()
                    / (target_impedance + source_impedance))
                    .clamp(0.0, 0.999_999)
            };
            let mismatch_efficiency = (1.0 - gamma * gamma) * 100.0;
            let mismatch_loss_db = -10.0 * (1.0 - gamma * gamma).log10();

            let ratio_calculations = build_optimizer_calculations(config, ratio);
            let ratio_avg_half_wave = mean_half_wave_m(&ratio_calculations);
            let average_length_shift_pct = if baseline_avg_half_wave > 0.0 {
                ((ratio_avg_half_wave - baseline_avg_half_wave).abs() / baseline_avg_half_wave)
                    * 100.0
            } else {
                0.0
            };

            // Optimizer score: prefer better impedance transfer and lightly penalize
            // larger geometry shifts introduced by correction heuristics.
            let score = mismatch_efficiency - (average_length_shift_pct * 0.35);

            TransformerOptimizerCandidate {
                ratio,
                target_impedance_ohm: target_impedance,
                mismatch_gamma: gamma,
                estimated_efficiency_pct: mismatch_efficiency,
                mismatch_loss_db,
                average_length_shift_pct,
                score,
            }
        })
        .collect();

    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    TransformerOptimizerView {
        assumed_feedpoint_ohm: source_impedance,
        candidate_count: candidates.len(),
        candidates,
    }
}

/// Generate a one-sentence tradeoff note for an advise candidate.
///
/// The note describes the key practical advantage or limitation relative to
/// the baseline 1:1 case, so the user can make a quick decision without
/// having to interpret raw mismatch percentages.
fn generate_tradeoff_note(
    ratio: TransformerRatio,
    assumed_feedpoint_ohm: f64,
    estimated_efficiency_pct: f64,
    mismatch_loss_db: f64,
    min_resonance_clearance_pct: f64,
    antenna_model: Option<AntennaModel>,
) -> String {
    let target_z = 50.0 * ratio.impedance_ratio();
    let swr_at_target = if assumed_feedpoint_ohm > 0.0 {
        assumed_feedpoint_ohm.max(target_z) / assumed_feedpoint_ohm.min(target_z)
    } else {
        1.0
    };

    // Check if this is near-perfect match
    if swr_at_target < 1.15 {
        if min_resonance_clearance_pct >= 15.0 {
            return format!(
                "Best match: SWR ≈ {:.1}:1 into {:.0} Ω, wide resonance clearance ({:.0}%).",
                swr_at_target, target_z, min_resonance_clearance_pct
            );
        }
        return format!(
            "Best match: SWR ≈ {:.1}:1 into {:.0} Ω; check resonance clearance ({:.0}%).",
            swr_at_target, target_z, min_resonance_clearance_pct
        );
    }

    // EFHW / high-impedance case
    if let Some(AntennaModel::EndFedHalfWave) = antenna_model {
        if ratio.impedance_ratio() >= 49.0 {
            return format!(
                "Standard EFHW match ({} ratio, SWR ≈ {:.1}:1 into {:.0} Ω, loss {:.2} dB).",
                ratio.as_label(),
                swr_at_target,
                target_z,
                mismatch_loss_db
            );
        }
    }

    // Categorise by loss level
    if mismatch_loss_db < 0.5 {
        format!(
            "Good match: {:.1}% efficiency, {:.2} dB loss, SWR ≈ {:.1}:1 into {:.0} Ω.",
            estimated_efficiency_pct, mismatch_loss_db, swr_at_target, target_z
        )
    } else if mismatch_loss_db < 1.5 {
        format!(
            "Moderate mismatch: {:.2} dB loss (SWR {:.1}:1 into {:.0} Ω); usable with ATU.",
            mismatch_loss_db, swr_at_target, target_z
        )
    } else if mismatch_loss_db < 3.0 {
        format!(
            "High mismatch: {:.2} dB loss (SWR {:.1}:1 into {:.0} Ω); ATU strongly recommended.",
            mismatch_loss_db, swr_at_target, target_z
        )
    } else {
        format!(
            "Very high mismatch: {:.2} dB loss (SWR {:.1}:1 into {:.0} Ω); practical use limited.",
            mismatch_loss_db, swr_at_target, target_z
        )
    }
}

pub fn build_advise_candidates(config: &AppConfig, limit: usize) -> AdviseView {
    build_advise_candidates_with_thresholds(
        config,
        limit,
        crate::fnec_validation::DEFAULT_FNEC_PASS_MAX_MISMATCH,
        crate::fnec_validation::DEFAULT_FNEC_REJECT_MIN_MISMATCH,
    )
}

pub fn build_advise_candidates_with_thresholds(
    config: &AppConfig,
    limit: usize,
    pass_max_mismatch: f64,
    reject_min_mismatch: f64,
) -> AdviseView {
    let optimizer = optimize_transformer_candidates(config);
    let non_res_cfg = NonResonantSearchConfig {
        min_len_m: config.wire_min_m,
        max_len_m: config.wire_max_m,
        step_m: config.step_m,
        preferred_center_m: (config.wire_min_m + config.wire_max_m) / 2.0,
    };

    let max_rows = limit.max(1);
    let mut candidates = Vec::new();

    for ranked in optimizer.candidates.iter().take(max_rows) {
        let calculations = build_optimizer_calculations(config, ranked.ratio);
        let recommendation =
            calculate_best_non_resonant_length(&calculations, config.velocity_factor, non_res_cfg);

        if let Some(rec) = recommendation {
            let note = generate_tradeoff_note(
                ranked.ratio,
                optimizer.assumed_feedpoint_ohm,
                ranked.estimated_efficiency_pct,
                ranked.mismatch_loss_db,
                rec.min_resonance_clearance_pct,
                config.antenna_model,
            );
            candidates.push(AdviseCandidate {
                ratio: ranked.ratio,
                recommended_length_m: rec.length_m,
                recommended_length_ft: rec.length_ft,
                min_resonance_clearance_pct: rec.min_resonance_clearance_pct,
                estimated_efficiency_pct: ranked.estimated_efficiency_pct,
                mismatch_loss_db: ranked.mismatch_loss_db,
                average_length_shift_pct: ranked.average_length_shift_pct,
                score: ranked.score,
                tradeoff_note: note,
                validated: false,
                validation_status: None,
                validation_note: None,
            });
        }
    }

    // Optionally validate candidates with fnec-rust.
    if config.validate_with_fnec {
        for candidate in &mut candidates {
            // Use first selected band center as a representative frequency.
            if let Some(&band_idx) = config.band_indices.first() {
                if let Some(band) = get_band_by_index_for_region(band_idx, config.itu_region) {
                    let result = crate::fnec_validation::validate_candidate_with_thresholds(
                        candidate.recommended_length_m,
                        band.freq_center_mhz,
                        config.antenna_height_m,
                        "/tmp",
                        pass_max_mismatch,
                        reject_min_mismatch,
                    );
                    candidate.validated = result.validated;
                    candidate.validation_status = Some(result.status);
                    candidate.validation_note = result.validation_note;
                }
            }
        }
    }
    let efhw_comparison = if config.antenna_model == Some(AntennaModel::EndFedHalfWave) {
        Some(compare_efhw_transformers(optimizer.assumed_feedpoint_ohm))
    } else {
        None
    };
    AdviseView {
        assumed_feedpoint_ohm: optimizer.assumed_feedpoint_ohm,
        candidates,
        efhw_comparison,
    }
}

fn build_optimizer_calculations(
    config: &AppConfig,
    ratio: TransformerRatio,
) -> Vec<WireCalculation> {
    if !config.freq_list_mhz.is_empty() {
        return config
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
                calculate_for_band_with_environment(
                    &custom_band,
                    config.velocity_factor,
                    ratio,
                    config.antenna_height_m,
                    config.ground_class,
                    config.conductor_diameter_mm,
                )
            })
            .collect();
    }

    if let Some(freq_mhz) = config.custom_freq_mhz {
        let custom_band = Band {
            name: "custom",
            band_type: BandType::HF,
            freq_low_mhz: freq_mhz,
            freq_high_mhz: freq_mhz,
            freq_center_mhz: freq_mhz,
            typical_skip_km: (0.0, 0.0),
            regions: &[],
        };
        return vec![calculate_for_band_with_environment(
            &custom_band,
            config.velocity_factor,
            ratio,
            config.antenna_height_m,
            config.ground_class,
            config.conductor_diameter_mm,
        )];
    }

    let (calculations, _) = build_calculations(
        &config.band_indices,
        config.velocity_factor,
        config.itu_region,
        ratio,
        config.antenna_height_m,
        config.ground_class,
        config.conductor_diameter_mm,
    );
    calculations
}

fn mean_half_wave_m(calculations: &[WireCalculation]) -> f64 {
    if calculations.is_empty() {
        return 0.0;
    }
    let sum: f64 = calculations.iter().map(|c| c.corrected_half_wave_m).sum();
    sum / calculations.len() as f64
}

/// Structured explanation for the recommended transformer ratio.
///
/// Provides both the ratio value and a human-readable `reason` string
/// suitable for TUI help text, tooltips, or verbose CLI output.
#[derive(Debug, Clone)]
pub struct TransformerRatioExplanation {
    pub ratio: TransformerRatio,
    pub reason: &'static str,
}

/// Return the recommended transformer ratio and a one-sentence explanation
/// of why it is recommended for the given mode and antenna model.
///
/// Pure function; performs no I/O.
pub fn transformer_ratio_explanation(
    mode: CalcMode,
    antenna_model: Option<AntennaModel>,
) -> TransformerRatioExplanation {
    let ratio = recommended_transformer_ratio(mode, antenna_model);
    let reason = match antenna_model {
        Some(AntennaModel::Dipole) | Some(AntennaModel::InvertedVDipole) => {
            "Center-fed dipoles present ~50 \u{03a9} at resonance; a 1:1 balun is typical."
        }
        Some(AntennaModel::TrapDipole) => {
            "Trap dipoles present ~50\u{2013}75 \u{03a9} at resonance; a 1:1 balun is typical."
        }
        Some(AntennaModel::FullWaveLoop) => {
            "Full-wave loops present ~100 \u{03a9} at resonance; a 1:1 choke balun is common."
        }
        Some(AntennaModel::EndFedHalfWave) => {
            "EFHW antennas present ~2500\u{2013}3000 \u{03a9}; a 1:49 or 1:56 transformer matches to 50 \u{03a9}."
        }
        Some(AntennaModel::OffCenterFedDipole) => {
            "OCFDs fed at the 1/3 point present ~200 \u{03a9}; a 1:4 balun is standard."
        }
        None => match mode {
            CalcMode::Resonant => {
                "Resonant mode, no antenna model; 1:1 is used as a neutral starting point."
            }
            CalcMode::NonResonant => {
                "Non-resonant random-wire mode; 1:9 is a common matching ratio."
            }
        },
    };
    TransformerRatioExplanation { ratio, reason }
}
