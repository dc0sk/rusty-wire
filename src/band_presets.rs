use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

use crate::bands::OwnedBand;

#[derive(Debug, Deserialize)]
struct BandPresetFile {
    presets: Option<HashMap<String, Vec<String>>>,
    band: Option<Vec<OwnedBand>>,
}

fn read_bands_file(path: &str) -> Result<BandPresetFile, String> {
    let file_text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read bands config '{path}': {err}"))?;
    toml::from_str(&file_text)
        .map_err(|err| format!("failed to parse bands config '{path}': {err}"))
}

fn read_preset_map(path: &str) -> Result<HashMap<String, Vec<String>>, String> {
    let parsed = read_bands_file(path)?;
    parsed
        .presets
        .ok_or_else(|| format!("bands config '{path}' does not define a [presets] table"))
}

/// Load user-defined `[[band]]` entries from a bands TOML file.
///
/// Returns an empty `Vec` (not an error) when the file does not exist or
/// contains no `[[band]]` entries — both are valid states for callers that
/// load bands opportunistically.
pub(crate) fn load_custom_bands(path: &str) -> Result<Vec<OwnedBand>, String> {
    if !std::path::Path::new(path).exists() {
        return Ok(Vec::new());
    }
    let parsed = read_bands_file(path)?;
    let bands = parsed.band.unwrap_or_default();
    for band in &bands {
        band.validate().map_err(|e| format!("in '{path}': {e}"))?;
    }
    Ok(bands)
}

fn normalize_preset_tokens(
    tokens: &[String],
    preset_name: &str,
    path: &str,
) -> Result<String, String> {
    if tokens.is_empty() {
        return Err(format!(
            "preset '{preset_name}' in '{path}' has no band entries"
        ));
    }

    let normalized: Vec<String> = tokens
        .iter()
        .map(|token| token.trim())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect();

    if normalized.is_empty() {
        return Err(format!(
            "preset '{preset_name}' in '{path}' has only empty band entries"
        ));
    }

    Ok(normalized.join(","))
}

pub(crate) fn load_named_presets(path: &str) -> Result<Vec<(String, String)>, String> {
    let presets = read_preset_map(path)?;
    let mut named: Vec<(String, String)> = presets
        .into_iter()
        .map(|(name, tokens)| {
            let normalized = normalize_preset_tokens(&tokens, &name, path)?;
            Ok((name, normalized))
        })
        .collect::<Result<Vec<_>, String>>()?;

    named.sort_by(|left, right| {
        left.0
            .to_ascii_lowercase()
            .cmp(&right.0.to_ascii_lowercase())
    });
    Ok(named)
}

pub fn load_preset_selection(path: &str, preset_name: &str) -> Result<String, String> {
    let presets = read_preset_map(path)?;

    let requested = preset_name.trim();
    if requested.is_empty() {
        return Err("preset name cannot be empty".to_string());
    }

    let entry = presets
        .get(requested)
        .or_else(|| {
            let requested_lower = requested.to_ascii_lowercase();
            presets
                .iter()
                .find(|(name, _)| name.to_ascii_lowercase() == requested_lower)
                .map(|(_, bands)| bands)
        })
        .ok_or_else(|| {
            let mut known: Vec<&str> = presets.keys().map(String::as_str).collect();
            known.sort_unstable();
            format!(
                "unknown preset '{requested}' in '{path}'. Available presets: {}",
                known.join(", ")
            )
        })?;

    normalize_preset_tokens(entry, requested, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("rusty-wire-{name}-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("failed to create temp test dir");
        dir
    }

    fn write_file(path: &PathBuf, content: &str) {
        let mut file = std::fs::File::create(path).expect("failed to create test file");
        file.write_all(content.as_bytes())
            .expect("failed to write test file");
    }

    #[test]
    fn load_preset_selection_reads_known_preset() {
        let dir = temp_test_dir("preset-loader-ok");
        let config_path = dir.join("bands.toml");
        write_file(
            &config_path,
            r#"
[presets]
portable = ["40m", "20m", "15m", "10m"]
"#,
        );

        let result = load_preset_selection(config_path.to_str().unwrap(), "portable")
            .expect("preset should load");
        assert_eq!(result, "40m,20m,15m,10m");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_preset_selection_supports_case_insensitive_lookup() {
        let dir = temp_test_dir("preset-loader-case");
        let config_path = dir.join("bands.toml");
        write_file(
            &config_path,
            r#"
[presets]
PortableDX = ["80m-10m"]
"#,
        );

        let result = load_preset_selection(config_path.to_str().unwrap(), "portabledx")
            .expect("preset should load");
        assert_eq!(result, "80m-10m");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_preset_selection_rejects_unknown_preset() {
        let dir = temp_test_dir("preset-loader-missing");
        let config_path = dir.join("bands.toml");
        write_file(
            &config_path,
            r#"
[presets]
portable = ["40m", "20m"]
"#,
        );

        let err = load_preset_selection(config_path.to_str().unwrap(), "unknown")
            .expect_err("unknown preset should fail");
        assert!(err.contains("Available presets"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_named_presets_returns_sorted_normalized_entries() {
        let dir = temp_test_dir("preset-loader-list");
        let config_path = dir.join("bands.toml");
        write_file(
            &config_path,
            r#"
[presets]
zulu = ["40m", "20m"]
Alpha = [" 80m-10m "]
"#,
        );

        let presets =
            load_named_presets(config_path.to_str().unwrap()).expect("named presets should load");
        assert_eq!(
            presets,
            vec![
                ("Alpha".to_string(), "80m-10m".to_string()),
                ("zulu".to_string(), "40m,20m".to_string()),
            ]
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_custom_bands_returns_empty_when_file_missing() {
        let result = load_custom_bands("/tmp/nonexistent-rusty-wire-bands-12345.toml");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn load_custom_bands_parses_band_entries() {
        let dir = temp_test_dir("custom-bands-parse");
        let config_path = dir.join("bands.toml");
        write_file(
            &config_path,
            r#"
[[band]]
name = "FT8-40m"
freq_low_mhz = 7.074
freq_high_mhz = 7.076

[[band]]
name = "WSPR-20m"
freq_low_mhz = 14.0956
freq_high_mhz = 14.0956
freq_center_mhz = 14.0956
"#,
        );

        let bands =
            load_custom_bands(config_path.to_str().unwrap()).expect("custom bands should load");
        assert_eq!(bands.len(), 2);
        assert_eq!(bands[0].name, "FT8-40m");
        assert!((bands[0].center_mhz() - 7.075).abs() < 1e-6);
        assert_eq!(bands[1].name, "WSPR-20m");
        assert!((bands[1].center_mhz() - 14.0956).abs() < 1e-6);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_custom_bands_coexists_with_presets() {
        let dir = temp_test_dir("custom-bands-mixed");
        let config_path = dir.join("bands.toml");
        write_file(
            &config_path,
            r#"
[presets]
portable = ["40m", "20m"]

[[band]]
name = "FT8-40m"
freq_low_mhz = 7.074
freq_high_mhz = 7.076
"#,
        );

        let bands =
            load_custom_bands(config_path.to_str().unwrap()).expect("mixed file should load");
        assert_eq!(bands.len(), 1);
        assert_eq!(bands[0].name, "FT8-40m");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_custom_bands_rejects_invalid_frequency_range() {
        let dir = temp_test_dir("custom-bands-invalid");
        let config_path = dir.join("bands.toml");
        write_file(
            &config_path,
            r#"
[[band]]
name = "bad"
freq_low_mhz = 20.0
freq_high_mhz = 10.0
"#,
        );

        let err = load_custom_bands(config_path.to_str().unwrap())
            .expect_err("invalid range should fail");
        assert!(
            err.contains("freq_high_mhz"),
            "error should mention the field: {err}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
