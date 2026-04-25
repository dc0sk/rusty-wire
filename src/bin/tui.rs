use std::process;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let band_preset_config = match parse_band_config_arg(&args) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("Error: {err}");
            process::exit(2);
        }
    };

    if let Err(err) = rusty_wire::tui::run(band_preset_config.as_deref()) {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}

fn parse_band_config_arg(args: &[String]) -> Result<Option<String>, String> {
    let mut index = 0;
    let mut band_config = None;

    while index < args.len() {
        match args[index].as_str() {
            "--bands-config" => {
                let Some(path) = args.get(index + 1) else {
                    return Err("--bands-config requires a path argument".to_string());
                };
                band_config = Some(path.clone());
                index += 2;
            }
            "-h" | "--help" => {
                print_help();
                process::exit(0);
            }
            other => {
                return Err(format!(
                    "unknown TUI argument '{other}'. Supported: --bands-config <path>"
                ));
            }
        }
    }

    Ok(band_config)
}

fn print_help() {
    println!("Rusty Wire TUI\n\nUsage:\n  rusty-wire-tui [--bands-config <path>]\n\nOptions:\n  --bands-config <path>  Load named TUI band presets from an alternate TOML file\n  -h, --help             Show this help text");
}

#[cfg(test)]
mod tests {
    use super::parse_band_config_arg;

    #[test]
    fn parse_band_config_accepts_explicit_path() {
        let args = vec![
            "--bands-config".to_string(),
            "./profiles/bands.toml".to_string(),
        ];
        let parsed = parse_band_config_arg(&args).expect("flag should parse");
        assert_eq!(parsed.as_deref(), Some("./profiles/bands.toml"));
    }

    #[test]
    fn parse_band_config_rejects_missing_path() {
        let args = vec!["--bands-config".to_string()];
        let err = parse_band_config_arg(&args).expect_err("missing value should fail");
        assert!(err.contains("requires a path"));
    }

    #[test]
    fn parse_band_config_rejects_unknown_flag() {
        let args = vec!["--wat".to_string()];
        let err = parse_band_config_arg(&args).expect_err("unknown flag should fail");
        assert!(err.contains("unknown TUI argument"));
    }
}
