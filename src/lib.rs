//! # Rusty Wire
//!
//! Wire-antenna length planning for ham radio, exposed as a library so front-ends
//! other than the bundled CLI (e.g. an `iced` GUI, a bot, or a test harness) can
//! reuse the I/O-free application layer.
//!
//! ## Getting started
//!
//! Build an [`AppConfig`], run it, and read the per-band results:
//!
//! ```
//! use rusty_wire::prelude::*;
//!
//! let config = AppConfig::default();
//! let results = run_calculation(config);
//! for calc in &results.calculations {
//!     println!("{}: half-wave {:.2} m", calc.band_name, calc.half_wave_m);
//! }
//!
//! // Or render the same document the CLI/TUI print:
//! let doc = results_display_document(&results);
//! assert!(!doc.band_views.is_empty());
//! ```
//!
//! Use [`run_calculation_checked`] instead of [`run_calculation`] to validate the
//! configuration first (returns an [`AppError`] on invalid input).
//!
//! ## Public API and stability
//!
//! The **stable, semver-tracked** surface is re-exported from [`prelude`]. Prefer
//! importing from there.
//!
//! The [`app`], [`bands`], [`calculations`], [`prefs`], and [`sessions`] modules
//! are public for advanced use, but items *not* re-exported from [`prelude`] may
//! change in minor releases. The [`tui`] module and the `run_tui` entry point are
//! provided for embedding the terminal UI but are not part of the stable API. The
//! CLI, export, NEC-export, and fnec-validation modules are private implementation
//! details.
//!
//! Semantic-versioning policy: breaking changes to `prelude` items bump the major
//! version; additive changes bump the minor version.

pub mod app;
pub(crate) mod band_presets;
pub mod bands;
pub mod calculations;
pub(crate) mod cli;
pub(crate) mod export;
pub(crate) mod fnec_validation;
pub(crate) mod nec_export;
pub mod prefs;
pub mod sessions;
pub mod tui;

/// The stable, semver-tracked public API. Import everything with
/// `use rusty_wire::prelude::*;`.
///
/// Anything re-exported here is covered by the crate's semver policy; other
/// public items may change in minor releases.
pub mod prelude {
    #[doc(no_inline)]
    pub use crate::app::{
        results_display_document, run_calculation, run_calculation_checked, AppConfig, AppError,
        AppRequest, AppResponse, AppResults, CalcMode, ResultsDisplayDocument,
    };
    #[doc(no_inline)]
    pub use crate::bands::{Band, BandType, ITURegion};
    #[doc(no_inline)]
    pub use crate::calculations::{
        GroundClass, ImpedanceClass, NonResonantRecommendation, NonResonantSearchConfig,
        OcfdSplitRecommendation, ResonantCompromise, TransformerRatio, WireCalculation,
    };
}

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
