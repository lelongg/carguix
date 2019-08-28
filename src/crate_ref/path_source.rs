use crate::{
    crate_ref::{registry_source::RegistrySource, CrateRef},
    errors::CarguixError,
};
use heck::KebabCase;
use serde::{Deserialize, Serialize};
use std::{
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
}

impl CrateRef for PathSource {
    fn crate_name(&self) -> String {
        self.package.name.clone()
    }

    fn package_name(&self) -> String {
        format!(
            "{}-{}",
            self.crate_name().to_kebab_case(),
            self.package.version
        )
    }

    fn version(&self) -> String {
        self.package.version.clone()
    }

    fn source(&self) -> String {
        format!(
            "file://{}",
            canonicalize(&self.path)
                .expect("cannot canonicalize path")
                .parent()
                .expect("not a file path")
                .to_string_lossy()
        )
    }

    fn dependencies(&self) -> Result<Vec<Box<dyn CrateRef>>, CarguixError> {
        self.manifest
            .dependencies
            .iter()
            .chain(self.manifest.build_dependencies.iter())
            .map(|(name, dependency)| {
                Ok(Box::new(if dependency.is_crates_io() {
                    RegistrySource::new_with_requirement(
                        dependency.package().unwrap_or(name),
                        dependency.req(),
                    )?
                } else if dependency.git().is_some() {
                    unimplemented!()
                } else {
                    unimplemented!()
                }) as Box<dyn CrateRef>)
            })
            .collect()
    }
}
