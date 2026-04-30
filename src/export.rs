/// Export formatting and file writing.
///
/// Each `to_*` function is a pure string transform; `export_results` is the
/// only function that touches the file system.  Both are accessible from
/// future GUI code (e.g. to pipe content into a preview widget).
use crate::app::{AdviseCandidate, ExportFormat, UnitSystem};
use crate::calculations::{NonResonantRecommendation, WireCalculation};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

// ---------------------------------------------------------------------------
// File-name helpers
// ---------------------------------------------------------------------------

pub fn default_output_name(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Csv => "rusty-wire-results.csv",
        ExportFormat::Html => "rusty-wire-results.html",
        ExportFormat::Json => "rusty-wire-results.json",
        ExportFormat::Markdown => "rusty-wire-results.md",
        ExportFormat::Txt => "rusty-wire-results.txt",
        ExportFormat::Yaml => "rusty-wire-results.yaml",
    }
}

pub fn default_advise_output_name(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Csv => "rusty-wire-advise.csv",
        ExportFormat::Html => "rusty-wire-advise.html",
        ExportFormat::Json => "rusty-wire-advise.json",
        ExportFormat::Markdown => "rusty-wire-advise.md",
        ExportFormat::Txt => "rusty-wire-advise.txt",
        ExportFormat::Yaml => "rusty-wire-advise.yaml",
    }
}

// ---------------------------------------------------------------------------
// File write
// ---------------------------------------------------------------------------

pub fn validate_export_path(output: &str) -> Result<PathBuf, String> {
    let raw = output.trim();
    if raw.is_empty() {
        return Err("output path cannot be empty".to_string());
    }

    let path = Path::new(raw);
    if path.is_absolute() {
        return Err("absolute output paths are not permitted".to_string());
    }

    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("output path must not contain parent directory references ('..')".to_string());
    }

    if path.file_name().and_then(|name| name.to_str()).is_none() {
        return Err("output path must end with a file name".to_string());
    }

    if path
        .components()
        .any(|component| matches!(component, Component::Normal(part) if part.to_string_lossy().contains('\0')))
    {
        return Err("output path contains invalid characters".to_string());
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => continue,
            Component::Normal(part) => normalized.push(part),
            _ => continue,
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err("output path must end with a file name".to_string());
    }

    Ok(normalized)
}

pub fn export_results(
    format: ExportFormat,
    output: &str,
    calculations: &[WireCalculation],
    recommendation: Option<&NonResonantRecommendation>,
    units: UnitSystem,
    wire_min_m: f64,
    wire_max_m: f64,
) -> io::Result<()> {
    let output_path = validate_export_path(output)
        .map_err(|msg| io::Error::new(io::ErrorKind::InvalidInput, msg))?;
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let content = match format {
        ExportFormat::Csv => to_csv(calculations, recommendation, units, wire_min_m, wire_max_m),
        ExportFormat::Html => to_html(calculations, recommendation, units, wire_min_m, wire_max_m),
        ExportFormat::Json => to_json(calculations, recommendation, units, wire_min_m, wire_max_m),
        ExportFormat::Markdown => {
            to_markdown(calculations, recommendation, units, wire_min_m, wire_max_m)
        }
        ExportFormat::Txt => to_txt(calculations, recommendation, units, wire_min_m, wire_max_m),
        ExportFormat::Yaml => to_yaml(calculations, recommendation, units, wire_min_m, wire_max_m),
    };
    fs::write(output_path, content)
}

pub fn export_advise(
    format: ExportFormat,
    output: &str,
    assumed_feedpoint_ohm: f64,
    candidates: &[AdviseCandidate],
) -> io::Result<()> {
    let output_path = validate_export_path(output)
        .map_err(|msg| io::Error::new(io::ErrorKind::InvalidInput, msg))?;
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let content = match format {
        ExportFormat::Csv => to_advise_csv(assumed_feedpoint_ohm, candidates),
        ExportFormat::Html => to_advise_html(assumed_feedpoint_ohm, candidates),
        ExportFormat::Json => to_advise_json(assumed_feedpoint_ohm, candidates),
        ExportFormat::Markdown => to_advise_markdown(assumed_feedpoint_ohm, candidates),
        ExportFormat::Txt => to_advise_txt(assumed_feedpoint_ohm, candidates),
        ExportFormat::Yaml => to_advise_yaml(assumed_feedpoint_ohm, candidates),
    };
    fs::write(output_path, content)
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
            "band,frequency_mhz,transformer_ratio,half_wave_m,half_wave_corrected_m,full_wave_m,full_wave_corrected_m,quarter_wave_m,quarter_wave_corrected_m,end_fed_half_wave_m,full_wave_loop_circumference_m,full_wave_loop_square_side_m,inverted_v_total_m,inverted_v_leg_m,inverted_v_span_90_m,inverted_v_span_120_m,ocfd_33_short_leg_m,ocfd_33_long_leg_m,ocfd_20_short_leg_m,ocfd_20_long_leg_m,trap_dipole_total_m,trap_dipole_leg_m,skip_min_km,skip_max_km,skip_avg_km,best_non_resonant_m,resonance_clearance_pct,resonant_points_in_window\n",
        ),
        UnitSystem::Imperial => String::from(
            "band,frequency_mhz,transformer_ratio,half_wave_ft,half_wave_corrected_ft,full_wave_ft,full_wave_corrected_ft,quarter_wave_ft,quarter_wave_corrected_ft,end_fed_half_wave_ft,full_wave_loop_circumference_ft,full_wave_loop_square_side_ft,inverted_v_total_ft,inverted_v_leg_ft,inverted_v_span_90_ft,inverted_v_span_120_ft,ocfd_33_short_leg_ft,ocfd_33_long_leg_ft,ocfd_20_short_leg_ft,ocfd_20_long_leg_ft,trap_dipole_total_ft,trap_dipole_leg_ft,skip_min_km,skip_max_km,skip_avg_km,best_non_resonant_ft,resonance_clearance_pct,resonant_points_in_window\n",
        ),
        UnitSystem::Both => String::from(
            "band,frequency_mhz,transformer_ratio,half_wave_m,half_wave_corrected_m,full_wave_m,full_wave_corrected_m,quarter_wave_m,quarter_wave_corrected_m,end_fed_half_wave_m,full_wave_loop_circumference_m,full_wave_loop_square_side_m,inverted_v_total_m,inverted_v_leg_m,inverted_v_span_90_m,inverted_v_span_120_m,ocfd_33_short_leg_m,ocfd_33_long_leg_m,ocfd_20_short_leg_m,ocfd_20_long_leg_m,trap_dipole_total_m,trap_dipole_leg_m,half_wave_ft,half_wave_corrected_ft,full_wave_ft,full_wave_corrected_ft,quarter_wave_ft,quarter_wave_corrected_ft,end_fed_half_wave_ft,full_wave_loop_circumference_ft,full_wave_loop_square_side_ft,inverted_v_total_ft,inverted_v_leg_ft,inverted_v_span_90_ft,inverted_v_span_120_ft,ocfd_33_short_leg_ft,ocfd_33_long_leg_ft,ocfd_20_short_leg_ft,ocfd_20_long_leg_ft,trap_dipole_total_ft,trap_dipole_leg_ft,skip_min_km,skip_max_km,skip_avg_km,best_non_resonant_m,best_non_resonant_ft,resonance_clearance_pct,resonant_points_in_window\n",
        ),
    };
    for c in calculations {
        let points = csv_escape(&format_band_resonant_points(
            c, wire_min_m, wire_max_m, units,
        ));
        let row = match units {
            UnitSystem::Metric => format!(
                "\"{}\",{:.3},\"{}\",{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.0},{:.0},{:.0},{:.2},{:.2},\"{}\"\n",
                c.band_name, c.frequency_mhz,
                c.transformer_ratio_label,
                c.half_wave_m, c.corrected_half_wave_m,
                c.full_wave_m, c.corrected_full_wave_m,
                c.quarter_wave_m, c.corrected_quarter_wave_m,
                c.end_fed_half_wave_m,
                c.full_wave_loop_circumference_m,
                c.full_wave_loop_square_side_m,
                c.inverted_v_total_m,
                c.inverted_v_leg_m,
                c.inverted_v_span_90_m,
                c.inverted_v_span_120_m,
                c.ocfd_33_short_leg_m,
                c.ocfd_33_long_leg_m,
                c.ocfd_20_short_leg_m,
                c.ocfd_20_long_leg_m,
                c.trap_dipole_total_m,
                c.trap_dipole_leg_m,
                c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
                best_m, clear_pct, points,
            ),
            UnitSystem::Imperial => format!(
                "\"{}\",{:.3},\"{}\",{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.0},{:.0},{:.0},{:.2},{:.2},\"{}\"\n",
                c.band_name, c.frequency_mhz,
                c.transformer_ratio_label,
                c.half_wave_ft, c.corrected_half_wave_ft,
                c.full_wave_ft, c.corrected_full_wave_ft,
                c.quarter_wave_ft, c.corrected_quarter_wave_ft,
                c.end_fed_half_wave_ft,
                c.full_wave_loop_circumference_ft,
                c.full_wave_loop_square_side_ft,
                c.inverted_v_total_ft,
                c.inverted_v_leg_ft,
                c.inverted_v_span_90_ft,
                c.inverted_v_span_120_ft,
                c.ocfd_33_short_leg_ft,
                c.ocfd_33_long_leg_ft,
                c.ocfd_20_short_leg_ft,
                c.ocfd_20_long_leg_ft,
                c.trap_dipole_total_ft,
                c.trap_dipole_leg_ft,
                c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
                best_ft, clear_pct, points,
            ),
            UnitSystem::Both => format!(
                "\"{band}\",{freq:.3},\"{ratio}\",{half_m:.2},{half_corr_m:.2},{full_m:.2},{full_corr_m:.2},{quarter_m:.2},{quarter_corr_m:.2},{efhw_m:.2},{loop_circ_m:.2},{loop_side_m:.2},{inv_total_m:.2},{inv_leg_m:.2},{inv_span_90_m:.2},{inv_span_120_m:.2},{ocfd33_short_m:.2},{ocfd33_long_m:.2},{ocfd20_short_m:.2},{ocfd20_long_m:.2},{trap_total_m:.2},{trap_leg_m:.2},{half_ft:.2},{half_corr_ft:.2},{full_ft:.2},{full_corr_ft:.2},{quarter_ft:.2},{quarter_corr_ft:.2},{efhw_ft:.2},{loop_circ_ft:.2},{loop_side_ft:.2},{inv_total_ft:.2},{inv_leg_ft:.2},{inv_span_90_ft:.2},{inv_span_120_ft:.2},{ocfd33_short_ft:.2},{ocfd33_long_ft:.2},{ocfd20_short_ft:.2},{ocfd20_long_ft:.2},{trap_total_ft:.2},{trap_leg_ft:.2},{skip_min:.0},{skip_max:.0},{skip_avg:.0},{best_m:.2},{best_ft:.2},{clear_pct:.2},\"{points}\"\n",
                band = c.band_name,
                freq = c.frequency_mhz,
                ratio = c.transformer_ratio_label,
                half_m = c.half_wave_m,
                half_corr_m = c.corrected_half_wave_m,
                full_m = c.full_wave_m,
                full_corr_m = c.corrected_full_wave_m,
                quarter_m = c.quarter_wave_m,
                quarter_corr_m = c.corrected_quarter_wave_m,
                efhw_m = c.end_fed_half_wave_m,
                loop_circ_m = c.full_wave_loop_circumference_m,
                loop_side_m = c.full_wave_loop_square_side_m,
                inv_total_m = c.inverted_v_total_m,
                inv_leg_m = c.inverted_v_leg_m,
                inv_span_90_m = c.inverted_v_span_90_m,
                inv_span_120_m = c.inverted_v_span_120_m,
                ocfd33_short_m = c.ocfd_33_short_leg_m,
                ocfd33_long_m = c.ocfd_33_long_leg_m,
                ocfd20_short_m = c.ocfd_20_short_leg_m,
                ocfd20_long_m = c.ocfd_20_long_leg_m,
                trap_total_m = c.trap_dipole_total_m,
                trap_leg_m = c.trap_dipole_leg_m,
                half_ft = c.half_wave_ft,
                half_corr_ft = c.corrected_half_wave_ft,
                full_ft = c.full_wave_ft,
                full_corr_ft = c.corrected_full_wave_ft,
                quarter_ft = c.quarter_wave_ft,
                quarter_corr_ft = c.corrected_quarter_wave_ft,
                efhw_ft = c.end_fed_half_wave_ft,
                loop_circ_ft = c.full_wave_loop_circumference_ft,
                loop_side_ft = c.full_wave_loop_square_side_ft,
                inv_total_ft = c.inverted_v_total_ft,
                inv_leg_ft = c.inverted_v_leg_ft,
                inv_span_90_ft = c.inverted_v_span_90_ft,
                inv_span_120_ft = c.inverted_v_span_120_ft,
                ocfd33_short_ft = c.ocfd_33_short_leg_ft,
                ocfd33_long_ft = c.ocfd_33_long_leg_ft,
                ocfd20_short_ft = c.ocfd_20_short_leg_ft,
                ocfd20_long_ft = c.ocfd_20_long_leg_ft,
                trap_total_ft = c.trap_dipole_total_ft,
                trap_leg_ft = c.trap_dipole_leg_ft,
                skip_min = c.skip_distance_min_km,
                skip_max = c.skip_distance_max_km,
                skip_avg = c.skip_distance_avg_km,
                best_m = best_m,
                best_ft = best_ft,
                clear_pct = clear_pct,
                points = points,
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
                "\"half_wave_m\": {:.2},\n    \"half_wave_corrected_m\": {:.2},\n    \"full_wave_m\": {:.2},\n    \"full_wave_corrected_m\": {:.2},\n    \"quarter_wave_m\": {:.2},\n    \"quarter_wave_corrected_m\": {:.2},\n    \"end_fed_half_wave_m\": {:.2},\n    \"full_wave_loop_circumference_m\": {:.2},\n    \"full_wave_loop_square_side_m\": {:.2},\n    \"inverted_v_total_m\": {:.2},\n    \"inverted_v_leg_m\": {:.2},\n    \"inverted_v_span_90_m\": {:.2},\n    \"inverted_v_span_120_m\": {:.2},\n    \"ocfd_33_short_leg_m\": {:.2},\n    \"ocfd_33_long_leg_m\": {:.2},\n    \"ocfd_20_short_leg_m\": {:.2},\n    \"ocfd_20_long_leg_m\": {:.2},\n    \"trap_dipole_total_m\": {:.2},\n    \"trap_dipole_leg_m\": {:.2}",
                c.half_wave_m,
                c.corrected_half_wave_m,
                c.full_wave_m,
                c.corrected_full_wave_m,
                c.quarter_wave_m,
                c.corrected_quarter_wave_m,
                c.end_fed_half_wave_m,
                c.full_wave_loop_circumference_m,
                c.full_wave_loop_square_side_m,
                c.inverted_v_total_m,
                c.inverted_v_leg_m,
                c.inverted_v_span_90_m,
                c.inverted_v_span_120_m,
                c.ocfd_33_short_leg_m,
                c.ocfd_33_long_leg_m,
                c.ocfd_20_short_leg_m,
                c.ocfd_20_long_leg_m,
                c.trap_dipole_total_m,
                c.trap_dipole_leg_m,
            ),
            UnitSystem::Imperial => format!(
                "\"half_wave_ft\": {:.2},\n    \"half_wave_corrected_ft\": {:.2},\n    \"full_wave_ft\": {:.2},\n    \"full_wave_corrected_ft\": {:.2},\n    \"quarter_wave_ft\": {:.2},\n    \"quarter_wave_corrected_ft\": {:.2},\n    \"end_fed_half_wave_ft\": {:.2},\n    \"full_wave_loop_circumference_ft\": {:.2},\n    \"full_wave_loop_square_side_ft\": {:.2},\n    \"inverted_v_total_ft\": {:.2},\n    \"inverted_v_leg_ft\": {:.2},\n    \"inverted_v_span_90_ft\": {:.2},\n    \"inverted_v_span_120_ft\": {:.2},\n    \"ocfd_33_short_leg_ft\": {:.2},\n    \"ocfd_33_long_leg_ft\": {:.2},\n    \"ocfd_20_short_leg_ft\": {:.2},\n    \"ocfd_20_long_leg_ft\": {:.2},\n    \"trap_dipole_total_ft\": {:.2},\n    \"trap_dipole_leg_ft\": {:.2}",
                c.half_wave_ft,
                c.corrected_half_wave_ft,
                c.full_wave_ft,
                c.corrected_full_wave_ft,
                c.quarter_wave_ft,
                c.corrected_quarter_wave_ft,
                c.end_fed_half_wave_ft,
                c.full_wave_loop_circumference_ft,
                c.full_wave_loop_square_side_ft,
                c.inverted_v_total_ft,
                c.inverted_v_leg_ft,
                c.inverted_v_span_90_ft,
                c.inverted_v_span_120_ft,
                c.ocfd_33_short_leg_ft,
                c.ocfd_33_long_leg_ft,
                c.ocfd_20_short_leg_ft,
                c.ocfd_20_long_leg_ft,
                c.trap_dipole_total_ft,
                c.trap_dipole_leg_ft,
            ),
            UnitSystem::Both => format!(
                "\"half_wave_m\": {:.2},\n    \"half_wave_corrected_m\": {:.2},\n    \"full_wave_m\": {:.2},\n    \"full_wave_corrected_m\": {:.2},\n    \"quarter_wave_m\": {:.2},\n    \"quarter_wave_corrected_m\": {:.2},\n    \"end_fed_half_wave_m\": {:.2},\n    \"full_wave_loop_circumference_m\": {:.2},\n    \"full_wave_loop_square_side_m\": {:.2},\n    \"inverted_v_total_m\": {:.2},\n    \"inverted_v_leg_m\": {:.2},\n    \"inverted_v_span_90_m\": {:.2},\n    \"inverted_v_span_120_m\": {:.2},\n    \"ocfd_33_short_leg_m\": {:.2},\n    \"ocfd_33_long_leg_m\": {:.2},\n    \"ocfd_20_short_leg_m\": {:.2},\n    \"ocfd_20_long_leg_m\": {:.2},\n    \"trap_dipole_total_m\": {:.2},\n    \"trap_dipole_leg_m\": {:.2},\n    \"half_wave_ft\": {:.2},\n    \"half_wave_corrected_ft\": {:.2},\n    \"full_wave_ft\": {:.2},\n    \"full_wave_corrected_ft\": {:.2},\n    \"quarter_wave_ft\": {:.2},\n    \"quarter_wave_corrected_ft\": {:.2},\n    \"end_fed_half_wave_ft\": {:.2},\n    \"full_wave_loop_circumference_ft\": {:.2},\n    \"full_wave_loop_square_side_ft\": {:.2},\n    \"inverted_v_total_ft\": {:.2},\n    \"inverted_v_leg_ft\": {:.2},\n    \"inverted_v_span_90_ft\": {:.2},\n    \"inverted_v_span_120_ft\": {:.2},\n    \"ocfd_33_short_leg_ft\": {:.2},\n    \"ocfd_33_long_leg_ft\": {:.2},\n    \"ocfd_20_short_leg_ft\": {:.2},\n    \"ocfd_20_long_leg_ft\": {:.2},\n    \"trap_dipole_total_ft\": {:.2},\n    \"trap_dipole_leg_ft\": {:.2}",
                c.half_wave_m,
                c.corrected_half_wave_m,
                c.full_wave_m,
                c.corrected_full_wave_m,
                c.quarter_wave_m,
                c.corrected_quarter_wave_m,
                c.end_fed_half_wave_m,
                c.full_wave_loop_circumference_m,
                c.full_wave_loop_square_side_m,
                c.inverted_v_total_m,
                c.inverted_v_leg_m,
                c.inverted_v_span_90_m,
                c.inverted_v_span_120_m,
                c.ocfd_33_short_leg_m,
                c.ocfd_33_long_leg_m,
                c.ocfd_20_short_leg_m,
                c.ocfd_20_long_leg_m,
                c.trap_dipole_total_m,
                c.trap_dipole_leg_m,
                c.half_wave_ft,
                c.corrected_half_wave_ft,
                c.full_wave_ft,
                c.corrected_full_wave_ft,
                c.quarter_wave_ft,
                c.corrected_quarter_wave_ft,
                c.end_fed_half_wave_ft,
                c.full_wave_loop_circumference_ft,
                c.full_wave_loop_square_side_ft,
                c.inverted_v_total_ft,
                c.inverted_v_leg_ft,
                c.inverted_v_span_90_ft,
                c.inverted_v_span_120_ft,
                c.ocfd_33_short_leg_ft,
                c.ocfd_33_long_leg_ft,
                c.ocfd_20_short_leg_ft,
                c.ocfd_20_long_leg_ft,
                c.trap_dipole_total_ft,
                c.trap_dipole_leg_ft,
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
            out.push_str("| Band | Ratio | Freq (MHz) | Half-wave (m) | Half-wave corrected (m) | Full-wave (m) | Full-wave corrected (m) | Quarter-wave (m) | Quarter-wave corrected (m) | EFHW (m) | Loop circumference (m) | Loop side (m) | Inverted-V total (m) | Inverted-V leg (m) | Inverted-V span 90° (m) | Inverted-V span 120° (m) | OCFD 33 short (m) | OCFD 33 long (m) | OCFD 20 short (m) | OCFD 20 long (m) | Trap dipole total (m) | Trap dipole leg (m) | Skip Min (km) | Skip Max (km) | Skip Avg (km) |\n");
            out.push_str("|------|-------|------------|---------------|--------------------------|---------------|--------------------------|------------------|-----------------------------|----------|------------------------|---------------|----------------------|--------------------|------------------------|-------------------------|-------------------|------------------|-------------------|------------------|---------------|---------------|---------------|\n");
            for c in calculations {
                out.push_str(&format!(
                    "| {band} | {ratio} | {freq:.3} | {half:.2} | {half_corr:.2} | {full:.2} | {full_corr:.2} | {quarter:.2} | {quarter_corr:.2} | {efhw:.2} | {loop_circ:.2} | {loop_side:.2} | {inv_total:.2} | {inv_leg:.2} | {inv_span_90:.2} | {inv_span_120:.2} | {ocfd33_short:.2} | {ocfd33_long:.2} | {ocfd20_short:.2} | {ocfd20_long:.2} | {trap_total:.2} | {trap_leg:.2} | {skip_min:.0} | {skip_max:.0} | {skip_avg:.0} |\n",
                    band = c.band_name,
                    ratio = c.transformer_ratio_label,
                    freq = c.frequency_mhz,
                    half = c.half_wave_m,
                    half_corr = c.corrected_half_wave_m,
                    full = c.full_wave_m,
                    full_corr = c.corrected_full_wave_m,
                    quarter = c.quarter_wave_m,
                    quarter_corr = c.corrected_quarter_wave_m,
                    efhw = c.end_fed_half_wave_m,
                    loop_circ = c.full_wave_loop_circumference_m,
                    loop_side = c.full_wave_loop_square_side_m,
                    inv_total = c.inverted_v_total_m,
                    inv_leg = c.inverted_v_leg_m,
                    inv_span_90 = c.inverted_v_span_90_m,
                    inv_span_120 = c.inverted_v_span_120_m,
                    ocfd33_short = c.ocfd_33_short_leg_m,
                    ocfd33_long = c.ocfd_33_long_leg_m,
                    ocfd20_short = c.ocfd_20_short_leg_m,
                    ocfd20_long = c.ocfd_20_long_leg_m,
                    trap_total = c.trap_dipole_total_m,
                    trap_leg = c.trap_dipole_leg_m,
                    skip_min = c.skip_distance_min_km,
                    skip_max = c.skip_distance_max_km,
                    skip_avg = c.skip_distance_avg_km,
                ));
            }
        }
        UnitSystem::Imperial => {
            out.push_str("| Band | Ratio | Freq (MHz) | Half-wave (ft) | Half-wave corrected (ft) | Full-wave (ft) | Full-wave corrected (ft) | Quarter-wave (ft) | Quarter-wave corrected (ft) | EFHW (ft) | Loop circumference (ft) | Loop side (ft) | Inverted-V total (ft) | Inverted-V leg (ft) | Inverted-V span 90° (ft) | Inverted-V span 120° (ft) | OCFD 33 short (ft) | OCFD 33 long (ft) | OCFD 20 short (ft) | OCFD 20 long (ft) | Trap dipole total (ft) | Trap dipole leg (ft) | Skip Min (km) | Skip Max (km) | Skip Avg (km) |\n");
            out.push_str("|------|-------|------------|----------------|--------------------------|----------------|--------------------------|-------------------|-----------------------------|-----------|-------------------------|----------------|-----------------------|---------------------|-------------------------|--------------------------|--------------------|-------------------|--------------------|-------------------|---------------|---------------|---------------|\n");
            for c in calculations {
                out.push_str(&format!(
                    "| {band} | {ratio} | {freq:.3} | {half:.2} | {half_corr:.2} | {full:.2} | {full_corr:.2} | {quarter:.2} | {quarter_corr:.2} | {efhw:.2} | {loop_circ:.2} | {loop_side:.2} | {inv_total:.2} | {inv_leg:.2} | {inv_span_90:.2} | {inv_span_120:.2} | {ocfd33_short:.2} | {ocfd33_long:.2} | {ocfd20_short:.2} | {ocfd20_long:.2} | {trap_total:.2} | {trap_leg:.2} | {skip_min:.0} | {skip_max:.0} | {skip_avg:.0} |\n",
                    band = c.band_name,
                    ratio = c.transformer_ratio_label,
                    freq = c.frequency_mhz,
                    half = c.half_wave_ft,
                    half_corr = c.corrected_half_wave_ft,
                    full = c.full_wave_ft,
                    full_corr = c.corrected_full_wave_ft,
                    quarter = c.quarter_wave_ft,
                    quarter_corr = c.corrected_quarter_wave_ft,
                    efhw = c.end_fed_half_wave_ft,
                    loop_circ = c.full_wave_loop_circumference_ft,
                    loop_side = c.full_wave_loop_square_side_ft,
                    inv_total = c.inverted_v_total_ft,
                    inv_leg = c.inverted_v_leg_ft,
                    inv_span_90 = c.inverted_v_span_90_ft,
                    inv_span_120 = c.inverted_v_span_120_ft,
                    ocfd33_short = c.ocfd_33_short_leg_ft,
                    ocfd33_long = c.ocfd_33_long_leg_ft,
                    ocfd20_short = c.ocfd_20_short_leg_ft,
                    ocfd20_long = c.ocfd_20_long_leg_ft,
                    trap_total = c.trap_dipole_total_ft,
                    trap_leg = c.trap_dipole_leg_ft,
                    skip_min = c.skip_distance_min_km,
                    skip_max = c.skip_distance_max_km,
                    skip_avg = c.skip_distance_avg_km,
                ));
            }
        }
        UnitSystem::Both => {
            out.push_str("| Band | Ratio | Freq (MHz) | Half-wave (m) | Half-wave corr (m) | Half-wave (ft) | Half-wave corr (ft) | Full-wave (m) | Full-wave corr (m) | Full-wave (ft) | Full-wave corr (ft) | Quarter-wave (m) | Quarter-wave corr (m) | Quarter-wave (ft) | Quarter-wave corr (ft) | EFHW (m) | EFHW (ft) | Loop circ. (m) | Loop circ. (ft) | Loop side (m) | Loop side (ft) | Inverted-V total (m) | Inverted-V total (ft) | Inverted-V leg (m) | Inverted-V leg (ft) | Inverted-V span 90° (m) | Inverted-V span 90° (ft) | Inverted-V span 120° (m) | Inverted-V span 120° (ft) | OCFD 33 short (m) | OCFD 33 long (m) | OCFD 20 short (m) | OCFD 20 long (m) | OCFD 33 short (ft) | OCFD 33 long (ft) | OCFD 20 short (ft) | OCFD 20 long (ft) | Trap dipole total (m) | Trap dipole leg (m) | Trap dipole total (ft) | Trap dipole leg (ft) | Skip Min (km) | Skip Max (km) | Skip Avg (km) |\n");
            out.push_str("|------|-------|------------|---------------|--------------------|----------------|---------------------|---------------|--------------------|----------------|---------------------|------------------|-----------------------|-------------------|------------------------|----------|-----------|----------------|-----------------|---------------|----------------|----------------------|-----------------------|--------------------|---------------------|------------------------|-------------------------|-------------------------|--------------------------|-------------------|------------------|-------------------|------------------|--------------------|-------------------|--------------------|-------------------|---------------|---------------|---------------|\n");
            for c in calculations {
                out.push_str(&format!(
                    "| {band} | {ratio} | {freq:.3} | {half_m:.2} | {half_corr_m:.2} | {half_ft:.2} | {half_corr_ft:.2} | {full_m:.2} | {full_corr_m:.2} | {full_ft:.2} | {full_corr_ft:.2} | {quarter_m:.2} | {quarter_corr_m:.2} | {quarter_ft:.2} | {quarter_corr_ft:.2} | {efhw_m:.2} | {efhw_ft:.2} | {loop_circ_m:.2} | {loop_circ_ft:.2} | {loop_side_m:.2} | {loop_side_ft:.2} | {inv_total_m:.2} | {inv_total_ft:.2} | {inv_leg_m:.2} | {inv_leg_ft:.2} | {inv_span_90_m:.2} | {inv_span_90_ft:.2} | {inv_span_120_m:.2} | {inv_span_120_ft:.2} | {ocfd33_short_m:.2} | {ocfd33_long_m:.2} | {ocfd20_short_m:.2} | {ocfd20_long_m:.2} | {ocfd33_short_ft:.2} | {ocfd33_long_ft:.2} | {ocfd20_short_ft:.2} | {ocfd20_long_ft:.2} | {trap_total_m:.2} | {trap_leg_m:.2} | {trap_total_ft:.2} | {trap_leg_ft:.2} | {skip_min:.0} | {skip_max:.0} | {skip_avg:.0} |\n",
                    band = c.band_name,
                    ratio = c.transformer_ratio_label,
                    freq = c.frequency_mhz,
                    half_m = c.half_wave_m,
                    half_corr_m = c.corrected_half_wave_m,
                    half_ft = c.half_wave_ft,
                    half_corr_ft = c.corrected_half_wave_ft,
                    full_m = c.full_wave_m,
                    full_corr_m = c.corrected_full_wave_m,
                    full_ft = c.full_wave_ft,
                    full_corr_ft = c.corrected_full_wave_ft,
                    quarter_m = c.quarter_wave_m,
                    quarter_corr_m = c.corrected_quarter_wave_m,
                    quarter_ft = c.quarter_wave_ft,
                    quarter_corr_ft = c.corrected_quarter_wave_ft,
                    efhw_m = c.end_fed_half_wave_m,
                    efhw_ft = c.end_fed_half_wave_ft,
                    loop_circ_m = c.full_wave_loop_circumference_m,
                    loop_circ_ft = c.full_wave_loop_circumference_ft,
                    loop_side_m = c.full_wave_loop_square_side_m,
                    loop_side_ft = c.full_wave_loop_square_side_ft,
                    inv_total_m = c.inverted_v_total_m,
                    inv_total_ft = c.inverted_v_total_ft,
                    inv_leg_m = c.inverted_v_leg_m,
                    inv_leg_ft = c.inverted_v_leg_ft,
                    inv_span_90_m = c.inverted_v_span_90_m,
                    inv_span_90_ft = c.inverted_v_span_90_ft,
                    inv_span_120_m = c.inverted_v_span_120_m,
                    inv_span_120_ft = c.inverted_v_span_120_ft,
                    ocfd33_short_m = c.ocfd_33_short_leg_m,
                    ocfd33_long_m = c.ocfd_33_long_leg_m,
                    ocfd20_short_m = c.ocfd_20_short_leg_m,
                    ocfd20_long_m = c.ocfd_20_long_leg_m,
                    ocfd33_short_ft = c.ocfd_33_short_leg_ft,
                    ocfd33_long_ft = c.ocfd_33_long_leg_ft,
                    ocfd20_short_ft = c.ocfd_20_short_leg_ft,
                    ocfd20_long_ft = c.ocfd_20_long_leg_ft,
                    trap_total_m = c.trap_dipole_total_m,
                    trap_leg_m = c.trap_dipole_leg_m,
                    trap_total_ft = c.trap_dipole_total_ft,
                    trap_leg_ft = c.trap_dipole_leg_ft,
                    skip_min = c.skip_distance_min_km,
                    skip_max = c.skip_distance_max_km,
                    skip_avg = c.skip_distance_avg_km,
                ));
            }
        }
    }

    out.push_str("\n## Non-Resonant Recommendation\n\n");
    match (recommendation, units) {
        (Some(r), UnitSystem::Metric) => {
            out.push_str("| Length (m) | Resonance Clearance (%) |\n");
            out.push_str("|------------|-------------------------|\n");
            out.push_str(&format!(
                "| {:.2} | {:.2} |\n",
                r.length_m, r.min_resonance_clearance_pct
            ));
        }
        (Some(r), UnitSystem::Imperial) => {
            out.push_str("| Length (ft) | Resonance Clearance (%) |\n");
            out.push_str("|-------------|-------------------------|\n");
            out.push_str(&format!(
                "| {:.2} | {:.2} |\n",
                r.length_ft, r.min_resonance_clearance_pct
            ));
        }
        (Some(r), UnitSystem::Both) => {
            out.push_str("| Length (m) | Length (ft) | Resonance Clearance (%) |\n");
            out.push_str("|------------|-------------|-------------------------|\n");
            out.push_str(&format!(
                "| {:.2} | {:.2} | {:.2} |\n",
                r.length_m, r.length_ft, r.min_resonance_clearance_pct
            ));
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
                "  Transformer ratio: {}\n  Half-wave: {:.2} m (corrected: {:.2} m)\n  Full-wave: {:.2} m (corrected: {:.2} m)\n  Quarter-wave: {:.2} m (corrected: {:.2} m)\n  End-fed half-wave: {:.2} m\n  Full-wave loop circumference: {:.2} m\n  Full-wave loop square side: {:.2} m\n  Inverted-V total: {:.2} m\n  Inverted-V leg: {:.2} m\n  Inverted-V span at 90 deg apex: {:.2} m\n  Inverted-V span at 120 deg apex: {:.2} m\n  OCFD 33/67 legs: {:.2} m / {:.2} m\n  OCFD 20/80 legs: {:.2} m / {:.2} m\n  Trap dipole: {:.2} m total / {:.2} m each element",
                c.transformer_ratio_label,
                c.half_wave_m,
                c.corrected_half_wave_m,
                c.full_wave_m,
                c.corrected_full_wave_m,
                c.quarter_wave_m,
                c.corrected_quarter_wave_m,
                c.end_fed_half_wave_m,
                c.full_wave_loop_circumference_m,
                c.full_wave_loop_square_side_m,
                c.inverted_v_total_m,
                c.inverted_v_leg_m,
                c.inverted_v_span_90_m,
                c.inverted_v_span_120_m,
                c.ocfd_33_short_leg_m,
                c.ocfd_33_long_leg_m,
                c.ocfd_20_short_leg_m,
                c.ocfd_20_long_leg_m,
                c.trap_dipole_total_m,
                c.trap_dipole_leg_m,
            ),
            UnitSystem::Imperial => format!(
                "  Transformer ratio: {}\n  Half-wave: {:.2} ft (corrected: {:.2} ft)\n  Full-wave: {:.2} ft (corrected: {:.2} ft)\n  Quarter-wave: {:.2} ft (corrected: {:.2} ft)\n  End-fed half-wave: {:.2} ft\n  Full-wave loop circumference: {:.2} ft\n  Full-wave loop square side: {:.2} ft\n  Inverted-V total: {:.2} ft\n  Inverted-V leg: {:.2} ft\n  Inverted-V span at 90 deg apex: {:.2} ft\n  Inverted-V span at 120 deg apex: {:.2} ft\n  OCFD 33/67 legs: {:.2} ft / {:.2} ft\n  OCFD 20/80 legs: {:.2} ft / {:.2} ft\n  Trap dipole: {:.2} ft total / {:.2} ft each element",
                c.transformer_ratio_label,
                c.half_wave_ft,
                c.corrected_half_wave_ft,
                c.full_wave_ft,
                c.corrected_full_wave_ft,
                c.quarter_wave_ft,
                c.corrected_quarter_wave_ft,
                c.end_fed_half_wave_ft,
                c.full_wave_loop_circumference_ft,
                c.full_wave_loop_square_side_ft,
                c.inverted_v_total_ft,
                c.inverted_v_leg_ft,
                c.inverted_v_span_90_ft,
                c.inverted_v_span_120_ft,
                c.ocfd_33_short_leg_ft,
                c.ocfd_33_long_leg_ft,
                c.ocfd_20_short_leg_ft,
                c.ocfd_20_long_leg_ft,
                c.trap_dipole_total_ft,
                c.trap_dipole_leg_ft,
            ),
            UnitSystem::Both => format!(
                "  Transformer ratio: {}\n  Half-wave: {:.2} m ({:.2} ft), corrected: {:.2} m ({:.2} ft)\n  Full-wave: {:.2} m ({:.2} ft), corrected: {:.2} m ({:.2} ft)\n  Quarter-wave: {:.2} m ({:.2} ft), corrected: {:.2} m ({:.2} ft)\n  End-fed half-wave: {:.2} m ({:.2} ft)\n  Full-wave loop circumference: {:.2} m ({:.2} ft)\n  Full-wave loop square side: {:.2} m ({:.2} ft)\n  Inverted-V total: {:.2} m ({:.2} ft)\n  Inverted-V leg: {:.2} m ({:.2} ft)\n  Inverted-V span at 90 deg apex: {:.2} m ({:.2} ft)\n  Inverted-V span at 120 deg apex: {:.2} m ({:.2} ft)\n  OCFD 33/67 legs: {:.2} m / {:.2} m ({:.2} ft / {:.2} ft)\n  OCFD 20/80 legs: {:.2} m / {:.2} m ({:.2} ft / {:.2} ft)\n  Trap dipole: {:.2} m / {:.2} m each ({:.2} ft / {:.2} ft each)",
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
                c.end_fed_half_wave_m,
                c.end_fed_half_wave_ft,
                c.full_wave_loop_circumference_m,
                c.full_wave_loop_circumference_ft,
                c.full_wave_loop_square_side_m,
                c.full_wave_loop_square_side_ft,
                c.inverted_v_total_m,
                c.inverted_v_total_ft,
                c.inverted_v_leg_m,
                c.inverted_v_leg_ft,
                c.inverted_v_span_90_m,
                c.inverted_v_span_90_ft,
                c.inverted_v_span_120_m,
                c.inverted_v_span_120_ft,
                c.ocfd_33_short_leg_m,
                c.ocfd_33_long_leg_m,
                c.ocfd_33_short_leg_ft,
                c.ocfd_33_long_leg_ft,
                c.ocfd_20_short_leg_m,
                c.ocfd_20_long_leg_m,
                c.ocfd_20_short_leg_ft,
                c.ocfd_20_long_leg_ft,
                c.trap_dipole_total_m,
                c.trap_dipole_leg_m,
                c.trap_dipole_total_ft,
                c.trap_dipole_leg_ft,
            ),
        };
        out.push_str(&format!(
            "{}\n  Frequency: {:.3} MHz\n{}\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)\n\n",
            c.band_name,
            c.frequency_mhz,
            lengths,
            c.skip_distance_min_km,
            c.skip_distance_max_km,
            c.skip_distance_avg_km,
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

pub fn to_advise_csv(assumed_feedpoint_ohm: f64, candidates: &[AdviseCandidate]) -> String {
    let mut out = String::from(
        "rank,ratio,recommended_length_m,recommended_length_ft,clearance_pct,estimated_efficiency_pct,mismatch_loss_db,average_length_shift_pct,score,tradeoff_note,validated,validation_status,validation_note,assumed_feedpoint_ohm\n",
    );
    for (idx, c) in candidates.iter().enumerate() {
        let note = c.validation_note.as_deref().unwrap_or("");
        let escaped_note = csv_escape(note);
        let escaped_tradeoff = csv_escape(&c.tradeoff_note);
        let status = c
            .validation_status
            .map(|value| value.as_str())
            .unwrap_or("");
        out.push_str(&format!(
            "{},{},{:.2},{:.2},{:.2},{:.2},{:.3},{:.2},{:.2},\"{}\",{},\"{}\",\"{}\",{:.0}\n",
            idx + 1,
            c.ratio.as_label(),
            c.recommended_length_m,
            c.recommended_length_ft,
            c.min_resonance_clearance_pct,
            c.estimated_efficiency_pct,
            c.mismatch_loss_db,
            c.average_length_shift_pct,
            c.score,
            escaped_tradeoff,
            c.validated,
            status,
            escaped_note,
            assumed_feedpoint_ohm,
        ));
    }
    out
}

pub fn to_advise_json(assumed_feedpoint_ohm: f64, candidates: &[AdviseCandidate]) -> String {
    let mut out = String::from("{\n");
    out.push_str(&format!(
        "  \"assumed_feedpoint_ohm\": {:.0},\n",
        assumed_feedpoint_ohm
    ));
    out.push_str("  \"candidates\": [\n");
    for (idx, c) in candidates.iter().enumerate() {
        let comma = if idx + 1 == candidates.len() { "" } else { "," };
        let note_json = c
            .validation_note
            .as_ref()
            .map(|note| format!("\"{}\"", json_escape(note)))
            .unwrap_or_else(|| "null".to_string());
        let status_json = c
            .validation_status
            .map(|status| format!("\"{}\"", status.as_str()))
            .unwrap_or_else(|| "null".to_string());
        let tradeoff_json = format!("\"{}\"", json_escape(&c.tradeoff_note));
        out.push_str(&format!(
            "    {{\"rank\": {}, \"ratio\": \"{}\", \"recommended_length_m\": {:.2}, \"recommended_length_ft\": {:.2}, \"clearance_pct\": {:.2}, \"estimated_efficiency_pct\": {:.2}, \"mismatch_loss_db\": {:.3}, \"average_length_shift_pct\": {:.2}, \"score\": {:.2}, \"tradeoff_note\": {}, \"validated\": {}, \"validation_status\": {}, \"validation_note\": {}}}{}\n",
            idx + 1,
            c.ratio.as_label(),
            c.recommended_length_m,
            c.recommended_length_ft,
            c.min_resonance_clearance_pct,
            c.estimated_efficiency_pct,
            c.mismatch_loss_db,
            c.average_length_shift_pct,
            c.score,
            tradeoff_json,
            c.validated,
            status_json,
            note_json,
            comma,
        ));
    }
    out.push_str("  ]\n}");
    out
}

pub fn to_advise_markdown(assumed_feedpoint_ohm: f64, candidates: &[AdviseCandidate]) -> String {
    let mut out = String::from("# Rusty Wire Advise Candidates\n\n");
    out.push_str(&format!(
        "Assumed feedpoint impedance: {:.0} ohm\n\n",
        assumed_feedpoint_ohm
    ));
    out.push_str("| Rank | Ratio | Wire (m) | Wire (ft) | Clearance (%) | Efficiency (%) | Mismatch Loss (dB) | Shift (%) | Score | Validated | Validation Status | Validation Note | Tradeoff Note |\n");
    out.push_str("|------|-------|----------|-----------|---------------|----------------|--------------------|-----------|-------|-----------|-------------------|-----------------|---------------|\n");
    for (idx, c) in candidates.iter().enumerate() {
        let status = c
            .validation_status
            .map(|value| value.as_str())
            .unwrap_or("");
        let note = c
            .validation_note
            .as_deref()
            .unwrap_or("")
            .replace('|', "\\|")
            .replace('\n', " ");
        let tradeoff = c.tradeoff_note.replace('|', "\\|").replace('\n', " ");
        out.push_str(&format!(
            "| {} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.3} | {:.2} | {:.2} | {} | {} | {} | {} |\n",
            idx + 1,
            c.ratio.as_label(),
            c.recommended_length_m,
            c.recommended_length_ft,
            c.min_resonance_clearance_pct,
            c.estimated_efficiency_pct,
            c.mismatch_loss_db,
            c.average_length_shift_pct,
            c.score,
            if c.validated { "yes" } else { "no" },
            status,
            note,
            tradeoff,
        ));
    }
    out
}

pub fn to_advise_txt(assumed_feedpoint_ohm: f64, candidates: &[AdviseCandidate]) -> String {
    let mut out = String::from("Rusty Wire Advise Candidates\n");
    out.push_str("============================================================\n");
    out.push_str(&format!(
        "Assumed feedpoint impedance: {:.0} ohm\n\n",
        assumed_feedpoint_ohm
    ));
    for (idx, c) in candidates.iter().enumerate() {
        out.push_str(&format!(
            "{:2}. ratio {}  wire {:.2} m ({:.2} ft)\n",
            idx + 1,
            c.ratio.as_label(),
            c.recommended_length_m,
            c.recommended_length_ft,
        ));
        out.push_str(&format!(
            "    efficiency {:.2}%  mismatch loss {:.3} dB  clearance {:.2}%\n",
            c.estimated_efficiency_pct, c.mismatch_loss_db, c.min_resonance_clearance_pct
        ));
        out.push_str(&format!(
            "    score {:.2}  correction shift {:.2}%\n",
            c.score, c.average_length_shift_pct
        ));
        out.push_str(&format!("    note: {}\n", c.tradeoff_note));
        out.push_str(&format!(
            "    fnec validated {}\n",
            if c.validated { "yes" } else { "no" }
        ));
        if let Some(status) = c.validation_status {
            out.push_str(&format!("    fnec status: {}\n", status.as_str()));
        }
        if let Some(note) = &c.validation_note {
            out.push_str(&format!("    fnec note: {}\n", note.replace('\n', " ")));
        }
        out.push('\n');
    }
    out
}

pub fn to_html(
    calculations: &[WireCalculation],
    recommendation: Option<&NonResonantRecommendation>,
    units: UnitSystem,
    wire_min_m: f64,
    wire_max_m: f64,
) -> String {
    let mut out = String::from(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"UTF-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n\
         <title>Rusty Wire Results</title>\n\
         <style>\n\
         body{font-family:system-ui,sans-serif;margin:2rem;color:#222;}\n\
         h1{color:#1a5276;}h2{color:#2874a6;margin-top:2rem;}\n\
         table{border-collapse:collapse;font-size:0.85rem;width:100%;overflow-x:auto;display:block;}\n\
         th{background:#2874a6;color:#fff;padding:6px 10px;text-align:left;white-space:nowrap;}\n\
         td{border:1px solid #ccc;padding:5px 9px;white-space:nowrap;}\n\
         tr:nth-child(even) td{background:#f4f6f9;}\n\
         .note{color:#555;font-style:italic;margin-top:0.5rem;}\n\
         </style>\n\
         </head>\n\
         <body>\n\
         <h1>Rusty Wire Results</h1>\n",
    );

    out.push_str("<h2>Band Calculations</h2>\n<table>\n<thead><tr>\n");

    // Build ordered column definitions: (header, value-fn) — unit-aware.
    // We use a Vec<(&str, String)> per row to avoid repeating all three unit branches.
    struct Col {
        header: &'static str,
        values: Vec<String>,
    }

    macro_rules! col {
        ($hdr:expr, $vals:expr) => {
            Col {
                header: $hdr,
                values: $vals,
            }
        };
    }

    let mut cols: Vec<Col> = vec![
        col!(
            "Band",
            calculations
                .iter()
                .map(|c| html_escape(&c.band_name))
                .collect()
        ),
        col!(
            "Ratio",
            calculations
                .iter()
                .map(|c| html_escape(c.transformer_ratio_label))
                .collect()
        ),
        col!(
            "Freq (MHz)",
            calculations
                .iter()
                .map(|c| format!("{:.3}", c.frequency_mhz))
                .collect()
        ),
    ];

    match units {
        UnitSystem::Metric | UnitSystem::Both => {
            cols.push(col!(
                "Half-wave (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.half_wave_m))
                    .collect()
            ));
            cols.push(col!(
                "Half-wave corr (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.corrected_half_wave_m))
                    .collect()
            ));
            cols.push(col!(
                "Full-wave (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.full_wave_m))
                    .collect()
            ));
            cols.push(col!(
                "Full-wave corr (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.corrected_full_wave_m))
                    .collect()
            ));
            cols.push(col!(
                "Quarter-wave (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.quarter_wave_m))
                    .collect()
            ));
            cols.push(col!(
                "Quarter-wave corr (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.corrected_quarter_wave_m))
                    .collect()
            ));
            cols.push(col!(
                "EFHW (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.end_fed_half_wave_m))
                    .collect()
            ));
            cols.push(col!(
                "Loop circ (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.full_wave_loop_circumference_m))
                    .collect()
            ));
            cols.push(col!(
                "Loop side (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.full_wave_loop_square_side_m))
                    .collect()
            ));
            cols.push(col!(
                "Inv-V total (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.inverted_v_total_m))
                    .collect()
            ));
            cols.push(col!(
                "Inv-V leg (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.inverted_v_leg_m))
                    .collect()
            ));
            cols.push(col!(
                "Inv-V span 90° (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.inverted_v_span_90_m))
                    .collect()
            ));
            cols.push(col!(
                "Inv-V span 120° (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.inverted_v_span_120_m))
                    .collect()
            ));
            cols.push(col!(
                "OCFD 33 short (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.ocfd_33_short_leg_m))
                    .collect()
            ));
            cols.push(col!(
                "OCFD 33 long (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.ocfd_33_long_leg_m))
                    .collect()
            ));
            cols.push(col!(
                "OCFD 20 short (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.ocfd_20_short_leg_m))
                    .collect()
            ));
            cols.push(col!(
                "OCFD 20 long (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.ocfd_20_long_leg_m))
                    .collect()
            ));
            cols.push(col!(
                "Trap total (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.trap_dipole_total_m))
                    .collect()
            ));
            cols.push(col!(
                "Trap leg (m)",
                calculations
                    .iter()
                    .map(|c| format!("{:.2}", c.trap_dipole_leg_m))
                    .collect()
            ));
        }
        _ => {}
    }
    if matches!(units, UnitSystem::Imperial | UnitSystem::Both) {
        cols.push(col!(
            "Half-wave (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.half_wave_ft))
                .collect()
        ));
        cols.push(col!(
            "Half-wave corr (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.corrected_half_wave_ft))
                .collect()
        ));
        cols.push(col!(
            "Full-wave (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.full_wave_ft))
                .collect()
        ));
        cols.push(col!(
            "Full-wave corr (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.corrected_full_wave_ft))
                .collect()
        ));
        cols.push(col!(
            "Quarter-wave (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.quarter_wave_ft))
                .collect()
        ));
        cols.push(col!(
            "Quarter-wave corr (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.corrected_quarter_wave_ft))
                .collect()
        ));
        cols.push(col!(
            "EFHW (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.end_fed_half_wave_ft))
                .collect()
        ));
        cols.push(col!(
            "Loop circ (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.full_wave_loop_circumference_ft))
                .collect()
        ));
        cols.push(col!(
            "Loop side (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.full_wave_loop_square_side_ft))
                .collect()
        ));
        cols.push(col!(
            "Inv-V total (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.inverted_v_total_ft))
                .collect()
        ));
        cols.push(col!(
            "Inv-V leg (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.inverted_v_leg_ft))
                .collect()
        ));
        cols.push(col!(
            "Inv-V span 90° (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.inverted_v_span_90_ft))
                .collect()
        ));
        cols.push(col!(
            "Inv-V span 120° (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.inverted_v_span_120_ft))
                .collect()
        ));
        cols.push(col!(
            "OCFD 33 short (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.ocfd_33_short_leg_ft))
                .collect()
        ));
        cols.push(col!(
            "OCFD 33 long (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.ocfd_33_long_leg_ft))
                .collect()
        ));
        cols.push(col!(
            "OCFD 20 short (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.ocfd_20_short_leg_ft))
                .collect()
        ));
        cols.push(col!(
            "OCFD 20 long (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.ocfd_20_long_leg_ft))
                .collect()
        ));
        cols.push(col!(
            "Trap total (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.trap_dipole_total_ft))
                .collect()
        ));
        cols.push(col!(
            "Trap leg (ft)",
            calculations
                .iter()
                .map(|c| format!("{:.2}", c.trap_dipole_leg_ft))
                .collect()
        ));
    }
    cols.push(col!(
        "Skip min (km)",
        calculations
            .iter()
            .map(|c| format!("{:.0}", c.skip_distance_min_km))
            .collect()
    ));
    cols.push(col!(
        "Skip max (km)",
        calculations
            .iter()
            .map(|c| format!("{:.0}", c.skip_distance_max_km))
            .collect()
    ));
    cols.push(col!(
        "Skip avg (km)",
        calculations
            .iter()
            .map(|c| format!("{:.0}", c.skip_distance_avg_km))
            .collect()
    ));

    for col in &cols {
        out.push_str(&format!("<th>{}</th>\n", col.header));
    }
    out.push_str("</tr></thead>\n<tbody>\n");

    for row_idx in 0..calculations.len() {
        out.push_str("<tr>\n");
        for col in &cols {
            out.push_str(&format!("<td>{}</td>\n", col.values[row_idx]));
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</tbody>\n</table>\n");

    // Non-resonant recommendation
    out.push_str("<h2>Non-Resonant Recommendation</h2>\n");
    match (recommendation, units) {
        (Some(r), UnitSystem::Metric) => {
            out.push_str(&format!(
                "<p><strong>{:.2} m</strong> — resonance clearance: {:.2}%</p>\n",
                r.length_m, r.min_resonance_clearance_pct
            ));
        }
        (Some(r), UnitSystem::Imperial) => {
            out.push_str(&format!(
                "<p><strong>{:.2} ft</strong> — resonance clearance: {:.2}%</p>\n",
                r.length_ft, r.min_resonance_clearance_pct
            ));
        }
        (Some(r), UnitSystem::Both) => {
            out.push_str(&format!(
                "<p><strong>{:.2} m ({:.2} ft)</strong> — resonance clearance: {:.2}%</p>\n",
                r.length_m, r.length_ft, r.min_resonance_clearance_pct
            ));
        }
        (None, _) => out.push_str("<p class=\"note\">No recommendation available.</p>\n"),
    }

    // Resonant points
    out.push_str("<h2>Resonant Points in Search Window</h2>\n");
    out.push_str(&format!(
        "<p class=\"note\">Window: {:.2}–{:.2} m ({:.2}–{:.2} ft)</p>\n",
        wire_min_m,
        wire_max_m,
        wire_min_m / 0.3048,
        wire_max_m / 0.3048,
    ));

    let mut points_rows = String::new();
    for c in calculations {
        for (harmonic, len_m) in collect_band_resonant_points_m(c, wire_min_m, wire_max_m) {
            let len_cell = match units {
                UnitSystem::Metric => format!("{:.2} m", len_m),
                UnitSystem::Imperial => format!("{:.2} ft", len_m / 0.3048),
                UnitSystem::Both => format!("{:.2} m ({:.2} ft)", len_m, len_m / 0.3048),
            };
            points_rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                html_escape(&c.band_name),
                harmonic,
                len_cell
            ));
        }
    }
    if points_rows.is_empty() {
        out.push_str("<p class=\"note\">No resonant points in this window.</p>\n");
    } else {
        out.push_str("<table>\n<thead><tr><th>Band</th><th>Harmonic</th><th>Length</th></tr></thead>\n<tbody>\n");
        out.push_str(&points_rows);
        out.push_str("</tbody>\n</table>\n");
    }

    out.push_str("</body>\n</html>\n");
    out
}

pub fn to_advise_html(assumed_feedpoint_ohm: f64, candidates: &[AdviseCandidate]) -> String {
    let mut out = String::from(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"UTF-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n\
         <title>Rusty Wire Advise</title>\n\
         <style>\n\
         body{font-family:system-ui,sans-serif;margin:2rem;color:#222;}\n\
         h1{color:#1a5276;}h2{color:#2874a6;margin-top:2rem;}\n\
         table{border-collapse:collapse;font-size:0.85rem;}\n\
         th{background:#2874a6;color:#fff;padding:6px 10px;text-align:left;white-space:nowrap;}\n\
         td{border:1px solid #ccc;padding:5px 9px;}\n\
         tr:nth-child(even) td{background:#f4f6f9;}\n\
         .note{color:#555;font-style:italic;}\n\
         </style>\n\
         </head>\n\
         <body>\n\
         <h1>Rusty Wire Advise Candidates</h1>\n",
    );
    out.push_str(&format!(
        "<p>Assumed feedpoint impedance: <strong>{:.0} Ω</strong></p>\n",
        assumed_feedpoint_ohm
    ));
    out.push_str(
        "<table>\n<thead><tr>\
         <th>Rank</th><th>Ratio</th>\
         <th>Length (m)</th><th>Length (ft)</th>\
         <th>Clearance (%)</th><th>Efficiency (%)</th>\
         <th>Mismatch loss (dB)</th><th>Score</th>\
         <th>Validated</th><th>Status</th><th>Tradeoff note</th>\
         </tr></thead>\n<tbody>\n",
    );
    for (idx, c) in candidates.iter().enumerate() {
        let status = c
            .validation_status
            .map(|s| s.as_str().to_string())
            .unwrap_or_else(|| "—".to_string());
        out.push_str(&format!(
            "<tr><td>{rank}</td><td>{ratio}</td>\
             <td>{len_m:.2}</td><td>{len_ft:.2}</td>\
             <td>{clear:.2}</td><td>{eff:.2}</td>\
             <td>{loss:.3}</td><td>{score:.2}</td>\
             <td>{validated}</td><td>{status}</td><td>{note}</td></tr>\n",
            rank = idx + 1,
            ratio = html_escape(c.ratio.as_label()),
            len_m = c.recommended_length_m,
            len_ft = c.recommended_length_ft,
            clear = c.min_resonance_clearance_pct,
            eff = c.estimated_efficiency_pct,
            loss = c.mismatch_loss_db,
            score = c.score,
            validated = if c.validated { "yes" } else { "no" },
            status = html_escape(&status),
            note = html_escape(&c.tradeoff_note),
        ));
    }
    out.push_str("</tbody>\n</table>\n</body>\n</html>\n");
    out
}

pub fn to_yaml(
    calculations: &[WireCalculation],
    recommendation: Option<&NonResonantRecommendation>,
    units: UnitSystem,
    wire_min_m: f64,
    wire_max_m: f64,
) -> String {
    const FT: f64 = 3.280_84;
    let mut out = String::from("---\n");
    out.push_str("results:\n");
    for c in calculations {
        out.push_str(&format!("  - band: \"{}\"\n", yaml_escape(&c.band_name)));
        out.push_str(&format!("    frequency_mhz: {:.3}\n", c.frequency_mhz));
        out.push_str(&format!(
            "    transformer_ratio: \"{}\"\n",
            c.transformer_ratio_label
        ));
        match units {
            UnitSystem::Metric => {
                out.push_str(&format!("    half_wave_m: {:.2}\n", c.half_wave_m));
                out.push_str(&format!(
                    "    half_wave_corrected_m: {:.2}\n",
                    c.corrected_half_wave_m
                ));
                out.push_str(&format!("    full_wave_m: {:.2}\n", c.full_wave_m));
                out.push_str(&format!(
                    "    full_wave_corrected_m: {:.2}\n",
                    c.corrected_full_wave_m
                ));
                out.push_str(&format!("    quarter_wave_m: {:.2}\n", c.quarter_wave_m));
                out.push_str(&format!(
                    "    quarter_wave_corrected_m: {:.2}\n",
                    c.corrected_quarter_wave_m
                ));
                out.push_str(&format!(
                    "    end_fed_half_wave_m: {:.2}\n",
                    c.end_fed_half_wave_m
                ));
                out.push_str(&format!(
                    "    full_wave_loop_circumference_m: {:.2}\n",
                    c.full_wave_loop_circumference_m
                ));
                out.push_str(&format!(
                    "    full_wave_loop_square_side_m: {:.2}\n",
                    c.full_wave_loop_square_side_m
                ));
                out.push_str(&format!(
                    "    inverted_v_total_m: {:.2}\n",
                    c.inverted_v_total_m
                ));
                out.push_str(&format!(
                    "    inverted_v_leg_m: {:.2}\n",
                    c.inverted_v_leg_m
                ));
                out.push_str(&format!(
                    "    inverted_v_span_90_m: {:.2}\n",
                    c.inverted_v_span_90_m
                ));
                out.push_str(&format!(
                    "    inverted_v_span_120_m: {:.2}\n",
                    c.inverted_v_span_120_m
                ));
                out.push_str(&format!(
                    "    ocfd_33_short_leg_m: {:.2}\n",
                    c.ocfd_33_short_leg_m
                ));
                out.push_str(&format!(
                    "    ocfd_33_long_leg_m: {:.2}\n",
                    c.ocfd_33_long_leg_m
                ));
                out.push_str(&format!(
                    "    ocfd_20_short_leg_m: {:.2}\n",
                    c.ocfd_20_short_leg_m
                ));
                out.push_str(&format!(
                    "    ocfd_20_long_leg_m: {:.2}\n",
                    c.ocfd_20_long_leg_m
                ));
                out.push_str(&format!(
                    "    trap_dipole_total_m: {:.2}\n",
                    c.trap_dipole_total_m
                ));
                out.push_str(&format!(
                    "    trap_dipole_leg_m: {:.2}\n",
                    c.trap_dipole_leg_m
                ));
            }
            UnitSystem::Imperial => {
                out.push_str(&format!("    half_wave_ft: {:.2}\n", c.half_wave_ft));
                out.push_str(&format!(
                    "    half_wave_corrected_ft: {:.2}\n",
                    c.corrected_half_wave_ft
                ));
                out.push_str(&format!("    full_wave_ft: {:.2}\n", c.full_wave_ft));
                out.push_str(&format!(
                    "    full_wave_corrected_ft: {:.2}\n",
                    c.corrected_full_wave_ft
                ));
                out.push_str(&format!("    quarter_wave_ft: {:.2}\n", c.quarter_wave_ft));
                out.push_str(&format!(
                    "    quarter_wave_corrected_ft: {:.2}\n",
                    c.corrected_quarter_wave_ft
                ));
                out.push_str(&format!(
                    "    end_fed_half_wave_ft: {:.2}\n",
                    c.end_fed_half_wave_ft
                ));
                out.push_str(&format!(
                    "    full_wave_loop_circumference_ft: {:.2}\n",
                    c.full_wave_loop_circumference_ft
                ));
                out.push_str(&format!(
                    "    full_wave_loop_square_side_ft: {:.2}\n",
                    c.full_wave_loop_square_side_ft
                ));
                out.push_str(&format!(
                    "    inverted_v_total_ft: {:.2}\n",
                    c.inverted_v_total_ft
                ));
                out.push_str(&format!(
                    "    inverted_v_leg_ft: {:.2}\n",
                    c.inverted_v_leg_ft
                ));
                out.push_str(&format!(
                    "    inverted_v_span_90_ft: {:.2}\n",
                    c.inverted_v_span_90_ft
                ));
                out.push_str(&format!(
                    "    inverted_v_span_120_ft: {:.2}\n",
                    c.inverted_v_span_120_ft
                ));
                out.push_str(&format!(
                    "    ocfd_33_short_leg_ft: {:.2}\n",
                    c.ocfd_33_short_leg_ft
                ));
                out.push_str(&format!(
                    "    ocfd_33_long_leg_ft: {:.2}\n",
                    c.ocfd_33_long_leg_ft
                ));
                out.push_str(&format!(
                    "    ocfd_20_short_leg_ft: {:.2}\n",
                    c.ocfd_20_short_leg_ft
                ));
                out.push_str(&format!(
                    "    ocfd_20_long_leg_ft: {:.2}\n",
                    c.ocfd_20_long_leg_ft
                ));
                out.push_str(&format!(
                    "    trap_dipole_total_ft: {:.2}\n",
                    c.trap_dipole_total_ft
                ));
                out.push_str(&format!(
                    "    trap_dipole_leg_ft: {:.2}\n",
                    c.trap_dipole_leg_ft
                ));
            }
            UnitSystem::Both => {
                out.push_str(&format!("    half_wave_m: {:.2}\n", c.half_wave_m));
                out.push_str(&format!(
                    "    half_wave_corrected_m: {:.2}\n",
                    c.corrected_half_wave_m
                ));
                out.push_str(&format!("    full_wave_m: {:.2}\n", c.full_wave_m));
                out.push_str(&format!(
                    "    full_wave_corrected_m: {:.2}\n",
                    c.corrected_full_wave_m
                ));
                out.push_str(&format!("    quarter_wave_m: {:.2}\n", c.quarter_wave_m));
                out.push_str(&format!(
                    "    quarter_wave_corrected_m: {:.2}\n",
                    c.corrected_quarter_wave_m
                ));
                out.push_str(&format!(
                    "    end_fed_half_wave_m: {:.2}\n",
                    c.end_fed_half_wave_m
                ));
                out.push_str(&format!(
                    "    full_wave_loop_circumference_m: {:.2}\n",
                    c.full_wave_loop_circumference_m
                ));
                out.push_str(&format!(
                    "    full_wave_loop_square_side_m: {:.2}\n",
                    c.full_wave_loop_square_side_m
                ));
                out.push_str(&format!(
                    "    inverted_v_total_m: {:.2}\n",
                    c.inverted_v_total_m
                ));
                out.push_str(&format!(
                    "    inverted_v_leg_m: {:.2}\n",
                    c.inverted_v_leg_m
                ));
                out.push_str(&format!(
                    "    inverted_v_span_90_m: {:.2}\n",
                    c.inverted_v_span_90_m
                ));
                out.push_str(&format!(
                    "    inverted_v_span_120_m: {:.2}\n",
                    c.inverted_v_span_120_m
                ));
                out.push_str(&format!(
                    "    ocfd_33_short_leg_m: {:.2}\n",
                    c.ocfd_33_short_leg_m
                ));
                out.push_str(&format!(
                    "    ocfd_33_long_leg_m: {:.2}\n",
                    c.ocfd_33_long_leg_m
                ));
                out.push_str(&format!(
                    "    ocfd_20_short_leg_m: {:.2}\n",
                    c.ocfd_20_short_leg_m
                ));
                out.push_str(&format!(
                    "    ocfd_20_long_leg_m: {:.2}\n",
                    c.ocfd_20_long_leg_m
                ));
                out.push_str(&format!(
                    "    trap_dipole_total_m: {:.2}\n",
                    c.trap_dipole_total_m
                ));
                out.push_str(&format!(
                    "    trap_dipole_leg_m: {:.2}\n",
                    c.trap_dipole_leg_m
                ));
                out.push_str(&format!("    half_wave_ft: {:.2}\n", c.half_wave_ft));
                out.push_str(&format!(
                    "    half_wave_corrected_ft: {:.2}\n",
                    c.corrected_half_wave_ft
                ));
                out.push_str(&format!("    full_wave_ft: {:.2}\n", c.full_wave_ft));
                out.push_str(&format!(
                    "    full_wave_corrected_ft: {:.2}\n",
                    c.corrected_full_wave_ft
                ));
                out.push_str(&format!("    quarter_wave_ft: {:.2}\n", c.quarter_wave_ft));
                out.push_str(&format!(
                    "    quarter_wave_corrected_ft: {:.2}\n",
                    c.corrected_quarter_wave_ft
                ));
                out.push_str(&format!(
                    "    end_fed_half_wave_ft: {:.2}\n",
                    c.end_fed_half_wave_ft
                ));
                out.push_str(&format!(
                    "    full_wave_loop_circumference_ft: {:.2}\n",
                    c.full_wave_loop_circumference_ft
                ));
                out.push_str(&format!(
                    "    full_wave_loop_square_side_ft: {:.2}\n",
                    c.full_wave_loop_square_side_ft
                ));
                out.push_str(&format!(
                    "    inverted_v_total_ft: {:.2}\n",
                    c.inverted_v_total_ft
                ));
                out.push_str(&format!(
                    "    inverted_v_leg_ft: {:.2}\n",
                    c.inverted_v_leg_ft
                ));
                out.push_str(&format!(
                    "    inverted_v_span_90_ft: {:.2}\n",
                    c.inverted_v_span_90_ft
                ));
                out.push_str(&format!(
                    "    inverted_v_span_120_ft: {:.2}\n",
                    c.inverted_v_span_120_ft
                ));
                out.push_str(&format!(
                    "    ocfd_33_short_leg_ft: {:.2}\n",
                    c.ocfd_33_short_leg_ft
                ));
                out.push_str(&format!(
                    "    ocfd_33_long_leg_ft: {:.2}\n",
                    c.ocfd_33_long_leg_ft
                ));
                out.push_str(&format!(
                    "    ocfd_20_short_leg_ft: {:.2}\n",
                    c.ocfd_20_short_leg_ft
                ));
                out.push_str(&format!(
                    "    ocfd_20_long_leg_ft: {:.2}\n",
                    c.ocfd_20_long_leg_ft
                ));
                out.push_str(&format!(
                    "    trap_dipole_total_ft: {:.2}\n",
                    c.trap_dipole_total_ft
                ));
                out.push_str(&format!(
                    "    trap_dipole_leg_ft: {:.2}\n",
                    c.trap_dipole_leg_ft
                ));
            }
        }
        out.push_str(&format!(
            "    skip_distance_min_km: {:.0}\n",
            c.skip_distance_min_km
        ));
        out.push_str(&format!(
            "    skip_distance_max_km: {:.0}\n",
            c.skip_distance_max_km
        ));
        out.push_str(&format!(
            "    skip_distance_avg_km: {:.0}\n",
            c.skip_distance_avg_km
        ));

        let rec_yaml = match (recommendation, units) {
            (Some(r), UnitSystem::Metric) => format!(
                "best_non_resonant_m: {:.2}\n      resonance_clearance_pct: {:.2}",
                r.length_m, r.min_resonance_clearance_pct
            ),
            (Some(r), UnitSystem::Imperial) => format!(
                "best_non_resonant_ft: {:.2}\n      resonance_clearance_pct: {:.2}",
                r.length_ft, r.min_resonance_clearance_pct
            ),
            (Some(r), UnitSystem::Both) => format!(
                "best_non_resonant_m: {:.2}\n      best_non_resonant_ft: {:.2}\n      resonance_clearance_pct: {:.2}",
                r.length_m, r.length_ft, r.min_resonance_clearance_pct
            ),
            (None, _) => String::new(),
        };
        if rec_yaml.is_empty() {
            out.push_str("    non_resonant_recommendation: null\n");
        } else {
            out.push_str("    non_resonant_recommendation:\n");
            for line in rec_yaml.lines() {
                out.push_str(&format!("      {line}\n"));
            }
        }

        let points = collect_band_resonant_points_m(c, wire_min_m, wire_max_m);
        if points.is_empty() {
            out.push_str("    resonant_points_in_window: []\n");
        } else {
            out.push_str("    resonant_points_in_window:\n");
            for (harmonic, len_m) in points {
                match units {
                    UnitSystem::Metric => out.push_str(&format!(
                        "      - harmonic: {harmonic}\n        length_m: {len_m:.2}\n"
                    )),
                    UnitSystem::Imperial => out.push_str(&format!(
                        "      - harmonic: {harmonic}\n        length_ft: {:.2}\n",
                        len_m * FT
                    )),
                    UnitSystem::Both => out.push_str(&format!(
                        "      - harmonic: {harmonic}\n        length_m: {len_m:.2}\n        length_ft: {:.2}\n",
                        len_m * FT
                    )),
                }
            }
        }
    }
    out
}

pub fn to_advise_yaml(assumed_feedpoint_ohm: f64, candidates: &[AdviseCandidate]) -> String {
    let mut out = String::from("---\n");
    out.push_str(&format!(
        "assumed_feedpoint_ohm: {:.0}\n",
        assumed_feedpoint_ohm
    ));
    out.push_str("candidates:\n");
    for (idx, c) in candidates.iter().enumerate() {
        out.push_str(&format!("  - rank: {}\n", idx + 1));
        out.push_str(&format!("    ratio: \"{}\"\n", c.ratio.as_label()));
        out.push_str(&format!(
            "    recommended_length_m: {:.2}\n",
            c.recommended_length_m
        ));
        out.push_str(&format!(
            "    recommended_length_ft: {:.2}\n",
            c.recommended_length_ft
        ));
        out.push_str(&format!(
            "    clearance_pct: {:.2}\n",
            c.min_resonance_clearance_pct
        ));
        out.push_str(&format!(
            "    estimated_efficiency_pct: {:.2}\n",
            c.estimated_efficiency_pct
        ));
        out.push_str(&format!(
            "    mismatch_loss_db: {:.3}\n",
            c.mismatch_loss_db
        ));
        out.push_str(&format!(
            "    average_length_shift_pct: {:.2}\n",
            c.average_length_shift_pct
        ));
        out.push_str(&format!("    score: {:.2}\n", c.score));
        out.push_str(&format!(
            "    tradeoff_note: \"{}\"\n",
            yaml_escape(&c.tradeoff_note)
        ));
        out.push_str(&format!("    validated: {}\n", c.validated));
        match c.validation_status {
            Some(status) => {
                out.push_str(&format!("    validation_status: \"{}\"\n", status.as_str()))
            }
            None => out.push_str("    validation_status: null\n"),
        }
        match &c.validation_note {
            Some(note) => out.push_str(&format!(
                "    validation_note: \"{}\"\n",
                yaml_escape(&note.replace('\n', " "))
            )),
            None => out.push_str("    validation_note: null\n"),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn json_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Escape a string for embedding inside a YAML double-quoted scalar.
fn yaml_escape(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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
            UnitSystem::Metric => format!("{harmonic}x={len_m:.2}m"),
            UnitSystem::Imperial => format!("{harmonic}x={:.2}ft", len_m / 0.3048),
            UnitSystem::Both => format!("{harmonic}x={len_m:.2}m/{:.2}ft", len_m / 0.3048),
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
            UnitSystem::Metric => format!("{{\"harmonic\": {harmonic}, \"length_m\": {len_m:.2}}}"),
            UnitSystem::Imperial => format!(
                "{{\"harmonic\": {harmonic}, \"length_ft\": {:.2}}}",
                len_m / 0.3048
            ),
            UnitSystem::Both => format!(
                "{{\"harmonic\": {harmonic}, \"length_m\": {len_m:.2}, \"length_ft\": {:.2}}}",
                len_m / 0.3048
            ),
        })
        .collect::<Vec<String>>()
        .join(", ");

    format!("[{items}]")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ExportFormat;
    use crate::calculations::TransformerRatio;
    use crate::fnec_validation::ValidationStatus;

    #[test]
    fn validate_export_path_accepts_relative_paths() {
        assert_eq!(
            validate_export_path("results.txt").unwrap(),
            PathBuf::from("results.txt")
        );
        assert_eq!(
            validate_export_path("dir/sub/output.json").unwrap(),
            PathBuf::from("dir/sub/output.json")
        );
    }

    #[test]
    fn validate_export_path_rejects_absolute_paths() {
        assert!(validate_export_path("/tmp/output.txt").is_err());
    }

    #[test]
    fn validate_export_path_rejects_parent_directory_references() {
        assert!(validate_export_path("../evil.txt").is_err());
        assert!(validate_export_path("dir/../evil.txt").is_err());
    }

    #[test]
    fn export_results_rejects_invalid_output_path() {
        let calculations: Vec<WireCalculation> = Vec::new();
        let err = export_results(
            ExportFormat::Txt,
            "../evil.txt",
            &calculations,
            None,
            UnitSystem::Metric,
            8.0,
            35.0,
        )
        .unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    fn sample_advise_candidates() -> Vec<AdviseCandidate> {
        vec![
            AdviseCandidate {
                ratio: TransformerRatio::R1To9,
                recommended_length_m: 12.34,
                recommended_length_ft: 40.49,
                min_resonance_clearance_pct: 8.9,
                estimated_efficiency_pct: 95.2,
                mismatch_loss_db: 0.123,
                average_length_shift_pct: 1.4,
                score: 94.7,
                tradeoff_note: "Good match: 95.2% efficiency, 0.12 dB loss.".to_string(),
                validated: true,
                validation_status: Some(ValidationStatus::Passed),
                validation_note: Some("NEC cross-check OK".to_string()),
            },
            AdviseCandidate {
                ratio: TransformerRatio::R1To4,
                recommended_length_m: 11.11,
                recommended_length_ft: 36.45,
                min_resonance_clearance_pct: 7.1,
                estimated_efficiency_pct: 91.0,
                mismatch_loss_db: 0.456,
                average_length_shift_pct: 1.9,
                score: 89.1,
                tradeoff_note: "Good match: 91.0% efficiency, 0.46 dB loss.".to_string(),
                validated: false,
                validation_status: Some(ValidationStatus::Skipped),
                validation_note: Some("fnec-rust not found in PATH".to_string()),
            },
        ]
    }

    #[test]
    fn advise_csv_includes_validation_columns() {
        let csv = to_advise_csv(450.0, &sample_advise_candidates());

        assert!(csv.contains("tradeoff_note"));
        assert!(csv.contains("validated"));
        assert!(csv.contains("validation_status"));
        assert!(csv.contains("validation_note"));
        assert!(csv.contains("Good match: 95.2% efficiency"));
        assert!(csv.contains("true,\"passed\",\"NEC cross-check OK\""));
        assert!(csv.contains("false,\"skipped\",\"fnec-rust not found in PATH\""));
    }

    #[test]
    fn advise_json_includes_validation_fields() {
        let json = to_advise_json(450.0, &sample_advise_candidates());

        assert!(json.contains("\"tradeoff_note\""));
        assert!(json.contains("Good match: 95.2% efficiency"));
        assert!(json.contains("\"validated\": true"));
        assert!(json.contains("\"validated\": false"));
        assert!(json.contains("\"validation_status\": \"passed\""));
        assert!(json.contains("\"validation_status\": \"skipped\""));
        assert!(json.contains("\"validation_note\": \"NEC cross-check OK\""));
        assert!(json.contains("\"validation_note\": \"fnec-rust not found in PATH\""));
    }

    #[test]
    fn advise_markdown_includes_validation_columns() {
        let markdown = to_advise_markdown(450.0, &sample_advise_candidates());

        assert!(markdown
            .contains("| Validated | Validation Status | Validation Note | Tradeoff Note |"));
        assert!(markdown.contains("| 1 | 1:9"));
        assert!(markdown.contains("| yes | passed | NEC cross-check OK |"));
        assert!(markdown.contains("Good match: 95.2% efficiency"));
    }

    #[test]
    fn advise_txt_includes_validation_lines() {
        let txt = to_advise_txt(450.0, &sample_advise_candidates());

        assert!(txt.contains("note: Good match: 95.2% efficiency"));
        assert!(txt.contains("fnec validated yes"));
        assert!(txt.contains("fnec validated no"));
        assert!(txt.contains("fnec status: passed"));
        assert!(txt.contains("fnec note: NEC cross-check OK"));
    }

    #[test]
    fn to_yaml_produces_valid_structure() {
        use crate::app::run_calculation;
        use crate::app::AppConfig;
        let config = AppConfig {
            band_indices: vec![4, 6], // 40m + 20m
            ..Default::default()
        };
        let results = run_calculation(config);
        let yaml = to_yaml(
            &results.calculations,
            results.recommendation.as_ref(),
            UnitSystem::Metric,
            results.config.wire_min_m,
            results.config.wire_max_m,
        );
        assert!(
            yaml.starts_with("---\n"),
            "should start with YAML document marker"
        );
        assert!(yaml.contains("results:"), "should have results key");
        assert!(
            yaml.contains("band: \"40m\"") || yaml.contains("band: \""),
            "should have band field"
        );
        assert!(yaml.contains("frequency_mhz:"), "should have frequency_mhz");
        assert!(
            yaml.contains("half_wave_m:"),
            "should have half_wave_m in metric"
        );
        assert!(
            !yaml.contains("half_wave_ft:"),
            "should not have ft in metric mode"
        );
    }

    #[test]
    fn to_yaml_imperial_units_omits_metric_fields() {
        use crate::app::run_calculation;
        use crate::app::AppConfig;
        let config = AppConfig {
            band_indices: vec![4],
            units: UnitSystem::Imperial,
            ..Default::default()
        };
        let results = run_calculation(config);
        let yaml = to_yaml(
            &results.calculations,
            None,
            UnitSystem::Imperial,
            results.config.wire_min_m,
            results.config.wire_max_m,
        );
        assert!(yaml.contains("half_wave_ft:"), "should have ft field");
        assert!(!yaml.contains("half_wave_m:"), "should not have m field");
    }

    #[test]
    fn to_advise_yaml_includes_all_candidate_fields() {
        let yaml = to_advise_yaml(450.0, &sample_advise_candidates());
        assert!(
            yaml.starts_with("---\n"),
            "should start with document marker"
        );
        assert!(
            yaml.contains("assumed_feedpoint_ohm: 450"),
            "should include feedpoint"
        );
        assert!(yaml.contains("candidates:"), "should have candidates key");
        assert!(yaml.contains("rank: 1"), "should have rank");
        assert!(yaml.contains("ratio: \"1:9\""), "should have ratio");
        assert!(yaml.contains("tradeoff_note:"), "should have tradeoff note");
        assert!(
            yaml.contains("Good match: 95.2% efficiency"),
            "should include note text"
        );
        assert!(
            yaml.contains("validated: true"),
            "should have validated flag"
        );
        assert!(
            yaml.contains("validation_status: \"passed\""),
            "should have status"
        );
        assert!(
            yaml.contains("validation_note: \"NEC cross-check OK\""),
            "should have note"
        );
    }

    #[test]
    fn default_output_name_yaml() {
        assert_eq!(
            default_output_name(ExportFormat::Yaml),
            "rusty-wire-results.yaml"
        );
        assert_eq!(
            default_advise_output_name(ExportFormat::Yaml),
            "rusty-wire-advise.yaml"
        );
    }

    #[test]
    fn to_html_contains_doctype_and_table() {
        use crate::app::run_calculation;
        use crate::app::AppConfig;
        let results = run_calculation(AppConfig {
            band_indices: vec![4, 6],
            ..Default::default()
        });
        let html = to_html(
            &results.calculations,
            results.recommendation.as_ref(),
            UnitSystem::Both,
            results.config.wire_min_m,
            results.config.wire_max_m,
        );
        assert!(html.starts_with("<!DOCTYPE html>"), "should have DOCTYPE");
        assert!(html.contains("<table>"), "should have table");
        assert!(html.contains("Half-wave (m)"), "should have metric header");
        assert!(
            html.contains("Half-wave (ft)"),
            "should have imperial header"
        );
        assert!(html.contains("Band Calculations"), "should have heading");
    }

    #[test]
    fn to_html_metric_only_no_ft_headers() {
        use crate::app::run_calculation;
        use crate::app::AppConfig;
        let results = run_calculation(AppConfig {
            band_indices: vec![4],
            ..Default::default()
        });
        let html = to_html(
            &results.calculations,
            results.recommendation.as_ref(),
            UnitSystem::Metric,
            results.config.wire_min_m,
            results.config.wire_max_m,
        );
        assert!(html.contains("Half-wave (m)"), "should have metric header");
        assert!(
            !html.contains("Half-wave (ft)"),
            "should not have imperial header"
        );
    }

    #[test]
    fn to_html_imperial_only_no_m_headers() {
        use crate::app::run_calculation;
        use crate::app::AppConfig;
        let config = AppConfig {
            band_indices: vec![4],
            units: UnitSystem::Imperial,
            ..Default::default()
        };
        let results = run_calculation(config);
        let html = to_html(
            &results.calculations,
            results.recommendation.as_ref(),
            UnitSystem::Imperial,
            results.config.wire_min_m,
            results.config.wire_max_m,
        );
        assert!(
            !html.contains("Half-wave (m)"),
            "should not have metric header"
        );
        assert!(
            html.contains("Half-wave (ft)"),
            "should have imperial header"
        );
    }

    #[test]
    fn to_advise_html_contains_candidates() {
        let html = to_advise_html(450.0, &sample_advise_candidates());
        assert!(html.starts_with("<!DOCTYPE html>"), "should have DOCTYPE");
        assert!(html.contains("<table>"), "should have table");
        assert!(html.contains("450"), "should contain feedpoint ohm");
        assert!(html.contains("1:9"), "should contain ratio");
        assert!(html.contains("Good match"), "should contain tradeoff note");
        assert!(html.contains("yes"), "should mark validated candidate");
    }

    #[test]
    fn html_escape_converts_special_chars() {
        assert_eq!(html_escape("a&b"), "a&amp;b");
        assert_eq!(html_escape("<tag>"), "&lt;tag&gt;");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(html_escape("plain"), "plain");
    }

    #[test]
    fn default_output_name_html() {
        assert_eq!(
            default_output_name(ExportFormat::Html),
            "rusty-wire-results.html"
        );
        assert_eq!(
            default_advise_output_name(ExportFormat::Html),
            "rusty-wire-advise.html"
        );
    }
}
