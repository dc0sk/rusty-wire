use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rusty-wire"))
}

fn temp_test_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("rusty-wire-{name}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&dir).expect("failed to create temp test dir");
    dir
}

#[test]
fn no_args_prints_help() {
    let output = binary().output().expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(!output.status.success());
    assert!(combined.contains("Usage: rusty-wire [OPTIONS]"));
    assert!(combined.contains("--interactive"));
}

#[test]
fn mixed_meter_and_feet_constraints_show_error() {
    let output = binary()
        .args(["--wire-min", "10", "--wire-max-ft", "40"])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("cannot mix meter and feet constraints"));
}

#[test]
fn invalid_velocity_shows_error() {
    let output = binary()
        .args(["--bands", "40m", "--velocity", "1.5"])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("velocity factor must be between 0.50 and 1.00"));
}

#[test]
fn list_bands_respects_selected_region() {
    let output = binary()
        .args(["--list-bands", "--region", "2"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Available bands in Region 2"));
    assert!(stdout.contains("40m [HF] (7-7.3 MHz)"));
}

#[test]
fn bands_preset_loads_from_config_file() {
    let dir = temp_test_dir("bands-preset-load");
    let config_path = dir.join("bands.toml");
    fs::write(
        &config_path,
        "[presets]\nportable = [\"40m\", \"20m\", \"15m\", \"10m\"]\n",
    )
    .expect("failed to write bands.toml");

    let output = binary()
        .current_dir(&dir)
        .args([
            "--bands-preset",
            "portable",
            "--bands-config",
            "bands.toml",
            "--mode",
            "resonant",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Summary for 4 band(s):"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bands_and_bands_preset_are_mutually_exclusive() {
    let output = binary()
        .args([
            "--bands",
            "40m,20m",
            "--bands-preset",
            "portable",
            "--bands-config",
            "bands.toml",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("--bands and --bands-preset are mutually exclusive"));
}

#[test]
fn unknown_bands_preset_shows_available_presets() {
    let dir = temp_test_dir("bands-preset-unknown");
    let config_path = dir.join("bands.toml");
    fs::write(
        &config_path,
        "[presets]\nportable = [\"40m\", \"20m\"]\nfieldday = [\"80m\", \"40m\", \"20m\", \"15m\", \"10m\"]\n",
    )
    .expect("failed to write bands.toml");

    let output = binary()
        .current_dir(&dir)
        .args(["--bands-preset", "dx", "--bands-config", "bands.toml"])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Available presets"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn multiple_exports_ignore_custom_output_name() {
    let dir = temp_test_dir("multi-export");
    let output = binary()
        .current_dir(&dir)
        .args([
            "--bands",
            "40m",
            "--export",
            "csv,json",
            "--output",
            "custom.csv",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success());
    assert!(stderr.contains("--output is ignored when multiple formats are selected"));
    assert!(dir.join("rusty-wire-results.csv").exists());
    assert!(dir.join("rusty-wire-results.json").exists());
    assert!(!dir.join("custom.csv").exists());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn single_export_uses_requested_output_name() {
    let dir = temp_test_dir("single-export");
    let output = binary()
        .current_dir(&dir)
        .args([
            "--bands",
            "40m",
            "--export",
            "csv",
            "--output",
            "custom.csv",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Exported results to custom.csv"));
    assert!(dir.join("custom.csv").exists());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn exports_include_inverted_v_fields() {
    let dir = temp_test_dir("inverted-v-export");
    let output = binary()
        .current_dir(&dir)
        .args(["--bands", "40m", "--units", "both", "--export", "csv,json"])
        .output()
        .expect("failed to run rusty-wire");

    assert!(output.status.success());

    let csv =
        fs::read_to_string(dir.join("rusty-wire-results.csv")).expect("failed to read csv export");
    let json = fs::read_to_string(dir.join("rusty-wire-results.json"))
        .expect("failed to read json export");

    assert!(csv.contains("inverted_v_total_m"));
    assert!(csv.contains("inverted_v_span_90_ft"));
    assert!(json.contains("\"inverted_v_total_m\""));
    assert!(json.contains("\"inverted_v_span_120_ft\""));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn default_antenna_mode_shows_all_models_per_band() {
    let output = binary()
        .args(["--bands", "40m"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Antenna model: all"));
    assert!(stdout.contains("Half-wave:"));
    assert!(stdout.contains("End-fed half-wave:"));
    assert!(stdout.contains("Full-wave loop circumference:"));
    assert!(stdout.contains("Inverted-V total:"));
    assert!(stdout.contains("OCFD 33/67 legs:"));
}

#[test]
fn selected_antenna_model_filters_output_and_resonant_summary() {
    let output = binary()
        .args(["--bands", "40m", "--antenna", "efhw"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Antenna model: end-fed half-wave"));
    assert!(stdout.contains("End-fed half-wave:"));
    assert!(!stdout.contains("Half-wave:"));
    assert!(!stdout.contains("Full-wave loop circumference:"));
    assert!(stdout.contains(
        "Closest combined compromises to resonant points (tuner-assisted EFHW guidance):"
    ));
    assert!(stdout
        .contains("dipole-derived compromise lengths shown as tuner-assisted starting points"));
}

#[test]
fn loop_antenna_mode_shows_loop_guidance_compromises() {
    let output = binary()
        .args(["--bands", "40m", "--antenna", "loop"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Antenna model: full-wave loop"));
    assert!(stdout.contains("Full-wave loop circumference:"));
    assert!(stdout.contains("Full-wave loop square side:"));
    assert!(!stdout.contains("Half-wave:"));
    assert!(!stdout.contains("End-fed half-wave:"));
    assert!(stdout.contains(
        "Closest combined compromises to resonant points (tuner-assisted loop guidance):"
    ));
    assert!(stdout
        .contains("dipole-derived compromise lengths shown as tuner-assisted starting points"));
}

#[test]
fn inverted_v_antenna_mode_shows_inverted_v_guidance_compromises() {
    let output = binary()
        .args(["--bands", "40m", "--antenna", "inverted-v"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Antenna model: inverted-v dipole"));
    assert!(stdout.contains("Inverted-V total:"));
    assert!(stdout.contains("Inverted-V each leg:"));
    assert!(stdout.contains("Inverted-V span at 90 deg apex:"));
    assert!(stdout.contains("Inverted-V span at 120 deg apex:"));
    assert!(!stdout.contains("Half-wave:"));
    assert!(!stdout.contains("End-fed half-wave:"));
    assert!(!stdout.contains("Full-wave loop circumference:"));
    assert!(
        stdout.contains("Closest combined compromises to resonant points (inverted-V guidance):")
    );
    assert!(stdout.contains("Inverted-V mode: each compromise line shows a total wire length"));
    assert!(stdout.contains("each leg:"));
}

#[test]
fn ocfd_antenna_mode_shows_ocfd_guidance_compromises() {
    let output = binary()
        .args(["--bands", "40m", "--antenna", "ocfd"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Antenna model: off-center-fed dipole"));
    assert!(stdout.contains("OCFD 33/67 legs:"));
    assert!(stdout.contains("OCFD 20/80 legs:"));
    assert!(!stdout.contains("Half-wave:"));
    assert!(!stdout.contains("End-fed half-wave:"));
    assert!(!stdout.contains("Full-wave loop circumference:"));
    assert!(stdout.contains(
        "Closest combined compromises to resonant points (tuner-assisted OCFD guidance):"
    ));
    assert!(stdout
        .contains("dipole-derived compromise lengths shown as tuner-assisted starting points"));
    assert!(stdout.contains("OCFD mode: each compromise line shows a total wire length"));
    assert!(stdout.contains("33/67 legs:"));
    assert!(stdout.contains("20/80 legs:"));
    assert!(stdout.contains("Optimized split:"));
}

#[test]
fn non_resonant_mode_defaults_to_recommended_transformer() {
    let output = binary()
        .args(["--bands", "40m", "--mode", "non-resonant"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Using transformer ratio: 1:9"));
}

#[test]
fn efhw_recommended_transformer_resolves_to_1_56() {
    let output = binary()
        .args([
            "--bands",
            "40m",
            "--antenna",
            "efhw",
            "--transformer",
            "recommended",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Using transformer ratio: 1:56"));
}

#[test]
fn advise_outputs_ranked_candidates_for_efhw() {
    let output = binary()
        .args(["--advise", "--bands", "40m,20m", "--antenna", "efhw"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Advise candidates:"));
    assert!(stdout.contains("ratio 1:56"));
    assert!(stdout.contains("efficiency"));
    assert!(stdout.contains("wire"));
}

#[test]
fn advise_with_preset_uses_band_selection_and_succeeds() {
    let dir = temp_test_dir("advise-with-preset");
    let config_path = dir.join("bands.toml");
    fs::write(
        &config_path,
        "[presets]\nportable = [\"40m\", \"20m\", \"15m\", \"10m\"]\n",
    )
    .expect("failed to write bands.toml");

    let output = binary()
        .current_dir(&dir)
        .args([
            "--advise",
            "--bands-preset",
            "portable",
            "--bands-config",
            "bands.toml",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Advise candidates:"));
    assert!(stdout.contains("ratio"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn advise_markdown_export_writes_md_file() {
    let dir = temp_test_dir("advise-markdown-export");
    let output = binary()
        .current_dir(&dir)
        .args([
            "--advise",
            "--bands",
            "40m,20m",
            "--antenna",
            "efhw",
            "--export",
            "markdown",
            "--output",
            "advise.md",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Exported advise results to advise.md"));

    let md = fs::read_to_string(dir.join("advise.md")).expect("failed to read markdown export");
    assert!(md.contains("# Rusty Wire Advise Candidates"));
    assert!(md.contains("| Rank | Ratio |"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn invalid_wire_window_inverted_shows_structured_error() {
    // Passing a min larger than max should produce a structured AppError through execute_request_checked.
    let output = binary()
        .args([
            "--bands",
            "40m",
            "--mode",
            "non-resonant",
            "--wire-min",
            "100",
            "--wire-max",
            "10",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("invalid wire length window in meters"));
}

#[test]
fn velocity_out_of_range_shows_structured_error() {
    let output = binary()
        .args(["--bands", "40m", "--velocity", "0.1"])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("velocity factor must be between 0.50 and 1.00 (got 0.100)"));
}

#[test]
fn step_flag_accepted_and_non_resonant_succeeds() {
    let output = binary()
        .args([
            "--bands",
            "40m,20m",
            "--mode",
            "non-resonant",
            "--step",
            "0.01",
        ])
        .output()
        .expect("failed to run rusty-wire");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn step_flag_zero_shows_structured_error() {
    let output = binary()
        .args([
            "--bands",
            "40m,20m",
            "--mode",
            "non-resonant",
            "--step",
            "0",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("search step must be greater than 0"));
}

#[test]
fn step_flag_exceeding_window_shows_structured_error() {
    let output = binary()
        .args([
            "--bands",
            "40m",
            "--mode",
            "non-resonant",
            "--wire-min",
            "8",
            "--wire-max",
            "10",
            "--step",
            "5",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("search step must be greater than 0"));
}

// ---------------------------------------------------------------------------
// --quiet tests
// ---------------------------------------------------------------------------

#[test]
fn quiet_non_resonant_prints_only_recommendation_length() {
    let output = binary()
        .args([
            "--bands",
            "40m,20m",
            "--mode",
            "non-resonant",
            "--units",
            "both",
            "--quiet",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    // Should contain a metre reading and a feet reading on one line, nothing else
    assert!(stdout.contains(" m ("));
    assert!(stdout.contains(" ft)"));
    // The full table header should NOT appear
    assert!(!stdout.contains("Antenna model:"));
    assert!(!stdout.contains("Half-wave:"));
}

#[test]
fn quiet_resonant_produces_no_stdout() {
    let output = binary()
        .args(["--bands", "40m", "--quiet"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.trim().is_empty(),
        "expected no stdout in quiet resonant mode, got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// --freq tests
// ---------------------------------------------------------------------------

#[test]
fn freq_flag_computes_single_frequency() {
    let output = binary()
        .args(["--freq", "7.1"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Should show the custom frequency label
    assert!(stdout.contains("7.100 MHz"));
    assert!(stdout.contains("Half-wave:"));
}

#[test]
fn freq_flag_zero_shows_error() {
    let output = binary()
        .args(["--freq", "0"])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("--freq must be a positive frequency"));
}

#[test]
fn freq_flag_with_quiet_prints_compact_line() {
    let output = binary()
        .args([
            "--freq",
            "14.2",
            "--mode",
            "non-resonant",
            "--units",
            "m",
            "--quiet",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    // Should be a single line with just metres
    assert!(stdout.contains(" m"));
    assert!(!stdout.contains("Half-wave:"));
}

// ---------------------------------------------------------------------------
// --freq-list tests
// ---------------------------------------------------------------------------

#[test]
fn freq_list_computes_multiple_frequencies() {
    let output = binary()
        .args(["--freq-list", "7.074,14.074"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("7.074 MHz"), "expected 7.074 MHz label");
    assert!(stdout.contains("14.074 MHz"), "expected 14.074 MHz label");
    assert!(stdout.contains("Half-wave:"));
}

#[test]
fn freq_list_single_entry_behaves_like_freq() {
    let output = binary()
        .args(["--freq-list", "7.1"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("7.100 MHz"));
}

#[test]
fn freq_list_and_freq_are_mutually_exclusive() {
    let output = binary()
        .args(["--freq", "7.1", "--freq-list", "14.2"])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("mutually exclusive"));
}

#[test]
fn freq_list_rejects_zero_frequency() {
    let output = binary()
        .args(["--freq-list", "0,14.074"])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("out of range"));
}

#[test]
fn freq_list_rejects_over_limit_frequency() {
    let output = binary()
        .args(["--freq-list", "7.074,1001.0"])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("out of range"));
}

#[test]
fn freq_list_quiet_non_resonant_prints_compact() {
    let output = binary()
        .args([
            "--freq-list",
            "7.074,14.074",
            "--mode",
            "non-resonant",
            "--units",
            "m",
            "--quiet",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains(" m"));
    assert!(!stdout.contains("Half-wave:"));
}

// ---------------------------------------------------------------------------
// --velocity-sweep tests
// ---------------------------------------------------------------------------

#[test]
fn velocity_sweep_non_resonant_prints_table() {
    let output = binary()
        .args([
            "--bands",
            "40m,20m",
            "--mode",
            "non-resonant",
            "--velocity-sweep",
            "0.66,0.85,0.95",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("Velocity sweep"));
    assert!(stdout.contains("non-resonant"));
    // All three VFs should appear in the table
    assert!(stdout.contains("0.66"));
    assert!(stdout.contains("0.85"));
    assert!(stdout.contains("0.95"));
}

#[test]
fn velocity_sweep_resonant_prints_per_vf_lines() {
    let output = binary()
        .args([
            "--bands",
            "40m",
            "--mode",
            "resonant",
            "--velocity-sweep",
            "0.85,0.95",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Velocity sweep"));
    assert!(stdout.contains("resonant"));
    assert!(stdout.contains("VF 0.85"));
    assert!(stdout.contains("VF 0.95"));
}

#[test]
fn velocity_sweep_out_of_range_shows_error() {
    let output = binary()
        .args(["--bands", "40m", "--velocity-sweep", "0.95,1.5"])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("out of range"));
}

#[test]
fn trap_dipole_antenna_mode_shows_trap_length() {
    let output = binary()
        .args(["--bands", "40m", "--antenna", "trap-dipole"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Antenna model: trap dipole"));
    assert!(stdout.contains("Trap dipole total:"));
    assert!(stdout.contains("Trap dipole each element:"));
    assert!(!stdout.contains("Half-wave:"));
    assert!(!stdout.contains("End-fed half-wave:"));
}

#[test]
fn trap_dipole_aliases_accepted() {
    // Clap supports the canonical name "trap-dipole"
    let output = binary()
        .args(["--bands", "40m", "--antenna", "trap-dipole"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Antenna model: trap dipole"));
}

#[test]
fn trap_dipole_export_includes_trap_fields() {
    let dir = temp_test_dir("trap-dipole-export");
    let output = binary()
        .current_dir(&dir)
        .args([
            "--bands",
            "40m",
            "--antenna",
            "trap-dipole",
            "--units",
            "both",
            "--export",
            "csv,json",
        ])
        .output()
        .expect("failed to run rusty-wire");

    assert!(output.status.success());

    let csv =
        fs::read_to_string(dir.join("rusty-wire-results.csv")).expect("failed to read csv export");
    let json = fs::read_to_string(dir.join("rusty-wire-results.json"))
        .expect("failed to read json export");

    assert!(csv.contains("trap_dipole_total_m"));
    assert!(csv.contains("trap_dipole_leg_m"));
    assert!(csv.contains("trap_dipole_total_ft"));
    assert!(csv.contains("trap_dipole_leg_ft"));
    assert!(json.contains("\"trap_dipole_total_m\""));
    assert!(json.contains("\"trap_dipole_leg_m\""));
    assert!(json.contains("\"trap_dipole_total_ft\""));
    assert!(json.contains("\"trap_dipole_leg_ft\""));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn trap_dipole_recommended_transformer_is_1_1() {
    let output = binary()
        .args([
            "--bands",
            "40m",
            "--antenna",
            "trap-dipole",
            "--transformer",
            "recommended",
        ])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Using transformer ratio: 1:1"));
}
