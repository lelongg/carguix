use crate::{
    crate_ref::{lock_source::LockSource, CrateRef},
    errors::CarguixError,
};
use heck::KebabCase;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSource {
    path: PathBuf,
    package: cargo_toml::Package,
    manifest: cargo_toml::Manifest,
    lock_source: Option<LockSource>,
    crate_paths: HashMap<String, PathBuf>,
}

impl PathSource {
    pub fn new(
        path: impl AsRef<Path>,
        crate_paths: &HashMap<String, PathBuf>,
    ) -> Result<Self, CarguixError> {
        let path = path.as_ref().canonicalize().map_err(|err| {
            CarguixError::CanonicalizationFailed(err, path.as_ref().to_string_lossy().to_string())
        })?;
        let mut cargo_toml_path = path.to_path_buf();
        cargo_toml_path.push("Cargo.toml");
        let manifest = cargo_toml::Manifest::from_path(cargo_toml_path.clone()).map_err(|err| {
            CarguixError::ManifestParsingError(err, cargo_toml_path.to_string_lossy().to_string())
        })?;
        Self::new_with_manifest(path, &manifest, crate_paths)
    }

    pub fn new_with_manifest(
        path: impl AsRef<Path>,
        manifest: &cargo_toml::Manifest,
        crate_paths: &HashMap<String, PathBuf>,
    ) -> Result<Self, CarguixError> {
        let path = path.as_ref();
        let package = manifest
            .package
            .clone()
            .ok_or_else(|| CarguixError::NoPackageInManifest(path.to_string_lossy().to_string()))?;
        let lock_source = Self::find_cargo_lock(path)
            .map(|lockfile_path| LockSource::new(&package.name, &None, lockfile_path))
            .transpose()?;
        let mut crate_paths = crate_paths.clone();
        crate_paths.extend(
            manifest
                .dependencies
                .iter()
                .chain(manifest.build_dependencies.iter())
                .chain(manifest.target.iter().flat_map(|(_, target)| {
                    target
                        .dependencies
                        .iter()
                        .chain(target.build_dependencies.iter())
                }))
                .chain(
                    manifest
                        .patch
                        .values()
                        .flat_map(|dependencies| dependencies.iter()),
                )
                .filter_map(|(name, dependency)| {
                    dbg!(name);
                    dependency
                        .detail()
                        .and_then(|detail| detail.path.as_ref())
                        .map(|crate_path| {
                            (name.clone(), [path, Path::new(crate_path)].iter().collect())
                        })
                }),
        );
        Ok(Self {
            path: path.to_path_buf(),
            package,
            manifest: manifest.clone(),
            lock_source,
            crate_paths: dbg!(crate_paths),
        })
    }

    pub fn find_cargo_lock(path: impl AsRef<Path>) -> Option<PathBuf> {
        let mut current = path.as_ref().to_path_buf();
        loop {
            log::debug!("looking for Cargo.lock in {:?}", current);
            current.push("Cargo.lock");
            if std::fs::metadata(&current).is_ok() {
                log::debug!("Cargo.lock found at {:?}", current);
                return Some(current);
            }
            current.pop();
            if !current.pop() {
                return None;
            }
        }
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
        format!("file://{}", self.path.to_string_lossy())
    }

    fn dependencies(&self) -> Result<Vec<Box<dyn CrateRef>>, CarguixError> {
        if let Some(lock_source) = &self.lock_source {
            lock_source
                .package
                .dependencies
                .iter()
                .map(|dependency| {
                    let dependency_split = dependency.split(' ').collect::<Vec<_>>();
                    Ok(match &*dependency_split {
                        [crate_name, version, _] => Box::new(LockSource::new_with_manifest(
                            crate_name,
                            &Some(version.to_string()),
                            lock_source.manifest.clone(),
                            &self.crate_paths,
                        )?)
                            as Box<dyn CrateRef>,
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
        } else {
            unimplemented!()
        }
    }
}
