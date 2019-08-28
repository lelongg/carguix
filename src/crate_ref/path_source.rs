use crate::{
    crate_ref::{registry_source::RegistrySource, CrateRef, CrateSource},
    errors::CarguixError,
    guix::{self, ToGuixPackage},
    INDEX,
};
use crates_index::{Dependency as CrateDependency, Version as CrateVersion};
use heck::KebabCase;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    error::Error,
    fs::canonicalize,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSource {
    path: PathBuf,
    package: cargo_toml::Package,
    manifest: cargo_toml::Manifest,
}

impl PathSource {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, CarguixError> {
        let path = path.as_ref();
        let manifest = cargo_toml::Manifest::from_path(path).map_err(|err| {
            CarguixError::ManifestParsingError(err, path.to_string_lossy().to_string())
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            package: manifest.package.clone().ok_or_else(|| {
                CarguixError::NoPackageInManifest(path.to_string_lossy().to_string())
            })?,
            manifest,
        })
    }

    pub fn crate_name(&self) -> String {
        self.package.name.clone()
    }

    pub fn package_name(&self) -> String {
        format!(
            "{}-{}",
            self.crate_name().to_kebab_case(),
            self.package.version
        )
    }

    pub fn version(&self) -> String {
        self.package.version.clone()
    }

    pub fn source(&self) -> String {
        format!(
            "file://{}",
            canonicalize(&self.path)
                .expect("cannot canonicalize path")
                .parent()
                .expect("not a file path")
                .to_string_lossy()
        )
    }

    pub fn dependencies(&self) -> Result<Vec<CrateRef>, CarguixError> {
        self.manifest
            .dependencies
            .iter()
            .chain(self.manifest.build_dependencies.iter())
            .map(|(name, dependency)| {
                let crate_name = dependency.package().unwrap_or(name);
                let source = if dependency.is_crates_io() {
                    CrateSource::Registry(RegistrySource::new_with_requirement(
                        crate_name,
                        dependency.req(),
                    )?)
                } else if dependency.git().is_some() {
                    unimplemented!()
                } else {
                    unimplemented!()
                };
                Ok(CrateRef::new(crate_name, &source))
            })
            .collect()
    }
}