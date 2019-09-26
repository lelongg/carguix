#![allow(dead_code)]

mod crate_ref;
mod errors;
mod guix;

use crate_ref::{CrateRef, PathSource};
use crates_index::Index as CrateIndex;
use errors::CarguixError;
use once_cell::sync::Lazy;
use rustbreak::Database;
use std::{
    collections::{HashMap, VecDeque},
    error::Error,
    ops::Not,
    path::PathBuf,
};
use structopt::StructOpt;
use tempdir::TempDir;

#[derive(Debug, StructOpt)]
#[structopt(about = "Generate Guix package definition for Rust crates")]
struct Cli {
    crate_name: Option<String>,
    #[structopt(short, long, help = "Path to crate directory (containing Cargo.toml)")]
    manifest_path: Option<PathBuf>,
    #[structopt(short, long, help = "Update crates.io index")]
    update: bool,
    #[structopt(
        short,
        long,
        help = "Generate package definition for specific version of the crate (default: earliest)"
    )]
    version: Option<String>,
}

type HashDbKey = String;
static TMPDIR: Lazy<TempDir> = Lazy::new(|| {
    TempDir::new("carguix")
        .map_err(CarguixError::TmpdirError)
        .unwrap_or_else(|err| exit_with_errors(&err))
});
static INDEX: Lazy<CrateIndex> = Lazy::new(|| CrateIndex::new("_index"));
static HASHDB: Lazy<Database<HashDbKey>> = Lazy::new(|| {
    Database::open("hash.db")
        .map_err(CarguixError::HashdbError)
        .unwrap_or_else(|err| exit_with_errors(&err))
});

fn main() {
    match run() {
        Ok(result) => print!("{}", result),
        Err(err) => exit_with_errors(&err),
    }
}

fn run() -> Result<String, CarguixError> {
    env_logger::init();
    let args = Cli::from_args();
    if args.update || INDEX.exists().not() {
        log::info!("fetching crates.io index...");
        INDEX
            .retrieve_or_update()
            .map_err(CarguixError::IndexUpdateError)?
    }
    let mut crates_queue = VecDeque::new();
    let mut guix_packages = HashMap::new();
    if let Some(_crate_name) = &args.crate_name {
        if let Some(manifest_path) = args.manifest_path {
            crates_queue.push_back(
                Box::new(PathSource::new(manifest_path, &HashMap::new())?) as Box<dyn CrateRef>
            );
        }
    }
    while let Some(crate_ref) = crates_queue.pop_front() {
        let key = (crate_ref.crate_name(), crate_ref.version());
        if guix_packages.contains_key(&key) {
            continue;
        }
        log::info!(
            "processing crate {} in version {}",
            crate_ref.crate_name(),
            crate_ref.version()
        );
        let (guix_package, dependencies) = crate_ref.to_guix_package()?;
        guix_packages.insert(key, guix_package);
        for dependency in dependencies {
            crates_queue.push_back(dependency);
        }
    }
    mustache::compile_str(include_str!("template.scm.mustache"))
        .map_err(CarguixError::TemplateCompilationFailed)?
        .render_to_string(&guix::Module {
            name: args.crate_name.unwrap_or_else(|| "".to_string()),
            packages: guix_packages.values().cloned().collect(),
        })
        .map_err(CarguixError::RenderError)
}

fn exit_with_errors(err: &dyn Error) -> ! {
    log::error!("error: {}", err);
    let mut cause = err.source();
    while let Some(err) = cause {
        log::error!("caused by: {}", err);
        cause = err.source();
    }
    std::process::exit(1)
}
