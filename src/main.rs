//! Command-line frontend for validating and exporting process-name rules.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use process_rules::{
    Catalog, Collection, Diagnostic, Error, PlatformSelection, ValidationOutcome, build_all,
    load_catalog, render_mihomo, write_atomic_file,
};

#[derive(Debug, Parser)]
#[command(
    name = "process-rules",
    version,
    about = "Validate and export cross-platform process-name rules"
)]
struct Cli {
    /// Directory containing categorized TOML rule files.
    #[arg(long, global = true, default_value = "rules")]
    rules_dir: PathBuf,

    /// How source validation diagnostics are written to stderr.
    #[arg(long, global = true, value_enum, default_value_t = DiagnosticFormat::Human)]
    diagnostic_format: DiagnosticFormat,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate every source file without generating output.
    Validate {
        /// Reject a repository containing no applications; intended for publishing gates.
        #[arg(long)]
        require_non_empty: bool,
    },

    /// Build one Mihomo classical rule-provider artifact.
    Build {
        /// Concrete OS or the deduplicated union of every OS.
        #[arg(long)]
        platform: PlatformSelection,

        /// Derived set: any, only-<region>, contain-<region>, or not-contain-<region>.
        #[arg(long)]
        collection: String,

        /// Destination file. Omit it or pass `-` to write only artifact data to stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Build the complete Mihomo tree through a staging directory.
    BuildAll {
        /// Destination directory, which must be absent or empty.
        #[arg(long, default_value = "dist")]
        out_dir: PathBuf,

        /// Reject a repository containing no applications; intended for publishing gates.
        #[arg(long)]
        require_non_empty: bool,
    },

    /// Show which source application and collections match an exact process name.
    Explain {
        /// Concrete OS or all OS source fields.
        #[arg(long)]
        platform: PlatformSelection,

        /// Process basename or Android package name, matched case-insensitively.
        #[arg(long)]
        process: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DiagnosticFormat {
    Human,
    Json,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error @ Error::ValidationFailed { .. }) => ExitCode::from(error.exit_code()),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

fn run(cli: Cli) -> Result<(), Error> {
    match cli.command {
        Command::Validate { require_non_empty } => {
            validate_command(&cli.rules_dir, cli.diagnostic_format, require_non_empty)
        }
        Command::Build {
            platform,
            collection,
            output,
        } => {
            let catalog = validated_catalog(&cli.rules_dir, cli.diagnostic_format, false)?;
            let collection = collection
                .parse::<Collection>()
                .map_err(|_| Error::InvalidCollection(collection))?;
            let names = catalog
                .select(platform, &collection)
                .ok_or_else(|| Error::UnavailableCollection(collection.name()))?;
            let rendered = render_mihomo(&names);
            match output.as_deref() {
                Some(path) if path != Path::new("-") => {
                    write_atomic_file(path, rendered.as_bytes())?;
                    println!("built {} rule(s) into {}", names.len(), path.display());
                    Ok(())
                }
                _ => write_stdout(rendered.as_bytes()),
            }
        }
        Command::BuildAll {
            out_dir,
            require_non_empty,
        } => {
            let catalog =
                validated_catalog(&cli.rules_dir, cli.diagnostic_format, require_non_empty)?;
            let manifest = build_all(&catalog, &out_dir)?;
            println!(
                "built {} artifact(s) from {} application(s) into {}",
                manifest.artifacts.len(),
                manifest.applications,
                out_dir.display()
            );
            Ok(())
        }
        Command::Explain { platform, process } => {
            let catalog = validated_catalog(&cli.rules_dir, cli.diagnostic_format, false)?;
            let explanations = catalog.explain(platform, &process);
            if explanations.is_empty() {
                return Err(Error::NotFound {
                    process,
                    platform: platform.to_string(),
                });
            }
            for (index, explanation) in explanations.iter().enumerate() {
                if index > 0 {
                    println!();
                }
                println!("application: {}", explanation.application);
                if let Some(display_name) = &explanation.display_name {
                    println!("display-name: {display_name}");
                }
                println!("category: {}", explanation.category);
                println!("platform: {}", explanation.platform);
                println!("process: {}", explanation.process);
                println!("regions: {}", explanation.regions);
                println!("collections: {}", explanation.collections.join(", "));
            }
            Ok(())
        }
    }
}

fn validate_command(
    rules_dir: &Path,
    format: DiagnosticFormat,
    require_non_empty: bool,
) -> Result<(), Error> {
    let outcome = load_and_report(rules_dir, format, require_non_empty)?;
    let errors = outcome.error_count();
    let warnings = outcome.warning_count();
    let applications = outcome.catalog().map_or(0, Catalog::application_count);
    if errors > 0 {
        return Err(Error::ValidationFailed { errors });
    }
    println!("valid: {applications} application(s), {warnings} warning(s)");
    Ok(())
}

fn validated_catalog(
    rules_dir: &Path,
    format: DiagnosticFormat,
    require_non_empty: bool,
) -> Result<Catalog, Error> {
    load_and_report(rules_dir, format, require_non_empty)?.into_catalog()
}

fn load_and_report(
    rules_dir: &Path,
    format: DiagnosticFormat,
    require_non_empty: bool,
) -> Result<ValidationOutcome, Error> {
    let outcome = load_catalog(rules_dir, require_non_empty);
    emit_diagnostics(outcome.diagnostics(), format)?;
    Ok(outcome)
}

fn emit_diagnostics(diagnostics: &[Diagnostic], format: DiagnosticFormat) -> Result<(), Error> {
    if diagnostics.is_empty() {
        return Ok(());
    }
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    match format {
        DiagnosticFormat::Human => {
            for diagnostic in diagnostics {
                writeln!(stderr, "{diagnostic}")
                    .map_err(|error| stream_error("write diagnostics", "<stderr>", error))?;
            }
        }
        DiagnosticFormat::Json => {
            serde_json::to_writer_pretty(&mut stderr, diagnostics)?;
            writeln!(stderr)
                .map_err(|error| stream_error("write diagnostics", "<stderr>", error))?;
        }
    }
    Ok(())
}

fn write_stdout(bytes: &[u8]) -> Result<(), Error> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    stdout
        .write_all(bytes)
        .map_err(|error| stream_error("write artifact", "<stdout>", error))?;
    stdout
        .flush()
        .map_err(|error| stream_error("flush artifact", "<stdout>", error))
}

fn stream_error(operation: &'static str, path: &str, source: io::Error) -> Error {
    Error::Io {
        operation,
        path: PathBuf::from(path),
        source,
    }
}
