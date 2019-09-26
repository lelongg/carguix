use crate::{
    crate_ref::{CrateRef, PathSource},
    errors::CarguixError,
};
use heck::KebabCase;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoLock {
    package: Vec<CargoLockPackage>,
}

impl CargoLock {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, CarguixError> {
        toml::from_str(&std::fs::read_to_string(path).map_err(CarguixError::LockFileReadError)?)
            .map_err(CarguixError::LockFileParsingError)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoLockPackage {
    pub name: String,
    pub version: String,
    pub source: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockSource {
    pub crate_name: String,
    pub version: String,
    pub package: CargoLockPackage,
    pub manifest: Box<CargoLock>,
    pub crate_paths: HashMap<String, PathBuf>,
}

impl LockSource {
    pub fn new(
        crate_name: &str,
        version: &Option<String>,
        path: impl AsRef<Path>,
    ) -> Result<Self, CarguixError> {
        LockSource::new_with_manifest(
            crate_name,
            version,
            Box::new(CargoLock::from_path(path)?),
            &HashMap::new(),
        )
    }

    pub fn new_with_manifest(
        crate_name: &str,
        version: &Option<String>,
        manifest: Box<CargoLock>,
        crate_paths: &HashMap<String, PathBuf>,
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
            crate_paths: crate_paths.clone(),
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
                Ok(match &*dependency_split {
                    [crate_name, version, _] => Box::new(LockSource::new_with_manifest(
                        crate_name,
                        &Some(version.to_string()),
                        self.manifest.clone(),
                        &self.crate_paths,
                    )?) as Box<dyn CrateRef>,
                    [crate_name, _] => Box::new(PathSource::new(
                        self.crate_paths
                            .get(&crate_name.to_string())
                            .unwrap_or_else(|| {
                                panic!(
                                    "dependency {} of {} path not found in {:?}",
                                    crate_name,
                                    self.crate_name(),
                                    self.crate_paths,
                                )
                            }),
                        &self.crate_paths,
                    )?) as Box<dyn CrateRef>,
                    _ => Err(CarguixError::BadLockFileDependency(dependency.to_string()))?,
                })
            })
            .collect()
    }
}
