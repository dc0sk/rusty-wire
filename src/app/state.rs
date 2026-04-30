use super::*;

// ---------------------------------------------------------------------------

/// The complete application state that any front-end (TUI, GUI) renders.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    /// Current configuration being shown or edited.
    pub config: AppConfig,
    /// Results of the last successful `RunCalculation` action, or `None`.
    pub results: Option<AppResults>,
    /// Last error produced by a failed `RunCalculation` or invalid action.
    pub error: Option<AppError>,
}

/// All actions that can be dispatched to `apply_action`.
///
/// Each variant mutates exactly one field of `AppConfig`, or triggers a
/// calculation / state reset.  The set intentionally mirrors the full
/// `AppConfig` field set so a TUI or GUI only needs to know about
/// `AppAction` — not `AppConfig` internals.
#[derive(Debug, Clone)]
pub enum AppAction {
    // --- Configuration changes ---
    SetBandIndices(Vec<usize>),
    SetMode(CalcMode),
    SetAntennaModel(Option<AntennaModel>),
    SetVelocityFactor(f64),
    SetTransformerRatio(TransformerRatio),
    SetWireMin(f64),
    SetWireMax(f64),
    SetStep(f64),
    SetUnits(UnitSystem),
    SetItuRegion(ITURegion),
    SetCustomFreq(Option<f64>),
    SetFreqList(Vec<f64>),
    SetAntennaHeight(f64),
    SetGroundClass(crate::calculations::GroundClass),
    SetConductorDiameter(f64),
    // --- Lifecycle ---
    /// Run `run_calculation_checked` against the current config.
    /// On success: replaces `results` and clears `error`.
    /// On failure: clears `results` and sets `error`.
    RunCalculation,
    /// Clear the last results without changing the config.
    ClearResults,
    /// Clear the last error without changing the config or results.
    ClearError,
}

/// Pure state-transition function.
///
/// Takes ownership of `state`, applies `action`, and returns the new state.
/// Never performs I/O.  Suitable as the single update function in a TUI event
/// loop or an iced `update()` handler.
pub fn apply_action(state: AppState, action: AppAction) -> AppState {
    match action {
        AppAction::SetBandIndices(indices) => AppState {
            config: AppConfig {
                band_indices: indices,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetMode(mode) => AppState {
            config: AppConfig {
                mode,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetAntennaModel(antenna_model) => AppState {
            config: AppConfig {
                antenna_model,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetVelocityFactor(vf) => AppState {
            config: AppConfig {
                velocity_factor: vf,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetTransformerRatio(ratio) => AppState {
            config: AppConfig {
                transformer_ratio: ratio,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetWireMin(min_m) => AppState {
            config: AppConfig {
                wire_min_m: min_m,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetWireMax(max_m) => AppState {
            config: AppConfig {
                wire_max_m: max_m,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetStep(step_m) => AppState {
            config: AppConfig {
                step_m,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetUnits(units) => AppState {
            config: AppConfig {
                units,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetItuRegion(region) => AppState {
            config: AppConfig {
                itu_region: region,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetCustomFreq(freq) => AppState {
            config: AppConfig {
                custom_freq_mhz: freq,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetFreqList(freqs) => AppState {
            config: AppConfig {
                freq_list_mhz: freqs,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetAntennaHeight(height_m) => AppState {
            config: AppConfig {
                antenna_height_m: height_m,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetGroundClass(ground_class) => AppState {
            config: AppConfig {
                ground_class,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::SetConductorDiameter(diameter_mm) => AppState {
            config: AppConfig {
                conductor_diameter_mm: diameter_mm,
                ..state.config
            },
            error: None,
            ..state
        },
        AppAction::RunCalculation => match run_calculation_checked(state.config.clone()) {
            Ok(results) => AppState {
                results: Some(results),
                error: None,
                ..state
            },
            Err(err) => AppState {
                results: None,
                error: Some(err),
                ..state
            },
        },
        AppAction::ClearResults => AppState {
            results: None,
            error: None,
            ..state
        },
        AppAction::ClearError => AppState {
            error: None,
            ..state
        },
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod state_machine_tests {
    use super::*;

    fn default_state() -> AppState {
        AppState::default()
    }

    #[test]
    fn apply_action_set_mode_updates_config() {
        let state = apply_action(default_state(), AppAction::SetMode(CalcMode::NonResonant));
        assert_eq!(state.config.mode, CalcMode::NonResonant);
        assert!(state.error.is_none());
    }

    #[test]
    fn apply_action_set_band_indices_replaces_bands() {
        let state = apply_action(default_state(), AppAction::SetBandIndices(vec![3, 5, 7]));
        assert_eq!(state.config.band_indices, vec![3, 5, 7]);
        assert!(state.error.is_none());
    }

    #[test]
    fn apply_action_set_velocity_factor_updates_config() {
        let state = apply_action(default_state(), AppAction::SetVelocityFactor(0.72));
        assert!((state.config.velocity_factor - 0.72).abs() < 1e-9);
    }

    #[test]
    fn apply_action_set_units_updates_config() {
        let state = apply_action(default_state(), AppAction::SetUnits(UnitSystem::Imperial));
        assert_eq!(state.config.units, UnitSystem::Imperial);
    }

    #[test]
    fn apply_action_set_itu_region_updates_config() {
        let state = apply_action(default_state(), AppAction::SetItuRegion(ITURegion::Region2));
        assert_eq!(state.config.itu_region, ITURegion::Region2);
    }

    #[test]
    fn apply_action_set_antenna_model_updates_config() {
        let state = apply_action(
            default_state(),
            AppAction::SetAntennaModel(Some(AntennaModel::EndFedHalfWave)),
        );
        assert_eq!(
            state.config.antenna_model,
            Some(AntennaModel::EndFedHalfWave)
        );
    }

    #[test]
    fn apply_action_set_custom_freq_updates_config() {
        let state = apply_action(default_state(), AppAction::SetCustomFreq(Some(14.225)));
        assert_eq!(state.config.custom_freq_mhz, Some(14.225));
    }

    #[test]
    fn apply_action_set_antenna_height_updates_config() {
        let state = apply_action(default_state(), AppAction::SetAntennaHeight(7.0));
        assert!((state.config.antenna_height_m - 7.0).abs() < 1e-9);
        let state = apply_action(state, AppAction::SetAntennaHeight(12.0));
        assert!((state.config.antenna_height_m - 12.0).abs() < 1e-9);
    }

    #[test]
    fn apply_action_set_ground_class_updates_config() {
        let state = apply_action(
            default_state(),
            AppAction::SetGroundClass(crate::calculations::GroundClass::Poor),
        );
        assert_eq!(
            state.config.ground_class,
            crate::calculations::GroundClass::Poor
        );
        let state = apply_action(
            state,
            AppAction::SetGroundClass(crate::calculations::GroundClass::Good),
        );
        assert_eq!(
            state.config.ground_class,
            crate::calculations::GroundClass::Good
        );
    }

    #[test]
    fn apply_action_set_conductor_diameter_updates_config() {
        let state = apply_action(default_state(), AppAction::SetConductorDiameter(1.0));
        assert!((state.config.conductor_diameter_mm - 1.0).abs() < 1e-9);
        let state = apply_action(state, AppAction::SetConductorDiameter(4.0));
        assert!((state.config.conductor_diameter_mm - 4.0).abs() < 1e-9);
    }

    #[test]
    fn apply_action_set_wire_min_max_updates_config() {
        let state = apply_action(default_state(), AppAction::SetWireMin(12.0));
        assert!((state.config.wire_min_m - 12.0).abs() < 1e-9);
        let state = apply_action(state, AppAction::SetWireMax(50.0));
        assert!((state.config.wire_max_m - 50.0).abs() < 1e-9);
    }

    #[test]
    fn apply_action_run_calculation_populates_results_on_success() {
        let state = apply_action(default_state(), AppAction::RunCalculation);
        assert!(state.results.is_some());
        assert!(state.error.is_none());
    }

    #[test]
    fn apply_action_run_calculation_sets_error_on_invalid_config() {
        // Velocity factor outside valid range → InvalidVelocityFactor
        let bad_state = AppState {
            config: AppConfig {
                velocity_factor: 2.0,
                ..AppConfig::default()
            },
            ..AppState::default()
        };
        let state = apply_action(bad_state, AppAction::RunCalculation);
        assert!(state.results.is_none());
        assert!(matches!(
            state.error,
            Some(AppError::InvalidVelocityFactor(_))
        ));
    }

    #[test]
    fn apply_action_clear_results_removes_results_and_error() {
        let state = apply_action(default_state(), AppAction::RunCalculation);
        assert!(state.results.is_some());
        let state = apply_action(state, AppAction::ClearResults);
        assert!(state.results.is_none());
        assert!(state.error.is_none());
    }

    #[test]
    fn apply_action_clear_error_removes_error_only() {
        let bad_state = AppState {
            config: AppConfig {
                velocity_factor: 2.0,
                ..AppConfig::default()
            },
            ..AppState::default()
        };
        let state = apply_action(bad_state, AppAction::RunCalculation);
        assert!(state.error.is_some());
        let state = apply_action(state, AppAction::ClearError);
        assert!(state.error.is_none());
        // Config is unchanged — velocity_factor is still 2.0
        assert!((state.config.velocity_factor - 2.0).abs() < 1e-9);
    }

    #[test]
    fn apply_action_sequence_builds_correct_config() {
        // Simulate a TUI user configuring from scratch
        let state = default_state();
        let state = apply_action(state, AppAction::SetMode(CalcMode::NonResonant));
        let state = apply_action(state, AppAction::SetBandIndices(vec![4, 5, 7]));
        let state = apply_action(state, AppAction::SetWireMin(10.0));
        let state = apply_action(state, AppAction::SetWireMax(40.0));
        let state = apply_action(state, AppAction::SetUnits(UnitSystem::Both));
        let state = apply_action(state, AppAction::RunCalculation);

        assert_eq!(state.config.mode, CalcMode::NonResonant);
        assert_eq!(state.config.band_indices, vec![4, 5, 7]);
        assert!(state.results.is_some());
        assert!(state.error.is_none());
    }

    #[test]
    fn transformer_ratio_explanation_dipole_returns_1to1() {
        let expl = transformer_ratio_explanation(CalcMode::Resonant, Some(AntennaModel::Dipole));
        assert_eq!(expl.ratio, TransformerRatio::R1To1);
        assert!(expl.reason.contains("50"));
        assert!(expl.reason.contains("1:1"));
    }

    #[test]
    fn transformer_ratio_explanation_inverted_v_returns_1to1() {
        let expl =
            transformer_ratio_explanation(CalcMode::Resonant, Some(AntennaModel::InvertedVDipole));
        assert_eq!(expl.ratio, TransformerRatio::R1To1);
        assert!(expl.reason.contains("50"));
        assert!(expl.reason.contains("1:1"));
    }

    #[test]
    fn transformer_ratio_explanation_trap_dipole_returns_1to1() {
        let expl =
            transformer_ratio_explanation(CalcMode::Resonant, Some(AntennaModel::TrapDipole));
        assert_eq!(expl.ratio, TransformerRatio::R1To1);
        assert!(expl.reason.contains("Trap"));
        assert!(expl.reason.contains("1:1"));
    }

    #[test]
    fn transformer_ratio_explanation_full_wave_loop_returns_1to1() {
        let expl =
            transformer_ratio_explanation(CalcMode::Resonant, Some(AntennaModel::FullWaveLoop));
        assert_eq!(expl.ratio, TransformerRatio::R1To1);
        assert!(expl.reason.contains("100"));
        assert!(expl.reason.contains("choke"));
    }

    #[test]
    fn transformer_ratio_explanation_ocfd_returns_1to4() {
        let expl = transformer_ratio_explanation(
            CalcMode::Resonant,
            Some(AntennaModel::OffCenterFedDipole),
        );
        assert_eq!(expl.ratio, TransformerRatio::R1To4);
        assert!(expl.reason.contains("200"));
        assert!(expl.reason.contains("1:4"));
    }

    // ── Trap-dipole guidance ──────────────────────────────────────────────────

    fn make_trap_dipole_results(band_indices: Vec<usize>) -> AppResults {
        let config = AppConfig {
            antenna_model: Some(AntennaModel::TrapDipole),
            band_indices,
            velocity_factor: 0.95,
            itu_region: ITURegion::Region2,
            ..Default::default()
        };
        run_calculation(config)
    }

    #[test]
    fn trap_dipole_guidance_view_returns_none_for_non_trap_model() {
        let config = AppConfig {
            antenna_model: Some(AntennaModel::Dipole),
            band_indices: vec![4, 6], // 40m + 20m
            ..Default::default()
        };
        let results = run_calculation(config);
        assert!(trap_dipole_guidance_view(&results).is_none());
    }

    #[test]
    fn trap_dipole_guidance_view_returns_none_for_single_band() {
        let results = make_trap_dipole_results(vec![6]); // only 20m (1-based index 6 → BANDS[5])
        assert!(trap_dipole_guidance_view(&results).is_none());
    }

    #[test]
    fn trap_dipole_guidance_view_40m_20m_pair_is_correct() {
        let results = make_trap_dipole_results(vec![4, 6]); // 40m (idx 4) + 20m (idx 6)
        let view = trap_dipole_guidance_view(&results).expect("should produce guidance");
        assert_eq!(view.sections.len(), 1);
        let s = &view.sections[0];
        // Upper band is 20m (~14.175 MHz), trap at that frequency.
        assert!((s.trap_freq_mhz - 14.175).abs() < 0.5);
        // Inner leg ≈ quarter-wave for 20m × VF 0.95: 71.58/14.175*0.95 ≈ 4.80 m
        assert!(s.inner_leg_m > 4.0 && s.inner_leg_m < 5.5);
        // Outer section should be positive (extends beyond 20m element to reach 40m).
        assert!(s.outer_section_m > 3.0);
        // Total leg ≈ 68.58/7.15*0.95 = 9.11*0.95 ≈ 8.65 m per side.
        assert!(s.total_leg_m > 7.5 && s.total_leg_m < 10.5);
        // Full span is 2× total leg.
        assert!((s.full_span_m - s.total_leg_m * 2.0).abs() < 0.01);
        // Three component examples.
        assert_eq!(s.component_examples.len(), 3);
        // Each example satisfies L*C ≈ 25330/f²
        for ex in &s.component_examples {
            let lc = ex.ind_uh * ex.cap_pf;
            let expected = 25_330.0 / (s.trap_freq_mhz * s.trap_freq_mhz);
            assert!(
                (lc - expected).abs() / expected < 0.001,
                "L×C mismatch: {lc} vs {expected}"
            );
        }
    }

    #[test]
    fn trap_dipole_guidance_view_80m_40m_pair_is_correct() {
        let results = make_trap_dipole_results(vec![2, 4]); // 80m (idx 2) + 40m (idx 4)
        let view = trap_dipole_guidance_view(&results).expect("should produce guidance");
        assert_eq!(view.sections.len(), 1);
        let s = &view.sections[0];
        // Trap should be at 40m band centre (~7.15 MHz).
        assert!((s.trap_freq_mhz - 7.15).abs() < 1.0);
        assert!(s.inner_leg_m > 4.0 && s.inner_leg_m < 12.0);
        assert!(s.outer_section_m > 0.0);
        assert!(s.full_span_m > 15.0);
    }

    #[test]
    fn trap_dipole_guidance_display_lines_contains_key_fields() {
        let results = make_trap_dipole_results(vec![4, 6]);
        let view = trap_dipole_guidance_view(&results).unwrap();
        let lines = trap_dipole_guidance_display_lines(&view, UnitSystem::Both);
        let combined = lines.join("\n");
        assert!(combined.contains("Trap dipole guidance"), "missing heading");
        assert!(combined.contains("MHz"), "missing MHz label");
        assert!(combined.contains("Inner section"), "missing inner section");
        assert!(combined.contains("Outer section"), "missing outer section");
        assert!(combined.contains("Full span"), "missing full span");
        assert!(combined.contains("pF"), "missing capacitor example");
        assert!(combined.contains("μH"), "missing inductor example");
    }

    #[test]
    fn trap_dipole_guidance_appears_in_results_display_document() {
        let results = make_trap_dipole_results(vec![4, 6]);
        let doc = results_display_document(&results);
        let all_lines: Vec<&str> = doc
            .sections
            .iter()
            .flat_map(|s| s.lines.iter().map(|l| l.as_str()))
            .collect();
        let combined = all_lines.join("\n");
        assert!(
            combined.contains("Trap dipole guidance"),
            "guidance section should appear in doc"
        );
    }
}
