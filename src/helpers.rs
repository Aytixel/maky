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

pub trait TryStripPrefixPath {
    fn try_strip_prefix<P: AsRef<Path>>(&self, parent: P) -> Self;
}

impl TryStripPrefixPath for &Path {
    fn try_strip_prefix<P: AsRef<Path>>(&self, parent: P) -> Self {
        self.strip_prefix(parent).unwrap_or(self)
    }
}

impl TryStripPrefixPath for PathBuf {
    fn try_strip_prefix<P: AsRef<Path>>(&self, parent: P) -> Self {
        self.strip_prefix(parent).unwrap_or(self).to_path_buf()
    }
}
