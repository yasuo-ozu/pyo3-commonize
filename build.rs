use cargo::core::package::{Package, PackageSet};
use cargo::core::package_id::PackageId;
use cargo::core::resolver::Resolve;
use cargo::sources::path::PathSource;
use cargo::util::context::GlobalContext;
use cargo::util::errors::CargoResult;
use filetime::FileTime;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

// Get manifest path on project root. Not equal to "CARGO_MANIFEST_DIR/Cargo.toml".
fn get_root_manifest_path() -> Option<PathBuf> {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let mut base_dir = Path::new(out_dir.as_str());
    while !base_dir.ends_with("target") {
        base_dir = base_dir.parent()?;
    }
    let mut base_dir = base_dir.parent()?.to_owned();
    base_dir.push("Cargo.toml");
    Some(base_dir)
}

const COMMONIZE_MODNAME: &'static str = "pyo3-commonize";

fn get_candidate_packages<'a>(
    package_set: &'a PackageSet,
) -> CargoResult<impl Iterator<Item = &'a Package>> {
    Ok(package_set
        .package_ids()
        .map(|pid| Ok(package_set.get_one(pid)?))
        .collect::<CargoResult<Vec<&'a Package>>>()?
        .into_iter()
        .filter(|pkg| {
            pkg.dependencies()
                .iter()
                .any(|dep| dep.package_name().as_str() == COMMONIZE_MODNAME)
        }))
}

fn resolve_deps(pid: PackageId, resolve: &Resolve) -> BTreeSet<PackageId> {
    let mut unresolved_deps = BTreeSet::new();
    unresolved_deps.insert(pid);
    let mut resolved_deps = BTreeSet::new();
    while unresolved_deps.len() > 0 {
        let ret = unresolved_deps
            .iter()
            .map(|d| resolve.deps(d.clone()).map(|(a, _)| a))
            .flatten()
            .collect::<BTreeSet<_>>();
        resolved_deps.extend(&unresolved_deps);
        unresolved_deps = ret.difference(&resolved_deps).cloned().collect();
    }
    resolved_deps
}

fn generate_fingerprint_of_root_package(pkg: &Package, gctx: &GlobalContext) -> Option<String> {
    let source_id = pkg.package_id().source_id();
    let path = source_id.local_path()?;
    let path_source = PathSource::new(path.as_path(), source_id, gctx);
    let mut max = FileTime::zero();
    let mut max_path = PathBuf::new();
    for file in path_source.list_files(pkg).ok()? {
        match file.extension().map(|s| s.to_str().unwrap()) {
            Some("pyd") | Some("pyc") | Some("pyi") | Some("so") | Some("dylib") => {
                // It seems not affect on representation of Rust types
                continue;
            }
            _ => {
                let mtime = std::fs::metadata(&file)
                    .map(|m| FileTime::from_last_modification_time(&m))
                    .unwrap_or_else(|_| FileTime::zero());
                if mtime > max {
                    max = mtime;
                    max_path = file;
                }
            }
        }
    }
    Some(format!("{} ({})", max, max_path.display()))
}

fn generate_state_tag_for_package(
    package_set: &PackageSet,
    pid: PackageId,
    resolve: &Resolve,
    gctx: &GlobalContext,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    let source_map = package_set.sources();
    for d in resolve_deps(pid, resolve) {
        let source_id = d.source_id();
        let package = package_set.get_one(d).unwrap();
        let base_fingerprint = if &pid == &d {
            // root package
            generate_fingerprint_of_root_package(package, gctx)
        } else {
            None
        };
        let fingerprint = base_fingerprint.unwrap_or_else(|| {
            source_map
                .get(source_id)
                .unwrap()
                .fingerprint(package)
                .unwrap()
        });
        d.name().as_str().hash(&mut hasher);
        fingerprint.hash(&mut hasher);
    }
    hasher.finish()
}

fn generate_state_tag_dict() -> Result<HashMap<String, u64>, String> {
    let manifest_path = get_root_manifest_path().ok_or("Cannot find root manifest")?;
    let context = cargo::util::context::GlobalContext::default().map_err(|e| format!("{}", e))?;
    let mut workspace = cargo::core::Workspace::new(&manifest_path, &context)
        .map_err(|e| format!("Cannot load manifest for {:?}: {}", &manifest_path, e))?;
    workspace.set_ignore_lock(true);
    let (package_set, resolve) =
        cargo::ops::resolve_ws(&workspace).map_err(|e| format!("{}", e))?;
    Ok(get_candidate_packages(&package_set)
        .unwrap()
        .into_iter()
        .map(|p| {
            (
                p.name().as_str().to_owned(),
                generate_state_tag_for_package(&package_set, p.package_id(), &resolve, &context),
            )
        })
        .collect())
}

fn main() {
    println!(
        "cargo::rustc-env=COMMONIZE_OUT_DIR={}",
        std::env::var("OUT_DIR").unwrap_or("".to_owned())
    );
    println!(
        "cargo::rustc-env=COMMONIZE_ENV={}{}{}{}",
        std::env::var("TARGET").unwrap_or("".to_owned()),
        std::env::var("HOST").unwrap_or("".to_owned()),
        std::env::var("OPT_LEVEL").unwrap_or("".to_owned()),
        std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or("".to_owned())
    );
    match generate_state_tag_dict() {
        Ok(dict) => {
            let s = dict
                .iter()
                .map(|(package_name, tag)| {
                    let modname = package_name.replace("-", "_");
                    format!("{}:{}", modname, tag)
                })
                .collect::<Vec<_>>()
                .join(",");
            println!("cargo::rustc-env=COMMONIZE_MODULE_STATE_TAG={}", s);
        }
        Err(e) => {
            println!("cargo::rustc-env=COMMONIZE_MODULE_STATE_TAG=ERROR: {}", e);
        }
    }
}
