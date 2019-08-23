use crates_index::{Crate, Dependency, Index};
use heck::KebabCase;
use lexpr::sexp;
use log::info;
use once_cell::sync::Lazy;
use quicli::prelude::*;
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

static TMPDIR: Lazy<TempDir> = Lazy::new(|| {
    TempDir::new(env!("CARGO_PKG_NAME")).expect("could not create temporary directory")
});

static INDEX: Lazy<Index> = Lazy::new(|| Index::new("_index"));

static HASHDB: Lazy<Database<(String, String)>> =
    Lazy::new(|| Database::open("crates_hash.db").unwrap());

#[derive(Debug, StructOpt)]
struct Cli {
    crate_name: String,
    #[structopt(short, long)]
    update: bool,
    #[structopt(short, long)]
    version: Option<String>,
}

fn format_name(name: &str) -> String {
    format!("rust-{}", name.to_kebab_case())
}

fn guix_hash(file_path: &str) -> Result<String, Box<dyn Error>> {
    #[shell]
    fn guix_hash_(file_path: &str) -> Result<String, Box<dyn Error>> {
        "guix hash $FILE_PATH"
    }
    Ok(guix_hash_(file_path)?.trim().to_string())
}

#[derive(Debug, Clone)]
struct CrateSExpr(lexpr::Value, Vec<(String, Version)>);

#[derive(Debug, Clone)]
struct DependencySExpr(lexpr::Value, (String, Version));

fn get_crate_hash(crate_name: &str, version: &str) -> Result<String, Box<dyn Error>> {
    let key = &(crate_name.to_string(), version.to_string());
    match HASHDB.retrieve::<String, _>(key) {
        Ok(hash) => return Ok(hash),
        Err(rustbreak::BreakError::NotFound) => (),
        Err(err) => Err(err)?,
    }

    let url = format!(
        "https://crates.io/api/v1/crates/{}/{}/download",
        crate_name, version
    );
    let mut download_request = reqwest::get(&url)?;
    let downloaded_crate_path = TMPDIR
        .path()
        .join(format!("{}-{}.tar.gz", crate_name, version));
    let mut downloaded_crate = File::create(downloaded_crate_path.clone())?;
    copy(&mut download_request, &mut downloaded_crate)?;
    let hash = guix_hash(&downloaded_crate_path.to_string_lossy())?;
    HASHDB.insert(key, hash.clone())?;
    HASHDB.flush()?;
    Ok(hash)
}

fn dependency_to_sexpr(dependency: &Dependency) -> Result<DependencySExpr, Box<dyn Error>> {
    let crate_name = dependency.crate_name();
    let crate_ = INDEX
        .crate_(crate_name)
        .ok_or_else(|| format!("cannot find dependency {}", crate_name))?;
    let mut crate_versions = crate_.versions().to_vec();
    crate_versions.sort_by_key(|crate_version| Version::parse(crate_version.version()).unwrap());
    let version_req = VersionReq::parse(dependency.requirement())?;
    let highest_matching_version = crate_versions
        .iter()
        .rev()
        .find(|crate_version| {
            version_req.matches(&Version::parse(crate_version.version()).unwrap())
        })
        .ok_or_else(|| {
            format!(
                "no version of crate {} matching requirement {} found",
                crate_name,
                dependency.requirement()
            )
        })?;
    let formatted_name = format_name(crate_name);
    let formatted_name = format!("{}-{}", formatted_name, highest_matching_version.version());
    Ok(DependencySExpr(
        sexp!((
            list,
            (formatted_name.clone()),
            (lexpr::Value::symbol(formatted_name))
        )),
        (
            crate_.name().to_string(),
            Version::parse(highest_matching_version.version())?,
        ),
    ))
}

fn crate_to_sexpr(crate_: &Crate, version: &Option<String>) -> Result<CrateSExpr, Box<dyn Error>> {
    let version = version
        .as_ref()
        .map(String::as_str)
        .unwrap_or_else(|| crate_.latest_version().version());
    let crate_version = crate_
        .versions()
        .iter()
        .find(|crate_version| crate_version.version() == version)
        .ok_or_else(|| {
            format!(
                "no version of crate {} matching {} found",
                crate_.name(),
                version
            )
        })?;
    let dependencies = crate_version
        .dependencies()
        .iter()
        .map(dependency_to_sexpr)
        .collect::<Result<Vec<_>, _>>()?;
    let dependencies_sexpr = dependencies
        .iter()
        .cloned()
        .map(|dependency| dependency.0)
        .collect::<Vec<_>>();
    let hash = get_crate_hash(crate_.name(), version)?;
    let formatted_name = format_name(&crate_.name());
    let value = sexp!(
        (#"define-public" ,(lexpr::Value::symbol(format!("{}-{}", formatted_name, version)))
            (package
                (name ,formatted_name)
                (version ,(version))
                (source
                    (origin
                        (method #"url-fetch")
                        (#"uri" (#"crate-uri" ,(crate_.name()) version))
                        (#"file-name"
                            (#"string-append" name "-" version ".tar.gz"))
                        (sha256
                            (base32 ,hash))))
                (#"build-system" #"cargo-build-system")
                (arguments
                    (list #:"cargo-inputs"
                        ,(lexpr::Value::append(vec![lexpr::Value::symbol("list")], lexpr::Value::list(dependencies_sexpr)))))
                (#"home-page" #f)
                (synopsis #f)
                (description #f)
                (license #f)))
    );
    let dependencies = dependencies
        .iter()
        .cloned()
        .map(|dependency| dependency.1)
        .collect();
    Ok(CrateSExpr(value, dependencies))
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let args = Cli::from_args();
    if INDEX.exists().not() || args.update {
        info!("fetching crates.io index...");
        INDEX.retrieve_or_update()?;
    }

    let mut crates = VecDeque::new();
    crates.push_back((args.crate_name, args.version));

    let mut already_added_crates = HashSet::new();

    while let Some((crate_name, crate_version)) = crates.pop_front() {
        if already_added_crates.contains(&(crate_name.clone(), crate_version.clone())) {
            continue;
        }
        let crate_sexpr = crate_to_sexpr(
            &INDEX
                .crate_(&crate_name)
                .ok_or_else(|| format!("cannot find crate {}", crate_name))?,
            &crate_version,
        )?;
        for dependency in crate_sexpr.1 {
            crates.push_back((dependency.0, Some(dependency.1.to_string())));
        }
        already_added_crates.insert((crate_name, crate_version));
        println!("{}\n", crate_sexpr.0);
    }

    Ok(())
}
