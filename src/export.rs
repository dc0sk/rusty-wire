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
) -> io::Result<()> {
    let content = match format {
        ExportFormat::Csv => to_csv(calculations, recommendation, units),
        ExportFormat::Json => to_json(calculations, recommendation, units),
        ExportFormat::Markdown => to_markdown(calculations, recommendation, units),
        ExportFormat::Txt => to_txt(calculations, recommendation, units),
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
) -> String {
    let (best_m, best_ft, clear_pct) = match recommendation {
        Some(r) => (r.length_m, r.length_ft, r.min_resonance_clearance_pct),
        None => (0.0, 0.0, 0.0),
    };
    let mut out = match units {
        UnitSystem::Metric => String::from(
            "band,frequency_mhz,half_wave_m,full_wave_m,quarter_wave_m,skip_min_km,skip_max_km,skip_avg_km,best_non_resonant_m,resonance_clearance_pct\n",
        ),
        UnitSystem::Imperial => String::from(
            "band,frequency_mhz,half_wave_ft,full_wave_ft,quarter_wave_ft,skip_min_km,skip_max_km,skip_avg_km,best_non_resonant_ft,resonance_clearance_pct\n",
        ),
        UnitSystem::Both => String::from(
            "band,frequency_mhz,half_wave_m,full_wave_m,quarter_wave_m,half_wave_ft,full_wave_ft,quarter_wave_ft,skip_min_km,skip_max_km,skip_avg_km,best_non_resonant_m,best_non_resonant_ft,resonance_clearance_pct\n",
        ),
    };
    for c in calculations {
        let row = match units {
            UnitSystem::Metric => format!(
                "\"{}\",{:.3},{:.2},{:.2},{:.2},{:.0},{:.0},{:.0},{:.2},{:.2}\n",
                c.band_name, c.frequency_mhz,
                c.half_wave_m, c.full_wave_m, c.quarter_wave_m,
                c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
                best_m, clear_pct,
            ),
            UnitSystem::Imperial => format!(
                "\"{}\",{:.3},{:.2},{:.2},{:.2},{:.0},{:.0},{:.0},{:.2},{:.2}\n",
                c.band_name, c.frequency_mhz,
                c.half_wave_ft, c.full_wave_ft, c.quarter_wave_ft,
                c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
                best_ft, clear_pct,
            ),
            UnitSystem::Both => format!(
                "\"{}\",{:.3},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.0},{:.0},{:.0},{:.2},{:.2},{:.2}\n",
                c.band_name, c.frequency_mhz,
                c.half_wave_m, c.full_wave_m, c.quarter_wave_m,
                c.half_wave_ft, c.full_wave_ft, c.quarter_wave_ft,
                c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
                best_m, best_ft, clear_pct,
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
) -> String {
    let mut out = String::from("[\n");
    for (i, c) in calculations.iter().enumerate() {
        let comma = if i + 1 == calculations.len() { "" } else { "," };
        let length_fields = match units {
            UnitSystem::Metric => format!(
                "\"half_wave_m\": {:.2},\n    \"full_wave_m\": {:.2},\n    \"quarter_wave_m\": {:.2}",
                c.half_wave_m, c.full_wave_m, c.quarter_wave_m,
            ),
            UnitSystem::Imperial => format!(
                "\"half_wave_ft\": {:.2},\n    \"full_wave_ft\": {:.2},\n    \"quarter_wave_ft\": {:.2}",
                c.half_wave_ft, c.full_wave_ft, c.quarter_wave_ft,
            ),
            UnitSystem::Both => format!(
                "\"half_wave_m\": {:.2},\n    \"full_wave_m\": {:.2},\n    \"quarter_wave_m\": {:.2},\n    \"half_wave_ft\": {:.2},\n    \"full_wave_ft\": {:.2},\n    \"quarter_wave_ft\": {:.2}",
                c.half_wave_m, c.full_wave_m, c.quarter_wave_m,
                c.half_wave_ft, c.full_wave_ft, c.quarter_wave_ft,
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
        out.push_str(&format!(
            "  {{\n    \"band\": \"{}\",\n    \"frequency_mhz\": {:.3},\n    {},\n    \"skip_min_km\": {:.0},\n    \"skip_max_km\": {:.0},\n    \"skip_avg_km\": {:.0},\n    \"non_resonant_recommendation\": {}\n  }}{}\n",
            json_escape(&c.band_name),
            c.frequency_mhz,
            length_fields,
            c.skip_distance_min_km,
            c.skip_distance_max_km,
            c.skip_distance_avg_km,
            recommendation_json,
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
) -> String {
    let mut out = String::from("# Rusty Wire Results\n\n");
    out.push_str("## Band Calculations\n\n");

    match units {
        UnitSystem::Metric => {
            out.push_str("| Band | Freq (MHz) | Half-wave (m) | Full-wave (m) | Quarter-wave (m) | Skip Min (km) | Skip Max (km) | Skip Avg (km) |\n");
            out.push_str("|------|------------|---------------|---------------|------------------|---------------|---------------|---------------|\n");
            for c in calculations {
                out.push_str(&format!(
                    "| {} | {:.3} | {:.2} | {:.2} | {:.2} | {:.0} | {:.0} | {:.0} |\n",
                    c.band_name, c.frequency_mhz,
                    c.half_wave_m, c.full_wave_m, c.quarter_wave_m,
                    c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
                ));
            }
        }
        UnitSystem::Imperial => {
            out.push_str("| Band | Freq (MHz) | Half-wave (ft) | Full-wave (ft) | Quarter-wave (ft) | Skip Min (km) | Skip Max (km) | Skip Avg (km) |\n");
            out.push_str("|------|------------|----------------|----------------|-------------------|---------------|---------------|---------------|\n");
            for c in calculations {
                out.push_str(&format!(
                    "| {} | {:.3} | {:.2} | {:.2} | {:.2} | {:.0} | {:.0} | {:.0} |\n",
                    c.band_name, c.frequency_mhz,
                    c.half_wave_ft, c.full_wave_ft, c.quarter_wave_ft,
                    c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
                ));
            }
        }
        UnitSystem::Both => {
            out.push_str("| Band | Freq (MHz) | Half-wave (m) | Half-wave (ft) | Full-wave (m) | Full-wave (ft) | Quarter-wave (m) | Quarter-wave (ft) | Skip Min (km) | Skip Max (km) | Skip Avg (km) |\n");
            out.push_str("|------|------------|---------------|----------------|---------------|----------------|------------------|-------------------|---------------|---------------|---------------|\n");
            for c in calculations {
                out.push_str(&format!(
                    "| {} | {:.3} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.0} | {:.0} | {:.0} |\n",
                    c.band_name, c.frequency_mhz,
                    c.half_wave_m, c.half_wave_ft,
                    c.full_wave_m, c.full_wave_ft,
                    c.quarter_wave_m, c.quarter_wave_ft,
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

    out
}

pub fn to_txt(
    calculations: &[WireCalculation],
    recommendation: Option<&NonResonantRecommendation>,
    units: UnitSystem,
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
                "  Half-wave: {:.2} m\n  Full-wave: {:.2} m\n  Quarter-wave: {:.2} m",
                c.half_wave_m, c.full_wave_m, c.quarter_wave_m,
            ),
            UnitSystem::Imperial => format!(
                "  Half-wave: {:.2} ft\n  Full-wave: {:.2} ft\n  Quarter-wave: {:.2} ft",
                c.half_wave_ft, c.full_wave_ft, c.quarter_wave_ft,
            ),
            UnitSystem::Both => format!(
                "  Half-wave: {:.2} m ({:.2} ft)\n  Full-wave: {:.2} m ({:.2} ft)\n  Quarter-wave: {:.2} m ({:.2} ft)",
                c.half_wave_m, c.half_wave_ft,
                c.full_wave_m, c.full_wave_ft,
                c.quarter_wave_m, c.quarter_wave_ft,
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

    out
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn json_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}
