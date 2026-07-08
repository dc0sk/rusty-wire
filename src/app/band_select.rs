//! Band-selection parsing, labelling, and listing views.
//!
//! User-facing band token/range parsing and the band-listing view model,
//! extracted from `app/mod.rs`.

use super::*;

pub fn parse_band_selection(selection: &str, region: ITURegion) -> Result<Vec<usize>, AppError> {
    let mut parsed = Vec::new();
    let mut seen = HashSet::new();

    for token in selection.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }

        if let Some((start, end)) = token.split_once('-') {
            let start_idx = parse_single_band_token(start.trim(), region)?;
            let end_idx = parse_single_band_token(end.trim(), region)?;

            let ordered = ordered_band_indices_for_region(region);
            let start_pos = ordered
                .iter()
                .position(|idx| *idx == start_idx)
                .ok_or_else(|| {
                    AppError::InvalidBandSelection(format!(
                        "unknown range start '{}'.",
                        start.trim()
                    ))
                })?;
            let end_pos = ordered
                .iter()
                .position(|idx| *idx == end_idx)
                .ok_or_else(|| {
                    AppError::InvalidBandSelection(format!("unknown range end '{}'.", end.trim()))
                })?;

            if start_pos <= end_pos {
                for idx in &ordered[start_pos..=end_pos] {
                    if seen.insert(*idx) {
                        parsed.push(*idx);
                    }
                }
            } else {
                for idx in ordered[end_pos..=start_pos].iter().rev() {
                    if seen.insert(*idx) {
                        parsed.push(*idx);
                    }
                }
            }

            continue;
        }

        let idx = parse_single_band_token(token, region)?;
        if seen.insert(idx) {
            parsed.push(idx);
        }
    }

    if parsed.is_empty() {
        return Err(AppError::EmptyBandSelection);
    }

    Ok(parsed)
}

pub fn parse_single_band_token(token: &str, region: ITURegion) -> Result<usize, AppError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(AppError::InvalidBandSelection(
            "empty band token".to_string(),
        ));
    }

    let aliases = band_alias_to_index(region);
    let key = token.to_ascii_lowercase();
    aliases
        .get(&key)
        .copied()
        .ok_or_else(|| AppError::InvalidBandSelection(format!("unknown band '{token}'.")))
}

pub fn band_label_for_index(index: usize, region: ITURegion) -> String {
    let zero_based = match index.checked_sub(1) {
        Some(v) => v,
        None => return index.to_string(),
    };

    for (idx, band) in crate::bands::get_bands_for_region(region) {
        if idx == zero_based {
            return band
                .name
                .split_whitespace()
                .next()
                .unwrap_or(band.name)
                .to_string();
        }
    }

    index.to_string()
}

fn ordered_band_indices_for_region(region: ITURegion) -> Vec<usize> {
    crate::bands::get_bands_for_region(region)
        .into_iter()
        .map(|(idx, _)| idx + 1)
        .collect()
}

fn band_alias_to_index(region: ITURegion) -> HashMap<String, usize> {
    let mut aliases = HashMap::new();

    for (idx, band) in crate::bands::get_bands_for_region(region) {
        let one_based = idx + 1;
        let full_name = band.name.to_ascii_lowercase();
        aliases.insert(full_name.clone(), one_based);

        if let Some(short_name) = full_name.split_whitespace().next() {
            aliases.insert(short_name.to_string(), one_based);
        }
    }

    aliases
}

/// Build a pure view model for the band listing of a given ITU region.
///
/// Pure function; performs no I/O.
pub fn band_listing_view(region: ITURegion) -> BandListingView {
    let rows = crate::bands::get_bands_for_region(region)
        .into_iter()
        .map(|(idx, band)| BandListingRow {
            index: idx + 1,
            display: format!("{band}"),
        })
        .collect();
    BandListingView {
        region_short_name: region.short_name().to_string(),
        region_long_name: region.long_name().to_string(),
        rows,
    }
}

/// Render a `BandListingView` to display lines (no I/O).
pub fn band_listing_display_lines(view: &BandListingView) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(String::new());
    lines.push(format!(
        "Available bands in Region {} ({} total):",
        view.region_short_name,
        view.rows.len()
    ));
    lines.push(format!("  ({})", view.region_long_name));
    lines.push("------------------------------------------------------------".to_string());
    for row in &view.rows {
        lines.push(format!("{:2}. {}", row.index, row.display));
    }
    lines.push(String::new());
    lines
}
