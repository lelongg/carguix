mod git_source;
mod lock_source;
mod path_source;
mod registry_source;
mod simple_source;

use crate::crate_ref::lock_source::CargoLock;
use crate::{errors::CarguixError, guix, INDEX};
use crates_index::{Dependency as CrateDependency, Version as CrateVersion};
pub use git_source::GitSource;
use heck::KebabCase;
pub use lock_source::LockSource;
pub use path_source::PathSource;
pub use registry_source::RegistrySource;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
pub use simple_source::SimpleSource;
use std::{
    convert::TryFrom,
    error::Error,
    fs::canonicalize,
    path::{Path, PathBuf},
};

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct CrateRef {
//     crate_name: String,
//     source: CrateSource,
// }

// #[allow(clippy::large_enum_variant)]
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum CrateSource {
//     Path(PathSource),
//     Lock(LockSource),
//     Git(GitSource),
//     Registry(RegistrySource),
//     Simple(SimpleSource),
// }

// impl CrateRef {
//     pub fn new(crate_name: &str, source: &CrateSource) -> Self {
//         Self {
//             crate_name: crate_name.to_string(),
//             source: source.clone(),
//         }
//     }

//     pub fn path(path: &str) -> Result<Self, CarguixError> {
//         let source = PathSource::new(path)?;
//         Ok(Self::new(&source.crate_name(), &CrateSource::Path(source)))
//     }

//     pub fn lock(
//         crate_name: &str,
//         version: &Option<String>,
//         path: impl AsRef<Path>,
//     ) -> Result<Self, CarguixError> {
//         let cargo_lock: CargoLock = toml::from_str(
//             &std::fs::read_to_string(path).map_err(CarguixError::LockFileReadError)?,
//         )
//         .map_err(CarguixError::LockFileParsingError)?;
//         let source = LockSource::new(crate_name, version, Box::new(cargo_lock))?;

//         Ok(Self::new(crate_name, &CrateSource::Lock(source)))
//     }

//     pub fn registry(crate_name: &str, version: &Option<String>) -> Result<Self, CarguixError> {
//         Ok(Self::new(
//             crate_name,
//             &CrateSource::Registry(RegistrySource::new(crate_name, version)?),
//         ))
//     }

//     pub fn crate_name(&self) -> String {
//         match &self.source {
//             CrateSource::Path(source) => source.crate_name(),
//             CrateSource::Lock(source) => source.crate_name(),
//             CrateSource::Simple(source) => source.crate_name(),
//             CrateSource::Git(source) => source.crate_name(),
//             CrateSource::Registry(source) => source.crate_name(),
//         }
//     }

//     pub fn definition_name(&self) -> String {
//         format!("rust-{}", self.crate_name().to_kebab_case())
//     }

//     pub fn package_name(&self) -> String {
//         format!(
//             "rust-{}",
//             match &self.source {
//                 CrateSource::Path(source) => source.package_name(),
//                 CrateSource::Lock(source) => source.package_name(),
//                 CrateSource::Simple(source) => source.package_name(),
//                 CrateSource::Git(source) => source.package_name(),
//                 CrateSource::Registry(source) => source.package_name(),
//             }
//         )
//     }

//     pub fn version(&self) -> String {
//         match &self.source {
//             CrateSource::Path(source) => source.version(),
//             CrateSource::Lock(source) => source.version(),
//             CrateSource::Simple(source) => source.version(),
//             CrateSource::Git(source) => source.version(),
//             CrateSource::Registry(source) => source.version(),
//         }
//     }

//     pub fn source(&self) -> String {
//         match &self.source {
//             CrateSource::Path(source) => source.source(),
//             CrateSource::Lock(source) => source.source(),
//             CrateSource::Simple(source) => source.source(),
//             CrateSource::Git(source) => source.source(),
//             CrateSource::Registry(source) => source.source(),
//         }
//     }

//     pub fn dependencies(&self) -> Result<Vec<CrateRef>, CarguixError> {
//         match &self.source {
//             CrateSource::Path(source) => source.dependencies(),
//             CrateSource::Lock(source) => source.dependencies(),
//             CrateSource::Simple(source) => source.dependencies(),
//             CrateSource::Git(source) => source.dependencies(),
//             CrateSource::Registry(source) => source.dependencies(),
//         }
//         .map_err(|err| {
//             CarguixError::DependencyProcessingFailed(
//                 Box::new(err),
//                 self.crate_name(),
//                 self.version(),
//             )
//         })
//     }
// }

pub trait CrateRef {
    fn crate_name(&self) -> String;
    fn package_name(&self) -> String;
    fn version(&self) -> String;
    fn source(&self) -> String;
    fn dependencies(&self) -> Result<Vec<Box<dyn CrateRef>>, CarguixError>;

    fn definition_name(&self) -> String {
        format!("rust-{}", self.crate_name().to_kebab_case())
    }

    fn to_guix_package(&self) -> Result<(guix::Package, Vec<Box<dyn CrateRef>>), CarguixError> {
        let source = self.source();
        let dependencies = self.dependencies()?;
        Ok((
            guix::Package {
                name: self.definition_name(),
                package_name: self.package_name(),
                version: self.version(),
                hash: guix::hash(&source)?,
                source,
                build_system: "cargo-build-system".to_string(),
                cargo_inputs: dependencies
                    .iter()
                    .map(|dependency| dependency.package_name())
                    .collect(),
                ..guix::Package::default()
            },
            dependencies,
        ))
    }
}
