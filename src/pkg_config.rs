use pkg_config::Config;

pub trait ParsePkgVersion {
    fn parse_version(&mut self, pkg_version: &str) -> &mut Self;
}

impl ParsePkgVersion for Config {
    fn parse_version(&mut self, pkg_version: &str) -> &mut Self {
        if pkg_version.starts_with("=") {
            self.exactly_version(&pkg_version[1..]);
        } else if pkg_version.starts_with(">=") {
            self.atleast_version(&pkg_version[2..]);
        } else if pkg_version.contains("..") {
            let pkg_version_split: Vec<&str> = pkg_version.split("..").collect();

            if pkg_version_split.len() == 2 {
                self.range_version(pkg_version_split[0]..pkg_version_split[1]);
            }
        }

        self
    }
}
