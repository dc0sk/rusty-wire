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
pub mod sessions;
pub mod tui;

/// Run the command-line interface with the given argument list.
///
/// Returns `true` on success, `false` if any error prevented completion.
/// The binary uses this to drive the process exit code.
pub fn run_cli(args: &[String]) -> bool {
    cli::run_from_args(args)
}

/// Run the Text User Interface (TUI) with optional band preset config path.
///
/// Returns `Ok(())` on successful exit or user quit, `Err(msg)` on initialization failure.
/// The TUI auto-discovers `~/.config/rusty-wire/bands.toml` and `./bands.toml` for presets.
pub fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    tui::run(None)
}

/// Shared test utilities used by unit tests across multiple modules.
///
/// The ENV_MUTEX serialises tests that mutate the `HOME` environment variable
/// so that `UserPrefs` and `SessionStore` tests do not interfere with each other
/// when the full test suite runs with the default multi-threaded executor.
#[cfg(test)]
pub(crate) mod test_env {
    use std::sync::Mutex;

    pub static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// Run `f` with `HOME` temporarily redirected to a fresh `TempDir`.
    /// The mutex ensures only one test mutates HOME at a time.
    pub fn with_temp_home<F: FnOnce()>(f: F) {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::TempDir::new().expect("create temp dir");
        let old = std::env::var("HOME").ok();
        // SAFETY: serialised by ENV_MUTEX; no other thread mutates HOME concurrently.
        unsafe { std::env::set_var("HOME", tmp.path()) };
        f();
        unsafe {
            match old {
                Some(h) => std::env::set_var("HOME", h),
                None => std::env::remove_var("HOME"),
            }
        }
        drop(tmp);
    }
}
