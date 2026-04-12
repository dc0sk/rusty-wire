/// CLI argument parsing, interactive prompts, and terminal output.
///
/// This module owns everything that is specific to the command-line interface:
/// argument parsing, stdin/stdout prompts, and formatted terminal output.
/// The computation itself is delegated to `app::run_calculation`; the only
/// imports from the core modules that this file needs are for display helpers.
use crate::app::{
    run_calculation, AntennaModel, AppConfig, AppResults, CalcMode, ExportFormat, UnitSystem,
    DEFAULT_BAND_SELECTION, DEFAULT_ITU_REGION, DEFAULT_TRANSFORMER_RATIO, FEET_TO_METERS,
};
use crate::bands::{get_bands_for_region, ITURegion, ALL_REGIONS};
use crate::calculations::{
    calculate_average_max_distance, calculate_average_min_distance, optimize_ocfd_split_for_length,
    TransformerRatio, WireCalculation, DEFAULT_NON_RESONANT_CONFIG,
};
use crate::export::{default_output_name, export_results, validate_export_path};
use clap::Parser;
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
    #[arg(short, long, value_enum, default_value_t = DEFAULT_ITU_REGION)]
    region: ITURegion,

    /// Calculation mode
    #[arg(short, long, value_enum, default_value_t = CalcMode::Resonant)]
    mode: CalcMode,

    /// Band numbers (comma-separated, e.g., "4,5,6,7,8,9,10")
    #[arg(short, long, value_delimiter = ',')]
    bands: Option<Vec<usize>>,

    /// Velocity factor (0.50-1.00)
    #[arg(short, long, default_value_t = 0.95)]
    velocity: f64,

    /// Transformer ratio (1:1, 1:2, 1:4, 1:5, 1:6, 1:9, 1:16, 1:49, 1:56, 1:64)
    #[arg(short, long, value_enum, default_value_t = DEFAULT_TRANSFORMER_RATIO)]
    transformer: TransformerRatio,

    /// Antenna model (omit to show all models per band)
    #[arg(long, value_enum)]
    antenna: Option<AntennaModel>,

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
    units: Option<UnitSystem>,

    /// Export formats (comma-separated: csv, json, markdown, txt)
    #[arg(short, long, value_delimiter = ',')]
    export: Option<Vec<ExportFormat>>,

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

#[derive(clap::ValueEnum, Clone, Debug)]
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

#[derive(clap::ValueEnum, Clone, Debug)]
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

#[derive(clap::ValueEnum, Clone, Debug)]
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

#[derive(clap::ValueEnum, Clone, Debug)]
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
pub fn run_from_args(args: &[String]) {
    let cli = Cli::parse_from(args.iter().map(|s| s.as_str()));

    if cli.interactive {
        run_interactive();
        return;
    }

    if cli.list_bands {
        show_all_bands_for_region(cli.region);
        return;
    }

    let bands = cli.bands.unwrap_or_else(|| DEFAULT_BAND_SELECTION.to_vec());

    // Validate velocity factor
    if !(0.5..=1.0).contains(&cli.velocity) {
        eprintln!("Error: velocity factor must be between 0.50 and 1.00");
        return;
    }

    // Validate wire length constraints
    let using_ft = cli.wire_min_ft.is_some() || cli.wire_max_ft.is_some();
    let using_m = cli.wire_min.is_some() || cli.wire_max.is_some();

    if using_ft && using_m {
        eprintln!("Error: cannot mix meter and feet constraints; choose one unit system");
        return;
    }

    let (wire_min_m, wire_max_m) = if using_ft {
        let min_ft = cli
            .wire_min_ft
            .unwrap_or(DEFAULT_NON_RESONANT_CONFIG.min_len_m / FEET_TO_METERS);
        let max_ft = cli
            .wire_max_ft
            .unwrap_or(DEFAULT_NON_RESONANT_CONFIG.max_len_m / FEET_TO_METERS);

        if min_ft <= 0.0 || max_ft <= min_ft {
            eprintln!("Error: invalid wire length window in feet");
            return;
        }

        (min_ft * FEET_TO_METERS, max_ft * FEET_TO_METERS)
    } else {
        let min_m = cli
            .wire_min
            .unwrap_or(DEFAULT_NON_RESONANT_CONFIG.min_len_m);
        let max_m = cli
            .wire_max
            .unwrap_or(DEFAULT_NON_RESONANT_CONFIG.max_len_m);

        if min_m <= 0.0 || max_m <= min_m {
            eprintln!("Error: invalid wire length window in meters");
            return;
        }

        (min_m, max_m)
    };

    // Validate output path if provided
    if let Some(ref output) = cli.output {
        if let Err(err) = validate_export_path(output) {
            eprintln!("Error: invalid output path: {}", err);
            return;
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

    let config = AppConfig {
        band_indices: bands,
        velocity_factor: cli.velocity,
        mode: CalcMode::from(cli.mode),
        wire_min_m,
        wire_max_m,
        units,
        itu_region: cli.region,
        transformer_ratio: TransformerRatio::from(cli.transformer),
        antenna_model: cli.antenna,
    };

    let results = run_calculation(config);
    if results.calculations.is_empty() {
        println!("No valid bands selected.");
        return;
    }

    print_results(&results);
    print_skipped_band_warnings(&results);

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
            std::process::exit(1);
        }
        println!("Exported results to {}", output);
    }
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
        "Enter band numbers separated by commas (Enter for default 4,5,6,7,8,9,10): ",
    );

    let band_input = read_line(input, "failed to read selection");

    let indices = if band_input.trim().is_empty() {
        DEFAULT_BAND_SELECTION.to_vec()
    } else {
        let parsed: Result<Vec<usize>, _> = band_input
            .trim()
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().parse::<usize>())
            .collect();

        match parsed {
            Ok(v) if !v.is_empty() => v,
            _ => {
                writeln!(output, "Invalid input. Use comma-separated numbers.\n")
                    .expect("failed to write invalid band selection message");
                return;
            }
        }
    };

    let mode = prompt_calc_mode(input, output);
    let antenna_model = prompt_antenna_model(input, output);
    let velocity = prompt_velocity_factor(input, output);
    let transformer_ratio = prompt_transformer_ratio(input, output);
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

    let results = run_calculation(config);
    if results.calculations.is_empty() {
        writeln!(output, "No valid bands selected.\n")
            .expect("failed to write empty result message");
        return;
    }

    print_results(&results);
    print_skipped_band_warnings(&results);
    print_equivalent_cli_call(&results.config, &[]);
    let export_choices = interactive_export_prompt(input, output, &results);
    if !export_choices.is_empty() {
        print_equivalent_cli_call(&results.config, &export_choices);
    }
}

fn quick_calculation(input: &mut dyn BufRead, output: &mut dyn Write, region: ITURegion) {
    show_all_bands_for_region_to_writer(output, region);
    prompt(output, "Enter one band number: ");

    let band_input = read_line(input, "failed to read selection");

    let idx = match band_input.trim().parse::<usize>() {
        Ok(v) => v,
        Err(_) => {
            writeln!(output, "Invalid number.\n").expect("failed to write invalid number message");
            return;
        }
    };

    let mode = prompt_calc_mode(input, output);
    let antenna_model = prompt_antenna_model(input, output);
    let velocity = prompt_velocity_factor(input, output);
    let transformer_ratio = prompt_transformer_ratio(input, output);
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

    let results = run_calculation(config);
    if results.calculations.is_empty() {
        writeln!(output, "Band not found.\n").expect("failed to write band not found message");
        return;
    }

    print_results(&results);
    print_skipped_band_warnings(&results);
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
        "o" | "ocfd" | "off-center-fed" | "off-center-fed-dipole" | "windom" => {
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

fn prompt_transformer_ratio(input: &mut dyn BufRead, output: &mut dyn Write) -> TransformerRatio {
    prompt(
        output,
        &format!(
            "Unun/Balun ratio (1:1,1:2,1:4,1:5,1:6,1:9,1:16,1:49,1:56,1:64; Enter for {}): ",
            DEFAULT_TRANSFORMER_RATIO.as_label()
        ),
    );

    let ratio_input = read_line(input, "failed to read transformer ratio");

    let trimmed = ratio_input.trim();
    if trimmed.is_empty() {
        return DEFAULT_TRANSFORMER_RATIO;
    }

    match TransformerRatio::parse(trimmed) {
        Some(r) => r,
        None => {
            writeln!(
                output,
                "Unknown ratio. Using default {}.",
                DEFAULT_TRANSFORMER_RATIO.as_label()
            )
            .expect("failed to write invalid ratio message");
            DEFAULT_TRANSFORMER_RATIO
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

fn print_equivalent_cli_call(config: &AppConfig, export_choices: &[(ExportFormat, String)]) {
    let bands_csv = config
        .band_indices
        .iter()
        .map(|v| v.to_string())
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
    let mode = results.config.mode;
    let units = results.config.units;
    let calculations = &results.calculations;

    let heading = if mode == CalcMode::Resonant {
        "Resonant Overview:"
    } else {
        "Non-resonant Overview (band context):"
    };

    println!("\n{}", heading);
    println!("------------------------------------------------------------");
    println!(
        "Using transformer ratio: {}",
        results.config.transformer_ratio.as_label()
    );
    println!(
        "Antenna model: {}",
        match results.config.antenna_model {
            None => "all",
            Some(AntennaModel::Dipole) => "dipole",
            Some(AntennaModel::InvertedVDipole) => "inverted-v dipole",
            Some(AntennaModel::EndFedHalfWave) => "end-fed half-wave",
            Some(AntennaModel::FullWaveLoop) => "full-wave loop",
            Some(AntennaModel::OffCenterFedDipole) => "off-center-fed dipole",
        }
    );
    println!("------------------------------------------------------------");
    for calc in calculations {
        println!(
            "{}\n",
            format_calc(calc, units, results.config.antenna_model)
        );
    }
    println!("Summary for {} band(s):", calculations.len());
    println!(
        "  Average minimum skip distance: {:.0} km",
        calculate_average_min_distance(calculations)
    );
    println!(
        "  Average maximum skip distance: {:.0} km\n",
        calculate_average_max_distance(calculations)
    );

    match mode {
        CalcMode::Resonant => {
            if matches!(
                results.config.antenna_model,
                None | Some(AntennaModel::Dipole) | Some(AntennaModel::InvertedVDipole)
            ) {
                print_resonant_points_in_window(results);
            }
            print_resonant_compromises(results);
        }
        CalcMode::NonResonant => {
            if results.recommendation.is_some() {
                print_non_resonant_recommendation(results);
            } else {
                println!("No non-resonant recommendation available for the current selection.\n");
            }
        }
    }
}

fn print_resonant_points_in_window(results: &AppResults) {
    let calculations = &results.calculations;
    let (min_m, max_m) = (results.config.wire_min_m, results.config.wire_max_m);
    let min_ft = min_m / FEET_TO_METERS;
    let max_ft = max_m / FEET_TO_METERS;
    let units = results.config.units;

    println!("Resonant points within search window:");
    match units {
        UnitSystem::Metric => println!("  Search window: {:.2}-{:.2} m", min_m, max_m),
        UnitSystem::Imperial => println!("  Search window: {:.2}-{:.2} ft", min_ft, max_ft),
        UnitSystem::Both => println!(
            "  Search window: {:.2}-{:.2} m ({:.2}-{:.2} ft)",
            min_m, max_m, min_ft, max_ft
        ),
    }

    let mut points: Vec<(f64, String, u32)> = Vec::new();
    for calc in calculations {
        let quarter_wave_m = calc.corrected_quarter_wave_m;
        if quarter_wave_m <= 0.0 {
            continue;
        }

        let mut harmonic = 1_u32;
        loop {
            let resonant_len_m = quarter_wave_m * f64::from(harmonic);
            if resonant_len_m > max_m + 1e-9 {
                break;
            }
            if resonant_len_m >= min_m - 1e-9 {
                points.push((resonant_len_m, calc.band_name.clone(), harmonic));
            }
            harmonic += 1;
        }
    }

    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    if points.is_empty() {
        println!("  (no resonant points fall within this window)\n");
        return;
    }

    for (len_m, band_name, harmonic) in points {
        let len_ft = len_m / FEET_TO_METERS;
        match units {
            UnitSystem::Metric => println!(
                "  - {}: {}x quarter-wave = {:.2} m",
                band_name, harmonic, len_m
            ),
            UnitSystem::Imperial => println!(
                "  - {}: {}x quarter-wave = {:.2} ft",
                band_name, harmonic, len_ft
            ),
            UnitSystem::Both => println!(
                "  - {}: {}x quarter-wave = {:.2} m ({:.2} ft)",
                band_name, harmonic, len_m, len_ft
            ),
        }
    }
    println!();
}

fn print_non_resonant_recommendation(results: &AppResults) {
    let rec = match results.recommendation.as_ref() {
        Some(r) => r,
        None => return,
    };
    let optima = &results.optima;
    let window_optima = &results.window_optima;
    let (min_m, max_m) = (results.config.wire_min_m, results.config.wire_max_m);
    let min_ft = min_m / FEET_TO_METERS;
    let max_ft = max_m / FEET_TO_METERS;
    let units = results.config.units;

    println!("Best non-resonant wire length for selected bands:");
    match units {
        UnitSystem::Metric => {
            println!("  Search window: {:.2}-{:.2} m", min_m, max_m);
            println!(
                "  {:.2} m, resonance clearance: {:.2}%\n",
                rec.length_m, rec.min_resonance_clearance_pct
            );
        }
        UnitSystem::Imperial => {
            println!("  Search window: {:.2}-{:.2} ft", min_ft, max_ft);
            println!(
                "  {:.2} ft, resonance clearance: {:.2}%\n",
                rec.length_ft, rec.min_resonance_clearance_pct
            );
        }
        UnitSystem::Both => {
            println!(
                "  Search window: {:.2}-{:.2} m ({:.2}-{:.2} ft)",
                min_m, max_m, min_ft, max_ft
            );
            println!(
                "  {:.2} m ({:.2} ft), resonance clearance: {:.2}%\n",
                rec.length_m, rec.length_ft, rec.min_resonance_clearance_pct
            );
        }
    }

    if optima.len() > 1 {
        println!("  Additional equal optima in range (ascending):");
        for (idx, o) in optima.iter().enumerate() {
            match units {
                UnitSystem::Metric => println!(
                    "    {:2}. {:.2} m (clearance: {:.2}%)",
                    idx + 1,
                    o.length_m,
                    o.min_resonance_clearance_pct
                ),
                UnitSystem::Imperial => println!(
                    "    {:2}. {:.2} ft (clearance: {:.2}%)",
                    idx + 1,
                    o.length_ft,
                    o.min_resonance_clearance_pct
                ),
                UnitSystem::Both => println!(
                    "    {:2}. {:.2} m ({:.2} ft, clearance: {:.2}%)",
                    idx + 1,
                    o.length_m,
                    o.length_ft,
                    o.min_resonance_clearance_pct
                ),
            }
        }
        println!();
    }

    if window_optima.len() > 1 {
        println!("  Local optima in search window (ascending):");
        for (idx, o) in window_optima.iter().enumerate() {
            let is_recommended = (o.length_m - rec.length_m).abs() < 1e-6;
            match units {
                UnitSystem::Metric => println!(
                    "    {:2}. {:.2} m (clearance: {:.2}%{})",
                    idx + 1,
                    o.length_m,
                    o.min_resonance_clearance_pct,
                    if is_recommended { ", recommended" } else { "" }
                ),
                UnitSystem::Imperial => println!(
                    "    {:2}. {:.2} ft (clearance: {:.2}%{})",
                    idx + 1,
                    o.length_ft,
                    o.min_resonance_clearance_pct,
                    if is_recommended { ", recommended" } else { "" }
                ),
                UnitSystem::Both => println!(
                    "    {:2}. {:.2} m ({:.2} ft, clearance: {:.2}%{})",
                    idx + 1,
                    o.length_m,
                    o.length_ft,
                    o.min_resonance_clearance_pct,
                    if is_recommended { ", recommended" } else { "" }
                ),
            }
        }
        println!();
    }
}

fn print_resonant_compromises(results: &AppResults) {
    let compromises = &results.resonant_compromises;
    let heading = match results.config.antenna_model {
        Some(AntennaModel::InvertedVDipole) => {
            "Closest combined compromises to resonant points (inverted-V guidance):"
        }
        Some(AntennaModel::EndFedHalfWave) => {
            "Closest combined compromises to resonant points (tuner-assisted EFHW guidance):"
        }
        Some(AntennaModel::FullWaveLoop) => {
            "Closest combined compromises to resonant points (tuner-assisted loop guidance):"
        }
        Some(AntennaModel::OffCenterFedDipole) => {
            "Closest combined compromises to resonant points (tuner-assisted OCFD guidance):"
        }
        _ => "Closest combined compromises to resonant points:",
    };

    if compromises.is_empty() {
        println!("{}", heading);
        println!("  (none available in this window)\n");
        return;
    }

    let units = results.config.units;
    println!("{}", heading);
    if matches!(
        results.config.antenna_model,
        Some(AntennaModel::EndFedHalfWave)
            | Some(AntennaModel::FullWaveLoop)
            | Some(AntennaModel::OffCenterFedDipole)
    ) {
        println!(
            "  Note: These are dipole-derived compromise lengths shown as tuner-assisted starting points."
        );
    }
    if matches!(
        results.config.antenna_model,
        Some(AntennaModel::InvertedVDipole)
    ) {
        println!(
            "  Inverted-V mode: each compromise line shows a total wire length; per-leg and span estimates are listed directly below."
        );
    }
    if matches!(
        results.config.antenna_model,
        Some(AntennaModel::OffCenterFedDipole)
    ) {
        println!(
            "  OCFD mode: each compromise line shows a total wire length; leg splits are listed directly below."
        );
    }
    for (idx, c) in compromises.iter().take(10).enumerate() {
        let is_inverted_v = matches!(
            results.config.antenna_model,
            Some(AntennaModel::InvertedVDipole)
        );
        let is_ocfd = matches!(
            results.config.antenna_model,
            Some(AntennaModel::OffCenterFedDipole)
        );
        match units {
            UnitSystem::Metric => println!(
                "  {:2}. {:.2} m (worst-band delta: {:.2} m)",
                idx + 1,
                c.length_m,
                c.worst_band_distance_m
            ),
            UnitSystem::Imperial => println!(
                "  {:2}. {:.2} ft (worst-band delta: {:.2} ft)",
                idx + 1,
                c.length_ft,
                c.worst_band_distance_m / FEET_TO_METERS
            ),
            UnitSystem::Both => println!(
                "  {:2}. {:.2} m ({:.2} ft), worst-band delta: {:.2} m ({:.2} ft)",
                idx + 1,
                c.length_m,
                c.length_ft,
                c.worst_band_distance_m,
                c.worst_band_distance_m / FEET_TO_METERS
            ),
        }

        if is_inverted_v {
            let leg_m = c.length_m / 2.0;
            let leg_ft = leg_m / FEET_TO_METERS;
            let span_90_m = leg_m * std::f64::consts::SQRT_2;
            let span_90_ft = span_90_m / FEET_TO_METERS;
            let span_120_m = leg_m * 3.0_f64.sqrt();
            let span_120_ft = span_120_m / FEET_TO_METERS;

            match units {
                UnitSystem::Metric => {
                    println!("      each leg: {:.2} m", leg_m);
                    println!("      span at 90 deg apex: {:.2} m", span_90_m);
                    println!("      span at 120 deg apex: {:.2} m", span_120_m);
                }
                UnitSystem::Imperial => {
                    println!("      each leg: {:.2} ft", leg_ft);
                    println!("      span at 90 deg apex: {:.2} ft", span_90_ft);
                    println!("      span at 120 deg apex: {:.2} ft", span_120_ft);
                }
                UnitSystem::Both => {
                    println!("      each leg: {:.2} m ({:.2} ft)", leg_m, leg_ft);
                    println!(
                        "      span at 90 deg apex: {:.2} m ({:.2} ft)",
                        span_90_m, span_90_ft
                    );
                    println!(
                        "      span at 120 deg apex: {:.2} m ({:.2} ft)",
                        span_120_m, span_120_ft
                    );
                }
            }
        }

        if is_ocfd {
            let split_33_short_m = c.length_m / 3.0;
            let split_33_long_m = c.length_m * 2.0 / 3.0;
            let split_20_short_m = c.length_m * 0.2;
            let split_20_long_m = c.length_m * 0.8;
            let split_33_short_ft = split_33_short_m / FEET_TO_METERS;
            let split_33_long_ft = split_33_long_m / FEET_TO_METERS;
            let split_20_short_ft = split_20_short_m / FEET_TO_METERS;
            let split_20_long_ft = split_20_long_m / FEET_TO_METERS;

            match units {
                UnitSystem::Metric => {
                    println!(
                        "      33/67 legs: {:.2} m / {:.2} m",
                        split_33_short_m, split_33_long_m
                    );
                    println!(
                        "      20/80 legs: {:.2} m / {:.2} m",
                        split_20_short_m, split_20_long_m
                    );
                }
                UnitSystem::Imperial => {
                    println!(
                        "      33/67 legs: {:.2} ft / {:.2} ft",
                        split_33_short_ft, split_33_long_ft
                    );
                    println!(
                        "      20/80 legs: {:.2} ft / {:.2} ft",
                        split_20_short_ft, split_20_long_ft
                    );
                }
                UnitSystem::Both => {
                    println!(
                        "      33/67 legs: {:.2} m / {:.2} m ({:.2} ft / {:.2} ft)",
                        split_33_short_m, split_33_long_m, split_33_short_ft, split_33_long_ft
                    );
                    println!(
                        "      20/80 legs: {:.2} m / {:.2} m ({:.2} ft / {:.2} ft)",
                        split_20_short_m, split_20_long_m, split_20_short_ft, split_20_long_ft
                    );
                }
            }

            if let Some(best) = optimize_ocfd_split_for_length(&results.calculations, c.length_m) {
                match units {
                    UnitSystem::Metric => println!(
                        "      Optimized split: {:.0}/{:.0} -> {:.2} m / {:.2} m (worst-leg clearance: {:.2}%)",
                        best.short_ratio * 100.0,
                        best.long_ratio * 100.0,
                        best.short_leg_m,
                        best.long_leg_m,
                        best.worst_leg_clearance_pct
                    ),
                    UnitSystem::Imperial => println!(
                        "      Optimized split: {:.0}/{:.0} -> {:.2} ft / {:.2} ft (worst-leg clearance: {:.2}%)",
                        best.short_ratio * 100.0,
                        best.long_ratio * 100.0,
                        best.short_leg_ft,
                        best.long_leg_ft,
                        best.worst_leg_clearance_pct
                    ),
                    UnitSystem::Both => println!(
                        "      Optimized split: {:.0}/{:.0} -> {:.2} m / {:.2} m ({:.2} ft / {:.2} ft), worst-leg clearance: {:.2}%",
                        best.short_ratio * 100.0,
                        best.long_ratio * 100.0,
                        best.short_leg_m,
                        best.long_leg_m,
                        best.short_leg_ft,
                        best.long_leg_ft,
                        best.worst_leg_clearance_pct
                    ),
                }
            }
        }
    }

    if compromises.len() > 10 {
        println!(
            "  ... and {} more equal compromises",
            compromises.len() - 10
        );
    }
    println!();
}

fn print_skipped_band_warnings(results: &AppResults) {
    if results.skipped_band_indices.is_empty() {
        return;
    }

    let skipped = results
        .skipped_band_indices
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "Warning: the following band selections were invalid and skipped: {}\n",
        skipped
    );
}

fn format_calc(
    c: &WireCalculation,
    units: UnitSystem,
    antenna_model: Option<AntennaModel>,
) -> String {
    match units {
        UnitSystem::Metric => match antenna_model {
            Some(AntennaModel::Dipole) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  Half-wave: {:.2} m (base: {:.2} m)\n  Full-wave: {:.2} m (base: {:.2} m)\n  Quarter-wave: {:.2} m (base: {:.2} m)\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name, c.frequency_mhz,
                c.transformer_ratio_label,
                c.corrected_half_wave_m, c.half_wave_m,
                c.corrected_full_wave_m, c.full_wave_m,
                c.corrected_quarter_wave_m, c.quarter_wave_m,
                c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
            ),
            Some(AntennaModel::EndFedHalfWave) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  End-fed half-wave: {:.2} m\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name,
                c.frequency_mhz,
                c.transformer_ratio_label,
                c.end_fed_half_wave_m,
                c.skip_distance_min_km,
                c.skip_distance_max_km,
                c.skip_distance_avg_km,
            ),
            Some(AntennaModel::InvertedVDipole) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  Inverted-V total: {:.2} m\n  Inverted-V each leg: {:.2} m\n  Inverted-V span at 90 deg apex: {:.2} m\n  Inverted-V span at 120 deg apex: {:.2} m\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name,
                c.frequency_mhz,
                c.transformer_ratio_label,
                c.inverted_v_total_m,
                c.inverted_v_leg_m,
                c.inverted_v_span_90_m,
                c.inverted_v_span_120_m,
                c.skip_distance_min_km,
                c.skip_distance_max_km,
                c.skip_distance_avg_km,
            ),
            Some(AntennaModel::FullWaveLoop) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  Full-wave loop circumference: {:.2} m\n  Full-wave loop square side: {:.2} m\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name,
                c.frequency_mhz,
                c.transformer_ratio_label,
                c.full_wave_loop_circumference_m,
                c.full_wave_loop_square_side_m,
                c.skip_distance_min_km,
                c.skip_distance_max_km,
                c.skip_distance_avg_km,
            ),
            Some(AntennaModel::OffCenterFedDipole) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  OCFD 33/67 legs: {:.2} m / {:.2} m\n  OCFD 20/80 legs: {:.2} m / {:.2} m\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name,
                c.frequency_mhz,
                c.transformer_ratio_label,
                c.ocfd_33_short_leg_m,
                c.ocfd_33_long_leg_m,
                c.ocfd_20_short_leg_m,
                c.ocfd_20_long_leg_m,
                c.skip_distance_min_km,
                c.skip_distance_max_km,
                c.skip_distance_avg_km,
            ),
            None => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  Half-wave: {:.2} m (base: {:.2} m)\n  Full-wave: {:.2} m (base: {:.2} m)\n  Quarter-wave: {:.2} m (base: {:.2} m)\n  End-fed half-wave: {:.2} m\n  Inverted-V total: {:.2} m\n  Inverted-V each leg: {:.2} m\n  Inverted-V span at 90 deg apex: {:.2} m\n  Inverted-V span at 120 deg apex: {:.2} m\n  Full-wave loop circumference: {:.2} m\n  Full-wave loop square side: {:.2} m\n  OCFD 33/67 legs: {:.2} m / {:.2} m\n  OCFD 20/80 legs: {:.2} m / {:.2} m\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name, c.frequency_mhz,
                c.transformer_ratio_label,
                c.corrected_half_wave_m, c.half_wave_m,
                c.corrected_full_wave_m, c.full_wave_m,
                c.corrected_quarter_wave_m, c.quarter_wave_m,
                c.end_fed_half_wave_m,
                c.inverted_v_total_m,
                c.inverted_v_leg_m,
                c.inverted_v_span_90_m,
                c.inverted_v_span_120_m,
                c.full_wave_loop_circumference_m,
                c.full_wave_loop_square_side_m,
                c.ocfd_33_short_leg_m,
                c.ocfd_33_long_leg_m,
                c.ocfd_20_short_leg_m,
                c.ocfd_20_long_leg_m,
                c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
            ),
        },
        UnitSystem::Imperial => match antenna_model {
            Some(AntennaModel::Dipole) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  Half-wave: {:.2} ft (base: {:.2} ft)\n  Full-wave: {:.2} ft (base: {:.2} ft)\n  Quarter-wave: {:.2} ft (base: {:.2} ft)\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name, c.frequency_mhz,
                c.transformer_ratio_label,
                c.corrected_half_wave_ft, c.half_wave_ft,
                c.corrected_full_wave_ft, c.full_wave_ft,
                c.corrected_quarter_wave_ft, c.quarter_wave_ft,
                c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
            ),
            Some(AntennaModel::EndFedHalfWave) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  End-fed half-wave: {:.2} ft\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name,
                c.frequency_mhz,
                c.transformer_ratio_label,
                c.end_fed_half_wave_ft,
                c.skip_distance_min_km,
                c.skip_distance_max_km,
                c.skip_distance_avg_km,
            ),
            Some(AntennaModel::InvertedVDipole) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  Inverted-V total: {:.2} ft\n  Inverted-V each leg: {:.2} ft\n  Inverted-V span at 90 deg apex: {:.2} ft\n  Inverted-V span at 120 deg apex: {:.2} ft\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name,
                c.frequency_mhz,
                c.transformer_ratio_label,
                c.inverted_v_total_ft,
                c.inverted_v_leg_ft,
                c.inverted_v_span_90_ft,
                c.inverted_v_span_120_ft,
                c.skip_distance_min_km,
                c.skip_distance_max_km,
                c.skip_distance_avg_km,
            ),
            Some(AntennaModel::FullWaveLoop) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  Full-wave loop circumference: {:.2} ft\n  Full-wave loop square side: {:.2} ft\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name,
                c.frequency_mhz,
                c.transformer_ratio_label,
                c.full_wave_loop_circumference_ft,
                c.full_wave_loop_square_side_ft,
                c.skip_distance_min_km,
                c.skip_distance_max_km,
                c.skip_distance_avg_km,
            ),
            Some(AntennaModel::OffCenterFedDipole) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  OCFD 33/67 legs: {:.2} ft / {:.2} ft\n  OCFD 20/80 legs: {:.2} ft / {:.2} ft\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name,
                c.frequency_mhz,
                c.transformer_ratio_label,
                c.ocfd_33_short_leg_ft,
                c.ocfd_33_long_leg_ft,
                c.ocfd_20_short_leg_ft,
                c.ocfd_20_long_leg_ft,
                c.skip_distance_min_km,
                c.skip_distance_max_km,
                c.skip_distance_avg_km,
            ),
            None => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  Half-wave: {:.2} ft (base: {:.2} ft)\n  Full-wave: {:.2} ft (base: {:.2} ft)\n  Quarter-wave: {:.2} ft (base: {:.2} ft)\n  End-fed half-wave: {:.2} ft\n  Inverted-V total: {:.2} ft\n  Inverted-V each leg: {:.2} ft\n  Inverted-V span at 90 deg apex: {:.2} ft\n  Inverted-V span at 120 deg apex: {:.2} ft\n  Full-wave loop circumference: {:.2} ft\n  Full-wave loop square side: {:.2} ft\n  OCFD 33/67 legs: {:.2} ft / {:.2} ft\n  OCFD 20/80 legs: {:.2} ft / {:.2} ft\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name, c.frequency_mhz,
                c.transformer_ratio_label,
                c.corrected_half_wave_ft, c.half_wave_ft,
                c.corrected_full_wave_ft, c.full_wave_ft,
                c.corrected_quarter_wave_ft, c.quarter_wave_ft,
                c.end_fed_half_wave_ft,
                c.inverted_v_total_ft,
                c.inverted_v_leg_ft,
                c.inverted_v_span_90_ft,
                c.inverted_v_span_120_ft,
                c.full_wave_loop_circumference_ft,
                c.full_wave_loop_square_side_ft,
                c.ocfd_33_short_leg_ft,
                c.ocfd_33_long_leg_ft,
                c.ocfd_20_short_leg_ft,
                c.ocfd_20_long_leg_ft,
                c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
            ),
        },
        UnitSystem::Both => match antenna_model {
            Some(AntennaModel::Dipole) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  Half-wave: {:.2} m ({:.2} ft, base: {:.2} m/{:.2} ft)\n  Full-wave: {:.2} m ({:.2} ft, base: {:.2} m/{:.2} ft)\n  Quarter-wave: {:.2} m ({:.2} ft, base: {:.2} m/{:.2} ft)\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name,
                c.frequency_mhz,
                c.transformer_ratio_label,
                c.corrected_half_wave_m,
                c.corrected_half_wave_ft,
                c.half_wave_m,
                c.half_wave_ft,
                c.corrected_full_wave_m,
                c.corrected_full_wave_ft,
                c.full_wave_m,
                c.full_wave_ft,
                c.corrected_quarter_wave_m,
                c.corrected_quarter_wave_ft,
                c.quarter_wave_m,
                c.quarter_wave_ft,
                c.skip_distance_min_km,
                c.skip_distance_max_km,
                c.skip_distance_avg_km,
            ),
            Some(AntennaModel::EndFedHalfWave) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  End-fed half-wave: {:.2} m ({:.2} ft)\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name,
                c.frequency_mhz,
                c.transformer_ratio_label,
                c.end_fed_half_wave_m,
                c.end_fed_half_wave_ft,
                c.skip_distance_min_km,
                c.skip_distance_max_km,
                c.skip_distance_avg_km,
            ),
            Some(AntennaModel::InvertedVDipole) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  Inverted-V total: {:.2} m ({:.2} ft)\n  Inverted-V each leg: {:.2} m ({:.2} ft)\n  Inverted-V span at 90 deg apex: {:.2} m ({:.2} ft)\n  Inverted-V span at 120 deg apex: {:.2} m ({:.2} ft)\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name,
                c.frequency_mhz,
                c.transformer_ratio_label,
                c.inverted_v_total_m,
                c.inverted_v_total_ft,
                c.inverted_v_leg_m,
                c.inverted_v_leg_ft,
                c.inverted_v_span_90_m,
                c.inverted_v_span_90_ft,
                c.inverted_v_span_120_m,
                c.inverted_v_span_120_ft,
                c.skip_distance_min_km,
                c.skip_distance_max_km,
                c.skip_distance_avg_km,
            ),
            Some(AntennaModel::FullWaveLoop) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  Full-wave loop circumference: {:.2} m ({:.2} ft)\n  Full-wave loop square side: {:.2} m ({:.2} ft)\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name,
                c.frequency_mhz,
                c.transformer_ratio_label,
                c.full_wave_loop_circumference_m,
                c.full_wave_loop_circumference_ft,
                c.full_wave_loop_square_side_m,
                c.full_wave_loop_square_side_ft,
                c.skip_distance_min_km,
                c.skip_distance_max_km,
                c.skip_distance_avg_km,
            ),
            Some(AntennaModel::OffCenterFedDipole) => format!(
                "{}\n  Frequency: {:.3} MHz\n  Transformer ratio: {}\n  OCFD 33/67 legs: {:.2} m / {:.2} m ({:.2} ft / {:.2} ft)\n  OCFD 20/80 legs: {:.2} m / {:.2} m ({:.2} ft / {:.2} ft)\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
                c.band_name,
                c.frequency_mhz,
                c.transformer_ratio_label,
                c.ocfd_33_short_leg_m,
                c.ocfd_33_long_leg_m,
                c.ocfd_33_short_leg_ft,
                c.ocfd_33_long_leg_ft,
                c.ocfd_20_short_leg_m,
                c.ocfd_20_long_leg_m,
                c.ocfd_20_short_leg_ft,
                c.ocfd_20_long_leg_ft,
                c.skip_distance_min_km,
                c.skip_distance_max_km,
                c.skip_distance_avg_km,
            ),
            None => format!("{}", c),
        },
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
    fn calculate_selected_bands_rejects_invalid_csv_input() {
        let mut input = Cursor::new(b"abc,4\n".to_vec());
        let mut output = Vec::new();

        calculate_selected_bands(&mut input, &mut output, ITURegion::Region1);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Invalid input. Use comma-separated numbers."));
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
        assert!(rendered.contains("Enter one band number: "));
        assert!(rendered.contains("Invalid number."));
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
