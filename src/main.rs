use crates_index::{Crate, Dependency, Index};
use err_derive::Error;
use heck::KebabCase;
use lexpr::sexp;
use rustbreak::Database;
use semver::{Version, VersionReq};
use shellfn::shell;
use std::{
    collections::{HashSet, VecDeque},
    error::Error,
    fs::File,
    io::copy,
    ops::Not,
};
use structopt::StructOpt;
use tempdir::TempDir;

#[derive(Debug, StructOpt)]
#[structopt(about = "Generate Guix package definition for Rust crates")]
struct Cli {
    crate_name: String,
    #[structopt(short, long, help = "Update crates.io index")]
    update: bool,
    #[structopt(
        short,
        long,
        help = "Generate package definition for specific version of the crate (default: earliest)"
    )]
    version: Option<String>,
}

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
    #[error(display = "failure while retrieving key {:?} in hash database", _0)]
    HashRetrieveFailed(#[error(cause)] rustbreak::BreakError, (String, String)),
    #[error(display = "could not download crate {}", _0)]
    CrateDownloadError(#[error(cause)] reqwest::Error, String),
    #[error(display = "could not create crate {} destination file", _0)]
    FileCreationFailed(#[error(cause)] std::io::Error, String),
    #[error(display = "failure while inserting key {:?} in hash database", _0)]
    HashInsertionFailed(#[error(cause)] rustbreak::BreakError, (String, String)),
    #[error(display = "could not flush hash database")]
    HashDatabaseFlushFailed(#[error(cause, no_from)] rustbreak::BreakError),
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
}

#[derive(Debug)]
pub struct Carguix {
    crates: VecDeque<(String, Option<String>)>,
    already_added_crates: HashSet<(String, Option<String>)>,
    index: Index,
    tmpdir: TempDir,
    hashdb: Database<(String, String)>,
}

impl Carguix {
    pub fn new(crate_name: &str, crate_version: &Option<String>) -> Result<Self, CarguixError> {
        let mut carguix = Carguix {
            crates: VecDeque::new(),
            already_added_crates: HashSet::new(),
            index: Index::new("_index"),
            tmpdir: TempDir::new(env!("CARGO_PKG_NAME")).map_err(CarguixError::TmpdirError)?,
            hashdb: Database::open("crates_hash.db").map_err(CarguixError::HashdbError)?,
        };
        carguix
            .crates
            .push_back((crate_name.to_string(), crate_version.clone()));
        if carguix.index.exists().not() {
            carguix.update_index()?;
        }
        Ok(carguix)
    }

    pub fn update_index(&self) -> Result<(), CarguixError> {
        log::info!("fetching crates.io index...");
        self.index
            .retrieve_or_update()
            .map_err(CarguixError::IndexUpdateError)
    }

    pub fn process_crate(
        &mut self,
        crate_name: &str,
        crate_version: &Option<String>,
    ) -> Result<lexpr::Value, CarguixError> {
        let crate_index = &self
            .index
            .crate_(&crate_name)
            .ok_or_else(|| CarguixError::CrateNotFound(crate_name.to_string()))?;
        let crate_package = self
            .crate_package(crate_index, &crate_version)
            .map_err(|_| CarguixError::CratePackagingFailed {
                name: crate_name.to_string(),
                version: crate_version.clone(),
            })?;
        for dependency in &crate_package.dependencies {
            self.crates
                .push_back((dependency.name.clone(), Some(dependency.version.clone())));
        }
        self.already_added_crates
            .insert((crate_name.to_string(), crate_version.clone()));
        Ok(crate_package.to_package_sexpr())
    }

    pub fn get_crate_hash(
        &mut self,
        crate_name: &str,
        version: &str,
    ) -> Result<String, CarguixError> {
        let key = &(crate_name.to_string(), version.to_string());
        match self.hashdb.retrieve::<String, _>(key) {
            Ok(hash) => return Ok(hash),
            Err(rustbreak::BreakError::NotFound) => (), // cache miss
            Err(err) => Err(CarguixError::HashRetrieveFailed(err, key.clone()))?,
        }
        let url = format!(
            "https://crates.io/api/v1/crates/{}/{}/download",
            crate_name, version
        );
        let mut download_request = reqwest::get(&url)
            .map_err(|err| CarguixError::CrateDownloadError(err, crate_name.to_string()))?;
        let downloaded_crate_path = self
            .tmpdir
            .path()
            .join(format!("{}-{}.tar.gz", crate_name, version));
        let mut downloaded_crate = File::create(downloaded_crate_path.clone())
            .map_err(|err| CarguixError::FileCreationFailed(err, crate_name.to_string()))?;
        copy(&mut download_request, &mut downloaded_crate)
            .map_err(|err| CarguixError::CopyError(err, crate_name.to_string()))?;
        let hash = Self::guix_hash(&downloaded_crate_path.to_string_lossy())
            .map_err(|err| CarguixError::GuixHashError(err, crate_name.to_string()))?;
        self.hashdb
            .insert(key, hash.clone())
            .map_err(|err| CarguixError::HashInsertionFailed(err, key.clone()))?;
        self.hashdb
            .flush()
            .map_err(CarguixError::HashDatabaseFlushFailed)?;
        Ok(hash)
    }

    pub fn guix_hash(file_path: &str) -> Result<String, shellfn::Error<std::convert::Infallible>> {
        #[shell]
        fn guix_hash_(file_path: &str) -> Result<String, shellfn::Error<std::convert::Infallible>> {
            "guix hash $FILE_PATH"
        }
        Ok(guix_hash_(file_path)?.trim().to_string())
    }

    pub fn crate_package(
        &mut self,
        crate_: &Crate,
        version: &Option<String>,
    ) -> Result<CratePackage, CarguixError> {
        let version = version
            .as_ref()
            .map(String::as_str)
            .unwrap_or_else(|| crate_.latest_version().version());
        let crate_version = crate_
            .versions()
            .iter()
            .find(|crate_version| crate_version.version() == version)
            .ok_or(CarguixError::NoMatchingVersion {
                name: crate_.name().to_string(),
                version: version.to_string(),
            })?;
        let dependencies = crate_version
            .dependencies()
            .iter()
            .map(|dependency| self.dependency_crate_ref(dependency))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| {
                CarguixError::DependencyProcessingFailed(
                    Box::new(err),
                    crate_.name().to_string(),
                    version.to_string(),
                )
            })?;
        let hash = self.get_crate_hash(crate_.name(), version)?;
        Ok(CratePackage::new(
            crate_.name(),
            version,
            &hash,
            &dependencies,
        ))
    }

    pub fn dependency_crate_ref(
        &mut self,
        dependency: &Dependency,
    ) -> Result<CrateRef, CarguixError> {
        let crate_name = dependency.crate_name();
        let crate_ = self
            .index
            .crate_(crate_name)
            .ok_or_else(|| CarguixError::CrateNotFound(crate_name.to_string()))?;
        let mut crate_versions = crate_
            .versions()
            .iter()
            .map(|crate_version| Version::parse(crate_version.version()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| {
                CarguixError::VersionParsingError(
                    err,
                    crate_name.to_string(),
                    dependency.requirement().to_string(),
                )
            })?;
        crate_versions.sort();
        let version_req = VersionReq::parse(dependency.requirement()).map_err(|err| {
            CarguixError::RequirementParsingError(
                err,
                crate_name.to_string(),
                dependency.requirement().to_string(),
            )
        })?;
        let highest_matching_version = crate_versions
            .iter()
            .rev()
            .find(|version| version_req.matches(&version))
            .ok_or(CarguixError::NoVersionMatchingRequirement {
                name: crate_name.to_string(),
                requirement: dependency.requirement().to_string(),
            })?;
        Ok(CrateRef::new(
            crate_name,
            &highest_matching_version.to_string(),
        ))
    }
}

impl Iterator for Carguix {
    type Item = Result<lexpr::Value, CarguixError>;
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((crate_name, crate_version)) = self.crates.pop_front() {
            if self
                .already_added_crates
                .contains(&(crate_name.clone(), crate_version.clone()))
            {
                continue;
            }
            return Some(self.process_crate(&crate_name, &crate_version));
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct CratePackage {
    pub crate_ref: CrateRef,
    pub hash: String,
    pub dependencies: Vec<CrateRef>,
}

impl CratePackage {
    pub fn new(name: &str, version: &str, hash: &str, dependencies: &[CrateRef]) -> Self {
        Self {
            crate_ref: CrateRef::new(name, version),
            hash: hash.to_string(),
            dependencies: dependencies.to_vec(),
        }
    }

    pub fn to_package_sexpr(&self) -> lexpr::Value {
        let dependencies_sexpr = self
            .dependencies
            .iter()
            .map(CrateRef::to_dependency_sexpr)
            .collect::<Vec<_>>();
        sexp!(
            (#"define-public" ,(lexpr::Value::symbol(self.crate_ref.format_name_version()))
                (package
                    (name ,(self.crate_ref.format_name()))
                    (version ,(self.crate_ref.version.clone()))
                    (source
                        (origin
                            (method #"url-fetch")
                            (#"uri" (#"crate-uri" ,(self.crate_ref.name.clone()) version))
                            (#"file-name"
                                (#"string-append" name "-" version ".tar.gz"))
                            (sha256
                                (base32 ,(self.hash.clone())))))
                    (#"build-system" #"cargo-build-system")
                    (arguments
                        (list #:"cargo-inputs"
                            ,(lexpr::Value::append(
                                vec![lexpr::Value::symbol("list")],
                                lexpr::Value::list(dependencies_sexpr)))))
                    (#"home-page" #f)
                    (synopsis #f)
                    (description #f)
                    (license #f)))
        )
    }
}

#[derive(Debug, Clone)]
pub struct CrateRef {
    pub name: String,
    pub version: String,
}

impl CrateRef {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
        }
    }

    pub fn to_dependency_sexpr(&self) -> lexpr::Value {
        let formatted_name = self.format_name_version();
        sexp!((
            list,
            (formatted_name.clone()),
            (lexpr::Value::symbol(formatted_name))
        ))
    }

    pub fn format_name(&self) -> String {
        format!("rust-{}", self.name.to_kebab_case())
    }

    pub fn format_name_version(&self) -> String {
        format!("rust-{}-{}", self.name.to_kebab_case(), self.version)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let args = Cli::from_args();
    let carguix = Carguix::new(&args.crate_name, &args.version)?;
    if args.update {
        carguix.update_index()?;
    }
    for crate_sexpr in carguix {
        match crate_sexpr {
            Ok(crate_sexpr) => println!("{}\n", crate_sexpr),
            Err(err) => print_error(&err),
        }
    }
    Ok(())
}

fn print_error(err: &dyn Error) {
    log::error!("error: {}", err);
    let mut cause = err.source();
    while let Some(err) = cause {
        log::error!("caused by: {}", err);
        cause = err.source();
    }
}
