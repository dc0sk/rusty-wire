/// Export formatting and file writing.
///
/// Each `to_*` function is a pure string transform; `export_results` is the
/// only function that touches the file system.  Both are accessible from
/// future GUI code (e.g. to pipe content into a preview widget).
use crate::app::{ExportFormat, UnitSystem};
use crate::calculations::{NonResonantRecommendation, WireCalculation};
use std::fs;
use std::io;

// ---------------------------------------------------------------------------
// File-name helpers
// ---------------------------------------------------------------------------

pub fn default_output_name(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Csv => "rusty-wire-results.csv",
        ExportFormat::Json => "rusty-wire-results.json",
        ExportFormat::Markdown => "rusty-wire-results.md",
        ExportFormat::Txt => "rusty-wire-results.txt",
    }
}

// ---------------------------------------------------------------------------
// File write
// ---------------------------------------------------------------------------

pub fn export_results(
    format: ExportFormat,
    output: &str,
    calculations: &[WireCalculation],
    recommendation: Option<&NonResonantRecommendation>,
    units: UnitSystem,
    wire_min_m: f64,
    wire_max_m: f64,
) -> io::Result<()> {
    let content = match format {
        ExportFormat::Csv => to_csv(calculations, recommendation, units, wire_min_m, wire_max_m),
        ExportFormat::Json => to_json(calculations, recommendation, units, wire_min_m, wire_max_m),
        ExportFormat::Markdown => {
            to_markdown(calculations, recommendation, units, wire_min_m, wire_max_m)
        }
        ExportFormat::Txt => to_txt(calculations, recommendation, units, wire_min_m, wire_max_m),
    };
    fs::write(output, content)
}

// ---------------------------------------------------------------------------
// Formatters (pure – no I/O)
// ---------------------------------------------------------------------------

pub fn to_csv(
    calculations: &[WireCalculation],
    recommendation: Option<&NonResonantRecommendation>,
    units: UnitSystem,
    wire_min_m: f64,
    wire_max_m: f64,
) -> String {
    let (best_m, best_ft, clear_pct) = match recommendation {
        Some(r) => (r.length_m, r.length_ft, r.min_resonance_clearance_pct),
        None => (0.0, 0.0, 0.0),
    };
    let mut out = match units {
        UnitSystem::Metric => String::from(
            "band,frequency_mhz,transformer_ratio,half_wave_m,half_wave_corrected_m,full_wave_m,full_wave_corrected_m,quarter_wave_m,quarter_wave_corrected_m,skip_min_km,skip_max_km,skip_avg_km,best_non_resonant_m,resonance_clearance_pct,resonant_points_in_window\n",
        ),
        UnitSystem::Imperial => String::from(
            "band,frequency_mhz,transformer_ratio,half_wave_ft,half_wave_corrected_ft,full_wave_ft,full_wave_corrected_ft,quarter_wave_ft,quarter_wave_corrected_ft,skip_min_km,skip_max_km,skip_avg_km,best_non_resonant_ft,resonance_clearance_pct,resonant_points_in_window\n",
        ),
        UnitSystem::Both => String::from(
            "band,frequency_mhz,transformer_ratio,half_wave_m,half_wave_corrected_m,full_wave_m,full_wave_corrected_m,quarter_wave_m,quarter_wave_corrected_m,half_wave_ft,half_wave_corrected_ft,full_wave_ft,full_wave_corrected_ft,quarter_wave_ft,quarter_wave_corrected_ft,skip_min_km,skip_max_km,skip_avg_km,best_non_resonant_m,best_non_resonant_ft,resonance_clearance_pct,resonant_points_in_window\n",
        ),
    };
    for c in calculations {
        let points = csv_escape(&format_band_resonant_points(
            c,
            wire_min_m,
            wire_max_m,
            units,
        ));
        let row = match units {
            UnitSystem::Metric => format!(
                "\"{}\",{:.3},\"{}\",{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.0},{:.0},{:.0},{:.2},{:.2},\"{}\"\n",
                c.band_name, c.frequency_mhz,
                c.transformer_ratio_label,
                c.half_wave_m, c.corrected_half_wave_m,
                c.full_wave_m, c.corrected_full_wave_m,
                c.quarter_wave_m, c.corrected_quarter_wave_m,
                c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
                best_m, clear_pct, points,
            ),
            UnitSystem::Imperial => format!(
                "\"{}\",{:.3},\"{}\",{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.0},{:.0},{:.0},{:.2},{:.2},\"{}\"\n",
                c.band_name, c.frequency_mhz,
                c.transformer_ratio_label,
                c.half_wave_ft, c.corrected_half_wave_ft,
                c.full_wave_ft, c.corrected_full_wave_ft,
                c.quarter_wave_ft, c.corrected_quarter_wave_ft,
                c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
                best_ft, clear_pct, points,
            ),
            UnitSystem::Both => format!(
                "\"{}\",{:.3},\"{}\",{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.0},{:.0},{:.0},{:.2},{:.2},{:.2},\"{}\"\n",
                c.band_name, c.frequency_mhz,
                c.transformer_ratio_label,
                c.half_wave_m, c.corrected_half_wave_m,
                c.full_wave_m, c.corrected_full_wave_m,
                c.quarter_wave_m, c.corrected_quarter_wave_m,
                c.half_wave_ft, c.corrected_half_wave_ft,
                c.full_wave_ft, c.corrected_full_wave_ft,
                c.quarter_wave_ft, c.corrected_quarter_wave_ft,
                c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
                best_m, best_ft, clear_pct, points,
            ),
        };
        out.push_str(&row);
    }
    out
}

pub fn to_json(
    calculations: &[WireCalculation],
    recommendation: Option<&NonResonantRecommendation>,
    units: UnitSystem,
    wire_min_m: f64,
    wire_max_m: f64,
) -> String {
    let mut out = String::from("[\n");
    for (i, c) in calculations.iter().enumerate() {
        let comma = if i + 1 == calculations.len() { "" } else { "," };
        let length_fields = match units {
            UnitSystem::Metric => format!(
                "\"half_wave_m\": {:.2},\n    \"half_wave_corrected_m\": {:.2},\n    \"full_wave_m\": {:.2},\n    \"full_wave_corrected_m\": {:.2},\n    \"quarter_wave_m\": {:.2},\n    \"quarter_wave_corrected_m\": {:.2}",
                c.half_wave_m,
                c.corrected_half_wave_m,
                c.full_wave_m,
                c.corrected_full_wave_m,
                c.quarter_wave_m,
                c.corrected_quarter_wave_m,
            ),
            UnitSystem::Imperial => format!(
                "\"half_wave_ft\": {:.2},\n    \"half_wave_corrected_ft\": {:.2},\n    \"full_wave_ft\": {:.2},\n    \"full_wave_corrected_ft\": {:.2},\n    \"quarter_wave_ft\": {:.2},\n    \"quarter_wave_corrected_ft\": {:.2}",
                c.half_wave_ft,
                c.corrected_half_wave_ft,
                c.full_wave_ft,
                c.corrected_full_wave_ft,
                c.quarter_wave_ft,
                c.corrected_quarter_wave_ft,
            ),
            UnitSystem::Both => format!(
                "\"half_wave_m\": {:.2},\n    \"half_wave_corrected_m\": {:.2},\n    \"full_wave_m\": {:.2},\n    \"full_wave_corrected_m\": {:.2},\n    \"quarter_wave_m\": {:.2},\n    \"quarter_wave_corrected_m\": {:.2},\n    \"half_wave_ft\": {:.2},\n    \"half_wave_corrected_ft\": {:.2},\n    \"full_wave_ft\": {:.2},\n    \"full_wave_corrected_ft\": {:.2},\n    \"quarter_wave_ft\": {:.2},\n    \"quarter_wave_corrected_ft\": {:.2}",
                c.half_wave_m,
                c.corrected_half_wave_m,
                c.full_wave_m,
                c.corrected_full_wave_m,
                c.quarter_wave_m,
                c.corrected_quarter_wave_m,
                c.half_wave_ft,
                c.corrected_half_wave_ft,
                c.full_wave_ft,
                c.corrected_full_wave_ft,
                c.quarter_wave_ft,
                c.corrected_quarter_wave_ft,
            ),
        };
        let recommendation_json = match (recommendation, units) {
            (Some(r), UnitSystem::Metric) => format!(
                "{{\"best_non_resonant_m\": {:.2}, \"resonance_clearance_pct\": {:.2}}}",
                r.length_m, r.min_resonance_clearance_pct
            ),
            (Some(r), UnitSystem::Imperial) => format!(
                "{{\"best_non_resonant_ft\": {:.2}, \"resonance_clearance_pct\": {:.2}}}",
                r.length_ft, r.min_resonance_clearance_pct
            ),
            (Some(r), UnitSystem::Both) => format!(
                "{{\"best_non_resonant_m\": {:.2}, \"best_non_resonant_ft\": {:.2}, \"resonance_clearance_pct\": {:.2}}}",
                r.length_m, r.length_ft, r.min_resonance_clearance_pct
            ),
            (None, _) => "null".to_string(),
        };
        let points_json = format_band_resonant_points_json(c, wire_min_m, wire_max_m, units);
        out.push_str(&format!(
            "  {{\n    \"band\": \"{}\",\n    \"frequency_mhz\": {:.3},\n    \"transformer_ratio\": \"{}\",\n    {},\n    \"skip_min_km\": {:.0},\n    \"skip_max_km\": {:.0},\n    \"skip_avg_km\": {:.0},\n    \"non_resonant_recommendation\": {},\n    \"resonant_points_in_window\": {}\n  }}{}\n",
            json_escape(&c.band_name),
            c.frequency_mhz,
            c.transformer_ratio_label,
            length_fields,
            c.skip_distance_min_km,
            c.skip_distance_max_km,
            c.skip_distance_avg_km,
            recommendation_json,
            points_json,
            comma,
        ));
    }
    out.push(']');
    out
}

pub fn to_markdown(
    calculations: &[WireCalculation],
    recommendation: Option<&NonResonantRecommendation>,
    units: UnitSystem,
    wire_min_m: f64,
    wire_max_m: f64,
) -> String {
    let mut out = String::from("# Rusty Wire Results\n\n");
    out.push_str("## Band Calculations\n\n");

    match units {
        UnitSystem::Metric => {
            out.push_str("| Band | Ratio | Freq (MHz) | Half-wave (m) | Half-wave corrected (m) | Full-wave (m) | Full-wave corrected (m) | Quarter-wave (m) | Quarter-wave corrected (m) | Skip Min (km) | Skip Max (km) | Skip Avg (km) |\n");
            out.push_str("|------|-------|------------|---------------|--------------------------|---------------|--------------------------|------------------|-----------------------------|---------------|---------------|---------------|\n");
            for c in calculations {
                out.push_str(&format!(
                    "| {} | {} | {:.3} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.0} | {:.0} | {:.0} |\n",
                    c.band_name,
                    c.transformer_ratio_label,
                    c.frequency_mhz,
                    c.half_wave_m,
                    c.corrected_half_wave_m,
                    c.full_wave_m,
                    c.corrected_full_wave_m,
                    c.quarter_wave_m,
                    c.corrected_quarter_wave_m,
                    c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
                ));
            }
        }
        UnitSystem::Imperial => {
            out.push_str("| Band | Ratio | Freq (MHz) | Half-wave (ft) | Half-wave corrected (ft) | Full-wave (ft) | Full-wave corrected (ft) | Quarter-wave (ft) | Quarter-wave corrected (ft) | Skip Min (km) | Skip Max (km) | Skip Avg (km) |\n");
            out.push_str("|------|-------|------------|----------------|--------------------------|----------------|--------------------------|-------------------|-----------------------------|---------------|---------------|---------------|\n");
            for c in calculations {
                out.push_str(&format!(
                    "| {} | {} | {:.3} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.0} | {:.0} | {:.0} |\n",
                    c.band_name,
                    c.transformer_ratio_label,
                    c.frequency_mhz,
                    c.half_wave_ft,
                    c.corrected_half_wave_ft,
                    c.full_wave_ft,
                    c.corrected_full_wave_ft,
                    c.quarter_wave_ft,
                    c.corrected_quarter_wave_ft,
                    c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
                ));
            }
        }
        UnitSystem::Both => {
            out.push_str("| Band | Ratio | Freq (MHz) | Half-wave (m) | Half-wave corr (m) | Half-wave (ft) | Half-wave corr (ft) | Full-wave (m) | Full-wave corr (m) | Full-wave (ft) | Full-wave corr (ft) | Quarter-wave (m) | Quarter-wave corr (m) | Quarter-wave (ft) | Quarter-wave corr (ft) | Skip Min (km) | Skip Max (km) | Skip Avg (km) |\n");
            out.push_str("|------|-------|------------|---------------|--------------------|----------------|---------------------|---------------|--------------------|----------------|---------------------|------------------|-----------------------|-------------------|------------------------|---------------|---------------|---------------|\n");
            for c in calculations {
                out.push_str(&format!(
                    "| {} | {} | {:.3} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.0} | {:.0} | {:.0} |\n",
                    c.band_name,
                    c.transformer_ratio_label,
                    c.frequency_mhz,
                    c.half_wave_m, c.corrected_half_wave_m,
                    c.half_wave_ft, c.corrected_half_wave_ft,
                    c.full_wave_m, c.corrected_full_wave_m,
                    c.full_wave_ft, c.corrected_full_wave_ft,
                    c.quarter_wave_m, c.corrected_quarter_wave_m,
                    c.quarter_wave_ft, c.corrected_quarter_wave_ft,
                    c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
                ));
            }
        }
    }

    out.push_str("\n## Non-Resonant Recommendation\n\n");
    match (recommendation, units) {
        (Some(r), UnitSystem::Metric) => {
            out.push_str("| Length (m) | Resonance Clearance (%) |\n");
            out.push_str("|------------|-------------------------|\n");
            out.push_str(&format!("| {:.2} | {:.2} |\n", r.length_m, r.min_resonance_clearance_pct));
        }
        (Some(r), UnitSystem::Imperial) => {
            out.push_str("| Length (ft) | Resonance Clearance (%) |\n");
            out.push_str("|-------------|-------------------------|\n");
            out.push_str(&format!("| {:.2} | {:.2} |\n", r.length_ft, r.min_resonance_clearance_pct));
        }
        (Some(r), UnitSystem::Both) => {
            out.push_str("| Length (m) | Length (ft) | Resonance Clearance (%) |\n");
            out.push_str("|------------|-------------|-------------------------|\n");
            out.push_str(&format!("| {:.2} | {:.2} | {:.2} |\n", r.length_m, r.length_ft, r.min_resonance_clearance_pct));
        }
        (None, _) => out.push_str("No recommendation available.\n"),
    }

    out.push_str("\n## Resonant Points Within Search Window\n\n");
    out.push_str(&format!(
        "Window: {:.2}-{:.2} m ({:.2}-{:.2} ft)\n\n",
        wire_min_m,
        wire_max_m,
        wire_min_m / 0.3048,
        wire_max_m / 0.3048,
    ));

    match units {
        UnitSystem::Metric => {
            out.push_str("| Band | Harmonic (x quarter-wave) | Length (m) |\n");
            out.push_str("|------|---------------------------|------------|\n");
            append_markdown_points_rows(&mut out, calculations, wire_min_m, wire_max_m, units);
        }
        UnitSystem::Imperial => {
            out.push_str("| Band | Harmonic (x quarter-wave) | Length (ft) |\n");
            out.push_str("|------|---------------------------|-------------|\n");
            append_markdown_points_rows(&mut out, calculations, wire_min_m, wire_max_m, units);
        }
        UnitSystem::Both => {
            out.push_str("| Band | Harmonic (x quarter-wave) | Length (m) | Length (ft) |\n");
            out.push_str("|------|---------------------------|------------|-------------|\n");
            append_markdown_points_rows(&mut out, calculations, wire_min_m, wire_max_m, units);
        }
    }

    out
}

pub fn to_txt(
    calculations: &[WireCalculation],
    recommendation: Option<&NonResonantRecommendation>,
    units: UnitSystem,
    wire_min_m: f64,
    wire_max_m: f64,
) -> String {
    let mut out = String::from("Rusty Wire Results\n");
    out.push_str(&"=".repeat(60));
    out.push('\n');

    out.push_str("\nBand Calculations\n");
    out.push_str(&"-".repeat(60));
    out.push('\n');
    for c in calculations {
        let lengths = match units {
            UnitSystem::Metric => format!(
                "  Transformer ratio: {}\n  Half-wave: {:.2} m (corrected: {:.2} m)\n  Full-wave: {:.2} m (corrected: {:.2} m)\n  Quarter-wave: {:.2} m (corrected: {:.2} m)",
                c.transformer_ratio_label,
                c.half_wave_m,
                c.corrected_half_wave_m,
                c.full_wave_m,
                c.corrected_full_wave_m,
                c.quarter_wave_m,
                c.corrected_quarter_wave_m,
            ),
            UnitSystem::Imperial => format!(
                "  Transformer ratio: {}\n  Half-wave: {:.2} ft (corrected: {:.2} ft)\n  Full-wave: {:.2} ft (corrected: {:.2} ft)\n  Quarter-wave: {:.2} ft (corrected: {:.2} ft)",
                c.transformer_ratio_label,
                c.half_wave_ft,
                c.corrected_half_wave_ft,
                c.full_wave_ft,
                c.corrected_full_wave_ft,
                c.quarter_wave_ft,
                c.corrected_quarter_wave_ft,
            ),
            UnitSystem::Both => format!(
                "  Transformer ratio: {}\n  Half-wave: {:.2} m ({:.2} ft), corrected: {:.2} m ({:.2} ft)\n  Full-wave: {:.2} m ({:.2} ft), corrected: {:.2} m ({:.2} ft)\n  Quarter-wave: {:.2} m ({:.2} ft), corrected: {:.2} m ({:.2} ft)",
                c.transformer_ratio_label,
                c.half_wave_m,
                c.half_wave_ft,
                c.corrected_half_wave_m,
                c.corrected_half_wave_ft,
                c.full_wave_m,
                c.full_wave_ft,
                c.corrected_full_wave_m,
                c.corrected_full_wave_ft,
                c.quarter_wave_m,
                c.quarter_wave_ft,
                c.corrected_quarter_wave_m,
                c.corrected_quarter_wave_ft,
            ),
        };
        out.push_str(&format!(
            "{}\n  Frequency: {:.3} MHz\n{}\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)\n\n",
            c.band_name, c.frequency_mhz, lengths,
            c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
        ));
    }

    out.push_str("Non-Resonant Recommendation\n");
    out.push_str(&"-".repeat(60));
    out.push('\n');
    match (recommendation, units) {
        (Some(r), UnitSystem::Metric) => out.push_str(&format!(
            "  {:.2} m, resonance clearance: {:.2}%\n",
            r.length_m, r.min_resonance_clearance_pct
        )),
        (Some(r), UnitSystem::Imperial) => out.push_str(&format!(
            "  {:.2} ft, resonance clearance: {:.2}%\n",
            r.length_ft, r.min_resonance_clearance_pct
        )),
        (Some(r), UnitSystem::Both) => out.push_str(&format!(
            "  {:.2} m ({:.2} ft), resonance clearance: {:.2}%\n",
            r.length_m, r.length_ft, r.min_resonance_clearance_pct
        )),
        (None, _) => out.push_str("  No recommendation available.\n"),
    }

    out.push_str("\nResonant Points Within Search Window\n");
    out.push_str(&"-".repeat(60));
    out.push('\n');
    out.push_str(&format!(
        "  Window: {:.2}-{:.2} m ({:.2}-{:.2} ft)\n",
        wire_min_m,
        wire_max_m,
        wire_min_m / 0.3048,
        wire_max_m / 0.3048,
    ));
    let mut any_points = false;
    for c in calculations {
        for (harmonic, len_m) in collect_band_resonant_points_m(c, wire_min_m, wire_max_m) {
            any_points = true;
            match units {
                UnitSystem::Metric => {
                    out.push_str(&format!(
                        "  {}: {}x quarter-wave = {:.2} m\n",
                        c.band_name, harmonic, len_m
                    ));
                }
                UnitSystem::Imperial => {
                    out.push_str(&format!(
                        "  {}: {}x quarter-wave = {:.2} ft\n",
                        c.band_name,
                        harmonic,
                        len_m / 0.3048
                    ));
                }
                UnitSystem::Both => {
                    out.push_str(&format!(
                        "  {}: {}x quarter-wave = {:.2} m ({:.2} ft)\n",
                        c.band_name,
                        harmonic,
                        len_m,
                        len_m / 0.3048
                    ));
                }
            }
        }
    }
    if !any_points {
        out.push_str("  No resonant points in this window.\n");
    }

    out
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn json_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

fn csv_escape(input: &str) -> String {
    input.replace('"', "\"\"")
}

fn collect_band_resonant_points_m(
    calc: &WireCalculation,
    wire_min_m: f64,
    wire_max_m: f64,
) -> Vec<(u32, f64)> {
    let mut points = Vec::new();
    let quarter_wave_m = calc.corrected_quarter_wave_m;
    if quarter_wave_m <= 0.0 || wire_max_m <= wire_min_m {
        return points;
    }

    let mut harmonic = 1_u32;
    loop {
        let resonant_len_m = quarter_wave_m * f64::from(harmonic);
        if resonant_len_m > wire_max_m + 1e-9 {
            break;
        }
        if resonant_len_m >= wire_min_m - 1e-9 {
            points.push((harmonic, resonant_len_m));
        }
        harmonic += 1;
    }

    points
}

fn format_band_resonant_points(
    calc: &WireCalculation,
    wire_min_m: f64,
    wire_max_m: f64,
    units: UnitSystem,
) -> String {
    let points = collect_band_resonant_points_m(calc, wire_min_m, wire_max_m);
    if points.is_empty() {
        return "none".to_string();
    }

    points
        .into_iter()
        .map(|(harmonic, len_m)| match units {
            UnitSystem::Metric => format!("{}x={:.2}m", harmonic, len_m),
            UnitSystem::Imperial => format!("{}x={:.2}ft", harmonic, len_m / 0.3048),
            UnitSystem::Both => format!("{}x={:.2}m/{:.2}ft", harmonic, len_m, len_m / 0.3048),
        })
        .collect::<Vec<String>>()
        .join("; ")
}

fn format_band_resonant_points_json(
    calc: &WireCalculation,
    wire_min_m: f64,
    wire_max_m: f64,
    units: UnitSystem,
) -> String {
    let points = collect_band_resonant_points_m(calc, wire_min_m, wire_max_m);
    if points.is_empty() {
        return "[]".to_string();
    }

    let items = points
        .into_iter()
        .map(|(harmonic, len_m)| match units {
            UnitSystem::Metric => {
                format!("{{\"harmonic\": {}, \"length_m\": {:.2}}}", harmonic, len_m)
            }
            UnitSystem::Imperial => format!(
                "{{\"harmonic\": {}, \"length_ft\": {:.2}}}",
                harmonic,
                len_m / 0.3048
            ),
            UnitSystem::Both => format!(
                "{{\"harmonic\": {}, \"length_m\": {:.2}, \"length_ft\": {:.2}}}",
                harmonic,
                len_m,
                len_m / 0.3048
            ),
        })
        .collect::<Vec<String>>()
        .join(", ");

    format!("[{}]", items)
}

fn append_markdown_points_rows(
    out: &mut String,
    calculations: &[WireCalculation],
    wire_min_m: f64,
    wire_max_m: f64,
    units: UnitSystem,
) {
    let mut any_points = false;
    for c in calculations {
        for (harmonic, len_m) in collect_band_resonant_points_m(c, wire_min_m, wire_max_m) {
            any_points = true;
            match units {
                UnitSystem::Metric => out.push_str(&format!(
                    "| {} | {} | {:.2} |\n",
                    c.band_name, harmonic, len_m
                )),
                UnitSystem::Imperial => out.push_str(&format!(
                    "| {} | {} | {:.2} |\n",
                    c.band_name,
                    harmonic,
                    len_m / 0.3048
                )),
                UnitSystem::Both => out.push_str(&format!(
                    "| {} | {} | {:.2} | {:.2} |\n",
                    c.band_name,
                    harmonic,
                    len_m,
                    len_m / 0.3048
                )),
            }
        }
    }
    if !any_points {
        match units {
            UnitSystem::Metric | UnitSystem::Imperial => {
                out.push_str("| (none) | - | - |\n");
            }
            UnitSystem::Both => {
                out.push_str("| (none) | - | - | - |\n");
            }
        }
    }
}
