use crate::{
    crate_ref::{CrateRef, CrateSource},
    errors::CarguixError,
    guix, INDEX,
};
use crates_index::{Dependency as CrateDependency, Version as CrateVersion};
use heck::KebabCase;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, error::Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySource {
    crate_version: CrateVersion,
}

impl RegistrySource {
    pub fn new(crate_name: &str, version: &Option<String>) -> Result<Self, CarguixError> {
        let indexed_crate = INDEX
            .crate_(crate_name)
            .ok_or_else(|| CarguixError::CrateNotFound(crate_name.to_string()))?;
        Ok(Self {
            crate_version: version
                .as_ref()
                .map(|version| {
                    indexed_crate
                        .versions()
                        .iter()
                        .find(|crate_version| crate_version.version() == version)
                        .ok_or(CarguixError::NoMatchingVersion {
                            name: crate_name.to_string(),
                            version: version.to_string(),
                        })
                })
                .unwrap_or_else(|| Ok(indexed_crate.latest_version()))?
                .clone(),
        })
    }

    pub fn new_with_requirement(crate_name: &str, requirement: &str) -> Result<Self, CarguixError> {
        Self::highest_matching_crate_version(crate_name, requirement).and_then(|crate_version| {
            Ok(RegistrySource::new(
                crate_version.name(),
                &Some(crate_version.version().to_string()),
            )?)
        })
    }

    pub fn crate_name(&self) -> String {
        self.crate_version.name().to_string()
    }

    pub fn package_name(&self) -> String {
        format!(
            "{}-{}",
            self.crate_name().to_kebab_case(),
            self.crate_version.version()
        )
    }

    pub fn version(&self) -> String {
        self.crate_version.version().to_string()
    }

    pub fn source(&self) -> String {
        format!(
            "https://crates.io/api/v1/crates/{}/{}/download",
            self.crate_version.name(),
            self.version()
        )
    }

    pub fn dependencies(&self) -> Result<Vec<CrateRef>, CarguixError> {
        self.crate_version
            .dependencies()
            .iter()
            .map(|dependency| {
                RegistrySource::new_with_requirement(
                    dependency.crate_name(),
                    dependency.requirement(),
                )
                .map(|source| {
                    CrateRef::new(dependency.crate_name(), &CrateSource::Registry(source))
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }

    fn highest_matching_crate_version(
        crate_name: &str,
        requirement: &str,
    ) -> Result<CrateVersion, CarguixError> {
        let crate_ = INDEX
            .crate_(crate_name)
            .ok_or_else(|| CarguixError::CrateNotFound(crate_name.to_string()))?;
        let mut crate_versions = crate_
            .versions()
            .iter()
            .map(|crate_version| {
                Version::parse(crate_version.version())
                    .map(|version| (crate_version.clone(), version))
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| {
                CarguixError::VersionParsingError(
                    err,
                    crate_name.to_string(),
                    requirement.to_string(),
                )
            })?;
        crate_versions.sort_by_key(|(_, version)| version.clone());
        let version_req = VersionReq::parse(requirement).map_err(|err| {
            CarguixError::RequirementParsingError(
                err,
                crate_name.to_string(),
                requirement.to_string(),
            )
        })?;
        crate_versions
            .into_iter()
            .rev()
            .find(|(_, version)| version_req.matches(&version))
            .map(|(crate_version, _)| crate_version)
            .ok_or(CarguixError::NoVersionMatchingRequirement {
                name: crate_name.to_string(),
                requirement: requirement.to_string(),
            })
    }
}
