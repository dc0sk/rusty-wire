/// Export formatting and file writing.
///
/// Each `to_*` function is a pure string transform; `export_results` is the
/// only function that touches the file system.  Both are accessible from
/// future GUI code (e.g. to pipe content into a preview widget).
use crate::app::{ExportFormat, UnitSystem};
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
        ExportFormat::Json => "rusty-wire-results.json",
        ExportFormat::Markdown => "rusty-wire-results.md",
        ExportFormat::Txt => "rusty-wire-results.txt",
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
        ExportFormat::Json => to_json(calculations, recommendation, units, wire_min_m, wire_max_m),
        ExportFormat::Markdown => {
            to_markdown(calculations, recommendation, units, wire_min_m, wire_max_m)
        }
        ExportFormat::Txt => to_txt(calculations, recommendation, units, wire_min_m, wire_max_m),
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
}
