use std::fs::remove_dir_all;

use crate::config::{DependencyConfig, ProjectConfig};

use super::get_project_path;

pub fn clean(config_file: String) -> anyhow::Result<()> {
    let (project_path, project_config_path) = &get_project_path(&config_file);

    match ProjectConfig::load(project_config_path) {
        Ok(project_config) => {
            remove_dir_all(project_path.join(".maky")).ok();
            remove_dir_all(project_path.join(project_config.binaries)).ok();
            remove_dir_all(project_path.join(project_config.objects)).ok();

            for os_specific_config in project_config.os_specific.values() {
                if let Some(binaries) = &os_specific_config.binaries {
                    remove_dir_all(project_path.join(binaries)).ok();
                }
                if let Some(objects) = &os_specific_config.objects {
                    remove_dir_all(project_path.join(objects)).ok();
                }
            }

            for arch_specific_config in project_config.arch_specific.values() {
                if let Some(binaries) = &arch_specific_config.binaries {
                    remove_dir_all(project_path.join(binaries)).ok();
                }
                if let Some(objects) = &arch_specific_config.objects {
                    remove_dir_all(project_path.join(objects)).ok();
                }
            }

            for feature_specific_config in project_config.feature_specific.values() {
                if let Some(binaries) = &feature_specific_config.binaries {
                    remove_dir_all(project_path.join(binaries)).ok();
                }
                if let Some(objects) = &feature_specific_config.objects {
                    remove_dir_all(project_path.join(objects)).ok();
                }
            }

            for dependency_config in project_config.dependencies.values() {
                let DependencyConfig::Local { path } = dependency_config else {
                    continue;
                };

                clean(project_path.join(path).to_string_lossy().to_string())?;
            }
        }
        Err(error) => ProjectConfig::handle_error(error, project_config_path)?,
    }

    Ok(())
}
