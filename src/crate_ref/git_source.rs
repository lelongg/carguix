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
pub struct GitSource {}

impl GitSource {
    pub fn crate_name(&self) -> String {
        unimplemented!()
    }

    pub fn package_name(&self) -> String {
        unimplemented!()
    }

    pub fn version(&self) -> String {
        unimplemented!()
    }

    pub fn source(&self) -> String {
        unimplemented!()
    }

    pub fn dependencies(&self) -> Result<Vec<CrateRef>, CarguixError> {
        unimplemented!()
    }
}
