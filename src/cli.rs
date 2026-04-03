/// CLI argument parsing, interactive prompts, and terminal output.
///
/// This module owns everything that is specific to the command-line interface:
/// argument parsing, stdin/stdout prompts, and formatted terminal output.
/// The computation itself is delegated to `app::run_calculation`; the only
/// imports from the core modules that this file needs are for display helpers.
use crate::app::{
    AppConfig, AppResults, CalcMode, ExportFormat, UnitSystem, DEFAULT_BAND_SELECTION,
    FEET_TO_METERS, run_calculation,
};
use crate::bands::{band_count, BANDS};
use crate::calculations::{
    calculate_average_max_distance, calculate_average_min_distance, WireCalculation,
    DEFAULT_NON_RESONANT_CONFIG,
};
use crate::export::{default_output_name, export_results};
use std::io::{self, Write};

// ---------------------------------------------------------------------------
// CLI-only options struct (not part of the public app API)
// ---------------------------------------------------------------------------

struct CliOptions {
    bands: Option<Vec<usize>>,
    velocity_factor: f64,
    export: Vec<ExportFormat>,
    output: Option<String>,
    wire_min_m: f64,
    wire_max_m: f64,
    wire_min_ft: Option<f64>,
    wire_max_ft: Option<f64>,
    mode: CalcMode,
    list_bands: bool,
    help: bool,
    units: Option<UnitSystem>,
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Entry point when CLI arguments are present.
pub fn run_from_args(args: &[String]) {
    match parse_cli_args(args) {
        Ok(opts) => run_non_interactive(opts),
        Err(err) => {
            eprintln!("Error: {}", err);
            print_usage();
            std::process::exit(2);
        }
    }
}

/// Entry point for interactive (no-argument) mode.
pub fn run_interactive() {
    println!("============================================================");
    println!("Rusty Wire v{} - Resonant Length and Skip Distance Calculator", env!("CARGO_PKG_VERSION"));
    println!("============================================================\n");

    loop {
        println!("Menu:");
        println!("  1) List all bands");
        println!("  2) Calculate selected bands");
        println!("  3) Quick single-band calculation");
        println!("  4) Exit");
        print!("\nSelect option (1-4): ");
        io::stdout().flush().expect("failed to flush stdout");

        let mut choice = String::new();
        io::stdin()
            .read_line(&mut choice)
            .expect("failed to read choice");

        match choice.trim() {
            "1" => show_all_bands(),
            "2" => calculate_selected_bands(),
            "3" => quick_calculation(),
            "4" => {
                println!("Exiting Rusty Wire.");
                break;
            }
            _ => println!("Invalid option. Try again.\n"),
        }
    }
}

// ---------------------------------------------------------------------------
// Non-interactive (CLI) runner
// ---------------------------------------------------------------------------

fn run_non_interactive(opts: CliOptions) {
    if opts.help {
        print_usage();
        return;
    }

    if opts.list_bands {
        show_all_bands();
    }

    let indices = match opts.bands {
        Some(v) => v,
        None => {
            if !opts.list_bands {
                println!("No --bands provided; using default 40m-10m set (4,5,6,7,8,9,10).\n");
            }
            DEFAULT_BAND_SELECTION.to_vec()
        }
    };

    let units = opts.units.unwrap_or(if opts.wire_min_ft.is_some() {
        UnitSystem::Imperial
    } else {
        UnitSystem::Metric
    });

    let config = AppConfig {
        band_indices: indices,
        velocity_factor: opts.velocity_factor,
        mode: opts.mode,
        wire_min_m: opts.wire_min_m,
        wire_max_m: opts.wire_max_m,
        units,
    };

    let results = run_calculation(config);
    if results.calculations.is_empty() {
        println!("No valid bands selected.");
        return;
    }

    print_results(&results);

    let single_output = opts.output;
    let export_count = opts.export.len();
    for (i, &fmt) in opts.export.iter().enumerate() {
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
            results.recommendation.as_ref(),
            results.config.units,
        ) {
            eprintln!("Failed to export {}: {}", output, err);
            std::process::exit(1);
        }
        println!("Exported results to {}", output);
    }
}

// ---------------------------------------------------------------------------
// Interactive mode helpers
// ---------------------------------------------------------------------------

fn show_all_bands() {
    println!("\nAvailable bands ({} total):", band_count());
    println!("------------------------------------------------------------");
    for (idx, band) in BANDS.iter().enumerate() {
        println!("{:2}. {}", idx + 1, band);
    }
    println!();
}

fn calculate_selected_bands() {
    show_all_bands();
    print!("Enter band numbers separated by commas (Enter for default 4,5,6,7,8,9,10): ");
    io::stdout().flush().expect("failed to flush stdout");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("failed to read selection");

    let indices = if input.trim().is_empty() {
        DEFAULT_BAND_SELECTION.to_vec()
    } else {
        let parsed: Result<Vec<usize>, _> = input
            .trim()
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().parse::<usize>())
            .collect();

        match parsed {
            Ok(v) if !v.is_empty() => v,
            _ => {
                println!("Invalid input. Use comma-separated numbers.\n");
                return;
            }
        }
    };

    let mode = prompt_calc_mode();
    let velocity = prompt_velocity_factor();
    let (wire_min_m, wire_max_m, auto_units) = if mode == CalcMode::NonResonant {
        prompt_wire_length_window()
    } else {
        (DEFAULT_NON_RESONANT_CONFIG.min_len_m, DEFAULT_NON_RESONANT_CONFIG.max_len_m, UnitSystem::Both)
    };
    let units = prompt_display_units(auto_units);

    let config = AppConfig {
        band_indices: indices,
        velocity_factor: velocity,
        mode,
        wire_min_m,
        wire_max_m,
        units,
    };

    let results = run_calculation(config);
    if results.calculations.is_empty() {
        println!("No valid bands selected.\n");
        return;
    }

    print_results(&results);
    print_equivalent_cli_call(&results.config, &[]);
    let export_choices = interactive_export_prompt(&results);
    if !export_choices.is_empty() {
        print_equivalent_cli_call(&results.config, &export_choices);
    }
}

fn quick_calculation() {
    show_all_bands();
    print!("Enter one band number: ");
    io::stdout().flush().expect("failed to flush stdout");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("failed to read selection");

    let idx = match input.trim().parse::<usize>() {
        Ok(v) => v,
        Err(_) => {
            println!("Invalid number.\n");
            return;
        }
    };

    let mode = prompt_calc_mode();
    let velocity = prompt_velocity_factor();
    let (wire_min_m, wire_max_m, auto_units) = if mode == CalcMode::NonResonant {
        prompt_wire_length_window()
    } else {
        (DEFAULT_NON_RESONANT_CONFIG.min_len_m, DEFAULT_NON_RESONANT_CONFIG.max_len_m, UnitSystem::Both)
    };
    let units = prompt_display_units(auto_units);

    let config = AppConfig {
        band_indices: vec![idx],
        velocity_factor: velocity,
        mode,
        wire_min_m,
        wire_max_m,
        units,
    };

    let results = run_calculation(config);
    if results.calculations.is_empty() {
        println!("Band not found.\n");
        return;
    }

    print_results(&results);
    print_equivalent_cli_call(&results.config, &[]);
    let export_choices = interactive_export_prompt(&results);
    if !export_choices.is_empty() {
        print_equivalent_cli_call(&results.config, &export_choices);
    }
}

// ---------------------------------------------------------------------------
// Prompts
// ---------------------------------------------------------------------------

fn prompt_calc_mode() -> CalcMode {
    print!("Calculation mode (resonant/non-resonant, Enter for resonant): ");
    io::stdout().flush().expect("failed to flush stdout");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("failed to read calculation mode");

    match input.trim().to_ascii_lowercase().as_str() {
        "" | "resonant" => CalcMode::Resonant,
        "non-resonant" | "nonresonant" | "non_resonant" => CalcMode::NonResonant,
        _ => {
            println!("Unknown mode. Using resonant.");
            CalcMode::Resonant
        }
    }
}

fn prompt_velocity_factor() -> f64 {
    print!("Velocity factor (0.50-1.00, Enter for 0.95): ");
    io::stdout().flush().expect("failed to flush stdout");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("failed to read velocity factor");
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return 0.95;
    }

    match trimmed.parse::<f64>() {
        Ok(vf) if (0.5..=1.0).contains(&vf) => vf,
        _ => {
            println!("Invalid value. Using default 0.95.");
            0.95
        }
    }
}

fn prompt_wire_length_window() -> (f64, f64, UnitSystem) {
    print!("Constraint units for wire length window (m/ft, Enter for m): ");
    io::stdout().flush().expect("failed to flush stdout");

    let mut unit_input = String::new();
    io::stdin()
        .read_line(&mut unit_input)
        .expect("failed to read wire length window units");
    let unit = unit_input.trim().to_ascii_lowercase();

    if unit == "ft" || unit == "feet" {
        let default_min_ft = DEFAULT_NON_RESONANT_CONFIG.min_len_m / FEET_TO_METERS;
        let default_max_ft = DEFAULT_NON_RESONANT_CONFIG.max_len_m / FEET_TO_METERS;

        print!("Wire min length in feet (Enter for {:.1}): ", default_min_ft);
        io::stdout().flush().expect("failed to flush stdout");
        let mut min_input = String::new();
        io::stdin()
            .read_line(&mut min_input)
            .expect("failed to read wire min length");

        print!("Wire max length in feet (Enter for {:.1}): ", default_max_ft);
        io::stdout().flush().expect("failed to flush stdout");
        let mut max_input = String::new();
        io::stdin()
            .read_line(&mut max_input)
            .expect("failed to read wire max length");

        let min_ft = min_input.trim().parse::<f64>().ok().unwrap_or(default_min_ft);
        let max_ft = max_input.trim().parse::<f64>().ok().unwrap_or(default_max_ft);

        if min_ft > 0.0 && max_ft > min_ft {
            return (
                min_ft * FEET_TO_METERS,
                max_ft * FEET_TO_METERS,
                UnitSystem::Imperial,
            );
        }

        println!(
            "Invalid wire length window, using defaults {:.1}-{:.1} m.",
            DEFAULT_NON_RESONANT_CONFIG.min_len_m, DEFAULT_NON_RESONANT_CONFIG.max_len_m
        );
        return (
            DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            UnitSystem::Imperial,
        );
    }

    print!(
        "Wire min length in meters (Enter for {:.1}): ",
        DEFAULT_NON_RESONANT_CONFIG.min_len_m
    );
    io::stdout().flush().expect("failed to flush stdout");
    let mut min_input = String::new();
    io::stdin()
        .read_line(&mut min_input)
        .expect("failed to read wire min length");

    print!(
        "Wire max length in meters (Enter for {:.1}): ",
        DEFAULT_NON_RESONANT_CONFIG.max_len_m
    );
    io::stdout().flush().expect("failed to flush stdout");
    let mut max_input = String::new();
    io::stdin()
        .read_line(&mut max_input)
        .expect("failed to read wire max length");

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
        println!(
            "Invalid wire length window, using defaults {:.1}-{:.1} m.",
            DEFAULT_NON_RESONANT_CONFIG.min_len_m, DEFAULT_NON_RESONANT_CONFIG.max_len_m
        );
        (
            DEFAULT_NON_RESONANT_CONFIG.min_len_m,
            DEFAULT_NON_RESONANT_CONFIG.max_len_m,
            UnitSystem::Metric,
        )
    }
}

fn prompt_display_units(auto_units: UnitSystem) -> UnitSystem {
    let label = match auto_units {
        UnitSystem::Metric => "m",
        UnitSystem::Imperial => "ft",
        UnitSystem::Both => "both",
    };
    print!("Display units (m/ft/both, Enter for {}): ", label);
    io::stdout().flush().expect("failed to flush stdout");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("failed to read display units");
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return auto_units;
    }
    match parse_unit_system(trimmed) {
        Ok(u) => u,
        Err(_) => {
            println!("Unknown unit system. Using {}.", label);
            auto_units
        }
    }
}

fn interactive_export_prompt(results: &AppResults) -> Vec<(ExportFormat, String)> {
    print!("Export results? (none, or comma-separated formats e.g. csv,json,markdown,txt): ");
    io::stdout().flush().expect("failed to flush stdout");

    let mut fmt_raw = String::new();
    io::stdin()
        .read_line(&mut fmt_raw)
        .expect("failed to read export format");
    let fmt_raw = fmt_raw.trim().to_ascii_lowercase();

    if fmt_raw.is_empty() || fmt_raw == "none" {
        return Vec::new();
    }

    let formats = match parse_export_format_list(&fmt_raw) {
        Ok(f) => f,
        Err(e) => {
            println!("{}: skipping export.", e);
            return Vec::new();
        }
    };

    let mut chosen = Vec::new();
    for &fmt in &formats {
        let output = if formats.len() == 1 {
            print!("Output file (Enter for {}): ", default_output_name(fmt));
            io::stdout().flush().expect("failed to flush stdout");
            let mut output_raw = String::new();
            io::stdin()
                .read_line(&mut output_raw)
                .expect("failed to read output file");
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
            &output,
            &results.calculations,
            results.recommendation.as_ref(),
            results.config.units,
        ) {
            Ok(()) => println!("Exported results to {}", output),
            Err(err) => println!("Failed to export {}: {}", output, err),
        }
        chosen.push((fmt, output));
    }

    chosen
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
    for calc in calculations {
        println!("{}\n", format_calc(calc, units));
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
            if let Some(rec) = results.recommendation.as_ref() {
                let (min_m, max_m) = (results.config.wire_min_m, results.config.wire_max_m);
                let min_ft = min_m / FEET_TO_METERS;
                let max_ft = max_m / FEET_TO_METERS;
                println!("Optimum common wire length (non-resonant compromise):");
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
            }
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

fn print_non_resonant_recommendation(results: &AppResults) {
    let rec = match results.recommendation.as_ref() {
        Some(r) => r,
        None => return,
    };
    let optima = &results.optima;
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
                    idx + 1, o.length_m, o.min_resonance_clearance_pct
                ),
                UnitSystem::Imperial => println!(
                    "    {:2}. {:.2} ft (clearance: {:.2}%)",
                    idx + 1, o.length_ft, o.min_resonance_clearance_pct
                ),
                UnitSystem::Both => println!(
                    "    {:2}. {:.2} m ({:.2} ft, clearance: {:.2}%)",
                    idx + 1, o.length_m, o.length_ft, o.min_resonance_clearance_pct
                ),
            }
        }
        println!();
    }
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
        "rusty-wire --mode {} --bands {} --velocity {:.2} --units {}",
        match config.mode {
            CalcMode::Resonant => "resonant",
            CalcMode::NonResonant => "non-resonant",
        },
        bands_csv,
        config.velocity_factor,
        units_str,
    );

    if config.mode == CalcMode::NonResonant {
        cmd.push_str(&format!(
            " --wire-min {:.2} --wire-max {:.2}",
            config.wire_min_m, config.wire_max_m
        ));
    }

    if !export_choices.is_empty() {
        let fmts = export_choices
            .iter()
            .map(|(fmt, _)| fmt.as_str())
            .collect::<Vec<_>>()
            .join(",");
        cmd.push_str(&format!(" --export {}", fmts));
        if export_choices.len() == 1 {
            cmd.push_str(&format!(" --output {}", export_choices[0].1));
        }
    }

    println!("Equivalent CLI call for this run:");
    println!("  {}\n", cmd);
}

fn format_calc(c: &WireCalculation, units: UnitSystem) -> String {
    match units {
        UnitSystem::Metric => format!(
            "{}\n  Frequency: {:.3} MHz\n  Half-wave: {:.2} m\n  Full-wave: {:.2} m\n  Quarter-wave: {:.2} m\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
            c.band_name, c.frequency_mhz,
            c.half_wave_m, c.full_wave_m, c.quarter_wave_m,
            c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
        ),
        UnitSystem::Imperial => format!(
            "{}\n  Frequency: {:.3} MHz\n  Half-wave: {:.2} ft\n  Full-wave: {:.2} ft\n  Quarter-wave: {:.2} ft\n  Skip: {:.0}-{:.0} km (avg: {:.0} km)",
            c.band_name, c.frequency_mhz,
            c.half_wave_ft, c.full_wave_ft, c.quarter_wave_ft,
            c.skip_distance_min_km, c.skip_distance_max_km, c.skip_distance_avg_km,
        ),
        UnitSystem::Both => format!("{}", c),
    }
}

fn print_usage() {
    println!("rusty-wire v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("rusty-wire usage:");
    println!("  rusty-wire                  # interactive mode");
    println!("  rusty-wire --list-bands");
    println!("  rusty-wire [--mode resonant|non-resonant] [--bands 1,6,10] [--velocity 0.95] [--wire-min 8] [--wire-max 35] [--units m|ft|both] [--export csv,json,markdown,txt] [--output file]");
    println!("  rusty-wire [--mode resonant|non-resonant] [--bands 1,6,10] [--velocity 0.95] [--wire-min-ft 26] [--wire-max-ft 115] [--units m|ft|both] [--export csv,json,markdown,txt] [--output file]");
    println!("  (--export accepts a comma-separated list; --output applies only when a single format is selected)");
    println!("\nNotes:");
    println!("  - Band numbers come from --list-bands");
    println!("  - Default selected bands are 40m-10m: 4,5,6,7,8,9,10");
    println!("  - Velocity factor range is 0.50 to 1.00");
    println!("  - Default mode is resonant");
    println!("  - --wire-min/--wire-max (meters) defaults to metric-only output");
    println!("  - --wire-min-ft/--wire-max-ft (feet) defaults to imperial-only output");
    println!("  - Use --units both to include all units in output and exports");
}

// ---------------------------------------------------------------------------
// Argument parsers
// ---------------------------------------------------------------------------

fn parse_cli_args(args: &[String]) -> Result<CliOptions, String> {
    let mut opts = CliOptions {
        bands: None,
        velocity_factor: 0.95,
        export: Vec::new(),
        output: None,
        wire_min_m: DEFAULT_NON_RESONANT_CONFIG.min_len_m,
        wire_max_m: DEFAULT_NON_RESONANT_CONFIG.max_len_m,
        wire_min_ft: None,
        wire_max_ft: None,
        mode: CalcMode::Resonant,
        list_bands: false,
        help: false,
        units: None,
    };

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => opts.help = true,
            "--list-bands" => opts.list_bands = true,
            "--bands" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--bands requires a value".to_string())?;
                opts.bands = Some(parse_band_list(value)?);
            }
            "--velocity" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--velocity requires a value".to_string())?;
                let vf = value
                    .parse::<f64>()
                    .map_err(|_| "invalid value for --velocity".to_string())?;
                if !(0.5..=1.0).contains(&vf) {
                    return Err("--velocity must be between 0.50 and 1.00".to_string());
                }
                opts.velocity_factor = vf;
            }
            "--export" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--export requires a value".to_string())?;
                opts.export = parse_export_format_list(value)?;
            }
            "--output" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--output requires a value".to_string())?;
                opts.output = Some(value.to_string());
            }
            "--wire-min" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--wire-min requires a value".to_string())?;
                opts.wire_min_m = value
                    .parse::<f64>()
                    .map_err(|_| "invalid value for --wire-min".to_string())?;
            }
            "--wire-max" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--wire-max requires a value".to_string())?;
                opts.wire_max_m = value
                    .parse::<f64>()
                    .map_err(|_| "invalid value for --wire-max".to_string())?;
            }
            "--wire-min-ft" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--wire-min-ft requires a value".to_string())?;
                opts.wire_min_ft = Some(
                    value
                        .parse::<f64>()
                        .map_err(|_| "invalid value for --wire-min-ft".to_string())?,
                );
            }
            "--wire-max-ft" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--wire-max-ft requires a value".to_string())?;
                opts.wire_max_ft = Some(
                    value
                        .parse::<f64>()
                        .map_err(|_| "invalid value for --wire-max-ft".to_string())?,
                );
            }
            "--mode" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--mode requires a value".to_string())?;
                opts.mode = parse_calc_mode(value)?;
            }
            "--units" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| "--units requires a value".to_string())?;
                opts.units = Some(parse_unit_system(value)?);
            }
            unknown => return Err(format!("unknown argument: {}", unknown)),
        }
        i += 1;
    }

    let using_ft = opts.wire_min_ft.is_some() || opts.wire_max_ft.is_some();
    if using_ft {
        if opts.wire_min_ft.is_none() || opts.wire_max_ft.is_none() {
            return Err(
                "both --wire-min-ft and --wire-max-ft are required when using feet".to_string(),
            );
        }
        if opts.wire_min_m != DEFAULT_NON_RESONANT_CONFIG.min_len_m
            || opts.wire_max_m != DEFAULT_NON_RESONANT_CONFIG.max_len_m
        {
            return Err(
                "do not combine meter and feet constraints; choose one unit system".to_string(),
            );
        }

        let min_ft = opts.wire_min_ft.unwrap_or_default();
        let max_ft = opts.wire_max_ft.unwrap_or_default();
        if min_ft <= 0.0 {
            return Err("--wire-min-ft must be > 0".to_string());
        }
        if max_ft <= min_ft {
            return Err("--wire-max-ft must be greater than --wire-min-ft".to_string());
        }

        opts.wire_min_m = min_ft * FEET_TO_METERS;
        opts.wire_max_m = max_ft * FEET_TO_METERS;
    }

    if opts.mode == CalcMode::NonResonant {
        if opts.wire_min_m <= 0.0 {
            return Err("--wire-min must be > 0".to_string());
        }
        if opts.wire_max_m <= opts.wire_min_m {
            return Err("--wire-max must be greater than --wire-min".to_string());
        }
    }

    Ok(opts)
}

fn parse_band_list(raw: &str) -> Result<Vec<usize>, String> {
    let values: Result<Vec<usize>, _> = raw
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().parse::<usize>())
        .collect();
    let bands = values.map_err(|_| "invalid --bands list".to_string())?;
    if bands.is_empty() {
        return Err("--bands cannot be empty".to_string());
    }
    Ok(bands)
}

fn parse_export_format(raw: &str) -> Result<ExportFormat, String> {
    match raw.to_ascii_lowercase().as_str() {
        "csv" => Ok(ExportFormat::Csv),
        "json" => Ok(ExportFormat::Json),
        "markdown" | "md" => Ok(ExportFormat::Markdown),
        "txt" | "text" => Ok(ExportFormat::Txt),
        _ => Err(format!(
            "unknown export format '{}'; must be csv, json, markdown, or txt",
            raw
        )),
    }
}

fn parse_export_format_list(raw: &str) -> Result<Vec<ExportFormat>, String> {
    let mut formats: Vec<ExportFormat> = Vec::new();
    for token in raw.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let fmt = parse_export_format(token)?;
        if !formats.contains(&fmt) {
            formats.push(fmt);
        }
    }
    if formats.is_empty() {
        return Err("--export requires at least one format".to_string());
    }
    Ok(formats)
}

fn parse_calc_mode(raw: &str) -> Result<CalcMode, String> {
    match raw.to_ascii_lowercase().as_str() {
        "resonant" => Ok(CalcMode::Resonant),
        "non-resonant" | "nonresonant" | "non_resonant" => Ok(CalcMode::NonResonant),
        _ => Err("--mode must be resonant or non-resonant".to_string()),
    }
}

fn parse_unit_system(raw: &str) -> Result<UnitSystem, String> {
    match raw.to_ascii_lowercase().as_str() {
        "m" | "metric" => Ok(UnitSystem::Metric),
        "ft" | "imperial" => Ok(UnitSystem::Imperial),
        "both" => Ok(UnitSystem::Both),
        _ => Err("--units must be m, ft, or both".to_string()),
    }
}
