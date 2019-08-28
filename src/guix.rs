use crate::{errors::CarguixError, HASHDB, TMPDIR};
use data_encoding::BASE64URL_NOPAD;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shellfn::shell;
use std::{convert::Infallible, fs::File, io::copy, path::Path};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Module {
    pub packages: Vec<Package>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub package_name: String,
    pub version: String,
    pub source: String,
    pub hash: String,
    pub build_system: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_page: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synopsis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub native_inputs: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub propagated_inputs: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cargo_inputs: Vec<String>,
}

pub fn hash(url_str: &str) -> Result<String, CarguixError> {
    match HASHDB.retrieve::<String, _>(url_str) {
        Ok(hash) => return Ok(hash),
        Err(rustbreak::BreakError::NotFound) => log::warn!("cache miss"), // cache miss
        Err(err) => Err(CarguixError::HashRetrieveFailed(err, url_str.to_string()))?,
    }

    let url = reqwest::Url::parse(url_str)
        .map_err(|err| CarguixError::UrlParsingError(err, url_str.to_string()))?;
    let downloaded_crate_path = if url.scheme() == "file" {
        url.to_file_path()
            .map_err(|_| CarguixError::UrlNotAFilePath(url_str.to_string()))?
    } else {
        let mut download_request = reqwest::get(url)
            .map_err(|err| CarguixError::CrateDownloadError(err, url_str.to_string()))?;
        let mut hasher = Sha256::new();
        hasher.input(url_str);
        let downloaded_crate_path = TMPDIR.path().join(format!(
            "{}.tar.gz",
            BASE64URL_NOPAD.encode(&hasher.result())
        ));
        let mut downloaded_crate = File::create(downloaded_crate_path.clone())
            .map_err(|err| CarguixError::FileCreationFailed(err, url_str.to_string()))?;
        copy(&mut download_request, &mut downloaded_crate)
            .map_err(|err| CarguixError::CopyError(err, url_str.to_string()))?;
        downloaded_crate_path
    };

    let hash = guix_hash(&downloaded_crate_path.to_string_lossy())
        .map_err(|err| CarguixError::GuixHashError(err, url_str.to_string()))?;
    HASHDB
        .insert(url_str, hash.clone())
        .map_err(|err| CarguixError::HashInsertionFailed(err, url_str.to_string()))?;
    HASHDB
        .flush()
        .map_err(CarguixError::HashDatabaseFlushFailed)?;
    Ok(hash)
}

fn guix_hash(path: &str) -> Result<String, shellfn::Error<Infallible>> {
    #[shell]
    fn guix_hash_file(file_path: &str) -> Result<String, shellfn::Error<Infallible>> {
        "guix hash $FILE_PATH"
    }

    #[shell]
    fn guix_hash_dir(dir_path: &str) -> Result<String, shellfn::Error<Infallible>> {
        "guix hash -rx $DIR_PATH"
    }

    Ok(if Path::new(path).is_dir() {
        guix_hash_dir(path)
    } else {
        guix_hash_file(path)
    }?
    .trim()
    .to_string())
}
