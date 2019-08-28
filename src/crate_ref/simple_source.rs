use crate::{crate_ref::lock_source::CargoLock, errors::CarguixError, guix, CrateRef, INDEX};
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
pub struct SimpleSource {
    crate_name: String,
    version: String,
    source: String,
    dependencies: Vec<SimpleSource>,
}

impl CrateRef for SimpleSource {
    fn crate_name(&self) -> String {
        self.crate_name.clone()
    }

    fn package_name(&self) -> String {
        format!("{}-{}", self.crate_name().to_kebab_case(), self.version())
    }

    fn version(&self) -> String {
        self.version.clone()
    }

    fn source(&self) -> String {
        self.source.clone()
    }

    fn dependencies(&self) -> Result<Vec<Box<dyn CrateRef>>, CarguixError> {
        Ok(self
            .dependencies
            .iter()
            .map(|dependency| Box::new(dependency.clone()) as Box<dyn CrateRef>)
            .collect())
    }
}
