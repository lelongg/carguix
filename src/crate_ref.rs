mod git_source;
mod lock_source;
mod path_source;
mod registry_source;
mod simple_source;

use crate::{errors::CarguixError, guix};
pub use git_source::GitSource;
use heck::KebabCase;
pub use lock_source::LockSource;
pub use path_source::PathSource;
pub use registry_source::RegistrySource;
pub use simple_source::SimpleSource;

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
