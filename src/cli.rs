/// CLI argument parsing, interactive prompts, and terminal output.
///
/// This module owns everything that is specific to the command-line interface:
/// argument parsing, stdin/stdout prompts, and formatted terminal output.
/// The computation itself is delegated to `app::run_calculation`; the only
/// imports from the core modules that this file needs are for display helpers.
use crate::app::{
    band_label_for_index, band_listing_display_lines, band_listing_view, build_advise_candidates,
    build_advise_candidates_with_thresholds, execute_request_checked, format_quiet_summary,
    parse_band_selection, parse_single_band_token, recommended_transformer_ratio,
    recommended_transformer_ratio_fallback_message, resolve_wire_window_inputs,
    results_display_document, transformer_sweep_display_lines, transformer_sweep_view,
    validate_config, validate_velocity_sweep, velocity_sweep_display_lines, velocity_sweep_view,
    AntennaModel, AppConfig, AppRequest, AppResults, CalcMode, ExportFormat, UnitSystem,
    DEFAULT_ANTENNA_HEIGHT_M, DEFAULT_BAND_SELECTION, DEFAULT_CONDUCTOR_DIAMETER_MM,
    DEFAULT_GROUND_CLASS, DEFAULT_ITU_REGION, FEET_TO_METERS,
};
use crate::band_presets::load_preset_selection;
use crate::bands::{ITURegion, ALL_REGIONS};
use crate::calculations::{
    GroundClass, TransformerRatio, DEFAULT_NON_RESONANT_CONFIG, MAX_CONDUCTOR_DIAMETER_MM,
    MIN_CONDUCTOR_DIAMETER_MM,
};
use crate::export::{default_advise_output_name, export_advise};
use crate::export::{default_output_name, export_results, validate_export_path};
use crate::fnec_validation::{DEFAULT_FNEC_PASS_MAX_MISMATCH, DEFAULT_FNEC_REJECT_MIN_MISMATCH};
use clap::{CommandFactory, FromArgMatches, Parser};
use std::io::{self, BufRead, Write};

const PROJECT_URL: &str = env!("CARGO_PKG_REPOSITORY");

/// Resolve the default bands config file path by probing standard locations
/// in priority order:
///   1. `~/.config/rusty-wire/bands.toml` (XDG-style user config dir)
///   2. `./bands.toml` in the current working directory
///
/// Falls back to `"bands.toml"` (cwd) when neither file exists, so that
/// any subsequent error message names a path the user can create.
fn resolve_bands_config_path() -> String {
    if let Ok(home) = std::env::var("HOME") {
        let user_path = std::path::PathBuf::from(home)
            .join(".config")
            .join("rusty-wire")
            .join("bands.toml");
        if user_path.exists() {
            return user_path.display().to_string();
        }
    }
    "bands.toml".to_string()
}

// ---------------------------------------------------------------------------
// CLI argument parsing with clap
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "rusty-wire")]
#[command(author = env!("CARGO_PKG_AUTHORS"))]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(
    about = "A Rust-based utility for wire-antenna planning across ham-radio and shortwave bands."
)]
#[command(long_about = None)]
#[command(after_help = "Project: https://github.com/dc0sk/rusty-wire\nLicense: GPL-2.0-or-later")]
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

    /// Named band preset from a TOML config file, e.g. portable-dx
    #[arg(long)]
    bands_preset: Option<String>,

    /// Band preset config file path (default: bands.toml)
    #[arg(long)]
    bands_config: Option<String>,

    /// Velocity factor (0.50-1.00)
    #[arg(short, long, default_value_t = 0.95)]
    velocity: f64,

    /// Antenna height in meters (standard presets: 7, 10, 12)
    #[arg(long, value_enum, default_value = "10")]
    height: CliAntennaHeight,

    /// Ground class model for skip-distance estimates
    #[arg(long, value_enum, default_value = "average")]
    ground: CliGroundClass,

    /// Conductor diameter in millimeters for first-order length correction
    #[arg(long, default_value_t = DEFAULT_CONDUCTOR_DIAMETER_MM)]
    conductor_mm: f64,

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

    /// Non-resonant search step in meters (default: 0.05 m)
    #[arg(long)]
    step: Option<f64>,

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

    /// Print project metadata (author, version, GitHub URL, license)
    #[arg(long)]
    info: bool,

    /// Compute wire lengths for a single explicit frequency in MHz (bypasses band selection)
    #[arg(long)]
    freq: Option<f64>,

    /// Compute wire lengths for multiple explicit frequencies in MHz (comma-separated, e.g. 7.074,14.074)
    /// Bypasses band selection. Mutually exclusive with --freq.
    #[arg(long, value_delimiter = ',')]
    freq_list: Option<Vec<f64>>,

    /// Suppress the full results table; print only the key recommendation
    #[arg(long)]
    quiet: bool,

    /// Print the resolved run configuration before executing
    #[arg(long)]
    verbose: bool,

    /// Validate inputs and print the resolved run without calculating or exporting
    #[arg(long)]
    dry_run: bool,

    /// Run a sweep over multiple velocity factors (comma-separated, e.g. 0.66,0.85,0.95)
    #[arg(long, value_delimiter = ',')]
    velocity_sweep: Option<Vec<f64>>,

    /// Run a sweep over multiple transformer ratios (comma-separated, e.g. 1:1,1:4,1:9,1:49)
    #[arg(long, value_delimiter = ',')]
    transformer_sweep: Option<Vec<TransformerRatio>>,

    /// Print ranked wire + balun/unun advise candidates for the selected setup
    #[arg(long)]
    advise: bool,

    /// Validate advise candidates with fnec-rust when available in PATH
    #[arg(long)]
    validate_with_fnec: bool,

    /// Maximum mismatch factor considered a pass for fnec validation (0.0..=1.0)
    #[arg(long, default_value_t = DEFAULT_FNEC_PASS_MAX_MISMATCH)]
    fnec_pass_max_mismatch: f64,

    /// Minimum mismatch factor considered rejected for fnec validation (0.0..=1.0)
    #[arg(long, default_value_t = DEFAULT_FNEC_REJECT_MIN_MISMATCH)]
    fnec_reject_min_mismatch: f64,

    /// Save the current resolved settings as persistent user defaults
    /// (~/.config/rusty-wire/config.toml).  Can be combined with any other
    /// flags; the calculation still runs after saving.
    #[arg(long)]
    save_prefs: bool,
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
    #[clap(name = "trap-dipole", help = "Trap dipole model")]
    TrapDipole,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum CliAntennaHeight {
    #[clap(name = "7")]
    H7,
    #[clap(name = "10")]
    H10,
    #[clap(name = "12")]
    H12,
}

impl From<CliAntennaHeight> for f64 {
    fn from(height: CliAntennaHeight) -> Self {
        match height {
            CliAntennaHeight::H7 => 7.0,
            CliAntennaHeight::H10 => 10.0,
            CliAntennaHeight::H12 => 12.0,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum CliGroundClass {
    Poor,
    Average,
    Good,
}

impl From<CliGroundClass> for GroundClass {
    fn from(value: CliGroundClass) -> Self {
        match value {
            CliGroundClass::Poor => GroundClass::Poor,
            CliGroundClass::Average => GroundClass::Average,
            CliGroundClass::Good => GroundClass::Good,
        }
    }
}

impl From<CliAntennaModel> for AntennaModel {
    fn from(model: CliAntennaModel) -> Self {
        match model {
            CliAntennaModel::Dipole => AntennaModel::Dipole,
            CliAntennaModel::InvertedVDipole => AntennaModel::InvertedVDipole,
            CliAntennaModel::EndFedHalfWave => AntennaModel::EndFedHalfWave,
            CliAntennaModel::FullWaveLoop => AntennaModel::FullWaveLoop,
            CliAntennaModel::OffCenterFedDipole => AntennaModel::OffCenterFedDipole,
            CliAntennaModel::TrapDipole => AntennaModel::TrapDipole,
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
    // Parse once using ArgMatches so we can detect which fields came from
    // compiled-in defaults (ValueSource::DefaultValue) vs. the command line.
    // This lets us apply persistent user preferences as fallbacks for any
    // flag the user did not explicitly pass.
    //
    // get_matches_from expects argv[0] (the binary name) as the first element,
    // matching the layout of std::env::args(). Callers (lib.rs run_cli) pass
    // args with the binary name already stripped, so we prepend a placeholder.
    let argv: Vec<&str> = std::iter::once("rusty-wire")
        .chain(args.iter().map(|s| s.as_str()))
        .collect();
    let matches = Cli::command().get_matches_from(argv);
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    // Load persistent preferences; all fields are Option<T> so absent prefs
    // silently fall through to compiled-in defaults.
    let prefs = crate::prefs::UserPrefs::load();

    // Returns true only when the arg was explicitly provided on the command
    // line, not when it came from a compiled-in default_value.
    let was_set =
        |id: &str| matches.value_source(id) == Some(clap::parser::ValueSource::CommandLine);

    // Resolve prefs-eligible fields: CLI wins when explicitly set, otherwise
    // fall back to the stored preference, then the compiled-in default.
    let region: ITURegion = if was_set("region") {
        cli.region.into()
    } else {
        prefs.itu_region().unwrap_or_else(|| cli.region.into())
    };

    if cli.info {
        let mut stdout = io::stdout();
        print_project_info(&mut stdout);
        return true;
    }

    if cli.interactive {
        run_interactive();
        return true;
    }

    if cli.list_bands {
        show_all_bands_for_region(region);
        return true;
    }

    // Validate --freq and --freq-list mutual exclusion
    if cli.freq.is_some() && cli.freq_list.is_some() {
        eprintln!("Error: --freq and --freq-list are mutually exclusive; use one or the other.");
        return false;
    }

    // Validate --freq early with a clear message
    if let Some(freq) = cli.freq {
        if freq <= 0.0 {
            eprintln!("Error: --freq must be a positive frequency in MHz (got {freq:.3})");
            return false;
        }
    }

    // Validate --freq-list values early
    if let Some(ref freqs) = cli.freq_list {
        if freqs.is_empty() {
            eprintln!("Error: --freq-list requires at least one frequency value.");
            return false;
        }
        for &freq in freqs {
            if freq <= 0.0 || freq > 1000.0 {
                eprintln!("Error: --freq-list value {freq:.3} MHz is out of range (must be > 0 and ≤ 1000 MHz).");
                return false;
            }
        }
    }

    if cli.bands.is_some() && cli.bands_preset.is_some() {
        eprintln!(
            "Error: --bands and --bands-preset are mutually exclusive; use one or the other."
        );
        return false;
    }

    let bands = match (&cli.bands, &cli.bands_preset) {
        (Some(selection), None) => match parse_band_selection(selection, region) {
            Ok(parsed) => parsed,
            Err(err) => {
                eprintln!("Error: invalid --bands value: {err}");
                return false;
            }
        },
        (None, Some(preset_name)) => {
            let config_path = cli.bands_config.as_deref().unwrap_or_else(|| {
                // When --bands-config is not specified, probe the standard
                // locations in priority order so the user doesn't need to
                // pass the path explicitly when the file is in the default
                // XDG config directory.
                "bands.toml" // placeholder; resolved below
            });
            // Resolve the actual path (explicit > XDG config dir > cwd).
            let resolved_path: String = if cli.bands_config.is_some() {
                config_path.to_string()
            } else {
                resolve_bands_config_path()
            };
            let config_path = resolved_path.as_str();
            let resolved_selection = match load_preset_selection(config_path, preset_name) {
                Ok(selection) => selection,
                Err(err) => {
                    eprintln!("Error: invalid --bands-preset value: {err}");
                    return false;
                }
            };
            match parse_band_selection(&resolved_selection, region) {
                Ok(parsed) => parsed,
                Err(err) => {
                    eprintln!(
                        "Error: preset '{}' in '{}' resolved to an invalid band selection: {err}",
                        preset_name, config_path
                    );
                    return false;
                }
            }
        }
        (None, None) => DEFAULT_BAND_SELECTION.to_vec(),
        (Some(_), Some(_)) => unreachable!("validated above"),
    };

    let resolved_window = match resolve_wire_window_inputs(
        cli.wire_min,
        cli.wire_max,
        cli.wire_min_ft,
        cli.wire_max_ft,
    ) {
        Ok(window) => window,
        Err(err) => {
            eprintln!("Error: {err}");
            return false;
        }
    };

    // Validate output path if provided
    if let Some(ref output) = cli.output {
        if let Err(err) = validate_export_path(output) {
            eprintln!("Error: invalid output path: {err}");
            return false;
        }
    }

    // Resolve further prefs-eligible fields using the same fallback logic.
    let mode = if was_set("mode") {
        CalcMode::from(cli.mode)
    } else {
        prefs
            .calc_mode()
            .unwrap_or_else(|| CalcMode::from(cli.mode))
    };
    let velocity_factor: f64 = if was_set("velocity") {
        cli.velocity
    } else {
        prefs.velocity_factor.unwrap_or(cli.velocity)
    };
    let antenna_height_m: f64 = if was_set("height") {
        cli.height.into()
    } else {
        prefs.antenna_height_m.unwrap_or_else(|| cli.height.into())
    };
    let ground_class: GroundClass = if was_set("ground") {
        cli.ground.into()
    } else {
        prefs
            .ground_class_value()
            .unwrap_or_else(|| cli.ground.into())
    };
    let conductor_diameter_mm: f64 = if was_set("conductor_mm") {
        cli.conductor_mm
    } else {
        prefs.conductor_diameter_mm.unwrap_or(cli.conductor_mm)
    };
    let units = cli
        .units
        .map(UnitSystem::from)
        .or_else(|| prefs.unit_system())
        .unwrap_or(resolved_window.inferred_display_units);

    let export_formats: Vec<ExportFormat> = cli
        .export
        .unwrap_or_default()
        .into_iter()
        .map(ExportFormat::from)
        .collect();

    let antenna_model = cli.antenna.map(AntennaModel::from);
    let transformer_ratio = cli.transformer.resolve(mode, antenna_model);

    if !(0.0..=1.0).contains(&cli.fnec_pass_max_mismatch) {
        eprintln!(
            "Error: --fnec-pass-max-mismatch must be between 0.0 and 1.0 (got {:.3})",
            cli.fnec_pass_max_mismatch
        );
        return false;
    }
    if !(0.0..=1.0).contains(&cli.fnec_reject_min_mismatch) {
        eprintln!(
            "Error: --fnec-reject-min-mismatch must be between 0.0 and 1.0 (got {:.3})",
            cli.fnec_reject_min_mismatch
        );
        return false;
    }
    if cli.fnec_pass_max_mismatch >= cli.fnec_reject_min_mismatch {
        eprintln!(
            "Error: --fnec-pass-max-mismatch ({:.3}) must be less than --fnec-reject-min-mismatch ({:.3})",
            cli.fnec_pass_max_mismatch,
            cli.fnec_reject_min_mismatch
        );
        return false;
    }

    let config = AppConfig {
        band_indices: bands,
        velocity_factor,
        mode,
        wire_min_m: resolved_window.min_m,
        wire_max_m: resolved_window.max_m,
        step_m: cli.step.unwrap_or(DEFAULT_NON_RESONANT_CONFIG.step_m),
        units,
        itu_region: region,
        transformer_ratio,
        antenna_model,
        antenna_height_m,
        ground_class,
        conductor_diameter_mm,
        custom_freq_mhz: cli.freq,
        freq_list_mhz: cli.freq_list.unwrap_or_default(),
        validate_with_fnec: cli.validate_with_fnec,
    };

    // Persist preferences when requested.  The calculation continues after
    // saving so the user sees output in the same run.
    if cli.save_prefs {
        let prefs_to_save = crate::prefs::UserPrefs::from_config(&config);
        match prefs_to_save.save() {
            Ok(()) => println!(
                "Preferences saved to {}",
                crate::prefs::UserPrefs::prefs_path_display()
            ),
            Err(err) => eprintln!("Warning: could not save preferences: {err}"),
        }
    }

    if let Some(ref velocity_values) = cli.velocity_sweep {
        if let Err(err) = validate_velocity_sweep(velocity_values) {
            eprintln!("Error: {err}");
            return false;
        }
    }

    if cli.dry_run {
        if let Err(err) = validate_config(&config) {
            eprintln!("Error: {err}");
            return false;
        }
    }

    if cli.verbose || cli.dry_run {
        print_resolved_run(
            &config,
            &export_formats,
            cli.output.as_deref(),
            cli.quiet,
            cli.advise,
            cli.velocity_sweep.as_deref(),
            cli.dry_run,
        );
    }

    if cli.dry_run {
        return true;
    }

    // Velocity sweep overrides single-run output
    if let Some(velocity_values) = cli.velocity_sweep {
        return run_velocity_sweep(&velocity_values, config, units);
    }

    // Transformer sweep overrides single-run output
    if let Some(ratios) = cli.transformer_sweep {
        return run_transformer_sweep(&ratios, config, units);
    }

    if cli.advise {
        return print_advise_candidates(
            &config,
            &export_formats,
            cli.output.as_deref(),
            cli.fnec_pass_max_mismatch,
            cli.fnec_reject_min_mismatch,
        );
    }

    let results = match execute_request_checked(AppRequest::new(config)) {
        Ok(response) => response.results,
        Err(err) => {
            eprintln!("Error: {err}");
            return false;
        }
    };

    if cli.quiet {
        print_quiet_summary(&results);
    } else {
        print_results(&results);
    }

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
            eprintln!("Failed to export {output}: {err}");
            return false;
        }
        println!("Exported results to {output}");
    }
    true
}

fn print_advise_candidates(
    config: &AppConfig,
    export_formats: &[ExportFormat],
    single_output: Option<&str>,
    fnec_pass_max_mismatch: f64,
    fnec_reject_min_mismatch: f64,
) -> bool {
    if let Err(err) = execute_request_checked(AppRequest::new(config.clone())) {
        eprintln!("Error: {err}");
        return false;
    }

    let view = if config.validate_with_fnec {
        build_advise_candidates_with_thresholds(
            config,
            5,
            fnec_pass_max_mismatch,
            fnec_reject_min_mismatch,
        )
    } else {
        build_advise_candidates(config, 5)
    };
    if view.candidates.is_empty() {
        eprintln!("Error: no advise candidates available for the current selection.");
        return false;
    }

    println!("\nAdvise candidates:");
    println!("------------------------------------------------------------");
    println!(
        "Assumed feedpoint impedance: {:.0} ohm",
        view.assumed_feedpoint_ohm
    );

    if let Some(ref cmp) = view.efhw_comparison {
        println!(
            "EFHW transformer comparison (feedpoint R: {:.0} \u{03a9}):",
            cmp.feedpoint_r_ohm
        );
        println!("  {:<5}  {:<8}  {:<6}  {:<11}  {}", "Ratio", "Target Z", "SWR", "Efficiency", "Loss");
        for entry in &cmp.entries {
            let marker = if entry.is_best { "  <- recommended" } else { "" };
            println!(
                "  {:<5}  {:>5.0} \u{03a9}  {:>4.2}:1  {:>9.2}%  {:.3} dB{}",
                entry.ratio.as_label(),
                entry.target_z_ohm,
                entry.swr,
                entry.efficiency_pct,
                entry.mismatch_loss_db,
                marker
            );
        }
        println!("------------------------------------------------------------");
    }

    println!("Ranked combinations (wire length + balun/unun ratio):");
    println!();

    for (idx, candidate) in view.candidates.iter().enumerate() {
        println!(
            "{:2}. ratio {}  wire {:.2} m ({:.2} ft)",
            idx + 1,
            candidate.ratio.as_label(),
            candidate.recommended_length_m,
            candidate.recommended_length_ft
        );
        println!(
            "    efficiency {:.2}%  mismatch loss {:.3} dB  clearance {:.2}%",
            candidate.estimated_efficiency_pct,
            candidate.mismatch_loss_db,
            candidate.min_resonance_clearance_pct
        );
        println!(
            "    score {:.2}  correction shift {:.2}%",
            candidate.score, candidate.average_length_shift_pct
        );
        println!("    note: {}", candidate.tradeoff_note);
        if let Some(note) = &candidate.validation_note {
            let status = candidate
                .validation_status
                .map(|value| value.as_str())
                .unwrap_or(if candidate.validated {
                    "validated"
                } else {
                    "not-validated"
                });
            println!("    fnec: {status} ({note})");
        }
    }

    println!();
    println!(
        "Note: efficiency and score are model-based estimates for ranking, not lab measurements."
    );

    for (i, &fmt) in export_formats.iter().enumerate() {
        let output = if export_formats.len() == 1 {
            single_output
                .map(|s| s.to_string())
                .unwrap_or_else(|| default_advise_output_name(fmt).to_string())
        } else {
            if i == 0 && single_output.is_some() {
                eprintln!(
                    "Warning: --output is ignored when multiple formats are selected; using default names."
                );
            }
            default_advise_output_name(fmt).to_string()
        };

        if let Err(err) = export_advise(fmt, &output, view.assumed_feedpoint_ohm, &view.candidates)
        {
            eprintln!("Failed to export {output}: {err}");
            return false;
        }
        println!("Exported advise results to {output}");
    }

    true
}

// ---------------------------------------------------------------------------
// Velocity sweep
// ---------------------------------------------------------------------------

fn run_velocity_sweep(velocities: &[f64], base_config: AppConfig, units: UnitSystem) -> bool {
    if let Err(err) = validate_velocity_sweep(velocities) {
        eprintln!("Error: {err}");
        return false;
    }

    let mut results_by_vf: Vec<(f64, AppResults)> = Vec::new();
    for &vf in velocities {
        let mut sweep_config = base_config.clone();
        sweep_config.velocity_factor = vf;
        match execute_request_checked(AppRequest::new(sweep_config)) {
            Ok(response) => results_by_vf.push((vf, response.results)),
            Err(err) => {
                eprintln!("Error at VF {vf:.2}: {err}");
                return false;
            }
        }
    }

    if let Some(view) = velocity_sweep_view(&results_by_vf) {
        for line in velocity_sweep_display_lines(&view, units) {
            println!("{line}");
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Transformer sweep
// ---------------------------------------------------------------------------

fn run_transformer_sweep(
    ratios: &[crate::calculations::TransformerRatio],
    base_config: AppConfig,
    units: UnitSystem,
) -> bool {
    if ratios.is_empty() {
        eprintln!("Error: --transformer-sweep requires at least one ratio.");
        return false;
    }

    // Derive the feedpoint impedance from the optimizer (uses NEC calibration).
    let feedpoint_r =
        crate::app::optimize_transformer_candidates(&base_config).assumed_feedpoint_ohm;

    let mut results_by_ratio: Vec<(crate::calculations::TransformerRatio, AppResults)> =
        Vec::new();
    for &ratio in ratios {
        let mut sweep_config = base_config.clone();
        sweep_config.transformer_ratio = ratio;
        match execute_request_checked(AppRequest::new(sweep_config)) {
            Ok(response) => results_by_ratio.push((ratio, response.results)),
            Err(err) => {
                eprintln!("Error at ratio {}: {err}", ratio.as_label());
                return false;
            }
        }
    }

    if let Some(view) = transformer_sweep_view(&results_by_ratio, feedpoint_r) {
        for line in transformer_sweep_display_lines(&view, units) {
            println!("{line}");
        }
    }
    true
}

fn print_resolved_run(
    config: &AppConfig,
    export_formats: &[ExportFormat],
    single_output: Option<&str>,
    quiet: bool,
    advise: bool,
    velocity_sweep: Option<&[f64]>,
    dry_run: bool,
) {
    let operation = if advise {
        "advise"
    } else if velocity_sweep.is_some() {
        "velocity sweep"
    } else {
        "calculation"
    };

    println!("Resolved run:");
    println!("  Operation: {operation}");
    println!("  Selection: {}", selection_summary(config));
    println!("  Region: {}", config.itu_region.short_name());
    println!(
        "  Mode: {}",
        match config.mode {
            CalcMode::Resonant => "resonant",
            CalcMode::NonResonant => "non-resonant",
        }
    );
    println!("  Antenna: {}", antenna_summary(config.antenna_model));
    println!("  Transformer: {}", config.transformer_ratio.as_label());
    println!("  Velocity factor: {:.2}", config.velocity_factor);
    if let Some(values) = velocity_sweep {
        let summary = values
            .iter()
            .map(|value| format!("{value:.2}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!("  Sweep values: {summary}");
    }
    println!("  Height: {:.0} m", config.antenna_height_m);
    println!("  Ground: {}", config.ground_class.as_label());
    println!("  Conductor: {:.1} mm", config.conductor_diameter_mm);
    if config.mode == CalcMode::NonResonant {
        println!(
            "  Window: {:.2}..{:.2} m",
            config.wire_min_m, config.wire_max_m
        );
        println!("  Step: {:.2} m", config.step_m);
    }
    println!("  Units: {}", units_summary(config.units));
    if advise {
        println!(
            "  fnec validation: {}",
            if config.validate_with_fnec {
                "enabled"
            } else {
                "disabled"
            }
        );
    } else if velocity_sweep.is_none() {
        println!(
            "  Output: {}",
            if quiet {
                "quiet summary"
            } else {
                "full report"
            }
        );
    }
    println!(
        "  Exports: {}",
        export_summary(export_formats, single_output, advise)
    );
    if dry_run {
        println!("  Dry run only: no calculations or exports executed.");
    }
    println!();
}

fn selection_summary(config: &AppConfig) -> String {
    if !config.freq_list_mhz.is_empty() {
        let freqs = config
            .freq_list_mhz
            .iter()
            .map(|freq| format!("{freq:.3}"))
            .collect::<Vec<_>>()
            .join(", ");
        return format!("frequencies {freqs} MHz");
    }

    if let Some(freq) = config.custom_freq_mhz {
        return format!("frequency {freq:.3} MHz");
    }

    let bands = config
        .band_indices
        .iter()
        .map(|idx| band_label_for_index(*idx, config.itu_region))
        .collect::<Vec<_>>()
        .join(",");
    format!("bands {bands}")
}

fn antenna_summary(antenna_model: Option<AntennaModel>) -> &'static str {
    match antenna_model {
        Some(AntennaModel::Dipole) => "dipole",
        Some(AntennaModel::InvertedVDipole) => "inverted-v",
        Some(AntennaModel::EndFedHalfWave) => "efhw",
        Some(AntennaModel::FullWaveLoop) => "loop",
        Some(AntennaModel::OffCenterFedDipole) => "ocfd",
        Some(AntennaModel::TrapDipole) => "trap-dipole",
        None => "all models",
    }
}

fn units_summary(units: UnitSystem) -> &'static str {
    match units {
        UnitSystem::Metric => "m",
        UnitSystem::Imperial => "ft",
        UnitSystem::Both => "both",
    }
}

fn export_summary(
    export_formats: &[ExportFormat],
    single_output: Option<&str>,
    advise: bool,
) -> String {
    if export_formats.is_empty() {
        return "none".to_string();
    }

    if export_formats.len() == 1 {
        let format = export_formats[0];
        let output = single_output
            .map(|value| value.to_string())
            .unwrap_or_else(|| {
                if advise {
                    default_advise_output_name(format).to_string()
                } else {
                    default_output_name(format).to_string()
                }
            });
        return format!("{} -> {output}", format.as_str());
    }

    let outputs = export_formats
        .iter()
        .map(|format| {
            let name = if advise {
                default_advise_output_name(*format)
            } else {
                default_output_name(*format)
            };
            format!("{} -> {name}", format.as_str())
        })
        .collect::<Vec<_>>()
        .join(", ");

    if single_output.is_some() {
        format!("{outputs} (--output ignored for multiple formats)")
    } else {
        outputs
    }
}

// ---------------------------------------------------------------------------
// Quiet summary
// ---------------------------------------------------------------------------

fn print_quiet_summary(results: &AppResults) {
    match format_quiet_summary(results) {
        Some(line) => println!("{line}"),
        None => {
            // Quiet resonant mode: no output; exit 0 indicates success.
        }
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

#[derive(Clone)]
struct InteractiveDefaults {
    bands: Option<String>,
    mode: Option<CalcMode>,
    antenna_model: Option<AntennaModel>,
    velocity: Option<f64>,
    antenna_height_m: Option<f64>,
    ground_class: Option<GroundClass>,
    conductor_diameter_mm: Option<f64>,
    transformer_ratio: Option<String>,
    wire_min_m: Option<f64>,
    wire_max_m: Option<f64>,
    units: Option<UnitSystem>,
}

impl InteractiveDefaults {
    fn new() -> Self {
        Self {
            bands: None,
            mode: None,
            antenna_model: None,
            velocity: None,
            antenna_height_m: None,
            ground_class: None,
            conductor_diameter_mm: None,
            transformer_ratio: None,
            wire_min_m: None,
            wire_max_m: None,
            units: None,
        }
    }
}

fn prompt_calc_mode_with_default(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    default: Option<CalcMode>,
) -> CalcMode {
    writeln!(output, "\nCalculation mode:").ok();
    writeln!(output, "  1) Resonant (default)").ok();
    writeln!(output, "  2) Non-resonant").ok();
    let prompt_str = match default {
        Some(CalcMode::NonResonant) => "Select calculation mode (1-2) [2]: ",
        _ => "Select calculation mode (1-2) [1]: ",
    };
    prompt(output, prompt_str);
    let line = read_line(input, "failed to read mode");
    match line.trim() {
        "2" => CalcMode::NonResonant,
        "1" => CalcMode::Resonant,
        "" => default.unwrap_or(CalcMode::Resonant),
        _ => CalcMode::Resonant,
    }
}

fn prompt_antenna_model_with_default(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    default: Option<AntennaModel>,
) -> Option<AntennaModel> {
    writeln!(output, "\nAntenna model:").ok();
    writeln!(output, "  d) Dipole (default)").ok();
    writeln!(output, "  e) End-fed half-wave").ok();
    writeln!(output, "  l) Full-wave loop").ok();
    writeln!(output, "  v) Inverted-V").ok();
    writeln!(output, "  o) Off-center-fed dipole (OCFD)").ok();
    writeln!(output, "  t) Trap dipole").ok();
    let prompt_str = match default {
        Some(AntennaModel::EndFedHalfWave) => "Select antenna model (d/e/l/v/o/t) [e]: ",
        Some(AntennaModel::FullWaveLoop) => "Select antenna model (d/e/l/v/o/t) [l]: ",
        Some(AntennaModel::InvertedVDipole) => "Select antenna model (d/e/l/v/o/t) [v]: ",
        Some(AntennaModel::OffCenterFedDipole) => "Select antenna model (d/e/l/v/o/t) [o]: ",
        Some(AntennaModel::TrapDipole) => "Select antenna model (d/e/l/v/o/t) [t]: ",
        _ => "Select antenna model (d/e/l/v/o/t) [d]: ",
    };
    prompt(output, prompt_str);
    let line = read_line(input, "failed to read antenna model");
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return default;
    }
    match trimmed {
        "d" | "dipole" => Some(AntennaModel::Dipole),
        "e" | "efhw" | "end-fed" | "end-fed-half-wave" => Some(AntennaModel::EndFedHalfWave),
        "l" | "loop" | "full-wave-loop" => Some(AntennaModel::FullWaveLoop),
        "v" | "inverted-v" | "inverted-v-dipole" | "inv-v" | "invertedv" | "invv" => {
            Some(AntennaModel::InvertedVDipole)
        }
        "o" | "ocfd" | "off-center-fed" | "off-center-fed-dipole" => {
            Some(AntennaModel::OffCenterFedDipole)
        }
        "t" | "trap" | "trap-dipole" | "trapdipole" => Some(AntennaModel::TrapDipole),
        _ => default,
    }
}

fn prompt_velocity_factor_with_default(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    default: Option<f64>,
) -> f64 {
    let prompt_str = match default {
        Some(v) => format!("Enter velocity factor (0.5-1.0) [{v:.2}]: "),
        None => "Enter velocity factor (0.5-1.0) [0.95]: ".to_string(),
    };
    prompt(output, &prompt_str);
    let line = read_line(input, "failed to read velocity factor");
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return default.unwrap_or(0.95);
    }
    match trimmed.parse::<f64>() {
        Ok(v) if (0.5..=1.0).contains(&v) => v,
        _ => default.unwrap_or(0.95),
    }
}

fn prompt_antenna_height_with_default(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    default: Option<f64>,
) -> f64 {
    let prompt_str = match default {
        Some(v) => format!("Antenna height in meters (7/10/12) [{v:.0}]: "),
        None => format!("Antenna height in meters (7/10/12) [{DEFAULT_ANTENNA_HEIGHT_M:.0}]: "),
    };
    prompt(output, &prompt_str);
    let line = read_line(input, "failed to read antenna height");
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return default.unwrap_or(DEFAULT_ANTENNA_HEIGHT_M);
    }
    match trimmed {
        "7" => 7.0,
        "10" => 10.0,
        "12" => 12.0,
        _ => default.unwrap_or(DEFAULT_ANTENNA_HEIGHT_M),
    }
}

fn prompt_ground_class_with_default(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    default: Option<GroundClass>,
) -> GroundClass {
    let default_label = default.unwrap_or(DEFAULT_GROUND_CLASS).as_label();
    let prompt_str = format!("Ground class (poor/average/good) [{default_label}]: ");
    prompt(output, &prompt_str);
    let line = read_line(input, "failed to read ground class");
    let trimmed = line.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return default.unwrap_or(DEFAULT_GROUND_CLASS);
    }
    match trimmed.as_str() {
        "poor" => GroundClass::Poor,
        "average" => GroundClass::Average,
        "good" => GroundClass::Good,
        _ => default.unwrap_or(DEFAULT_GROUND_CLASS),
    }
}

fn prompt_conductor_diameter_with_default(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    default: Option<f64>,
) -> f64 {
    let default_mm = default.unwrap_or(DEFAULT_CONDUCTOR_DIAMETER_MM);
    let prompt_str = format!(
        "Conductor diameter in mm ({:.1}-{:.1}) [{default_mm:.1}]: ",
        MIN_CONDUCTOR_DIAMETER_MM, MAX_CONDUCTOR_DIAMETER_MM
    );
    prompt(output, &prompt_str);
    let line = read_line(input, "failed to read conductor diameter");
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return default_mm;
    }
    match trimmed.parse::<f64>() {
        Ok(v) if (MIN_CONDUCTOR_DIAMETER_MM..=MAX_CONDUCTOR_DIAMETER_MM).contains(&v) => v,
        _ => default_mm,
    }
}

fn prompt_transformer_ratio_with_default(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    mode: CalcMode,
    antenna_model: Option<AntennaModel>,
    default: Option<&str>,
) -> TransformerRatio {
    let prompt_str = match default {
        Some(val) => format!("Enter transformer ratio (e.g. 1:9, recommended) [{val}]: "),
        None => "Enter transformer ratio (e.g. 1:9, recommended) [recommended]: ".to_string(),
    };
    prompt(output, &prompt_str);
    let line = read_line(input, "failed to read transformer ratio");
    let trimmed = line.trim();
    let raw = if trimmed.is_empty() {
        default.unwrap_or("recommended")
    } else {
        trimmed
    };
    if raw.eq_ignore_ascii_case("recommended") {
        return recommended_transformer_ratio(mode, antenna_model);
    }

    match TransformerRatio::parse(raw) {
        Some(ratio) => ratio,
        None => {
            writeln!(
                output,
                "{}",
                recommended_transformer_ratio_fallback_message(mode, antenna_model)
            )
            .expect("failed to write invalid ratio message");
            recommended_transformer_ratio(mode, antenna_model)
        }
    }
}

fn prompt_wire_length_window_with_default(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    default_min: Option<f64>,
    default_max: Option<f64>,
) -> (f64, f64, UnitSystem) {
    prompt(
        output,
        "Constraint units for wire length window (m/ft, Enter for m): ",
    );
    let unit_input = read_line(input, "failed to read wire length window units");
    let use_feet = matches!(
        unit_input.trim().to_ascii_lowercase().as_str(),
        "ft" | "feet"
    );

    let default_min_m = default_min.unwrap_or(DEFAULT_NON_RESONANT_CONFIG.min_len_m);
    let default_max_m = default_max.unwrap_or(DEFAULT_NON_RESONANT_CONFIG.max_len_m);

    if use_feet {
        let default_min_ft = default_min_m / FEET_TO_METERS;
        let default_max_ft = default_max_m / FEET_TO_METERS;

        prompt(
            output,
            &format!("Wire min length in feet (Enter for {default_min_ft:.1}): "),
        );
        let min_input = read_line(input, "failed to read wire min length");
        prompt(
            output,
            &format!("Wire max length in feet (Enter for {default_max_ft:.1}): "),
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
            "Invalid wire length window, using defaults {default_min_m:.1}-{default_max_m:.1} m."
        )
        .expect("failed to write invalid wire window message");
        return (default_min_m, default_max_m, UnitSystem::Imperial);
    }

    prompt(
        output,
        &format!("Wire min length in meters (Enter for {default_min_m:.1}): "),
    );
    let min_input = read_line(input, "failed to read wire min length");
    prompt(
        output,
        &format!("Wire max length in meters (Enter for {default_max_m:.1}): "),
    );
    let max_input = read_line(input, "failed to read wire max length");

    let min_len = min_input
        .trim()
        .parse::<f64>()
        .ok()
        .unwrap_or(default_min_m);
    let max_len = max_input
        .trim()
        .parse::<f64>()
        .ok()
        .unwrap_or(default_max_m);

    if min_len > 0.0 && max_len > min_len {
        (min_len, max_len, UnitSystem::Metric)
    } else {
        writeln!(
            output,
            "Invalid wire length window, using defaults {default_min_m:.1}-{default_max_m:.1} m."
        )
        .expect("failed to write invalid wire window message");
        (default_min_m, default_max_m, UnitSystem::Metric)
    }
}

fn prompt_display_units_with_default(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    auto_units: UnitSystem,
    default: Option<UnitSystem>,
) -> UnitSystem {
    let prompt_str = match default {
        Some(u) => format!(
            "Select display units (m/ft/both) [{}]: ",
            match u {
                UnitSystem::Metric => "m",
                UnitSystem::Imperial => "ft",
                UnitSystem::Both => "both",
            }
        ),
        None => "Select display units (m/ft/both) [both]: ".to_string(),
    };
    prompt(output, &prompt_str);
    let line = read_line(input, "failed to read units");
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return default.unwrap_or(auto_units);
    }
    match trimmed {
        "m" | "meters" => UnitSystem::Metric,
        "ft" | "feet" => UnitSystem::Imperial,
        "both" => UnitSystem::Both,
        _ => default.unwrap_or(auto_units),
    }
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
    print_project_info(output);
    writeln!(output).expect("failed to write interactive info spacing");

    let mut itu_region = prompt_itu_region(input, output);
    let mut defaults = InteractiveDefaults::new();

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
        writeln!(output, "  5) About / project info").expect("failed to write menu");
        writeln!(output, "  6) Exit").expect("failed to write menu");
        prompt(output, "\nSelect option (1-6): ");

        let choice = read_line(input, "failed to read choice");
        if choice.is_empty() {
            writeln!(output, "Exiting Rusty Wire.").expect("failed to write exit message");
            break;
        }

        match choice.trim() {
            "1" => show_all_bands_for_region_to_writer(output, itu_region),
            "2" => calculate_selected_bands_with_defaults(input, output, itu_region, &mut defaults),
            "3" => quick_calculation_with_defaults(input, output, itu_region, &mut defaults),
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
                print_project_info(output);
                writeln!(output).expect("failed to write interactive info spacing");
            }
            "6" => {
                writeln!(output, "Exiting Rusty Wire.").expect("failed to write exit message");
                break;
            }
            _ => writeln!(output, "Invalid option. Try again.\n")
                .expect("failed to write invalid option message"),
        }
    }
}

fn print_project_info(output: &mut dyn Write) {
    writeln!(output, "Project info:").expect("failed to write project info heading");
    writeln!(output, "  Version: {}", env!("CARGO_PKG_VERSION"))
        .expect("failed to write project info version");
    writeln!(output, "  Author: {}", env!("CARGO_PKG_AUTHORS"))
        .expect("failed to write project info author");
    writeln!(output, "  GitHub: {PROJECT_URL}").expect("failed to write project info url");
    writeln!(output, "  License: {}", env!("CARGO_PKG_LICENSE"))
        .expect("failed to write project info license");
    writeln!(
        output,
        "  Platform: {}/{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
    .expect("failed to write project info platform");
}

fn calculate_selected_bands_with_defaults(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    region: ITURegion,
    defaults: &mut InteractiveDefaults,
) {
    show_all_bands_for_region_to_writer(output, region);

    let bands_prompt = if let Some(ref last) = defaults.bands {
        format!("Enter bands (e.g. 40m,20m,10m-15m; Enter for default set) [{last}]: ")
    } else {
        "Enter bands (e.g. 40m,20m,10m-15m; Enter for default set): ".to_string()
    };
    prompt(output, &bands_prompt);
    let band_input = read_line(input, "failed to read selection");
    let trimmed = if band_input.trim().is_empty() {
        defaults.bands.as_deref().unwrap_or("")
    } else {
        band_input.trim()
    };
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
    if !band_input.trim().is_empty() {
        defaults.bands = Some(band_input.trim().to_string());
    }

    let mode = prompt_calc_mode_with_default(input, output, defaults.mode);
    defaults.mode = Some(mode);
    let antenna_model = prompt_antenna_model_with_default(input, output, defaults.antenna_model);
    defaults.antenna_model = antenna_model;
    let velocity = prompt_velocity_factor_with_default(input, output, defaults.velocity);
    defaults.velocity = Some(velocity);
    let antenna_height_m =
        prompt_antenna_height_with_default(input, output, defaults.antenna_height_m);
    defaults.antenna_height_m = Some(antenna_height_m);
    let ground_class = prompt_ground_class_with_default(input, output, defaults.ground_class);
    defaults.ground_class = Some(ground_class);
    let conductor_diameter_mm =
        prompt_conductor_diameter_with_default(input, output, defaults.conductor_diameter_mm);
    defaults.conductor_diameter_mm = Some(conductor_diameter_mm);
    let transformer_ratio = prompt_transformer_ratio_with_default(
        input,
        output,
        mode,
        antenna_model,
        defaults.transformer_ratio.as_deref(),
    );
    defaults.transformer_ratio = Some(transformer_ratio.as_label().to_string());
    let (wire_min_m, wire_max_m, auto_units) = if mode == CalcMode::NonResonant {
        prompt_wire_length_window_with_default(
            input,
            output,
            defaults.wire_min_m,
            defaults.wire_max_m,
        )
    } else {
        (
            DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            UnitSystem::Both,
        )
    };
    defaults.wire_min_m = Some(wire_min_m);
    defaults.wire_max_m = Some(wire_max_m);
    let units = prompt_display_units_with_default(input, output, auto_units, defaults.units);
    defaults.units = Some(units);

    let config = AppConfig {
        band_indices: indices,
        velocity_factor: velocity,
        mode,
        wire_min_m,
        wire_max_m,
        step_m: DEFAULT_NON_RESONANT_CONFIG.step_m,
        units,
        itu_region: region,
        transformer_ratio,
        antenna_model,
        antenna_height_m,
        ground_class,
        conductor_diameter_mm,
        custom_freq_mhz: None,
        freq_list_mhz: vec![],
        validate_with_fnec: false,
    };

    let results = match execute_request_checked(AppRequest::new(config)) {
        Ok(response) => response.results,
        Err(err) => {
            writeln!(output, "Error: {err}\n").expect("failed to write validation error");
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

fn quick_calculation_with_defaults(
    input: &mut dyn BufRead,
    output: &mut dyn Write,
    region: ITURegion,
    defaults: &mut InteractiveDefaults,
) {
    show_all_bands_for_region_to_writer(output, region);

    let band_prompt = if let Some(ref last) = defaults.bands {
        format!("Enter one band (e.g. 20m) [{last}]: ")
    } else {
        "Enter one band (e.g. 20m): ".to_string()
    };
    prompt(output, &band_prompt);
    let band_input = read_line(input, "failed to read selection");
    let trimmed = if band_input.trim().is_empty() {
        defaults.bands.as_deref().unwrap_or("")
    } else {
        band_input.trim()
    };
    let idx = match parse_single_band_token(trimmed, region) {
        Ok(v) => v,
        Err(_) => {
            writeln!(output, "Invalid band. Use a single band name like 20m.\n")
                .expect("failed to write invalid number message");
            return;
        }
    };
    if !band_input.trim().is_empty() {
        defaults.bands = Some(band_input.trim().to_string());
    }

    let mode = prompt_calc_mode_with_default(input, output, defaults.mode);
    defaults.mode = Some(mode);
    let antenna_model = prompt_antenna_model_with_default(input, output, defaults.antenna_model);
    defaults.antenna_model = antenna_model;
    let velocity = prompt_velocity_factor_with_default(input, output, defaults.velocity);
    defaults.velocity = Some(velocity);
    let antenna_height_m =
        prompt_antenna_height_with_default(input, output, defaults.antenna_height_m);
    defaults.antenna_height_m = Some(antenna_height_m);
    let ground_class = prompt_ground_class_with_default(input, output, defaults.ground_class);
    defaults.ground_class = Some(ground_class);
    let conductor_diameter_mm =
        prompt_conductor_diameter_with_default(input, output, defaults.conductor_diameter_mm);
    defaults.conductor_diameter_mm = Some(conductor_diameter_mm);
    let transformer_ratio = prompt_transformer_ratio_with_default(
        input,
        output,
        mode,
        antenna_model,
        defaults.transformer_ratio.as_deref(),
    );
    defaults.transformer_ratio = Some(transformer_ratio.as_label().to_string());
    let (wire_min_m, wire_max_m, auto_units) = if mode == CalcMode::NonResonant {
        prompt_wire_length_window_with_default(
            input,
            output,
            defaults.wire_min_m,
            defaults.wire_max_m,
        )
    } else {
        (
            DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            UnitSystem::Both,
        )
    };
    defaults.wire_min_m = Some(wire_min_m);
    defaults.wire_max_m = Some(wire_max_m);
    let units = prompt_display_units_with_default(input, output, auto_units, defaults.units);
    defaults.units = Some(units);

    let config = AppConfig {
        band_indices: vec![idx],
        velocity_factor: velocity,
        mode,
        wire_min_m,
        wire_max_m,
        step_m: DEFAULT_NON_RESONANT_CONFIG.step_m,
        units,
        itu_region: region,
        transformer_ratio,
        antenna_model,
        antenna_height_m,
        ground_class,
        conductor_diameter_mm,
        custom_freq_mhz: None,
        freq_list_mhz: vec![],
        validate_with_fnec: false,
    };

    let results = match execute_request_checked(AppRequest::new(config)) {
        Ok(response) => response.results,
        Err(err) => {
            writeln!(output, "Error: {err}\n").expect("failed to write validation error");
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
                    err_msg = Some(format!("unknown format '{other}'; skipping export."));
                    break;
                }
            }
        }
        if let Some(msg) = err_msg {
            writeln!(output, "{msg}").expect("failed to write export error message");
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
            Ok(()) => writeln!(output, "Exported results to {output_path}")
                .expect("failed to write export success message"),
            Err(err) => writeln!(output, "Failed to export {output_path}: {err}")
                .expect("failed to write export failure message"),
        }
        chosen.push((fmt, output_path));
    }

    chosen
}

fn prompt(output: &mut dyn Write, text: &str) {
    write!(output, "{text}").expect("failed to write interactive prompt");
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
        .map(|v| band_label_for_index(*v, config.itu_region))
        .collect::<Vec<String>>()
        .join(",");

    let units_str = match config.units {
        UnitSystem::Metric => "m",
        UnitSystem::Imperial => "ft",
        UnitSystem::Both => "both",
    };
    let mut cmd = format!(
        "rusty-wire --region {} --mode {} --bands {} --velocity {:.2} --height {:.0} --ground {} --conductor-mm {:.1} --transformer {} --units {}",
        shell_quote(config.itu_region.short_name()),
        shell_quote(match config.mode {
            CalcMode::Resonant => "resonant",
            CalcMode::NonResonant => "non-resonant",
        }),
        shell_quote(&bands_csv),
        config.velocity_factor,
        config.antenna_height_m,
        shell_quote(config.ground_class.as_label()),
        config.conductor_diameter_mm,
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
            AntennaModel::TrapDipole => "trap-dipole",
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
    println!("  {cmd}\n");
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
    format!("'{escaped}'")
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
    let view = band_listing_view(region);
    for line in band_listing_display_lines(&view) {
        writeln!(output, "{line}").expect("failed to write band listing");
    }
}
// ---------------------------------------------------------------------------
// Terminal display
// ---------------------------------------------------------------------------

fn print_results(results: &AppResults) {
    let doc = results_display_document(results);

    println!("\n{}", doc.overview_heading);
    for line in doc.overview_header_lines {
        println!("{line}");
    }
    for view in doc.band_views {
        println!("{}", view.title);
        for line in view.lines {
            println!("{line}");
        }
        println!();
    }
    for line in doc.summary_lines {
        println!("{line}");
    }
    println!();

    for section in doc.sections {
        for line in section.lines {
            println!("{line}");
        }
        println!();
    }

    for line in doc.warning_lines {
        println!("{line}");
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use std::sync::Mutex;

    // Serialize all tests that read or write the default export file names to
    // prevent false failures caused by parallel test execution.
    static DEFAULT_EXPORT_PATH_LOCK: Mutex<()> = Mutex::new(());

    fn sample_results_for_export_tests() -> AppResults {
        AppResults {
            calculations: Vec::new(),
            recommendation: None,
            optima: Vec::new(),
            window_optima: Vec::new(),
            resonant_compromises: Vec::new(),
            config: AppConfig::default(),
            skipped_band_indices: Vec::new(),
        }
    }

    #[test]
    fn run_interactive_with_io_exits_cleanly() {
        let mut input = Cursor::new(b"\n6\n".to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Select option (1-6): "));
        assert!(rendered.contains("Exiting Rusty Wire."));
    }

    #[test]
    fn run_interactive_with_io_exits_cleanly_on_eof_menu_input() {
        let mut input = Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Select option (1-6): "));
        assert!(rendered.contains("Exiting Rusty Wire."));
    }

    #[test]
    fn run_interactive_with_io_about_menu_shows_project_info() {
        let mut input = Cursor::new(b"\n5\n6\n".to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Project info:"));
        assert!(rendered.contains("Version:"));
        assert!(rendered.contains("Author:"));
        assert!(rendered.contains("GitHub:"));
        assert!(rendered.contains("License:"));
        assert!(rendered.contains("Platform:"));
    }

    #[test]
    fn prompt_antenna_model_accepts_inverted_v_alias() {
        let mut input = Cursor::new(b"invv\n".to_vec());
        let mut output = Vec::new();

        let model = prompt_antenna_model_with_default(&mut input, &mut output, None);

        assert_eq!(model, Some(AntennaModel::InvertedVDipole));
    }

    #[test]
    fn prompt_wire_length_window_supports_feet_input() {
        let mut input = Cursor::new(b"ft\n40\n80\n".to_vec());
        let mut output = Vec::new();

        let (min_m, max_m, units) =
            prompt_wire_length_window_with_default(&mut input, &mut output, None, None);

        assert_eq!(units, UnitSystem::Imperial);
        assert!((min_m - 12.192).abs() < 1e-6);
        assert!((max_m - 24.384).abs() < 1e-6);
    }

    #[test]
    fn prompt_wire_length_window_supports_metric_input() {
        let mut input = Cursor::new(b"m\n12\n25\n".to_vec());
        let mut output = Vec::new();

        let (min_m, max_m, units) =
            prompt_wire_length_window_with_default(&mut input, &mut output, None, None);

        assert_eq!(units, UnitSystem::Metric);
        assert!((min_m - 12.0).abs() < 1e-9);
        assert!((max_m - 25.0).abs() < 1e-9);
    }

    #[test]
    fn prompt_wire_length_window_invalid_metric_input_falls_back_to_defaults() {
        let mut input = Cursor::new(b"\n30\n10\n".to_vec());
        let mut output = Vec::new();

        let (min_m, max_m, units) =
            prompt_wire_length_window_with_default(&mut input, &mut output, Some(9.0), Some(27.0));

        assert_eq!(units, UnitSystem::Metric);
        assert!((min_m - 9.0).abs() < 1e-9);
        assert!((max_m - 27.0).abs() < 1e-9);
        let rendered = String::from_utf8(output).expect("output should be utf-8");
        assert!(rendered.contains("Invalid wire length window, using defaults 9.0-27.0 m."));
    }

    #[test]
    fn prompt_wire_length_window_invalid_feet_input_falls_back_to_defaults() {
        let mut input = Cursor::new(b"ft\n80\n40\n".to_vec());
        let mut output = Vec::new();

        let (min_m, max_m, units) =
            prompt_wire_length_window_with_default(&mut input, &mut output, Some(9.0), Some(27.0));

        assert_eq!(units, UnitSystem::Imperial);
        assert!((min_m - 9.0).abs() < 1e-9);
        assert!((max_m - 27.0).abs() < 1e-9);
        let rendered = String::from_utf8(output).expect("output should be utf-8");
        assert!(rendered.contains("Invalid wire length window, using defaults 9.0-27.0 m."));
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

        let ratio = prompt_transformer_ratio_with_default(
            &mut input,
            &mut output,
            CalcMode::Resonant,
            Some(AntennaModel::EndFedHalfWave),
            None,
        );

        assert_eq!(ratio, TransformerRatio::R1To56);
    }

    #[test]
    fn prompt_transformer_ratio_invalid_value_falls_back_to_recommended_with_message() {
        let mut input = Cursor::new(b"1:13\n".to_vec());
        let mut output = Vec::new();

        let ratio = prompt_transformer_ratio_with_default(
            &mut input,
            &mut output,
            CalcMode::NonResonant,
            None,
            None,
        );

        assert_eq!(ratio, TransformerRatio::R1To9);
        let rendered = String::from_utf8(output).expect("output should be utf-8");
        assert!(
            rendered.contains(&recommended_transformer_ratio_fallback_message(
                CalcMode::NonResonant,
                None,
            ))
        );
    }

    #[test]
    fn calculate_selected_bands_with_defaults_rejects_invalid_csv_input() {
        let mut input = Cursor::new(b"abc,4\n".to_vec());
        let mut output = Vec::new();
        let mut defaults = InteractiveDefaults::new();

        calculate_selected_bands_with_defaults(
            &mut input,
            &mut output,
            ITURegion::Region1,
            &mut defaults,
        );

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
        assert!(err.to_string().contains("unknown band"));
    }

    #[test]
    fn parse_band_selection_rejects_unknown_band_name() {
        let err = parse_band_selection("banana", ITURegion::Region1).unwrap_err();
        assert!(err.to_string().contains("unknown band"));
    }

    #[test]
    fn print_equivalent_cli_call_uses_band_labels() {
        let config = AppConfig {
            band_indices: vec![4, 6, 10],
            velocity_factor: 0.95,
            mode: CalcMode::Resonant,
            wire_min_m: DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            wire_max_m: DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            step_m: DEFAULT_NON_RESONANT_CONFIG.step_m,
            units: UnitSystem::Metric,
            itu_region: ITURegion::Region1,
            transformer_ratio: TransformerRatio::R1To1,
            antenna_model: None,
            antenna_height_m: DEFAULT_ANTENNA_HEIGHT_M,
            ground_class: DEFAULT_GROUND_CLASS,
            conductor_diameter_mm: DEFAULT_CONDUCTOR_DIAMETER_MM,
            custom_freq_mhz: None,
            freq_list_mhz: vec![],
            validate_with_fnec: false,
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
        let results = sample_results_for_export_tests();

        let exports = interactive_export_prompt(&mut input, &mut output, &results);

        assert!(exports.is_empty());
        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("unknown format 'yaml'; skipping export."));
    }

    #[test]
    fn run_interactive_with_io_can_switch_region() {
        let mut input = Cursor::new(b"\n4\n2\n6\n".to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Switched to ITU Region 2."));
        assert!(rendered.contains("List all bands (for Region 2)"));
    }

    #[test]
    fn run_interactive_with_io_quick_calculation_invalid_number() {
        let mut input = Cursor::new(b"\n3\nabc\n6\n".to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Enter one band (e.g. 20m): "));
        assert!(rendered.contains("Invalid band. Use a single band name like 20m."));
    }

    #[test]
    fn run_interactive_with_io_lists_bands_to_writer_output() {
        let mut input = Cursor::new(b"\n1\n6\n".to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Available bands in Region 1"));
    }

    #[test]
    fn run_interactive_with_io_invalid_menu_option_shows_error() {
        // Select default region, enter invalid option "9", then exit
        let mut input = Cursor::new(b"\n9\n6\n".to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Invalid option. Try again."));
    }

    #[test]
    fn run_interactive_with_io_multi_band_calculation_completes() {
        // Default region → option 2 → bands 40m,20m → all defaults → no export → exit
        let input_bytes = b"\n2\n40m,20m\n\n\n\n\n\n\n\n\nnone\n6\n";
        let mut input = Cursor::new(input_bytes.to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        // Prompt sequence for bands should appear
        assert!(rendered.contains("Enter bands"));
        // Export prompt should appear (confirming we reached end of calculation flow)
        assert!(rendered.contains("Export results?"));
    }

    #[test]
    fn run_interactive_with_io_quick_calculation_happy_path() {
        // Default region → option 3 → 20m band → all defaults → no export → exit
        let input_bytes = b"\n3\n20m\n\n\n\n\n\n\n\n\nnone\n6\n";
        let mut input = Cursor::new(input_bytes.to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        // Quick calc prompts single band
        assert!(rendered.contains("Enter one band"));
        // Export prompt should appear (confirming calculation completed)
        assert!(rendered.contains("Export results?"));
    }

    #[test]
    fn run_interactive_with_io_reuses_last_defaults_on_followup_quick_calculation() {
        let input_bytes = concat!(
            "\n", "3\n", "20m\n", "2\n", "e\n", "0.85\n", "12\n", "good\n", "3.5\n", "1:49\n",
            "\n", "9\n", "27\n", "m\n", "none\n", "3\n", "\n", "\n", "\n", "\n", "\n", "\n", "\n",
            "\n", "\n", "\n", "\n", "none\n", "6\n"
        );
        let mut input = Cursor::new(input_bytes.as_bytes().to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Enter one band (e.g. 20m) [20m]: "));
        assert!(rendered.contains("Select calculation mode (1-2) [2]: "));
        assert!(rendered.contains("Select antenna model (d/e/l/v/o/t) [e]: "));
        assert!(rendered.contains("Enter velocity factor (0.5-1.0) [0.85]: "));
        assert!(rendered.contains("Antenna height in meters (7/10/12) [12]: "));
        assert!(rendered.contains("Ground class (poor/average/good) [good]: "));
        assert!(rendered.contains("Conductor diameter in mm (1.0-4.0) [3.5]: "));
        assert!(rendered.contains("Enter transformer ratio (e.g. 1:9, recommended) [1:49]: "));
        assert!(rendered.contains("Wire min length in meters (Enter for 9.0): "));
        assert!(rendered.contains("Wire max length in meters (Enter for 27.0): "));
        assert!(rendered.contains("Select display units (m/ft/both) [m]: "));
    }

    #[test]
    fn interactive_export_prompt_requires_at_least_one_format_when_only_commas() {
        let mut input = Cursor::new(b" , , \n".to_vec());
        let mut output = Vec::new();
        let results = sample_results_for_export_tests();

        let exports = interactive_export_prompt(&mut input, &mut output, &results);

        assert!(exports.is_empty());
        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("--export requires at least one format; skipping export."));
    }

    #[test]
    fn interactive_export_prompt_none_returns_empty_without_errors() {
        let mut input = Cursor::new(b"none\n".to_vec());
        let mut output = Vec::new();
        let results = sample_results_for_export_tests();

        let exports = interactive_export_prompt(&mut input, &mut output, &results);

        assert!(exports.is_empty());
        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(!rendered.contains("unknown format"));
        assert!(!rendered.contains("skipping export"));
    }

    #[test]
    fn interactive_export_prompt_single_format_uses_explicit_output_path() {
        let out_path = format!("rusty-wire-export-test-{}-single.csv", std::process::id());
        let input_bytes = format!("csv\n{}\n", out_path);
        let mut input = Cursor::new(input_bytes.into_bytes());
        let mut output = Vec::new();
        let results = sample_results_for_export_tests();

        let exports = interactive_export_prompt(&mut input, &mut output, &results);

        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].0, ExportFormat::Csv);
        assert_eq!(exports[0].1, out_path);
        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Exported results to"));
        assert!(std::path::Path::new(&exports[0].1).exists());

        let _ = fs::remove_file(&exports[0].1);
    }

    #[test]
    fn interactive_export_prompt_multiple_formats_use_default_output_paths() {
        let _guard = DEFAULT_EXPORT_PATH_LOCK.lock().unwrap();
        let mut input = Cursor::new(b"csv,json\n".to_vec());
        let mut output = Vec::new();
        let results = sample_results_for_export_tests();

        let exports = interactive_export_prompt(&mut input, &mut output, &results);

        assert_eq!(exports.len(), 2);
        assert_eq!(exports[0].0, ExportFormat::Csv);
        assert_eq!(exports[0].1, default_output_name(ExportFormat::Csv));
        assert_eq!(exports[1].0, ExportFormat::Json);
        assert_eq!(exports[1].1, default_output_name(ExportFormat::Json));

        for (_, path) in &exports {
            assert!(std::path::Path::new(path).exists());
            let _ = fs::remove_file(path);
        }
    }

    #[test]
    fn interactive_export_prompt_deduplicates_alias_formats_in_input_order() {
        let _guard = DEFAULT_EXPORT_PATH_LOCK.lock().unwrap();
        let mut input = Cursor::new(b"md,text,markdown,txt,text\n".to_vec());
        let mut output = Vec::new();
        let results = sample_results_for_export_tests();

        let exports = interactive_export_prompt(&mut input, &mut output, &results);

        assert_eq!(exports.len(), 2);
        assert_eq!(exports[0].0, ExportFormat::Markdown);
        assert_eq!(exports[0].1, default_output_name(ExportFormat::Markdown));
        assert_eq!(exports[1].0, ExportFormat::Txt);
        assert_eq!(exports[1].1, default_output_name(ExportFormat::Txt));

        for (_, path) in &exports {
            assert!(std::path::Path::new(path).exists());
            let _ = fs::remove_file(path);
        }
    }

    #[test]
    fn interactive_export_prompt_mixed_valid_and_invalid_formats_aborts_export() {
        let _guard = DEFAULT_EXPORT_PATH_LOCK.lock().unwrap();
        let csv_path = default_output_name(ExportFormat::Csv);
        let _ = fs::remove_file(csv_path);

        let mut input = Cursor::new(b"csv,yaml\n".to_vec());
        let mut output = Vec::new();
        let results = sample_results_for_export_tests();

        let exports = interactive_export_prompt(&mut input, &mut output, &results);

        assert!(exports.is_empty());
        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("unknown format 'yaml'; skipping export."));
        assert!(!std::path::Path::new(csv_path).exists());
    }

    // --- prompt_calc_mode_with_default ---

    #[test]
    fn prompt_calc_mode_empty_with_non_resonant_default_returns_non_resonant() {
        let mut input = Cursor::new(b"\n".to_vec());
        let mut output = Vec::new();

        let mode =
            prompt_calc_mode_with_default(&mut input, &mut output, Some(CalcMode::NonResonant));

        assert_eq!(mode, CalcMode::NonResonant);
        let rendered = String::from_utf8(output).expect("output should be utf-8");
        assert!(rendered.contains("[2]: "));
    }

    #[test]
    fn prompt_calc_mode_unknown_input_falls_back_to_resonant() {
        let mut input = Cursor::new(b"xyz\n".to_vec());
        let mut output = Vec::new();

        let mode = prompt_calc_mode_with_default(&mut input, &mut output, None);

        assert_eq!(mode, CalcMode::Resonant);
    }

    // --- prompt_velocity_factor_with_default ---

    #[test]
    fn prompt_velocity_factor_valid_value_is_accepted() {
        let mut input = Cursor::new(b"0.75\n".to_vec());
        let mut output = Vec::new();

        let vf = prompt_velocity_factor_with_default(&mut input, &mut output, None);

        assert!((vf - 0.75).abs() < 1e-9);
    }

    #[test]
    fn prompt_velocity_factor_out_of_range_falls_back_to_default() {
        let mut input = Cursor::new(b"2.0\n".to_vec());
        let mut output = Vec::new();

        let vf = prompt_velocity_factor_with_default(&mut input, &mut output, Some(0.85));

        assert!((vf - 0.85).abs() < 1e-9);
    }

    // --- prompt_antenna_height_with_default ---

    #[test]
    fn prompt_antenna_height_valid_seven_is_accepted() {
        let mut input = Cursor::new(b"7\n".to_vec());
        let mut output = Vec::new();

        let h = prompt_antenna_height_with_default(&mut input, &mut output, None);

        assert!((h - 7.0).abs() < 1e-9);
    }

    #[test]
    fn prompt_antenna_height_invalid_input_falls_back_to_default() {
        let mut input = Cursor::new(b"rooftop\n".to_vec());
        let mut output = Vec::new();

        let h = prompt_antenna_height_with_default(&mut input, &mut output, Some(10.0));

        assert!((h - 10.0).abs() < 1e-9);
    }

    // --- prompt_ground_class_with_default ---

    #[test]
    fn prompt_ground_class_poor_is_accepted() {
        let mut input = Cursor::new(b"poor\n".to_vec());
        let mut output = Vec::new();

        let gc = prompt_ground_class_with_default(&mut input, &mut output, None);

        assert_eq!(gc, GroundClass::Poor);
    }

    #[test]
    fn prompt_ground_class_average_is_accepted() {
        let mut input = Cursor::new(b"average\n".to_vec());
        let mut output = Vec::new();

        let gc = prompt_ground_class_with_default(&mut input, &mut output, None);

        assert_eq!(gc, GroundClass::Average);
    }

    #[test]
    fn prompt_ground_class_unknown_input_falls_back_to_default() {
        let mut input = Cursor::new(b"swamp\n".to_vec());
        let mut output = Vec::new();

        let gc = prompt_ground_class_with_default(&mut input, &mut output, Some(GroundClass::Poor));

        assert_eq!(gc, GroundClass::Poor);
    }

    // --- prompt_conductor_diameter_with_default ---

    #[test]
    fn prompt_conductor_diameter_valid_value_is_accepted() {
        let mut input = Cursor::new(b"2.5\n".to_vec());
        let mut output = Vec::new();

        let d = prompt_conductor_diameter_with_default(&mut input, &mut output, None);

        assert!((d - 2.5).abs() < 1e-9);
    }

    #[test]
    fn prompt_conductor_diameter_out_of_range_falls_back_to_default() {
        let mut input = Cursor::new(b"0.1\n".to_vec());
        let mut output = Vec::new();

        let d = prompt_conductor_diameter_with_default(&mut input, &mut output, Some(3.0));

        assert!((d - 3.0).abs() < 1e-9);
    }

    // --- prompt_display_units_with_default ---

    #[test]
    fn prompt_display_units_empty_uses_default_value() {
        let mut input = Cursor::new(b"\n".to_vec());
        let mut output = Vec::new();

        let units = prompt_display_units_with_default(
            &mut input,
            &mut output,
            UnitSystem::Both,
            Some(UnitSystem::Metric),
        );

        assert_eq!(units, UnitSystem::Metric);
        let rendered = String::from_utf8(output).expect("output should be utf-8");
        assert!(rendered.contains("Select display units (m/ft/both) [m]: "));
    }

    #[test]
    fn prompt_display_units_accepts_both_keyword() {
        let mut input = Cursor::new(b"both\n".to_vec());
        let mut output = Vec::new();

        let units =
            prompt_display_units_with_default(&mut input, &mut output, UnitSystem::Metric, None);

        assert_eq!(units, UnitSystem::Both);
    }

    #[test]
    fn prompt_display_units_unknown_input_falls_back_to_auto_units() {
        let mut input = Cursor::new(b"yards\n".to_vec());
        let mut output = Vec::new();

        let units =
            prompt_display_units_with_default(&mut input, &mut output, UnitSystem::Imperial, None);

        assert_eq!(units, UnitSystem::Imperial);
    }

    // --- prompt_itu_region ---

    #[test]
    fn prompt_itu_region_accepts_region_three() {
        let mut input = Cursor::new(b"3\n".to_vec());
        let mut output = Vec::new();

        let region = prompt_itu_region(&mut input, &mut output);

        assert_eq!(region, ITURegion::Region3);
    }

    #[test]
    fn prompt_itu_region_invalid_input_falls_back_to_default() {
        let mut input = Cursor::new(b"9\n".to_vec());
        let mut output = Vec::new();

        let region = prompt_itu_region(&mut input, &mut output);

        assert_eq!(region, ITURegion::Region1);
        let rendered = String::from_utf8(output).expect("output should be utf-8");
        assert!(rendered.contains("Invalid region. Using default Region 1."));
    }

    // --- prompt_antenna_model_with_default (trap aliases) ---

    #[test]
    fn prompt_antenna_model_accepts_trap_short_alias() {
        let mut input = Cursor::new(b"t\n".to_vec());
        let mut output = Vec::new();

        let model = prompt_antenna_model_with_default(&mut input, &mut output, None);

        assert_eq!(model, Some(AntennaModel::TrapDipole));
    }

    #[test]
    fn prompt_antenna_model_accepts_trap_word_alias() {
        let mut input = Cursor::new(b"trap\n".to_vec());
        let mut output = Vec::new();

        let model = prompt_antenna_model_with_default(&mut input, &mut output, None);

        assert_eq!(model, Some(AntennaModel::TrapDipole));
    }

    #[test]
    fn prompt_antenna_model_unknown_input_falls_back_to_default() {
        let mut input = Cursor::new(b"yagi\n".to_vec());
        let mut output = Vec::new();

        let model =
            prompt_antenna_model_with_default(&mut input, &mut output, Some(AntennaModel::Dipole));

        assert_eq!(model, Some(AntennaModel::Dipole));
    }

    #[test]
    fn run_interactive_with_io_option_4_changes_region() {
        // Default region (1) → option 4 → select region 3 → exit
        let mut input = Cursor::new(b"\n4\n3\n6\n".to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Switched to ITU Region 3"));
    }

    #[test]
    fn run_interactive_with_io_option_5_shows_project_info() {
        // Default region → option 5 (about/project info) → exit
        let mut input = Cursor::new(b"\n5\n6\n".to_vec());
        let mut output = Vec::new();

        run_interactive_with_io(&mut input, &mut output);

        let rendered = String::from_utf8(output).expect("interactive output should be utf-8");
        assert!(rendered.contains("Project info:"));
        assert!(rendered.contains("Version:"));
        assert!(rendered.contains("GitHub:"));
    }

    #[test]
    fn interactive_export_prompt_uppercase_tokens_are_normalized() {
        // Uppercase format tokens should be lowercased and accepted
        let mut input = Cursor::new(b"CSV,JSON\n".to_vec());
        let mut output = Vec::new();
        let results = sample_results_for_export_tests();

        let exports = interactive_export_prompt(&mut input, &mut output, &results);

        assert_eq!(exports.len(), 2);
        assert_eq!(exports[0].0, ExportFormat::Csv);
        assert_eq!(exports[1].0, ExportFormat::Json);
    }
}
