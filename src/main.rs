mod command;
mod config;
mod file;
mod pkg_config;

use std::{io::stderr, path::PathBuf};

use clap::{command, ArgAction, Parser, Subcommand};
use command::{BuildFlags, FormatOptions};

use crate::command::{build, clean, format, init, run};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a project folder
    Init {
        /// Folder to initialize
        path: Option<PathBuf>,
    },

    /// Build files
    Build {
        /// Maky config file or folder
        #[arg(short = 'f', long = "file", default_value_t = ("./Maky.toml").to_string())]
        config_file: String,

        /// Build in release mode
        #[arg(long)]
        release: bool,

        /// Rebuild every time
        #[arg(long)]
        rebuild: bool,

        /// Enable a pretty display
        #[arg(long, action=ArgAction::Set, default_value_t = true)]
        pretty: bool,
    },

    /// Build files then run the specified file
    Run {
        /// Maky config file or folder
        #[arg(short = 'f', long = "file", default_value_t = ("./Maky.toml").to_string())]
        config_file: String,

        /// Build in release mode
        #[arg(long)]
        release: bool,

        /// Rebuild every time
        #[arg(long)]
        rebuild: bool,

        /// Path of the source file to build and run
        file: PathBuf,

        /// Arguments for the source file to run
        args: Vec<String>,
    },

    /// [Experimental] Formats all bin and lib files of the current project
    Fmt {
        /// Files to format
        files: Vec<String>,

        /// Maky config file or folder
        #[arg(short = 'f', long = "file", default_value_t = ("./Maky.toml").to_string())]
        config_file: String,

        /// Tabulation size in spaces
        #[arg(short = 't', long = "tab", default_value_t = 4)]
        tab_size: usize,
    },

    /// Remove artifacts generated by Maky in the past
    Clean {
        /// Maky config file or folder
        #[arg(short = 'f', long = "file", default_value_t = ("./Maky.toml").to_string())]
        config_file: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    kdam::term::init(true);

    let args = Args::parse();

    if let Some(command) = args.command {
        match command {
            Commands::Init { path } => init(path)?,
            Commands::Build {
                config_file,
                release,
                rebuild,
                pretty,
            } => build(
                config_file,
                &BuildFlags {
                    release,
                    rebuild,
                    pretty,
                },
                &mut stderr(),
            )?,
            Commands::Run {
                config_file,
                release,
                rebuild,
                file,
                args,
            } => run(config_file, release, rebuild, file, args)?,
            Commands::Fmt {
                files,
                config_file,
                tab_size,
            } => {
                format(
                    files,
                    config_file,
                    &FormatOptions {
                        tab: " ".repeat(tab_size).to_string(),
                    },
                )
                .await?
            }
            Commands::Clean { config_file } => clean(config_file)?,
        }
    }

    return Ok(());
}
