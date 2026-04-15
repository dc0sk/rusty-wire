#![allow(dead_code)]

use crate::app::{AntennaModel, AppRequestDraft, CalcMode, UnitSystem, DEFAULT_BAND_SELECTION};
use crate::bands::ITURegion;
use crate::calculations::{TransformerRatio, DEFAULT_NON_RESONANT_CONFIG};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiFocus {
    Inputs,
    Results,
    Export,
}

#[derive(Debug, Clone)]
pub struct TuiState {
    pub focus: TuiFocus,
    pub status_message: Option<String>,
    pub band_indices: Vec<usize>,
    pub mode: CalcMode,
    pub antenna_model: Option<AntennaModel>,
    pub velocity_factor: f64,
    pub transformer_ratio: TransformerRatio,
    pub wire_min_m: f64,
    pub wire_max_m: f64,
    pub default_units: UnitSystem,
    pub selected_units: Option<UnitSystem>,
    pub itu_region: ITURegion,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            focus: TuiFocus::Inputs,
            status_message: None,
            band_indices: DEFAULT_BAND_SELECTION.to_vec(),
            mode: CalcMode::Resonant,
            antenna_model: None,
            velocity_factor: 0.95,
            transformer_ratio: TransformerRatio::R1To1,
            wire_min_m: DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            wire_max_m: DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            default_units: UnitSystem::Both,
            selected_units: None,
            itu_region: ITURegion::Region1,
        }
    }
}

impl TuiState {
    pub fn to_request_draft(&self) -> AppRequestDraft {
        AppRequestDraft {
            band_indices: self.band_indices.clone(),
            velocity_factor: self.velocity_factor,
            mode: self.mode,
            wire_min_m: self.wire_min_m,
            wire_max_m: self.wire_max_m,
            default_units: self.default_units,
            selected_units: self.selected_units,
            itu_region: self.itu_region,
            transformer_ratio: self.transformer_ratio,
            antenna_model: self.antenna_model,
        }
    }

    pub fn set_focus(&mut self, focus: TuiFocus) {
        self.focus = focus;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tui_state_defaults_to_inputs_focus() {
        let state = TuiState::default();

        assert_eq!(state.focus, TuiFocus::Inputs);
        assert_eq!(state.band_indices, DEFAULT_BAND_SELECTION.to_vec());
        assert_eq!(state.mode, CalcMode::Resonant);
        assert!(state.selected_units.is_none());
    }

    #[test]
    fn tui_state_translates_to_request_draft() {
        let state = TuiState {
            focus: TuiFocus::Results,
            status_message: Some("ready".to_string()),
            band_indices: vec![4, 6],
            mode: CalcMode::NonResonant,
            antenna_model: Some(AntennaModel::EndFedHalfWave),
            velocity_factor: 0.9,
            transformer_ratio: TransformerRatio::R1To56,
            wire_min_m: 12.0,
            wire_max_m: 24.0,
            default_units: UnitSystem::Metric,
            selected_units: Some(UnitSystem::Both),
            itu_region: ITURegion::Region2,
        };

        let draft = state.to_request_draft();

        assert_eq!(draft.band_indices, vec![4, 6]);
        assert_eq!(draft.mode, CalcMode::NonResonant);
        assert_eq!(draft.selected_units, Some(UnitSystem::Both));
        assert_eq!(draft.itu_region, ITURegion::Region2);
        assert_eq!(draft.antenna_model, Some(AntennaModel::EndFedHalfWave));
    }
}