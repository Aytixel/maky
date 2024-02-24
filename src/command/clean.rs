use std::{fs::remove_dir_all, io};

use crate::config::{LoadConfig, ProjectConfig};

use super::get_project_path;

pub fn clean(config_file: String) -> io::Result<()> {
    let (project_path, project_config_path) = &get_project_path(&config_file);

    match ProjectConfig::load(project_config_path) {
        Ok(project_config) => {
            remove_dir_all(project_path.join(project_config.binaries))?;
            remove_dir_all(project_path.join(project_config.objects))?;

            for os_specific_config in project_config.os_specific.values() {
                if let Some(binaries) = &os_specific_config.binaries {
                    remove_dir_all(project_path.join(binaries))?;
                }
                if let Some(objects) = &os_specific_config.objects {
                    remove_dir_all(project_path.join(objects))?;
                }
            }

            for arch_specific_config in project_config.arch_specific.values() {
                if let Some(binaries) = &arch_specific_config.binaries {
                    remove_dir_all(project_path.join(binaries))?;
                }
                if let Some(objects) = &arch_specific_config.objects {
                    remove_dir_all(project_path.join(objects))?;
                }
            }

            for feature_specific_config in project_config.feature_specific.values() {
                if let Some(binaries) = &feature_specific_config.binaries {
                    remove_dir_all(project_path.join(binaries))?;
                }
                if let Some(objects) = &feature_specific_config.objects {
                    remove_dir_all(project_path.join(objects))?;
                }
            }

            Ok(())
        }
        Err(error) => ProjectConfig::handle_error(error, project_config_path),
    }
}
