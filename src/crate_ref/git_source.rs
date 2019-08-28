use crate::{
    crate_ref::{registry_source::RegistrySource, CrateRef},
    errors::CarguixError,
    guix, INDEX,
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

impl CrateRef for GitSource {
    fn crate_name(&self) -> String {
        unimplemented!()
    }

    fn package_name(&self) -> String {
        unimplemented!()
    }

    fn version(&self) -> String {
        unimplemented!()
    }

    fn source(&self) -> String {
        unimplemented!()
    }

    fn dependencies(&self) -> Result<Vec<Box<dyn CrateRef>>, CarguixError> {
        unimplemented!()
    }
}
