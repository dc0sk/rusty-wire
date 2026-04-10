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

    assert!(stderr.contains("cannot mix meter and feet constraints"));
}

#[test]
fn invalid_velocity_shows_error() {
    let output = binary()
        .args(["--bands", "4", "--velocity", "1.5"])
        .output()
        .expect("failed to run rusty-wire");
    let stderr = String::from_utf8_lossy(&output.stderr);

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
fn multiple_exports_ignore_custom_output_name() {
    let dir = temp_test_dir("multi-export");
    let output = binary()
        .current_dir(&dir)
        .args([
            "--bands",
            "4",
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
        .args(["--bands", "4", "--export", "csv", "--output", "custom.csv"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Exported results to custom.csv"));
    assert!(dir.join("custom.csv").exists());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn default_antenna_mode_shows_all_models_per_band() {
    let output = binary()
        .args(["--bands", "4"])
        .output()
        .expect("failed to run rusty-wire");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Antenna model: all"));
    assert!(stdout.contains("Half-wave:"));
    assert!(stdout.contains("End-fed half-wave:"));
    assert!(stdout.contains("Full-wave loop circumference:"));
}

#[test]
fn selected_antenna_model_filters_output_and_resonant_summary() {
    let output = binary()
        .args(["--bands", "4", "--antenna", "efhw"])
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
        .args(["--bands", "4", "--antenna", "loop"])
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
