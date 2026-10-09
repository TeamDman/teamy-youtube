#![deny(clippy::disallowed_methods)]
#![deny(clippy::disallowed_macros)]

pub mod channel_sync;
pub mod cli;
pub mod elapsed_duration;
pub mod fs_db;
pub mod logging_init;
mod mft_query;
pub mod paths;
pub mod sync_progress;
pub mod takeout;
pub mod takeout_zip;
pub mod youtube_api;

use crate::cli::Cli;

/// Version string combining package version and git revision.
const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (rev ",
    env!("GIT_REVISION"),
    ")"
);

/// Entrypoint for the program.
///
/// # Errors
///
/// This function will return an error if `color_eyre` installation, CLI parsing, logging initialization, command execution, or command output rendering fails.
///
/// # Panics
///
/// Panics if the CLI schema is invalid (should never happen with correct code).
pub fn main() -> eyre::Result<()> {
    // Install color_eyre for better error reports
    color_eyre::install()?;

    #[cfg(windows)]
    {
        // Enable ANSI support on Windows
        // This fails in a pipe scenario, so we ignore the error
        let _ = teamy_windows::console::enable_ansi_support();

        // Warn if UTF-8 is not enabled on Windows
        #[cfg(windows)]
        teamy_windows::string::warn_if_utf8_not_enabled();
    };

    // Parse command line arguments using figue
    // unwrap() is figue's intended CLI entry behavior:
    // it exits with proper codes for --help/--version/completions/parse-errors.
    let cli: Cli = figue::Driver::new(
        figue::builder::<Cli>()
            .expect("schema should be valid")
            .cli(move |cli| cli.args_os(std::env::args_os().skip(1)).strict())
            .help(move |help| {
                help.version(VERSION)
                    .include_implementation_source_file(true)
                    .include_implementation_git_url("TeamDman/teamy-youtube", env!("GIT_REVISION"))
            })
            .build(),
    )
    .run()
    .unwrap();

    // Initialize logging
    logging_init::init_logging(&cli.global_args)?;

    // Invoke whatever command was requested and render its output once at the top level
    let requested_output_format = cli.global_args.output_format;
    let command_name = cli.command.name();
    let started = std::time::Instant::now();
    tracing::debug!(command = command_name, "Dispatching command");
    let result = (|| -> eyre::Result<()> {
        let _command_span = tracing::info_span!("command", command = command_name).entered();
        let output = cli.invoke()?;
        let threshold = output.elapsed_duration_warn_threshold;
        let result = output.emit(requested_output_format);
        elapsed_duration::warn_if_exceeded(command_name, started.elapsed(), threshold);
        result
    })();
    tracing::debug!(
        command = command_name,
        success = result.is_ok(),
        "Command finished"
    );
    result
}
