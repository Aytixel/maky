use serde::{Deserialize, Serialize};

pub use crate::config::package::Package;

mod package;

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    pub package: Option<Package>,
}
