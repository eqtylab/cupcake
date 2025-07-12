use clap::Parser;
use cupcake::{
    cli::commands::{
        audit::AuditCommand, init::InitCommand, run::RunCommand, sync::SyncCommand,
        validate::ValidateCommand, CommandHandler,
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
            timeout,
            config,
            debug,
        } => {
            let command = RunCommand::new(event, timeout, config, debug);
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
        Commands::Audit {
            tail,
            follow,
            session,
            event,
            format,
            clear,
        } => {
            let command = AuditCommand::new(tail, follow, session, event, format, clear);
            command.execute()?;
        }
    }

    Ok(())
}
