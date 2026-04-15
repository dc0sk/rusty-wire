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

#[derive(Debug, Clone, PartialEq)]
pub enum TuiAction {
    SetFocus(TuiFocus),
    SetStatusMessage(Option<String>),
    SetBandIndices(Vec<usize>),
    SetMode(CalcMode),
    SetAntennaModel(Option<AntennaModel>),
    SetVelocityFactor(f64),
    SetTransformerRatio(TransformerRatio),
    SetWireWindow { min_m: f64, max_m: f64 },
    SetDefaultUnits(UnitSystem),
    SetSelectedUnits(Option<UnitSystem>),
    SetRegion(ITURegion),
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

    pub fn update(&mut self, action: TuiAction) {
        match action {
            TuiAction::SetFocus(focus) => self.focus = focus,
            TuiAction::SetStatusMessage(message) => self.status_message = message,
            TuiAction::SetBandIndices(band_indices) => self.band_indices = band_indices,
            TuiAction::SetMode(mode) => self.mode = mode,
            TuiAction::SetAntennaModel(antenna_model) => self.antenna_model = antenna_model,
            TuiAction::SetVelocityFactor(velocity_factor) => {
                self.velocity_factor = velocity_factor;
            }
            TuiAction::SetTransformerRatio(transformer_ratio) => {
                self.transformer_ratio = transformer_ratio;
            }
            TuiAction::SetWireWindow { min_m, max_m } => {
                self.wire_min_m = min_m;
                self.wire_max_m = max_m;
            }
            TuiAction::SetDefaultUnits(default_units) => self.default_units = default_units,
            TuiAction::SetSelectedUnits(selected_units) => {
                self.selected_units = selected_units;
            }
            TuiAction::SetRegion(itu_region) => self.itu_region = itu_region,
        }
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

    #[test]
    fn tui_state_update_applies_actions() {
        let mut state = TuiState::default();

        state.update(TuiAction::SetFocus(TuiFocus::Results));
        state.update(TuiAction::SetStatusMessage(Some("updated".to_string())));
        state.update(TuiAction::SetMode(CalcMode::NonResonant));
        state.update(TuiAction::SetVelocityFactor(0.9));
        state.update(TuiAction::SetRegion(ITURegion::Region3));
        state.update(TuiAction::SetSelectedUnits(Some(UnitSystem::Both)));

        assert_eq!(state.focus, TuiFocus::Results);
        assert_eq!(state.status_message.as_deref(), Some("updated"));
        assert_eq!(state.mode, CalcMode::NonResonant);
        assert_eq!(state.velocity_factor, 0.9);
        assert_eq!(state.itu_region, ITURegion::Region3);
        assert_eq!(state.selected_units, Some(UnitSystem::Both));
    }
}
