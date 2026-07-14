use std::{
    fs::{read_to_string, write},
    path::{Path, PathBuf},
};

use git2::Repository;
use parse_git_url::GitUrl;

#[derive(Debug)]
pub struct GitRepositoryInfo {
    pub url: GitUrl,
    pub rev: Option<String>,
    pub path: PathBuf,
}

pub fn pull(git: &GitRepositoryInfo, update: bool) -> anyhow::Result<()> {
    let repository = if git.path.is_dir() {
        Repository::open(&git.path)?
    } else {
        Repository::clone_recurse(&git.url.to_string(), &git.path)?
    };

    let default_branch = fetch_default_branch(&repository, &git.path, update)?;
    let rev = git.rev.clone().unwrap_or(default_branch);

    let (object, reference) = repository.revparse_ext(&rev)?;

    repository.checkout_tree(&object, None)?;

    if let Some(mut reference) = reference {
        let fetch_head = if update {
            &repository.find_reference("FETCH_HEAD")?
        } else {
            &reference
        };
        let fetch_commit = repository.reference_to_annotated_commit(fetch_head)?;
        let analysis = repository.merge_analysis(&[&fetch_commit])?;

        if analysis.0.is_up_to_date() {
            repository.set_head(reference.name().unwrap())?;
        } else if analysis.0.is_fast_forward() {
            reference.set_target(fetch_commit.id(), "Fast-Forward")?;
            repository.set_head(reference.name().unwrap())?;
            repository.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        } else {
            return Err(anyhow::anyhow!("git can't fast-forward"));
        }
    } else {
        repository.set_head_detached(object.id())?;
    }

    Ok(())
}

fn fetch_default_branch(
    repository: &Repository,
    project_path: &Path,
    update: bool,
) -> anyhow::Result<String> {
    let default_branch_path = project_path.join(".git/default_branch");

    if update || !default_branch_path.exists() {
        let mut remote = repository.find_remote("origin")?;

        remote.fetch(&[] as &[&str], None, None)?;

        let default_branch = remote
            .default_branch()?
            .as_str()
            .unwrap_or("main")
            .to_string();

        write(&default_branch_path, &default_branch)?;

        Ok(default_branch)
    } else {
        Ok(read_to_string(default_branch_path)?)
    }
}
