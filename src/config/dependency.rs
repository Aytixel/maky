use std::path::PathBuf;

use semver::VersionReq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum DependencyConfig {
    Local {
        #[serde(serialize_with = "DependencyConfig::serialize_version_req")]
        #[serde(deserialize_with = "DependencyConfig::deserialize_version_req")]
        #[serde(default)]
        version: Option<VersionReq>,
        path: PathBuf,
    },
    Git {
        #[serde(serialize_with = "DependencyConfig::serialize_version_req")]
        #[serde(deserialize_with = "DependencyConfig::deserialize_version_req")]
        #[serde(default)]
        version: Option<VersionReq>,
        git: String,
        rev: Option<String>,
    },
}

impl DependencyConfig {
    fn serialize_version_req<S>(
        version_req: &Option<VersionReq>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(version_req) = version_req {
            serializer.serialize_some(&version_req.to_string())
        } else {
            serializer.serialize_none()
        }
    }

    fn deserialize_version_req<'de, D>(deserializer: D) -> Result<Option<VersionReq>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let version_req = Option::<String>::deserialize(deserializer)?;

        Ok(version_req.map(|version_req| {
            VersionReq::parse(&version_req).expect("Malformed version requirement in config")
        }))
    }
}
