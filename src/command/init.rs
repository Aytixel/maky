use std::{
    fs::{create_dir, create_dir_all, write},
    path::{Path, PathBuf},
};

use git2::Repository;

pub fn init(path: Option<PathBuf>) -> anyhow::Result<()> {
    let project_path = path.unwrap_or(Path::new("./").to_path_buf());

    if !project_path.join("Maky.toml").exists() {
        create_dir_all(&project_path).ok();

        Repository::init(&project_path).ok();

        create_dir(project_path.join("src")).ok();
        write(project_path.join(".gitignore"), "/.maky\n/obj\n/bin").ok();
        write(
            project_path.join("Maky.toml"),
            "[package]\nversion = \"0.1.0\"",
        )
        .ok();
        write(project_path.join("src/main.c"), "#include <stdio.h>\n\nint main()\n{\n\tprintf(\"Hello world !\\n\");\n\n\treturn 0;\n}").ok();
    }

    Ok(())
}
