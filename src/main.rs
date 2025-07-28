use clap::Parser;
use cupcake::{
    cli::commands::{
        init::InitCommand, inspect::InspectCommand, run::RunCommand, 
        sync::SyncCommand, validate::ValidateCommand, CommandHandler,
    },
    cli::{Cli, Commands},
    Result,
};

fn main() -> Result<()> {
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
