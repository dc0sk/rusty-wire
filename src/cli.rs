/// CLI argument parsing, interactive prompts, and terminal output.
///
/// This module owns everything that is specific to the command-line interface:
/// argument parsing, stdin/stdout prompts, and formatted terminal output.
/// The computation itself is delegated to `app::run_calculation`; the only
/// imports from the core modules that this file needs are for display helpers.
use crate::app::{
    execute_request_checked, recommended_transformer_ratio, results_display_document, AntennaModel,
    AppConfig, AppRequest, AppResults, CalcMode, ExportFormat, UnitSystem, DEFAULT_BAND_SELECTION,
    DEFAULT_ITU_REGION, FEET_TO_METERS,
};
use crate::bands::{get_bands_for_region, ITURegion, ALL_REGIONS};
use crate::calculations::{TransformerRatio, DEFAULT_NON_RESONANT_CONFIG};
use crate::export::{default_output_name, export_results, validate_export_path};
use clap::Parser;
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, Write};

// ---------------------------------------------------------------------------
// CLI argument parsing with clap
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "rusty-wire")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(
    about = "A Rust-based utility for wire-antenna planning across ham-radio and shortwave bands."
)]
#[command(long_about = None)]
#[command(arg_required_else_help = true)]
struct Cli {
    /// ITU Region (1=Europe/Africa/Middle East, 2=Americas, 3=Asia-Pacific)
    #[arg(short, long, value_enum, default_value = "1")]
    region: CliITURegion,

    /// Calculation mode
    #[arg(short, long, value_enum, default_value = "resonant")]
    mode: CliCalcMode,

    /// Band names/ranges (comma-separated, e.g., "40m,20m,10m-15m,60m-80m")
    #[arg(short, long)]
    bands: Option<String>,

    /// Velocity factor (0.50-1.00)
    #[arg(short, long, default_value_t = 0.95)]
    velocity: f64,

    /// Transformer ratio (default: recommended for the selected mode/antenna)
    #[arg(short, long, value_enum, default_value = "recommended")]
    transformer: CliTransformerSelection,

    /// Antenna model (omit to show all models per band)
    #[arg(long, value_enum)]
    antenna: Option<CliAntennaModel>,

    /// Wire length window minimum in meters
    #[arg(long)]
    wire_min: Option<f64>,

    /// Wire length window maximum in meters
    #[arg(long)]
    wire_max: Option<f64>,

    /// Wire length window minimum in feet
    #[arg(long)]
    wire_min_ft: Option<f64>,

    /// Wire length window maximum in feet
    #[arg(long)]
    wire_max_ft: Option<f64>,

    /// Display units (m, ft, both)
    #[arg(short, long, value_enum)]
    units: Option<CliUnitSystem>,

    /// Export formats (comma-separated: csv, json, markdown, txt)
    #[arg(short, long, value_delimiter = ',')]
    export: Option<Vec<CliExportFormat>>,

    /// Output file path for exports
    #[arg(short, long)]
    output: Option<String>,

    /// List available bands for the selected region
    #[arg(long)]
    list_bands: bool,

    /// Launch interactive mode
    #[arg(short = 'i', long)]
    interactive: bool,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum CliAntennaModel {
    #[clap(name = "dipole", help = "Center-fed dipole model")]
    Dipole,
    #[clap(name = "inverted-v", help = "Inverted-V dipole model")]
    InvertedVDipole,
    #[clap(name = "efhw", help = "End-fed half-wave model")]
    EndFedHalfWave,
    #[clap(name = "loop", help = "Full-wave loop model")]
    FullWaveLoop,
    #[clap(name = "ocfd", help = "Off-center-fed dipole (OCFD) model")]
    OffCenterFedDipole,
}

impl From<CliAntennaModel> for AntennaModel {
    fn from(model: CliAntennaModel) -> Self {
        match model {
            CliAntennaModel::Dipole => AntennaModel::Dipole,
            CliAntennaModel::InvertedVDipole => AntennaModel::InvertedVDipole,
            CliAntennaModel::EndFedHalfWave => AntennaModel::EndFedHalfWave,
            CliAntennaModel::FullWaveLoop => AntennaModel::FullWaveLoop,
            CliAntennaModel::OffCenterFedDipole => AntennaModel::OffCenterFedDipole,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum CliCalcMode {
    Resonant,
    NonResonant,
}

impl From<CliCalcMode> for CalcMode {
    fn from(mode: CliCalcMode) -> Self {
        match mode {
            CliCalcMode::Resonant => CalcMode::Resonant,
            CliCalcMode::NonResonant => CalcMode::NonResonant,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum CliUnitSystem {
    M,
    Ft,
    Both,
}

impl From<CliUnitSystem> for UnitSystem {
    fn from(units: CliUnitSystem) -> Self {
        match units {
            CliUnitSystem::M => UnitSystem::Metric,
            CliUnitSystem::Ft => UnitSystem::Imperial,
            CliUnitSystem::Both => UnitSystem::Both,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum CliExportFormat {
    Csv,
    Json,
    Markdown,
    Txt,
}

impl From<CliExportFormat> for ExportFormat {
    fn from(format: CliExportFormat) -> Self {
        match format {
            CliExportFormat::Csv => ExportFormat::Csv,
            CliExportFormat::Json => ExportFormat::Json,
            CliExportFormat::Markdown => ExportFormat::Markdown,
            CliExportFormat::Txt => ExportFormat::Txt,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum CliTransformerRatio {
    #[clap(name = "1:1")]
    R1To1,
    #[clap(name = "1:2")]
    R1To2,
    #[clap(name = "1:4")]
    R1To4,
    #[clap(name = "1:5")]
    R1To5,
    #[clap(name = "1:6")]
    R1To6,
    #[clap(name = "1:9")]
    R1To9,
    #[clap(name = "1:16")]
    R1To16,
    #[clap(name = "1:49")]
    R1To49,
    #[clap(name = "1:56")]
    R1To56,
    #[clap(name = "1:64")]
    R1To64,
}

impl From<CliTransformerRatio> for TransformerRatio {
    fn from(ratio: CliTransformerRatio) -> Self {
        match ratio {
            CliTransformerRatio::R1To1 => TransformerRatio::R1To1,
            CliTransformerRatio::R1To2 => TransformerRatio::R1To2,
            CliTransformerRatio::R1To4 => TransformerRatio::R1To4,
            CliTransformerRatio::R1To5 => TransformerRatio::R1To5,
            CliTransformerRatio::R1To6 => TransformerRatio::R1To6,
            CliTransformerRatio::R1To9 => TransformerRatio::R1To9,
            CliTransformerRatio::R1To16 => TransformerRatio::R1To16,
            CliTransformerRatio::R1To49 => TransformerRatio::R1To49,
            CliTransformerRatio::R1To56 => TransformerRatio::R1To56,
            CliTransformerRatio::R1To64 => TransformerRatio::R1To64,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum CliTransformerSelection {
    Recommended,
    #[clap(name = "1:1")]
    R1To1,
    #[clap(name = "1:2")]
    R1To2,
    #[clap(name = "1:4")]
    R1To4,
    #[clap(name = "1:5")]
    R1To5,
    #[clap(name = "1:6")]
    R1To6,
    #[clap(name = "1:9")]
    R1To9,
    #[clap(name = "1:16")]
    R1To16,
    #[clap(name = "1:49")]
    R1To49,
    #[clap(name = "1:56")]
    R1To56,
    #[clap(name = "1:64")]
    R1To64,
}

impl CliTransformerSelection {
    fn resolve(self, mode: CalcMode, antenna_model: Option<AntennaModel>) -> TransformerRatio {
        match self {
            CliTransformerSelection::Recommended => {
                recommended_transformer_ratio(mode, antenna_model)
            }
            CliTransformerSelection::R1To1 => TransformerRatio::R1To1,
            CliTransformerSelection::R1To2 => TransformerRatio::R1To2,
            CliTransformerSelection::R1To4 => TransformerRatio::R1To4,
            CliTransformerSelection::R1To5 => TransformerRatio::R1To5,
            CliTransformerSelection::R1To6 => TransformerRatio::R1To6,
            CliTransformerSelection::R1To9 => TransformerRatio::R1To9,
            CliTransformerSelection::R1To16 => TransformerRatio::R1To16,
            CliTransformerSelection::R1To49 => TransformerRatio::R1To49,
            CliTransformerSelection::R1To56 => TransformerRatio::R1To56,
            CliTransformerSelection::R1To64 => TransformerRatio::R1To64,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum CliITURegion {
    #[clap(name = "1")]
    Region1,
    #[clap(name = "2")]
    Region2,
    #[clap(name = "3")]
    Region3,
}

impl From<CliITURegion> for ITURegion {
    fn from(region: CliITURegion) -> Self {
        match region {
            CliITURegion::Region1 => ITURegion::Region1,
            CliITURegion::Region2 => ITURegion::Region2,
            CliITURegion::Region3 => ITURegion::Region3,
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Entry point when CLI arguments are present.
/// Returns `true` on success, `false` if an error prevented completion.
pub fn run_from_args(args: &[String]) -> bool {
    let cli = Cli::parse_from(args.iter().map(|s| s.as_str()));

    if cli.interactive {
        run_interactive();
        return true;
    }

    if cli.list_bands {
        show_all_bands_for_region(cli.region.into());
        return true;
    }

    let bands = match &cli.bands {
        Some(selection) => match parse_band_selection(selection, cli.region.into()) {
            Ok(parsed) => parsed,
            Err(err) => {
                eprintln!("Error: invalid --bands value: {}", err);
                return false;
            }
        },
        None => DEFAULT_BAND_SELECTION.to_vec(),
    };

    // Validate wire length constraints
    let using_ft = cli.wire_min_ft.is_some() || cli.wire_max_ft.is_some();
    let using_m = cli.wire_min.is_some() || cli.wire_max.is_some();

    if using_ft && using_m {
        eprintln!("Error: cannot mix meter and feet constraints; choose one unit system");
        return false;
    }

    let (wire_min_m, wire_max_m) = if using_ft {
        let min_ft = cli
            .wire_min_ft
            .unwrap_or(DEFAULT_NON_RESONANT_CONFIG.min_len_m / FEET_TO_METERS);
        let max_ft = cli
            .wire_max_ft
            .unwrap_or(DEFAULT_NON_RESONANT_CONFIG.max_len_m / FEET_TO_METERS);

        (min_ft * FEET_TO_METERS, max_ft * FEET_TO_METERS)
    } else {
        let min_m = cli
            .wire_min
            .unwrap_or(DEFAULT_NON_RESONANT_CONFIG.min_len_m);
        let max_m = cli
            .wire_max
            .unwrap_or(DEFAULT_NON_RESONANT_CONFIG.max_len_m);
        (min_m, max_m)
    };

    // Validate output path if provided
    if let Some(ref output) = cli.output {
        if let Err(err) = validate_export_path(output) {
            eprintln!("Error: invalid output path: {}", err);
            return false;
        }
    }

    let units = cli.units.map(UnitSystem::from).unwrap_or_else(|| {
        if using_ft {
            UnitSystem::Imperial
        } else {
            UnitSystem::Metric
        }
    });

    let export_formats = cli
        .export
        .unwrap_or_default()
        .into_iter()
        .map(ExportFormat::from)
        .collect::<Vec<_>>();

    let mode = CalcMode::from(cli.mode);
    let antenna_model = cli.antenna.map(AntennaModel::from);
    let transformer_ratio = cli.transformer.resolve(mode, antenna_model);

    let config = AppConfig {
        band_indices: bands,
        velocity_factor: cli.velocity,
        mode,
        wire_min_m,
        wire_max_m,
        units,
        itu_region: cli.region.into(),
        transformer_ratio,
        antenna_model,
    };

    let results = match execute_request_checked(AppRequest::new(config)) {
        Ok(response) => response.results,
        Err(err) => {
            eprintln!("Error: {}", err);
            return false;
        }
    };

    print_results(&results);

    let single_output = cli.output;
    let export_count = export_formats.len();
    let export_recommendation = if results.config.mode == CalcMode::NonResonant {
        results.recommendation.as_ref()
    } else {
        None
    };

    for (i, &fmt) in export_formats.iter().enumerate() {
        let output = if export_count == 1 {
            single_output
                .clone()
                .unwrap_or_else(|| default_output_name(fmt).to_string())
        } else {
            if i == 0 && single_output.is_some() {
                eprintln!(
                    "Warning: --output is ignored when multiple formats are selected; using default names."
                );
            }
            default_output_name(fmt).to_string()
        };
        if let Err(err) = export_results(
            fmt,
            &output,
            &results.calculations,
            export_recommendation,
            results.config.units,
            results.config.wire_min_m,
            results.config.wire_max_m,
        ) {
            eprintln!("Failed to export {}: {}", output, err);
            return false;
        }
        println!("Exported results to {}", output);
    }
    true
}

// ---------------------------------------------------------------------------
// Interactive mode
// ---------------------------------------------------------------------------

pub fn run_interactive() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    run_interactive_with_io(&mut input, &mut output);
}

fn run_interactive_with_io(input: &mut dyn BufRead, output: &mut dyn Write) {
    writeln!(
        output,
        "============================================================"
    )
    .expect("failed to write interactive banner");
    writeln!(
        output,
        "Rusty Wire v{} - Resonant Length and Skip Distance Calculator",
        env!("CARGO_PKG_VERSION")
    )
    .expect("failed to write interactive banner");
    writeln!(
        output,
        "============================================================\n"
    )
    .expect("failed to write interactive banner");

    let mut itu_region = prompt_itu_region(input, output);

    loop {
        writeln!(output, "Menu:").expect("failed to write menu");
        writeln!(
            output,
            "  1) List all bands (for Region {})",
            itu_region.short_name()
        )
        .expect("failed to write menu");
        writeln!(output, "  2) Calculate selected bands").expect("failed to write menu");
        writeln!(output, "  3) Quick single-band calculation").expect("failed to write menu");
        writeln!(output, "  4) Change ITU Region").expect("failed to write menu");
        writeln!(output, "  5) Exit").expect("failed to write menu");
        prompt(output, "\nSelect option (1-5): ");

        let choice = read_line(input, "failed to read choice");
        if choice.is_empty() {
            writeln!(output, "Exiting Rusty Wire.").expect("failed to write exit message");
            break;
        }

        match choice.trim() {
            "1" => show_all_bands_for_region_to_writer(output, itu_region),
            "2" => calculate_selected_bands(input, output, itu_region),
            "3" => quick_calculation(input, output, itu_region),
            "4" => {
                itu_region = prompt_itu_region(input, output);
                writeln!(
                    output,
                    "Switched to ITU Region {}.\n",
                    itu_region.short_name()
                )
                .expect("failed to write region update");
            }
            "5" => {
                writeln!(output, "Exiting Rusty Wire.").expect("failed to write exit message");
                break;
            }
            _ => writeln!(output, "Invalid option. Try again.\n")
                .expect("failed to write invalid option message"),
        }
    }
}

fn calculate_selected_bands(input: &mut dyn BufRead, output: &mut dyn Write, region: ITURegion) {
    show_all_bands_for_region_to_writer(output, region);
    prompt(
        output,
        "Enter bands (e.g. 40m,20m,10m-15m; Enter for default set): ",
    );

    let band_input = read_line(input, "failed to read selection");

    let trimmed = band_input.trim();
    let indices = if trimmed.is_empty() {
        DEFAULT_BAND_SELECTION.to_vec()
    } else {
        match parse_band_selection(trimmed, region) {
            Ok(v) if !v.is_empty() => v,
            Ok(_) | Err(_) => {
                writeln!(
                    output,
                    "Invalid input. Use band names/ranges like 40m,20m,10m-15m.\n"
                )
                .expect("failed to write invalid band selection message");
                return;
            }
        }
    };

    let mode = prompt_calc_mode(input, output);
    let antenna_model = prompt_antenna_model(input, output);
    let velocity = prompt_velocity_factor(input, output);
    let transformer_ratio = prompt_transformer_ratio(input, output, mode, antenna_model);
    let (wire_min_m, wire_max_m, auto_units) = if mode == CalcMode::NonResonant {
        prompt_wire_length_window(input, output)
    } else {
        (
            DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            UnitSystem::Both,
        )
    };
    let units = prompt_display_units(input, output, auto_units);

    let config = AppConfig {
        band_indices: indices,
        velocity_factor: velocity,
        mode,
        wire_min_m,
        wire_max_m,
        units,
        itu_region: region,
        transformer_ratio,
        antenna_model,
    };

    let results = match execute_request_checked(AppRequest::new(config)) {
        Ok(response) => response.results,
        Err(err) => {
            writeln!(output, "Error: {}\n", err).expect("failed to write validation error");
            return;
        }
    };

    print_results(&results);
    print_equivalent_cli_call(&results.config, &[]);
    let export_choices = interactive_export_prompt(input, output, &results);
    if !export_choices.is_empty() {
        print_equivalent_cli_call(&results.config, &export_choices);
    }
}

fn quick_calculation(input: &mut dyn BufRead, output: &mut dyn Write, region: ITURegion) {
    show_all_bands_for_region_to_writer(output, region);
    prompt(output, "Enter one band (e.g. 20m): ");

    let band_input = read_line(input, "failed to read selection");

    let idx = match parse_single_band_token(band_input.trim(), region) {
        Ok(v) => v,
        Err(_) => {
            writeln!(output, "Invalid band. Use a single band name like 20m.\n")
                .expect("failed to write invalid number message");
            return;
        }
    };

    let mode = prompt_calc_mode(input, output);
    let antenna_model = prompt_antenna_model(input, output);
    let velocity = prompt_velocity_factor(input, output);
    let transformer_ratio = prompt_transformer_ratio(input, output, mode, antenna_model);
    let (wire_min_m, wire_max_m, auto_units) = if mode == CalcMode::NonResonant {
        prompt_wire_length_window(input, output)
    } else {
        (
            DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            UnitSystem::Both,
        )
    };
    let units = prompt_display_units(input, output, auto_units);

    let config = AppConfig {
        band_indices: vec![idx],
        velocity_factor: velocity,
        mode,
        wire_min_m,
        wire_max_m,
        units,
        itu_region: region,
        transformer_ratio,
        antenna_model,
    };

    let results = match execute_request_checked(AppRequest::new(config)) {
        Ok(response) => response.results,
        Err(err) => {
            writeln!(output, "Error: {}\n", err).expect("failed to write validation error");
            return;
        }
    };

    print_results(&results);
    print_equivalent_cli_call(&results.config, &[]);
    let export_choices = interactive_export_prompt(input, output, &results);
    if !export_choices.is_empty() {
        print_equivalent_cli_call(&results.config, &export_choices);
    }
}

fn prompt_itu_region(input: &mut dyn BufRead, output: &mut dyn Write) -> ITURegion {
    writeln!(output, "\nITU Regions:").expect("failed to write region header");
    for region in ALL_REGIONS {
        let is_default = *region == DEFAULT_ITU_REGION;
        let default_str = if is_default { " (default)" } else { "" };
        writeln!(
            output,
            "  {}) Region {}{}",
            region.short_name(),
            region.long_name(),
            default_str
        )
        .expect("failed to write region option");
    }
    prompt(
        output,
        &format!(
            "Select region (1/2/3, Enter for {}): ",
            DEFAULT_ITU_REGION.short_name()
        ),
    );

    let region_input = read_line(input, "failed to read region");

    match region_input.trim() {
        "" | "1" => ITURegion::Region1,
        "2" => ITURegion::Region2,
        "3" => ITURegion::Region3,
        _ => {
            writeln!(
                output,
                "Invalid region. Using default Region {}.",
                DEFAULT_ITU_REGION.short_name()
            )
            .expect("failed to write invalid region message");
            DEFAULT_ITU_REGION
        }
    }
}

fn prompt_calc_mode(input: &mut dyn BufRead, output: &mut dyn Write) -> CalcMode {
    prompt(
        output,
        "Calculation mode (resonant/non-resonant, Enter for resonant): ",
    );

    let mode_input = read_line(input, "failed to read calculation mode");

    match mode_input.trim().to_ascii_lowercase().as_str() {
        "" | "resonant" => CalcMode::Resonant,
        "non-resonant" | "nonresonant" | "non_resonant" => CalcMode::NonResonant,
        _ => {
            writeln!(output, "Unknown mode. Using resonant.")
                .expect("failed to write invalid mode message");
            CalcMode::Resonant
        }
    }
}

fn prompt_antenna_model(input: &mut dyn BufRead, output: &mut dyn Write) -> Option<AntennaModel> {
    prompt(
        output,
        "Antenna model: (d)ipole, (i)nverted-V, (e)nd-fed half-wave, (l)oop, (o)ff-center-fed dipole, (a)ll [a]: "
    );

    let antenna_input = read_line(input, "failed to read antenna model");

    match antenna_input.trim().to_ascii_lowercase().as_str() {
        "" | "a" | "all" => None,
        "d" | "dipole" => Some(AntennaModel::Dipole),
        "i" | "inverted-v" | "inv-v" | "invertedv" | "invv" => Some(AntennaModel::InvertedVDipole),
        "e" | "efhw" | "end-fed" | "end-fed-half-wave" => Some(AntennaModel::EndFedHalfWave),
        "l" | "loop" | "full-wave-loop" => Some(AntennaModel::FullWaveLoop),
        "o" | "ocfd" | "off-center-fed" | "off-center-fed-dipole" => {
            Some(AntennaModel::OffCenterFedDipole)
        }
        _ => {
            writeln!(
                output,
                "Unknown antenna model. Showing all models per band."
            )
            .expect("failed to write invalid antenna model message");
            None
        }
    }
}

fn prompt_transformer_ratio(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    mode: CalcMode,
    antenna_model: Option<AntennaModel>,
) -> TransformerRatio {
    prompt(
        output,
        "Unun/Balun ratio (recommended,1:1,1:2,1:4,1:5,1:6,1:9,1:16,1:49,1:56,1:64; Enter for recommended): ",
    );

    let ratio_input = read_line(input, "failed to read transformer ratio");

    let trimmed = ratio_input.trim();
    if trimmed.is_empty() {
        return recommended_transformer_ratio(mode, antenna_model);
    }

    if trimmed.eq_ignore_ascii_case("recommended") {
        return recommended_transformer_ratio(mode, antenna_model);
    }

    match TransformerRatio::parse(trimmed) {
        Some(r) => r,
        None => {
            let recommended = recommended_transformer_ratio(mode, antenna_model);
            writeln!(
                output,
                "Unknown ratio. Using recommended {}.",
                recommended.as_label()
            )
            .expect("failed to write invalid ratio message");
            recommended
        }
    }
}

fn prompt_velocity_factor(input: &mut dyn BufRead, output: &mut dyn Write) -> f64 {
    prompt(output, "Velocity factor (0.50-1.00, Enter for 0.95): ");

    let velocity_input = read_line(input, "failed to read velocity factor");
    let trimmed = velocity_input.trim();
    if trimmed.is_empty() {
        return 0.95;
    }

    match trimmed.parse::<f64>() {
        Ok(vf) if (0.5..=1.0).contains(&vf) => vf,
        _ => {
            writeln!(output, "Invalid value. Using default 0.95.")
                .expect("failed to write invalid velocity message");
            0.95
        }
    }
}

fn prompt_wire_length_window(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
) -> (f64, f64, UnitSystem) {
    prompt(
        output,
        "Constraint units for wire length window (m/ft, Enter for m): ",
    );

    let unit_input = read_line(input, "failed to read wire length window units");
    let unit = unit_input.trim().to_ascii_lowercase();

    if unit == "ft" || unit == "feet" {
        let default_min_ft = DEFAULT_NON_RESONANT_CONFIG.min_len_m / FEET_TO_METERS;
        let default_max_ft = DEFAULT_NON_RESONANT_CONFIG.max_len_m / FEET_TO_METERS;

        prompt(
            output,
            &format!(
                "Wire min length in feet (Enter for {:.1}): ",
                default_min_ft
            ),
        );
        let min_input = read_line(input, "failed to read wire min length");

        prompt(
            output,
            &format!(
                "Wire max length in feet (Enter for {:.1}): ",
                default_max_ft
            ),
        );
        let max_input = read_line(input, "failed to read wire max length");

        let min_ft = min_input
            .trim()
            .parse::<f64>()
            .ok()
            .unwrap_or(default_min_ft);
        let max_ft = max_input
            .trim()
            .parse::<f64>()
            .ok()
            .unwrap_or(default_max_ft);

        if min_ft > 0.0 && max_ft > min_ft {
            return (
                min_ft * FEET_TO_METERS,
                max_ft * FEET_TO_METERS,
                UnitSystem::Imperial,
            );
        }

        writeln!(
            output,
            "Invalid wire length window, using defaults {:.1}-{:.1} m.",
            DEFAULT_NON_RESONANT_CONFIG.min_len_m, DEFAULT_NON_RESONANT_CONFIG.max_len_m
        )
        .expect("failed to write invalid wire window message");
        return (
            DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            UnitSystem::Imperial,
        );
    }

    prompt(
        output,
        &format!(
            "Wire min length in meters (Enter for {:.1}): ",
            DEFAULT_NON_RESONANT_CONFIG.min_len_m
        ),
    );
    let min_input = read_line(input, "failed to read wire min length");

    prompt(
        output,
        &format!(
            "Wire max length in meters (Enter for {:.1}): ",
            DEFAULT_NON_RESONANT_CONFIG.max_len_m
        ),
    );
    let max_input = read_line(input, "failed to read wire max length");

    let min_len = min_input
        .trim()
        .parse::<f64>()
        .ok()
        .unwrap_or(DEFAULT_NON_RESONANT_CONFIG.min_len_m);
    let max_len = max_input
        .trim()
        .parse::<f64>()
        .ok()
        .unwrap_or(DEFAULT_NON_RESONANT_CONFIG.max_len_m);

    if min_len > 0.0 && max_len > min_len {
        (min_len, max_len, UnitSystem::Metric)
    } else {
        writeln!(
            output,
            "Invalid wire length window, using defaults {:.1}-{:.1} m.",
            DEFAULT_NON_RESONANT_CONFIG.min_len_m, DEFAULT_NON_RESONANT_CONFIG.max_len_m
        )
        .expect("failed to write invalid wire window message");
        (
            DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            UnitSystem::Metric,
        )
    }
}

fn prompt_display_units(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    auto_units: UnitSystem,
) -> UnitSystem {
    let label = match auto_units {
        UnitSystem::Metric => "m",
        UnitSystem::Imperial => "ft",
        UnitSystem::Both => "both",
    };
    prompt(
        output,
        &format!("Display units (m/ft/both, Enter for {}): ", label),
    );
    let unit_input = read_line(input, "failed to read display units");
    let trimmed = unit_input.trim();
    if trimmed.is_empty() {
        return auto_units;
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "m" | "metric" => UnitSystem::Metric,
        "ft" | "imperial" => UnitSystem::Imperial,
        "both" => UnitSystem::Both,
        _ => {
            writeln!(output, "Unknown unit system. Using {}.", label)
                .expect("failed to write invalid display unit message");
            auto_units
        }
    }
}

fn interactive_export_prompt(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    results: &AppResults,
) -> Vec<(ExportFormat, String)> {
    prompt(
        output,
        "Export results? (none, or comma-separated formats e.g. csv,json,markdown,txt): ",
    );

    let fmt_raw = read_line(input, "failed to read export format")
        .trim()
        .to_ascii_lowercase();

    if fmt_raw.is_empty() || fmt_raw == "none" {
        return Vec::new();
    }

    let formats: Vec<ExportFormat> = {
        let mut out = Vec::new();
        let mut err_msg = None;
        for token in fmt_raw.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            match token {
                "csv" => {
                    if !out.contains(&ExportFormat::Csv) {
                        out.push(ExportFormat::Csv);
                    }
                }
                "json" => {
                    if !out.contains(&ExportFormat::Json) {
                        out.push(ExportFormat::Json);
                    }
                }
                "markdown" | "md" => {
                    if !out.contains(&ExportFormat::Markdown) {
                        out.push(ExportFormat::Markdown);
                    }
                }
                "txt" | "text" => {
                    if !out.contains(&ExportFormat::Txt) {
                        out.push(ExportFormat::Txt);
                    }
                }
                other => {
                    err_msg = Some(format!("unknown format '{}'; skipping export.", other));
                    break;
                }
            }
        }
        if let Some(msg) = err_msg {
            writeln!(output, "{}", msg).expect("failed to write export error message");
            return Vec::new();
        }
        if out.is_empty() {
            writeln!(
                output,
                "--export requires at least one format; skipping export."
            )
            .expect("failed to write export error message");
            return Vec::new();
        }
        out
    };

    let export_recommendation = if results.config.mode == CalcMode::NonResonant {
        results.recommendation.as_ref()
    } else {
        None
    };

    let mut chosen = Vec::new();
    for &fmt in &formats {
        let output_path = if formats.len() == 1 {
            prompt(
                output,
                &format!("Output file (Enter for {}): ", default_output_name(fmt)),
            );
            let output_raw = read_line(input, "failed to read output file");
            if output_raw.trim().is_empty() {
                default_output_name(fmt).to_string()
            } else {
                output_raw.trim().to_string()
            }
        } else {
            default_output_name(fmt).to_string()
        };

        match export_results(
            fmt,
            &output_path,
            &results.calculations,
            export_recommendation,
            results.config.units,
            results.config.wire_min_m,
            results.config.wire_max_m,
        ) {
            Ok(()) => writeln!(output, "Exported results to {}", output_path)
                .expect("failed to write export success message"),
            Err(err) => writeln!(output, "Failed to export {}: {}", output_path, err)
                .expect("failed to write export failure message"),
        }
        chosen.push((fmt, output_path));
    }

    chosen
}

fn prompt(output: &mut dyn Write, text: &str) {
    write!(output, "{}", text).expect("failed to write interactive prompt");
    output.flush().expect("failed to flush interactive prompt");
}

fn read_line(input: &mut dyn BufRead, error_message: &str) -> String {
    let mut line = String::new();
    input.read_line(&mut line).expect(error_message);
    line
}

fn parse_band_selection(selection: &str, region: ITURegion) -> Result<Vec<usize>, String> {
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
                .ok_or_else(|| format!("unknown range start '{}'.", start.trim()))?;
            let end_pos = ordered
                .iter()
                .position(|idx| *idx == end_idx)
                .ok_or_else(|| format!("unknown range end '{}'.", end.trim()))?;

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
        return Err("empty selection; provide at least one band name.".to_string());
    }

    Ok(parsed)
}

fn parse_single_band_token(token: &str, region: ITURegion) -> Result<usize, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("empty band token".to_string());
    }

    let aliases = band_alias_to_index(region);
    let key = token.to_ascii_lowercase();
    aliases
        .get(&key)
        .copied()
        .ok_or_else(|| format!("unknown band '{}'.", token))
}

fn ordered_band_indices_for_region(region: ITURegion) -> Vec<usize> {
    get_bands_for_region(region)
        .into_iter()
        .map(|(idx, _)| idx + 1)
        .collect()
}

fn band_alias_to_index(region: ITURegion) -> HashMap<String, usize> {
    let mut aliases = HashMap::new();

    for (idx, band) in get_bands_for_region(region) {
        let one_based = idx + 1;
        let full_name = band.name.to_ascii_lowercase();
        aliases.insert(full_name.clone(), one_based);

        if let Some(short_name) = full_name.split_whitespace().next() {
            aliases.insert(short_name.to_string(), one_based);
        }
    }

    aliases
}

fn band_label_for_index(index: usize, region: ITURegion) -> String {
    let zero_based = match index.checked_sub(1) {
        Some(v) => v,
        None => return index.to_string(),
    };

    for (idx, band) in get_bands_for_region(region) {
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

fn print_equivalent_cli_call(config: &AppConfig, export_choices: &[(ExportFormat, String)]) {
    let bands_csv = config
        .band_indices
        .iter()
        .map(|v| band_label_for_index(*v, config.itu_region))
        .collect::<Vec<String>>()
        .join(",");

    let units_str = match config.units {
        UnitSystem::Metric => "m",
        UnitSystem::Imperial => "ft",
        UnitSystem::Both => "both",
    };
    let mut cmd = format!(
        "rusty-wire --region {} --mode {} --bands {} --velocity {:.2} --transformer {} --units {}",
        shell_quote(config.itu_region.short_name()),
        shell_quote(match config.mode {
            CalcMode::Resonant => "resonant",
            CalcMode::NonResonant => "non-resonant",
        }),
        shell_quote(&bands_csv),
        config.velocity_factor,
        shell_quote(config.transformer_ratio.as_label()),
        shell_quote(units_str),
    );

    if config.mode == CalcMode::NonResonant {
        cmd.push_str(&format!(
            " --wire-min {:.2} --wire-max {:.2}",
            config.wire_min_m, config.wire_max_m
        ));
    }

    if let Some(antenna_model) = config.antenna_model {
        let antenna = match antenna_model {
            AntennaModel::Dipole => "dipole",
            AntennaModel::InvertedVDipole => "inverted-v",
            AntennaModel::EndFedHalfWave => "efhw",
            AntennaModel::FullWaveLoop => "loop",
            AntennaModel::OffCenterFedDipole => "ocfd",
        };
        cmd.push_str(&format!(" --antenna {}", shell_quote(antenna)));
    }

    if !export_choices.is_empty() {
        let fmts = export_choices
            .iter()
            .map(|(fmt, _)| fmt.as_str())
            .collect::<Vec<_>>()
            .join(",");
        cmd.push_str(&format!(" --export {}", shell_quote(&fmts)));
        if export_choices.len() == 1 {
            cmd.push_str(&format!(" --output {}", shell_quote(&export_choices[0].1)));
        }
    }

    println!("Equivalent CLI call for this run:");
    println!("  {}\n", cmd);
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    let safe = value.chars().all(|ch| {
        matches!(ch,
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '/'
        )
    });
    if safe {
        return value.to_string();
    }

    let escaped = value.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

// ---------------------------------------------------------------------------
// Terminal display
// ---------------------------------------------------------------------------

fn show_all_bands_for_region(region: ITURegion) {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    show_all_bands_for_region_to_writer(&mut output, region);
}

fn show_all_bands_for_region_to_writer(output: &mut dyn Write, region: ITURegion) {
    let bands = get_bands_for_region(region);
    writeln!(
        output,
        "\nAvailable bands in Region {} ({} total):",
        region.short_name(),
        bands.len()
    )
    .expect("failed to write band listing header");
    writeln!(output, "  ({})", region.long_name()).expect("failed to write band listing");
    writeln!(
        output,
        "------------------------------------------------------------"
    )
    .expect("failed to write band listing separator");
    for (idx, band) in bands {
        writeln!(output, "{:2}. {}", idx + 1, band).expect("failed to write band line");
    }
    writeln!(output).expect("failed to write band listing trailing newline");
}
// ---------------------------------------------------------------------------
// Terminal display
// ---------------------------------------------------------------------------

fn print_results(results: &AppResults) {
    let doc = results_display_document(results);

    println!("\n{}", doc.overview_heading);
    for line in doc.overview_header_lines {
        println!("{}", line);
    }
    for view in doc.band_views {
        println!("{}", view.title);
        for line in view.lines {
            println!("{}", line);
        }
        println!();
    }
    for line in doc.summary_lines {
        println!("{}", line);
    }
    println!();

    for section in doc.sections {
        for line in section.lines {
            println!("{}", line);
        }
        println!();
    }

    for line in doc.warning_lines {
        println!("{}", line);
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn run_interactive_with_io_exits_cleanly() {
        let mut input = Cursor::new(b"\n5\n".to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Select option (1-5): "));
        assert!(rendered.contains("Exiting Rusty Wire."));
    }

    #[test]
    fn prompt_antenna_model_accepts_inverted_v_alias() {
        let mut input = Cursor::new(b"invv\n".to_vec());
        let mut output = Vec::new();

        let model = prompt_antenna_model(&mut input, &mut output);

        assert_eq!(model, Some(AntennaModel::InvertedVDipole));
    }

    #[test]
    fn prompt_wire_length_window_supports_feet_input() {
        let mut input = Cursor::new(b"ft\n40\n80\n".to_vec());
        let mut output = Vec::new();

        let (min_m, max_m, units) = prompt_wire_length_window(&mut input, &mut output);

        assert_eq!(units, UnitSystem::Imperial);
        assert!((min_m - 12.192).abs() < 1e-6);
        assert!((max_m - 24.384).abs() < 1e-6);
    }

    #[test]
    fn cli_transformer_selection_recommended_resolves_by_mode_and_antenna() {
        assert_eq!(
            CliTransformerSelection::Recommended.resolve(CalcMode::Resonant, None),
            TransformerRatio::R1To1
        );
        assert_eq!(
            CliTransformerSelection::Recommended.resolve(CalcMode::NonResonant, None),
            TransformerRatio::R1To9
        );
        assert_eq!(
            CliTransformerSelection::Recommended
                .resolve(CalcMode::Resonant, Some(AntennaModel::EndFedHalfWave)),
            TransformerRatio::R1To56
        );
    }

    #[test]
    fn prompt_transformer_ratio_accepts_recommended_keyword() {
        let mut input = Cursor::new(b"recommended\n".to_vec());
        let mut output = Vec::new();

        let ratio = prompt_transformer_ratio(
            &mut input,
            &mut output,
            CalcMode::Resonant,
            Some(AntennaModel::EndFedHalfWave),
        );

        assert_eq!(ratio, TransformerRatio::R1To56);
    }

    #[test]
    fn calculate_selected_bands_rejects_invalid_csv_input() {
        let mut input = Cursor::new(b"abc,4\n".to_vec());
        let mut output = Vec::new();

        calculate_selected_bands(&mut input, &mut output, ITURegion::Region1);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Invalid input. Use band names/ranges"));
    }

    #[test]
    fn parse_band_selection_supports_band_names_and_ranges() {
        let parsed = parse_band_selection("10m-15m,30m,60m-80m", ITURegion::Region1)
            .expect("expected valid named/range selection");

        assert_eq!(parsed, vec![10, 9, 8, 5, 3, 2]);
    }

    #[test]
    fn parse_band_selection_rejects_numeric_indices() {
        let err = parse_band_selection("4,6,10", ITURegion::Region1).unwrap_err();
        assert!(err.contains("unknown band"));
    }

    #[test]
    fn parse_band_selection_rejects_unknown_band_name() {
        let err = parse_band_selection("banana", ITURegion::Region1).unwrap_err();
        assert!(err.contains("unknown band"));
    }

    #[test]
    fn print_equivalent_cli_call_uses_band_labels() {
        let config = AppConfig {
            band_indices: vec![4, 6, 10],
            velocity_factor: 0.95,
            mode: CalcMode::Resonant,
            wire_min_m: DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            wire_max_m: DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            units: UnitSystem::Metric,
            itu_region: ITURegion::Region1,
            transformer_ratio: TransformerRatio::R1To1,
            antenna_model: None,
        };

        // Assert the formatter input mapping separately since this function prints to stdout.
        let bands_csv = config
            .band_indices
            .iter()
            .map(|v| band_label_for_index(*v, config.itu_region))
            .collect::<Vec<String>>()
            .join(",");
        assert_eq!(bands_csv, "40m,20m,10m");
    }

    #[test]
    fn interactive_export_prompt_rejects_unknown_format() {
        let mut input = Cursor::new(b"yaml\n".to_vec());
        let mut output = Vec::new();
        let results = AppResults {
            calculations: Vec::new(),
            recommendation: None,
            optima: Vec::new(),
            window_optima: Vec::new(),
            resonant_compromises: Vec::new(),
            config: AppConfig::default(),
            skipped_band_indices: Vec::new(),
        };

        let exports = interactive_export_prompt(&mut input, &mut output, &results);

        assert!(exports.is_empty());
        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("unknown format 'yaml'; skipping export."));
    }

    #[test]
    fn run_interactive_with_io_can_switch_region() {
        let mut input = Cursor::new(b"\n4\n2\n5\n".to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Switched to ITU Region 2."));
        assert!(rendered.contains("List all bands (for Region 2)"));
    }

    #[test]
    fn run_interactive_with_io_quick_calculation_invalid_number() {
        let mut input = Cursor::new(b"\n3\nabc\n5\n".to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Enter one band (e.g. 20m): "));
        assert!(rendered.contains("Invalid band. Use a single band name like 20m."));
    }

    #[test]
    fn run_interactive_with_io_lists_bands_to_writer_output() {
        let mut input = Cursor::new(b"\n1\n5\n".to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Available bands in Region 1"));
    }
}
