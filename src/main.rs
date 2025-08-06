use clap::Parser;
use cupcake::{
    cli::commands::{
        init::InitCommand, inspect::InspectCommand, run::RunCommand, sync::SyncCommand,
        validate::ValidateCommand, CommandHandler,
    },
    cli::{Cli, Commands},
    Result,
};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() -> Result<()> {
    // Initialize tracing based on RUST_LOG env var (only if set)
    if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_target(false)
                    .with_file(false)
                    .with_writer(std::io::stderr)
            )
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new("cupcake=info")),
            )
            .init();
    }
    
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            output,
            yes,
            verbose,
        } => {
            let command = InitCommand::new(output, yes, verbose);
            command.execute()?;
        }
        Commands::Run {
            event,
            config,
            debug,
        } => {
            let command = RunCommand::new(event, config, debug);
            command.execute()?;
        }
        Commands::Sync {
            settings_path,
            dry_run,
            force,
        } => {
            let command = SyncCommand::new(settings_path, dry_run, force);
            command.execute()?;
        }
        Commands::Validate {
            policy_file,
            strict,
            format,
        } => {
            let command = ValidateCommand::new(policy_file, strict, format);
            command.execute()?;
        }
        Commands::Inspect { config } => {
            let command = InspectCommand::new(config);
            command.execute()?;
        }
    }

    Ok(())
}
