//! Named session persistence.
//!
//! Sessions store a complete snapshot of `AppConfig` so the user can recall
//! a full working setup — bands, wire window, transformer ratio, units, etc.
//! — without having to reconfigure everything by hand.
//!
//! Sessions are stored in `~/.config/rusty-wire/sessions.toml` as a TOML
//! array of tables:
//!
//! ```toml
//! [[sessions]]
//! name = "40m EFHW"
//! bands = [4]
//! velocity_factor = 0.95
//! mode = "resonant"
//! wire_min_m = 8.0
//! wire_max_m = 35.0
//! step_m = 0.1
//! units = "both"
//! transformer_ratio = "1:49"
//! antenna_model = "efhw"
//! antenna_height_m = 10.0
//! ground_class = "average"
//! conductor_diameter_mm = 2.0
//! validate_with_fnec = false
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::app::{AntennaModel, AppConfig, CalcMode, UnitSystem};
use crate::bands::ITURegion;
use crate::calculations::{GroundClass, TransformerRatio};

// ---------------------------------------------------------------------------
// Serializable snapshot of a full AppConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// ITU Region number (1, 2, or 3).
    pub region: u8,
    /// Band index list (same indices as used internally).
    pub bands: Vec<usize>,
    /// Velocity factor (0.50–1.00).
    pub velocity_factor: f64,
    /// Calculation mode: `"resonant"` or `"non-resonant"`.
    pub mode: String,
    /// Minimum wire length in metres.
    pub wire_min_m: f64,
    /// Maximum wire length in metres.
    pub wire_max_m: f64,
    /// Search step in metres.
    pub step_m: f64,
    /// Display units: `"metric"`, `"imperial"`, or `"both"`.
    pub units: String,
    /// Transformer impedance ratio, e.g. `"1:9"`.
    pub transformer_ratio: String,
    /// Antenna model label, e.g. `"dipole"`, `"efhw"`.  `null` means "all".
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub antenna_model: Option<String>,
    /// Antenna height in metres.
    pub antenna_height_m: f64,
    /// Ground class: `"poor"`, `"average"`, or `"good"`.
    pub ground_class: String,
    /// Conductor diameter in millimetres.
    pub conductor_diameter_mm: f64,
    /// Whether fnec validation is enabled.
    pub validate_with_fnec: bool,
}

impl SessionConfig {
    /// Build a `SessionConfig` from a live `AppConfig`.
    pub fn from_app_config(config: &AppConfig) -> Self {
        SessionConfig {
            region: match config.itu_region {
                ITURegion::Region1 => 1,
                ITURegion::Region2 => 2,
                ITURegion::Region3 => 3,
            },
            bands: config.band_indices.clone(),
            velocity_factor: config.velocity_factor,
            mode: match config.mode {
                CalcMode::Resonant => "resonant",
                CalcMode::NonResonant => "non-resonant",
            }
            .to_string(),
            wire_min_m: config.wire_min_m,
            wire_max_m: config.wire_max_m,
            step_m: config.step_m,
            units: match config.units {
                UnitSystem::Metric => "metric",
                UnitSystem::Imperial => "imperial",
                UnitSystem::Both => "both",
            }
            .to_string(),
            transformer_ratio: config.transformer_ratio.as_label().to_string(),
            antenna_model: config.antenna_model.map(|m| {
                match m {
                    AntennaModel::Dipole => "dipole",
                    AntennaModel::InvertedVDipole => "inverted-v",
                    AntennaModel::EndFedHalfWave => "efhw",
                    AntennaModel::FullWaveLoop => "loop",
                    AntennaModel::OffCenterFedDipole => "ocfd",
                    AntennaModel::TrapDipole => "trap-dipole",
                    AntennaModel::HybridMultiSection => "hybrid-multi",
                }
                .to_string()
            }),
            antenna_height_m: config.antenna_height_m,
            ground_class: config.ground_class.as_label().to_string(),
            conductor_diameter_mm: config.conductor_diameter_mm,
            validate_with_fnec: config.validate_with_fnec,
        }
    }

    /// Convert back to an `AppConfig`.  Fields that fail to parse fall back
    /// to their compiled-in defaults rather than returning an error.
    pub fn to_app_config(&self) -> AppConfig {
        let mut config = AppConfig::default();

        config.itu_region = match self.region {
            1 => ITURegion::Region1,
            2 => ITURegion::Region2,
            3 => ITURegion::Region3,
            _ => config.itu_region,
        };
        config.band_indices = self.bands.clone();
        config.velocity_factor = self.velocity_factor;
        config.mode = match self.mode.to_ascii_lowercase().as_str() {
            "resonant" => CalcMode::Resonant,
            "non-resonant" | "nonresonant" => CalcMode::NonResonant,
            _ => config.mode,
        };
        config.wire_min_m = self.wire_min_m;
        config.wire_max_m = self.wire_max_m;
        config.step_m = self.step_m;
        config.units = match self.units.to_ascii_lowercase().as_str() {
            "metric" | "m" => UnitSystem::Metric,
            "imperial" | "ft" => UnitSystem::Imperial,
            "both" => UnitSystem::Both,
            _ => config.units,
        };
        if let Some(ratio) = TransformerRatio::parse(&self.transformer_ratio) {
            config.transformer_ratio = ratio;
        }
        config.antenna_model = self
            .antenna_model
            .as_deref()
            .and_then(|s| s.parse::<AntennaModel>().ok());
        config.antenna_height_m = self.antenna_height_m;
        config.ground_class = match self.ground_class.to_ascii_lowercase().as_str() {
            "poor" => GroundClass::Poor,
            "average" => GroundClass::Average,
            "good" => GroundClass::Good,
            _ => config.ground_class,
        };
        config.conductor_diameter_mm = self.conductor_diameter_mm;
        config.validate_with_fnec = self.validate_with_fnec;
        config
    }
}

// ---------------------------------------------------------------------------
// Named session entry and store
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedSession {
    pub name: String,
    #[serde(flatten)]
    pub config: SessionConfig,
}

/// The top-level container serialised to `sessions.toml`.
#[derive(Debug, Default, Serialize, Deserialize)]
struct SessionFile {
    #[serde(default)]
    sessions: Vec<NamedSession>,
}

/// Manages the on-disk session store.
pub struct SessionStore;

impl SessionStore {
    /// Path to `~/.config/rusty-wire/sessions.toml`.
    pub fn sessions_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("rusty-wire")
                .join("sessions.toml"),
        )
    }

    /// Human-readable path string for status messages.
    pub fn sessions_path_display() -> String {
        Self::sessions_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~/.config/rusty-wire/sessions.toml".to_string())
    }

    fn load_file() -> SessionFile {
        let path = match Self::sessions_path() {
            Some(p) => p,
            None => return SessionFile::default(),
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => return SessionFile::default(),
        };
        toml::from_str(&text).unwrap_or_default()
    }

    fn save_file(file: &SessionFile) -> Result<(), String> {
        let path = Self::sessions_path()
            .ok_or_else(|| "HOME environment variable is not set".to_string())?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create config directory: {e}"))?;
        }
        let text = toml::to_string_pretty(file)
            .map_err(|e| format!("failed to serialise sessions: {e}"))?;
        std::fs::write(&path, text).map_err(|e| format!("failed to write sessions: {e}"))?;
        Ok(())
    }

    /// Return all saved session names in insertion order.
    pub fn list() -> Vec<String> {
        Self::load_file()
            .sessions
            .into_iter()
            .map(|s| s.name)
            .collect()
    }

    /// Load all sessions (name + config pairs).
    pub fn load_all() -> Vec<NamedSession> {
        Self::load_file().sessions
    }

    /// Save (or overwrite) a named session.
    pub fn save(name: &str, config: &AppConfig) -> Result<(), String> {
        let mut file = Self::load_file();
        let session_config = SessionConfig::from_app_config(config);
        if let Some(existing) = file.sessions.iter_mut().find(|s| s.name == name) {
            existing.config = session_config;
        } else {
            file.sessions.push(NamedSession {
                name: name.to_string(),
                config: session_config,
            });
        }
        Self::save_file(&file)
    }

    /// Delete a session by name.  Returns `Ok(true)` if it was found and
    /// removed, `Ok(false)` if no session with that name exists.
    pub fn delete(name: &str) -> Result<bool, String> {
        let mut file = Self::load_file();
        let before = file.sessions.len();
        file.sessions.retain(|s| s.name != name);
        let removed = file.sessions.len() < before;
        if removed {
            Self::save_file(&file)?;
        }
        Ok(removed)
    }

    /// Load a single named session's `AppConfig`, or `None` if not found.
    pub fn load_config(name: &str) -> Option<AppConfig> {
        Self::load_file()
            .sessions
            .into_iter()
            .find(|s| s.name == name)
            .map(|s| s.config.to_app_config())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> AppConfig {
        use crate::app::{AntennaModel, CalcMode, UnitSystem};
        use crate::bands::ITURegion;
        use crate::calculations::{GroundClass, TransformerRatio};
        AppConfig {
            band_indices: vec![4, 6],
            velocity_factor: 0.97,
            mode: CalcMode::Resonant,
            wire_min_m: 10.0,
            wire_max_m: 40.0,
            step_m: 0.1,
            units: UnitSystem::Both,
            itu_region: ITURegion::Region1,
            transformer_ratio: TransformerRatio::R1To9,
            antenna_model: Some(AntennaModel::EndFedHalfWave),
            antenna_height_m: 12.0,
            ground_class: GroundClass::Good,
            conductor_diameter_mm: 2.0,
            custom_freq_mhz: None,
            freq_list_mhz: vec![],
            validate_with_fnec: false,
            extra_bands: vec![],
        }
    }

    #[test]
    fn session_config_round_trips_through_app_config() {
        let original = sample_config();
        let session = SessionConfig::from_app_config(&original);
        let restored = session.to_app_config();

        assert_eq!(restored.band_indices, original.band_indices);
        assert_eq!(restored.velocity_factor, original.velocity_factor);
        assert_eq!(restored.mode, original.mode);
        assert_eq!(restored.wire_min_m, original.wire_min_m);
        assert_eq!(restored.wire_max_m, original.wire_max_m);
        assert_eq!(restored.step_m, original.step_m);
        assert_eq!(restored.units, original.units);
        assert_eq!(restored.transformer_ratio, original.transformer_ratio);
        assert_eq!(restored.antenna_model, original.antenna_model);
        assert_eq!(restored.antenna_height_m, original.antenna_height_m);
        assert_eq!(restored.ground_class, original.ground_class);
        assert_eq!(
            restored.conductor_diameter_mm,
            original.conductor_diameter_mm
        );
        assert_eq!(restored.validate_with_fnec, original.validate_with_fnec);
    }

    #[test]
    fn session_config_serialises_to_toml_with_expected_keys() {
        let config = sample_config();
        let session = SessionConfig::from_app_config(&config);
        let toml_str = toml::to_string_pretty(&session).expect("should serialise");

        assert!(toml_str.contains("transformer_ratio = \"1:9\""));
        assert!(toml_str.contains("antenna_model = \"efhw\""));
        assert!(toml_str.contains("mode = \"resonant\""));
        assert!(toml_str.contains("units = \"both\""));
        assert!(toml_str.contains("velocity_factor = 0.97"));
    }

    #[test]
    fn session_config_none_antenna_model_omits_key() {
        let mut config = sample_config();
        config.antenna_model = None;
        let session = SessionConfig::from_app_config(&config);
        let toml_str = toml::to_string_pretty(&session).expect("should serialise");
        assert!(
            !toml_str.contains("antenna_model"),
            "None should omit the key"
        );
    }

    #[test]
    fn session_config_to_app_config_tolerates_unknown_values() {
        let session = SessionConfig {
            region: 99,
            bands: vec![4],
            velocity_factor: 0.95,
            mode: "bogus".to_string(),
            wire_min_m: 8.0,
            wire_max_m: 35.0,
            step_m: 0.1,
            units: "bogus".to_string(),
            transformer_ratio: "1:9".to_string(),
            antenna_model: None,
            antenna_height_m: 10.0,
            ground_class: "average".to_string(),
            conductor_diameter_mm: 2.0,
            validate_with_fnec: false,
        };
        // Should not panic — falls back to defaults.
        let config = session.to_app_config();
        assert_eq!(config.band_indices, vec![4]);
    }
}

// ---------------------------------------------------------------------------
// Persistence roundtrip tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use crate::test_env::with_temp_home;

    fn sample_config() -> AppConfig {
        use crate::app::{AntennaModel, CalcMode, UnitSystem};
        use crate::bands::ITURegion;
        use crate::calculations::{GroundClass, TransformerRatio};
        AppConfig {
            band_indices: vec![4, 6],
            velocity_factor: 0.97,
            mode: CalcMode::Resonant,
            wire_min_m: 10.0,
            wire_max_m: 40.0,
            step_m: 0.1,
            units: UnitSystem::Both,
            itu_region: ITURegion::Region1,
            transformer_ratio: TransformerRatio::R1To9,
            antenna_model: Some(AntennaModel::EndFedHalfWave),
            antenna_height_m: 12.0,
            ground_class: GroundClass::Good,
            conductor_diameter_mm: 2.0,
            custom_freq_mhz: None,
            freq_list_mhz: vec![],
            validate_with_fnec: false,
            extra_bands: vec![],
        }
    }

    #[test]
    fn save_and_list_round_trips() {
        with_temp_home(|| {
            let cfg = sample_config();
            SessionStore::save("my-session", &cfg).expect("save");
            let names = SessionStore::list();
            assert_eq!(names, vec!["my-session"]);
        });
    }

    #[test]
    fn save_load_config_round_trips_fields() {
        with_temp_home(|| {
            let cfg = sample_config();
            SessionStore::save("round-trip", &cfg).expect("save");
            let loaded = SessionStore::load_config("round-trip").expect("load");
            assert_eq!(loaded.band_indices, cfg.band_indices);
            assert_eq!(loaded.velocity_factor, cfg.velocity_factor);
            assert_eq!(loaded.mode, cfg.mode);
            assert_eq!(loaded.units, cfg.units);
            assert_eq!(loaded.antenna_model, cfg.antenna_model);
            assert_eq!(loaded.transformer_ratio, cfg.transformer_ratio);
        });
    }

    #[test]
    fn save_overwrites_existing_session() {
        with_temp_home(|| {
            let mut cfg = sample_config();
            SessionStore::save("overwrite-me", &cfg).expect("first save");
            cfg.velocity_factor = 0.85;
            SessionStore::save("overwrite-me", &cfg).expect("second save");
            let names = SessionStore::list();
            assert_eq!(
                names.len(),
                1,
                "should still be one session after overwrite"
            );
            let loaded = SessionStore::load_config("overwrite-me").expect("load");
            assert_eq!(loaded.velocity_factor, 0.85);
        });
    }

    #[test]
    fn delete_removes_session_and_returns_true() {
        with_temp_home(|| {
            let cfg = sample_config();
            SessionStore::save("to-delete", &cfg).expect("save");
            let removed = SessionStore::delete("to-delete").expect("delete");
            assert!(removed);
            assert!(SessionStore::list().is_empty());
        });
    }

    #[test]
    fn delete_missing_session_returns_false() {
        with_temp_home(|| {
            let removed = SessionStore::delete("does-not-exist").expect("delete");
            assert!(!removed);
        });
    }

    #[test]
    fn load_config_missing_session_returns_none() {
        with_temp_home(|| {
            let result = SessionStore::load_config("ghost");
            assert!(result.is_none());
        });
    }

    #[test]
    fn multiple_sessions_preserved_in_insertion_order() {
        with_temp_home(|| {
            let cfg = sample_config();
            SessionStore::save("alpha", &cfg).expect("save alpha");
            SessionStore::save("beta", &cfg).expect("save beta");
            SessionStore::save("gamma", &cfg).expect("save gamma");
            let names = SessionStore::list();
            assert_eq!(names, vec!["alpha", "beta", "gamma"]);
        });
    }
}
