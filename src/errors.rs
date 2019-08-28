use err_derive::Error;

#[derive(Debug, Error)]
pub enum CarguixError {
    #[error(display = "could not create temporary directory")]
    TmpdirError(#[error(cause)] std::io::Error),
    #[error(display = "could not open hash database (crates_hash.db)")]
    HashdbError(#[error(cause)] rustbreak::BreakError),
    #[error(display = "could not update index")]
    IndexUpdateError(#[error(cause)] crates_index::Error),
    #[error(display = "could not package version {:?} of crate {}", version, name)]
    CratePackagingFailed {
        name: String,
        version: Option<String>,
    },
    #[error(display = "could not find crate {}", _0)]
    CrateNotFound(String),
    #[error(display = "failure while retrieving hash of {:?} in hash database", _0)]
    HashRetrieveFailed(#[error(cause)] rustbreak::BreakError, String),
    #[error(display = "could not download crate {}", _0)]
    CrateDownloadError(#[error(cause)] reqwest::Error, String),
    #[error(display = "could not create crate {} destination file", _0)]
    FileCreationFailed(#[error(cause)] std::io::Error, String),
    #[error(display = "failure while inserting hash of {:?} in hash database", _0)]
    HashInsertionFailed(#[error(cause)] rustbreak::BreakError, String),
    #[error(display = "could not flush hash database")]
    HashDatabaseFlushFailed(#[error(cause)] rustbreak::BreakError),
    #[error(display = "could not compute hash of crate {}", _0)]
    GuixHashError(
        #[error(cause)] shellfn::Error<std::convert::Infallible>,
        String,
    ),
    #[error(display = "could not copy crate {} source to destination", _0)]
    CopyError(#[error(cause)] std::io::Error, String),
    #[error(display = "no version of crate {} matching {} found", name, version)]
    NoMatchingVersion { name: String, version: String },
    #[error(
        display = "no version of crate {} matching requirement {} found",
        name,
        requirement
    )]
    NoVersionMatchingRequirement { name: String, requirement: String },
    #[error(display = "parsing of version {} for crate {} failed", _1, _1)]
    VersionParsingError(#[error(cause)] semver::SemVerError, String, String),
    #[error(display = "parsing of requirement {} for crate {} failed", _1, _0)]
    RequirementParsingError(#[error(cause)] semver::ReqParseError, String, String),
    #[error(
        display = "could not process a dependency of crate {} in version {}",
        _0,
        _1
    )]
    DependencyProcessingFailed(#[error(cause)] Box<CarguixError>, String, String),
    #[error(display = "error while parsing crate at path {}", _0)]
    ManifestParsingError(#[error(cause)] cargo_toml::Error, String),
    #[error(display = "{} does not define a package", _0)]
    NoPackageInManifest(String),
    #[error(display = "cannot compile mustache template")]
    TemplateCompilationFailed(#[error(cause)] mustache::Error),
    #[error(display = "cannot render mustache template")]
    RenderError(#[error(cause)] mustache::Error),
    #[error(display = "error while parsing URL {}", _0)]
    UrlParsingError(#[error(cause)] reqwest::UrlError, String),
    #[error(display = "URL is not a file path: {}", _0)]
    UrlNotAFilePath(String),
    #[error(display = "Dependency found in lock file is ill-formed: {}", _0)]
    BadLockFileDependency(String),
    #[error(display = "Crate {} in version {} not found in lock file", _0, _1)]
    PackageNotFoundInLock(String, String),
    #[error(display = "cannot read Cargo.lock")]
    LockFileReadError(#[error(cause)] std::io::Error),
    #[error(display = "cannot parse Cargo.lock")]
    LockFileParsingError(#[error(cause)] toml::de::Error),
    #[error(display = "cannot canonicalize path: {}", _0)]
    CanonicalizationFailed(#[error(cause)] std::io::Error, String),
}
