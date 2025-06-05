use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;

pub use crate::config::package::Package;
use crate::config::target::Target;

mod package;
mod target;

#[serde_inline_default]
#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    pub package: Option<Package>,

    #[serde_inline_default(Vec::new())]
    #[serde(rename = "bin")]
    pub binaries: Vec<Target>,
}
