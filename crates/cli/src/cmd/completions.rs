use std::io;

use anyhow::Result;
use clap::{Args, CommandFactory, ValueEnum};
use clap_complete::{generate, Shell};

use crate::Cli;

#[derive(Debug, Args)]
pub(crate) struct CompletionsArgs {
    #[arg(value_enum, help = "Shell to generate completions for")]
    pub shell: ShellChoice,
}

#[derive(Debug, Clone, ValueEnum)]
pub(crate) enum ShellChoice {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

impl From<ShellChoice> for Shell {
    fn from(choice: ShellChoice) -> Self {
        match choice {
            ShellChoice::Bash => Shell::Bash,
            ShellChoice::Zsh => Shell::Zsh,
            ShellChoice::Fish => Shell::Fish,
            ShellChoice::PowerShell => Shell::PowerShell,
            ShellChoice::Elvish => Shell::Elvish,
        }
    }
}

pub(crate) fn execute(args: CompletionsArgs) -> Result<()> {
    let shell: Shell = args.shell.into();
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "ht", &mut io::stdout());
    Ok(())
}
