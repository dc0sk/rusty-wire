//! Persistent user preference storage.
//!
//! Preferences are loaded from and saved to
//! `$HOME/.config/rusty-wire/config.toml` using TOML format.  All fields
//! are optional; an absent field means "use the compiled-in default".
//!
//! Example file:
//! ```toml
//! region = 2
//! mode = "non-resonant"
//! velocity_factor = 0.97
//! antenna_height_m = 12.0
//! ground_class = "good"
//! conductor_diameter_mm = 2.5
//! units = "both"
//! ```
//!
//! The `--save-prefs` CLI flag and the `s` key in the TUI both write the
//! current resolved configuration back to this file.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::app::{AppConfig, CalcMode, UnitSystem};
use crate::bands::ITURegion;
use crate::calculations::GroundClass;

/// Persistent defaults that the user can set via `--save-prefs` (CLI) or
/// the `s` keybind (TUI).  All fields are `Option<T>` so that unset fields
/// fall through to the compiled-in defaults instead of overriding them.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UserPrefs {
    /// ITU Region: 1 (EU/AF/Middle East), 2 (Americas), 3 (Asia-Pacific).
    pub region: Option<u8>,
    /// Calculation mode: `"resonant"` or `"non-resonant"`.
    pub mode: Option<String>,
    /// Velocity factor (0.50–1.00).
    pub velocity_factor: Option<f64>,
    /// Antenna height in metres; standard presets are 7, 10, or 12.
    pub antenna_height_m: Option<f64>,
    /// Ground class: `"poor"`, `"average"`, or `"good"`.
    pub ground_class: Option<String>,
    /// Conductor diameter in millimetres (1.0–4.0).
    pub conductor_diameter_mm: Option<f64>,
    /// Display units: `"metric"`, `"imperial"`, or `"both"`.
    pub units: Option<String>,
}

impl UserPrefs {
    // ── Path helpers ─────────────────────────────────────────────────────────

    /// Return the absolute path to the preferences file, derived from `$HOME`.
    /// Returns `None` when `HOME` is not set (unusual, but possible in
    /// stripped containers or CI environments).
    pub fn prefs_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("rusty-wire")
                .join("config.toml"),
        )
    }

    /// Return a human-readable path string suitable for status messages.
    pub fn prefs_path_display() -> String {
        Self::prefs_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~/.config/rusty-wire/config.toml".to_string())
    }

    // ── I/O ──────────────────────────────────────────────────────────────────

    /// Load preferences from disk.  Returns `UserPrefs::default()` silently
    /// when the file does not exist, is unreadable, or cannot be parsed —
    /// callers never need to handle errors from this function.
    pub fn load() -> Self {
        let path = match Self::prefs_path() {
            Some(p) => p,
            None => return Self::default(),
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => return Self::default(),
        };
        toml::from_str(&text).unwrap_or_default()
    }

    /// Persist the current preferences to `~/.config/rusty-wire/config.toml`,
    /// creating the directory if it does not exist.
    pub fn save(&self) -> Result<(), String> {
        let path =
            Self::prefs_path().ok_or_else(|| "HOME environment variable is not set".to_string())?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create config directory: {e}"))?;
        }
        let text = toml::to_string_pretty(self)
            .map_err(|e| format!("failed to serialise preferences: {e}"))?;
        std::fs::write(&path, text).map_err(|e| format!("failed to write preferences: {e}"))?;
        Ok(())
    }

    // ── Type conversion helpers ───────────────────────────────────────────────

    /// Decode the stored `region` field to an `ITURegion`, if set and valid.
    pub fn itu_region(&self) -> Option<ITURegion> {
        match self.region? {
            1 => Some(ITURegion::Region1),
            2 => Some(ITURegion::Region2),
            3 => Some(ITURegion::Region3),
            _ => None,
        }
    }

    /// Decode the stored `mode` field to a `CalcMode`, if set and valid.
    pub fn calc_mode(&self) -> Option<CalcMode> {
        match self.mode.as_deref()?.to_ascii_lowercase().as_str() {
            "resonant" => Some(CalcMode::Resonant),
            "non-resonant" | "nonresonant" => Some(CalcMode::NonResonant),
            _ => None,
        }
    }

    /// Decode the stored `units` field to a `UnitSystem`, if set and valid.
    pub fn unit_system(&self) -> Option<UnitSystem> {
        match self.units.as_deref()?.to_ascii_lowercase().as_str() {
            "m" | "metric" => Some(UnitSystem::Metric),
            "ft" | "imperial" => Some(UnitSystem::Imperial),
            "both" => Some(UnitSystem::Both),
            _ => None,
        }
    }

    /// Decode the stored `ground_class` field to a `GroundClass`, if set and valid.
    pub fn ground_class_value(&self) -> Option<GroundClass> {
        match self.ground_class.as_deref()?.to_ascii_lowercase().as_str() {
            "poor" => Some(GroundClass::Poor),
            "average" => Some(GroundClass::Average),
            "good" => Some(GroundClass::Good),
            _ => None,
        }
    }

    // ── Apply / snapshot ─────────────────────────────────────────────────────

    /// Apply every stored field to `config` in-place, overriding only the
    /// fields for which a preference is explicitly stored.  Fields not present
    /// in the preference file are left at their current (default) values.
    pub fn apply_to_config(&self, config: &mut AppConfig) {
        if let Some(region) = self.itu_region() {
            config.itu_region = region;
        }
        if let Some(mode) = self.calc_mode() {
            config.mode = mode;
        }
        if let Some(vf) = self.velocity_factor {
            config.velocity_factor = vf;
        }
        if let Some(height) = self.antenna_height_m {
            config.antenna_height_m = height;
        }
        if let Some(gc) = self.ground_class_value() {
            config.ground_class = gc;
        }
        if let Some(cd) = self.conductor_diameter_mm {
            config.conductor_diameter_mm = cd;
        }
        if let Some(units) = self.unit_system() {
            config.units = units;
        }
    }

    /// Snapshot the preference-eligible fields from `config` into a new
    /// `UserPrefs`.  Used by `--save-prefs` and the TUI `s` keybind.
    pub fn from_config(config: &AppConfig) -> Self {
        UserPrefs {
            region: Some(match config.itu_region {
                ITURegion::Region1 => 1,
                ITURegion::Region2 => 2,
                ITURegion::Region3 => 3,
            }),
            mode: Some(
                match config.mode {
                    CalcMode::Resonant => "resonant",
                    CalcMode::NonResonant => "non-resonant",
                }
                .to_string(),
            ),
            velocity_factor: Some(config.velocity_factor),
            antenna_height_m: Some(config.antenna_height_m),
            ground_class: Some(
                match config.ground_class {
                    GroundClass::Poor => "poor",
                    GroundClass::Average => "average",
                    GroundClass::Good => "good",
                }
                .to_string(),
            ),
            conductor_diameter_mm: Some(config.conductor_diameter_mm),
            units: Some(
                match config.units {
                    UnitSystem::Metric => "metric",
                    UnitSystem::Imperial => "imperial",
                    UnitSystem::Both => "both",
                }
                .to_string(),
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Persistence roundtrip tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use crate::test_env::with_temp_home;

    #[test]
    fn save_and_load_round_trips_all_fields() {
        with_temp_home(|| {
            let prefs = UserPrefs {
                region: Some(2),
                mode: Some("non-resonant".to_string()),
                velocity_factor: Some(0.95),
                antenna_height_m: Some(12.0),
                ground_class: Some("good".to_string()),
                conductor_diameter_mm: Some(2.5),
                units: Some("both".to_string()),
            };
            prefs.save().expect("save prefs");
            let loaded = UserPrefs::load();
            assert_eq!(loaded.region, Some(2));
            assert_eq!(loaded.mode.as_deref(), Some("non-resonant"));
            assert_eq!(loaded.velocity_factor, Some(0.95));
            assert_eq!(loaded.antenna_height_m, Some(12.0));
            assert_eq!(loaded.ground_class.as_deref(), Some("good"));
            assert_eq!(loaded.conductor_diameter_mm, Some(2.5));
            assert_eq!(loaded.units.as_deref(), Some("both"));
        });
    }

    #[test]
    fn load_returns_defaults_when_no_file_exists() {
        with_temp_home(|| {
            let prefs = UserPrefs::load();
            assert!(prefs.region.is_none());
            assert!(prefs.mode.is_none());
            assert!(prefs.velocity_factor.is_none());
        });
    }

    #[test]
    fn save_creates_parent_directory() {
        with_temp_home(|| {
            let prefs = UserPrefs {
                region: Some(1),
                ..Default::default()
            };
            prefs.save().expect("save should create dir");
            let path = UserPrefs::prefs_path().expect("path");
            assert!(path.exists(), "config file should exist after save");
        });
    }

    #[test]
    fn itu_region_decoded_correctly_from_saved_prefs() {
        with_temp_home(|| {
            let prefs = UserPrefs {
                region: Some(3),
                ..Default::default()
            };
            prefs.save().expect("save");
            let loaded = UserPrefs::load();
            assert_eq!(loaded.itu_region(), Some(ITURegion::Region3));
        });
    }

    #[test]
    fn overwrite_updates_existing_file() {
        with_temp_home(|| {
            let first = UserPrefs {
                region: Some(1),
                ..Default::default()
            };
            first.save().expect("first save");
            let second = UserPrefs {
                region: Some(2),
                ..Default::default()
            };
            second.save().expect("second save");
            let loaded = UserPrefs::load();
            assert_eq!(loaded.region, Some(2));
        });
    }
}
