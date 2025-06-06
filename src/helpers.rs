use std::path::{Path, PathBuf};

pub trait PathTarget {
    fn target_release(&self, release: bool) -> PathBuf;
}

impl PathTarget for Path {
    fn target_release(&self, release: bool) -> PathBuf {
        self.join(if release { "release" } else { "debug" })
    }
}

impl PathTarget for PathBuf {
    fn target_release(&self, release: bool) -> PathBuf {
        self.join(if release { "release" } else { "debug" })
    }
}
