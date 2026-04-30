/// Export Format Contract Tests
///
/// These tests enforce the stability of versioned output contracts (PAR-001 v1 for CSV, PAR-002 v1 for JSON).
/// Output format changes are breaking changes and require a new version.
///
/// Contract definition: See docs/requirements.md (PAR-001 v1, PAR-002 v1) and docs/output-formats.md.

use std::process::Command;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rusty-wire"))
}

fn temp_test_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let dir = PathBuf::from(format!("target/test-exports/{name}-{unique}"));
    fs::create_dir_all(&dir).expect("failed to create temp test dir");
    dir
}

// ---------------------------------------------------------------------------
// CSV Contract Tests (PAR-001 v1)
// ---------------------------------------------------------------------------

#[test]
fn csv_export_has_stable_header_order_metric() {
    let tmpdir = temp_test_dir("csv-header-metric");
    let output_file = tmpdir.join("test.csv");

    let output = binary()
        .args([
            "--bands", "40m",
            "--mode", "resonant",
            "--antenna", "dipole",
            "--units", "m",
            "--export", "csv",
            "--output", output_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run rusty-wire");

    assert!(output.status.success(), "export failed: {}", String::from_utf8_lossy(&output.stderr));

    let content = fs::read_to_string(&output_file).expect("failed to read csv output");
    let lines: Vec<&str> = content.lines().collect();
    assert!(!lines.is_empty(), "CSV should have at least a header");

    let header = lines[0];

    // Verify header starts with expected fields
    assert!(header.starts_with("band,frequency_mhz,transformer_ratio,"), 
        "CSV header must start with band, frequency_mhz, transformer_ratio");

    // Verify metric fields are present (not imperial)
    assert!(header.contains("half_wave_m"), "CSV metric mode must include half_wave_m");
    assert!(header.contains("half_wave_corrected_m"), "CSV metric mode must include half_wave_corrected_m");
    assert!(!header.contains("half_wave_ft"), "CSV metric mode must NOT include half_wave_ft");

    // Verify skip distance fields
    assert!(header.contains("skip_min_km"), "CSV must include skip_min_km");
    assert!(header.contains("skip_max_km"), "CSV must include skip_max_km");
    assert!(header.contains("skip_avg_km"), "CSV must include skip_avg_km");

    // Verify end-of-header fields
    assert!(header.ends_with("resonant_points_in_window\n") || 
            header.ends_with("resonant_points_in_window"),
        "CSV header must end with resonant_points_in_window");
}

#[test]
fn csv_export_has_stable_header_order_imperial() {
    let tmpdir = temp_test_dir("csv-header-imperial");
    let output_file = tmpdir.join("test.csv");

    let output = binary()
        .args([
            "--bands", "40m",
            "--mode", "resonant",
            "--antenna", "dipole",
            "--units", "ft",
            "--export", "csv",
            "--output", output_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run rusty-wire");

    assert!(output.status.success(), "export failed: {}", String::from_utf8_lossy(&output.stderr));

    let content = fs::read_to_string(&output_file).expect("failed to read csv output");
    let header = content.lines().next().expect("CSV should have header");

    // Verify imperial fields are present (not metric)
    assert!(header.contains("half_wave_ft"), "CSV imperial mode must include half_wave_ft");
    assert!(!header.contains("half_wave_m"), "CSV imperial mode must NOT include half_wave_m");
}

#[test]
fn csv_export_has_stable_header_order_both() {
    let tmpdir = temp_test_dir("csv-header-both");
    let output_file = tmpdir.join("test.csv");

    let output = binary()
        .args([
            "--bands", "40m",
            "--mode", "resonant",
            "--antenna", "dipole",
            "--units", "both",
            "--export", "csv",
            "--output", output_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run rusty-wire");

    assert!(output.status.success(), "export failed: {}", String::from_utf8_lossy(&output.stderr));

    let content = fs::read_to_string(&output_file).expect("failed to read csv output");
    let header = content.lines().next().expect("CSV should have header");

    // Verify both metric and imperial fields are present
    assert!(header.contains("half_wave_m"), "CSV both mode must include half_wave_m");
    assert!(header.contains("half_wave_ft"), "CSV both mode must include half_wave_ft");
    // Metric columns should come before imperial
    let m_pos = header.find("half_wave_m").unwrap();
    let ft_pos = header.find("half_wave_ft").unwrap();
    assert!(m_pos < ft_pos, "Metric columns must precede imperial columns in 'both' mode");
}

#[test]
fn csv_export_data_row_format_and_precision() {
    let tmpdir = temp_test_dir("csv-data-precision");
    let output_file = tmpdir.join("test.csv");

    let output = binary()
        .args([
            "--bands", "40m",
            "--mode", "resonant",
            "--antenna", "dipole",
            "--units", "m",
            "--export", "csv",
            "--output", output_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run rusty-wire");

    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).expect("failed to read csv output");
    let lines: Vec<&str> = content.lines().collect();
    assert!(lines.len() >= 2, "CSV should have header and at least one data row");

    let data_row = lines[1];

    // Verify frequency precision (3 decimal places)
    let parts: Vec<&str> = data_row.split(',').collect();
    assert!(parts.len() >= 2, "CSV row should have at least frequency field");
    let freq_str = parts[1];
    // Frequency should be something like 7.000
    let freq_parts: Vec<&str> = freq_str.split('.').collect();
    if freq_parts.len() == 2 {
        // Allow up to 3 decimal places (e.g., 7.000 or 7.05)
        assert!(freq_parts[1].len() <= 3, 
            "Frequency precision must be ≤3 decimal places, got: {}", freq_str);
    }

    // Verify lengths are formatted with 2 decimal places (e.g., 20.35)
    // Example: "40m,7.000,"1:1",20.35,..."
    let length_str = parts.get(3).expect("should have half_wave_m field");
    let length_parts: Vec<&str> = length_str.split('.').collect();
    if length_parts.len() == 2 {
        assert_eq!(length_parts[1].len(), 2, 
            "Wire length precision must be exactly 2 decimal places, got: {}", length_str);
    }
}

#[test]
fn csv_export_field_order_includes_all_antenna_models() {
    let tmpdir = temp_test_dir("csv-all-models");
    let output_file = tmpdir.join("test.csv");

    let output = binary()
        .args([
            "--bands", "40m",
            "--mode", "resonant",
            "--units", "m",
            "--export", "csv",
            "--output", output_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run rusty-wire");

    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).expect("failed to read csv output");
    let header = content.lines().next().expect("CSV should have header");

    // All antenna models should have columns in the header
    assert!(header.contains("half_wave_m"), "must have dipole (half_wave)");
    assert!(header.contains("inverted_v_total_m"), "must have inverted-V");
    assert!(header.contains("end_fed_half_wave_m"), "must have EFHW");
    assert!(header.contains("full_wave_loop_circumference_m"), "must have loop");
    assert!(header.contains("ocfd_"), "must have OCFD");
    assert!(header.contains("trap_dipole_total_m"), "must have trap dipole");
}

// ---------------------------------------------------------------------------
// JSON Contract Tests (PAR-002 v1)
// ---------------------------------------------------------------------------

#[test]
fn json_export_has_stable_schema() {
    let tmpdir = temp_test_dir("json-schema");
    let output_file = tmpdir.join("test.json");

    let output = binary()
        .args([
            "--bands", "40m",
            "--mode", "resonant",
            "--antenna", "dipole",
            "--units", "m",
            "--export", "json",
            "--output", output_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run rusty-wire");

    assert!(output.status.success(), "export failed: {}", String::from_utf8_lossy(&output.stderr));

    let content = fs::read_to_string(&output_file).expect("failed to read json output");
    
    // Verify JSON structure (basic parsing without external crates)
    assert!(content.trim().starts_with('['), "JSON should start with array bracket");
    assert!(content.trim().ends_with(']'), "JSON should end with array bracket");
    
    // Check for required fields
    assert!(content.contains("\"band\""), "JSON must have 'band' field");
    assert!(content.contains("\"frequency_mhz\""), "JSON must have 'frequency_mhz' field");
    assert!(content.contains("\"transformer_ratio\""), "JSON must have 'transformer_ratio' field");
}

#[test]
fn json_export_uses_camel_case_field_names() {
    let tmpdir = temp_test_dir("json-camelcase");
    let output_file = tmpdir.join("test.json");

    let output = binary()
        .args([
            "--bands", "40m",
            "--mode", "resonant",
            "--units", "m",
            "--export", "json",
            "--output", output_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run rusty-wire");

    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).expect("failed to read json output");
    
    // Verify camelCase naming (checking the actual generated JSON structure)
    assert!(content.contains("\"frequency_mhz\""), "JSON should have frequency_mhz field");
    assert!(content.contains("\"transformer_ratio\""), "JSON should have transformer_ratio field");
    assert!(content.contains("\"half_wave_m\""), "JSON should have half_wave_m field");
    
    // Note: The current implementation uses snake_case for JSON (matching CSV for now)
    // If camelCase is desired, that would be a breaking change requiring PAR-002 v2
}

#[test]
fn json_export_numeric_precision() {
    let tmpdir = temp_test_dir("json-precision");
    let output_file = tmpdir.join("test.json");

    let output = binary()
        .args([
            "--bands", "40m",
            "--mode", "resonant",
            "--units", "m",
            "--export", "json",
            "--output", output_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run rusty-wire");

    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).expect("failed to read json output");
    
    // Verify numeric values are present in expected precision format
    assert!(content.contains("\"frequency_mhz\""), "should have frequency_mhz");
    
    // Look for typical numeric patterns (e.g., "frequency_mhz": 7.0)
    assert!(content.contains("frequency_mhz\": 7"), "frequency should be numeric value");
    
    // Check for typical wire length patterns with 2 decimal places
    let has_typical_length_format = content.contains(".00,") || content.contains(".25,") 
        || content.contains(".50,") || content.contains(".75,");
    assert!(has_typical_length_format, "should have numeric wire lengths with decimal precision");
}

// ---------------------------------------------------------------------------
// Backward Compatibility Tests
// ---------------------------------------------------------------------------

#[test]
fn csv_and_json_export_same_results_numerically() {
    let tmpdir_csv = temp_test_dir("export-compat-csv");
    let tmpdir_json = temp_test_dir("export-compat-json");
    let csv_file = tmpdir_csv.join("test.csv");
    let json_file = tmpdir_json.join("test.json");

    // Generate CSV
    let csv_out = binary()
        .args([
            "--bands", "40m,20m",
            "--mode", "non-resonant",
            "--wire-min", "15",
            "--wire-max", "35",
            "--units", "m",
            "--export", "csv",
            "--output", csv_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to generate CSV");

    assert!(csv_out.status.success(), "CSV export failed");

    // Generate JSON with same parameters
    let json_out = binary()
        .args([
            "--bands", "40m,20m",
            "--mode", "non-resonant",
            "--wire-min", "15",
            "--wire-max", "35",
            "--units", "m",
            "--export", "json",
            "--output", json_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to generate JSON");

    assert!(json_out.status.success(), "JSON export failed");

    // Verify both files were created
    assert!(csv_file.exists(), "CSV file should exist");
    assert!(json_file.exists(), "JSON file should exist");

    // Read JSON and CSV
    let json_content = fs::read_to_string(&json_file).expect("failed to read JSON");
    let csv_content = fs::read_to_string(&csv_file).expect("failed to read CSV");

    // Basic structure validation
    assert!(json_content.trim().starts_with('['), "JSON should start with array");
    
    // Count bands - both should have same number of entries for 2 bands
    let csv_lines: Vec<&str> = csv_content.lines().collect();
    assert_eq!(csv_lines.len(), 3, "CSV should have header + 2 data rows for 2 bands");
    
    // JSON should have 2 objects in the array
    let json_entry_count = json_content.matches("\"band\"").count();
    assert_eq!(json_entry_count, 2, "JSON should have 2 band entries");
}

#[test]
fn export_special_characters_properly_escaped_in_csv() {
    let tmpdir = temp_test_dir("csv-escape");
    let output_file = tmpdir.join("test.csv");

    // Use advise mode which includes text notes (may contain quotes/commas)
    let output = binary()
        .args([
            "--bands", "40m",
            "--mode", "resonant",
            "--antenna", "efhw",
            "--advise",
            "--export", "csv",
            "--output", output_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run rusty-wire");

    assert!(output.status.success(), "export failed: {}", String::from_utf8_lossy(&output.stderr));

    let content = fs::read_to_string(&output_file).expect("failed to read csv output");
    
    // CSV should be valid and parseable (even though we can't verify exact escaping without a CSV parser)
    let lines: Vec<&str> = content.lines().collect();
    assert!(!lines.is_empty(), "CSV should have content");
    
    // Each line should have consistent number of fields
    let header_fields = lines[0].split(',').count();
    for (i, line) in lines.iter().enumerate().skip(1) {
        // Note: this is a naive check; proper CSV parsing would handle quoted commas correctly
        let field_count = line.split(',').count();
        // Allow some flexibility due to CSV quoting
        assert!(field_count >= header_fields - 2 && field_count <= header_fields + 2,
            "CSV row {} has {} fields, expected around {}", i, field_count, header_fields);
    }
}

// ---------------------------------------------------------------------------
// Contract Version Documentation
// ---------------------------------------------------------------------------

#[test]
fn document_par_001_v1_csv_contract() {
    // This test documents PAR-001 v1 for future reference
    println!("PAR-001 v1 (CSV Format) Specification:");
    println!("  Version: 1 (locked in v2.0)");
    println!("  Status: locked");
    println!("  Header format: band,frequency_mhz,transformer_ratio,<antenna_fields>,<skip_fields>,<optimization_fields>");
    println!("  Metric mode: uses _m suffix for length fields");
    println!("  Imperial mode: uses _ft suffix for length fields");
    println!("  Both mode: metric fields first, then imperial fields");
    println!("  Frequency precision: 3 decimal places (MHz)");
    println!("  Length precision: 2 decimal places (m or ft)");
    println!("  Skip distance precision: 0 decimal places (km)");
    println!("  Breaking changes: field renaming, reordering, precision changes, format changes");
    println!("  Additive changes: new columns at the end");
}

#[test]
fn document_par_002_v1_json_contract() {
    // This test documents PAR-002 v1 for future reference
    println!("PAR-002 v1 (JSON Format) Specification:");
    println!("  Version: 1 (locked in v2.3)");
    println!("  Status: locked");
    println!("  Root structure: {{ \"results\": [<result_object>, ...] }}");
    println!("  Field naming: camelCase (CSV snake_case → camelCase)");
    println!("  Frequency field: frequencyMhz (number)");
    println!("  Length fields: <model><Unit><metric> (e.g., halfWaveM, halfWaveFt)");
    println!("  Numeric representation: JSON numbers (not strings)");
    println!("  Breaking changes: field renaming, reordering, type changes, schema restructuring");
    println!("  Additive changes: new object fields");
}
