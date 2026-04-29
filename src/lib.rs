/// Rusty Wire library entry point.
///
/// Exposes the I/O-free application layer so that front-ends other than the
/// bundled CLI (e.g. a future `iced` GUI or a test harness) can depend on
/// this crate without pulling in CLI-specific logic.
pub mod app;
pub(crate) mod band_presets;
pub mod bands;
pub mod calculations;
pub(crate) mod cli;
pub(crate) mod export;
pub(crate) mod fnec_validation;
pub mod prefs;
pub mod tui;

/// Run the command-line interface with the given argument list.
///
/// Returns `true` on success, `false` if any error prevented completion.
/// The binary uses this to drive the process exit code.
pub fn run_cli(args: &[String]) -> bool {
    cli::run_from_args(args)
}
