use std::{
    fs::{create_dir, create_dir_all, write},
    path::PathBuf,
};

use clap::Args;
use git2::Repository;

#[derive(Args, Debug, Clone)]
pub struct InitArguments {
    /// Folder to initialize
    #[arg(default_value = "./")]
    path: PathBuf,
}

pub fn init(arguments: InitArguments) -> anyhow::Result<()> {
    if !arguments.path.join("Maky.toml").exists() {
        create_dir_all(&arguments.path).ok();

        Repository::init(&arguments.path).ok();

        create_dir(arguments.path.join("src")).ok();
        write(arguments.path.join(".gitignore"), "/.maky\n/obj\n/bin").ok();
        write(
            arguments.path.join("Maky.toml"),
            "[package]\nversion = \"0.1.0\"",
        )
        .ok();
        write(arguments.path.join("src/main.c"), "#include <stdio.h>\n\nint main()\n{\n\tprintf(\"Hello world !\\n\");\n\n\treturn 0;\n}").ok();
    }

    Ok(())
}
