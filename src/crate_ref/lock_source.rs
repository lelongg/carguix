use crate::{
    crate_ref::{CrateRef, RegistrySource, SimpleSource},
    errors::CarguixError,
    guix, INDEX,
};
use crates_index::{Dependency as CrateDependency, Version as CrateVersion};
use heck::KebabCase;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    convert::TryFrom,
    error::Error,
    fs::canonicalize,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoLock {
    package: Vec<CargoLockPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoLockPackage {
    name: String,
    version: String,
    source: Option<String>,
    #[serde(default)]
    dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockSource {
    crate_name: String,
    version: String,
    package: CargoLockPackage,
    manifest: Box<CargoLock>,
}

impl LockSource {
    pub fn new(
        crate_name: &str,
        version: &Option<String>,
        path: impl AsRef<Path>,
    ) -> Result<Self, CarguixError> {
        let cargo_lock: CargoLock = toml::from_str(
            &std::fs::read_to_string(path).map_err(CarguixError::LockFileReadError)?,
        )
        .map_err(CarguixError::LockFileParsingError)?;
        LockSource::new_with_manifest(crate_name, version, Box::new(cargo_lock))
    }

    pub fn new_with_manifest(
        crate_name: &str,
        version: &Option<String>,
        manifest: Box<CargoLock>,
    ) -> Result<Self, CarguixError> {
        let package = manifest
            .package
            .iter()
            .find(|package| {
                package.name == crate_name
                    && version
                        .as_ref()
                        .map(|version| &package.version == version)
                        .unwrap_or(true)
            })
            .ok_or_else(|| {
                CarguixError::PackageNotFoundInLock(
                    crate_name.to_string(),
                    version
                        .as_ref()
                        .unwrap_or(&"any version".to_string())
                        .to_string(),
                )
            })?
            .clone();
        Ok(Self {
            crate_name: crate_name.to_string(),
            version: package.version.to_string(),
            package,
            manifest,
        })
    }
}

impl CrateRef for LockSource {
    fn crate_name(&self) -> String {
        self.crate_name.clone()
    }

    fn package_name(&self) -> String {
        format!("{}-{}", self.crate_name().to_kebab_case(), self.version())
    }

    fn version(&self) -> String {
        self.package.version.clone()
    }

    fn source(&self) -> String {
        if self.package.source.is_some() {
            format!(
                "https://crates.io/api/v1/crates/{}/{}/download",
                self.crate_name(),
                self.version()
            )
        } else {
            format!(
                "file://{}",
                std::env::current_dir()
                    .expect("cannot read current directory")
                    .to_string_lossy()
            )
        }
    }

    fn dependencies(&self) -> Result<Vec<Box<dyn CrateRef>>, CarguixError> {
        self.package
            .dependencies
            .iter()
            .map(|dependency| {
                let dependency_split = dependency.split(' ').collect::<Vec<_>>();
                let (crate_name, version) = match &*dependency_split {
                    [crate_name, version, _] => (crate_name, version),
                    _ => Err(CarguixError::BadLockFileDependency(dependency.to_string()))?,
                };
                Ok(Box::new(LockSource::new_with_manifest(
                    crate_name,
                    &Some(version.to_string()),
                    self.manifest.clone(),
                )?) as Box<dyn CrateRef>)
            })
            .collect()
    }
}
