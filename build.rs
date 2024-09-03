use cargo::core::package::{Package, PackageSet};
use cargo::core::package_id::PackageId;
use cargo::core::resolver::Resolve;
use cargo::util::errors::CargoResult;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

// Get manifest path on project root. Not equal to "CARGO_MANIFEST_DIR/Cargo.toml".
fn get_root_manifest_path() -> Option<PathBuf> {
    let mut base_dir = Path::new(std::env::var("OUT_DIR").unwrap().as_str())
        .parent()?
        .parent()?
        .parent()?
        .parent()?
        .parent()?
        .to_owned();
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

fn generate_state_tag_for_package(
    package_set: &PackageSet,
    pid: PackageId,
    resolve: &Resolve,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    let source_map = package_set.sources();
    for d in resolve_deps(pid, resolve) {
        let source_id = d.source_id();
        let package = package_set.get_one(d).unwrap();
        let fingerprint = source_map
            .get(source_id)
            .unwrap()
            .fingerprint(package)
            .unwrap();
        d.name().as_str().hash(&mut hasher);
        fingerprint.hash(&mut hasher);
    }
    hasher.finish()
}

fn generate_state_tag_dict() -> HashMap<String, u64> {
    let manifest_path = get_root_manifest_path().expect("Cannot find root manifest");
    dbg!(&manifest_path);
    let context = cargo::util::context::GlobalContext::default().unwrap();
    let workspace = cargo::core::Workspace::new(&manifest_path, &context)
        .unwrap_or_else(|e| panic!("Cannot load manifest for {:?}: {}", &manifest_path, e));
    let (package_set, resolve) = cargo::ops::resolve_ws(&workspace).unwrap();
    get_candidate_packages(&package_set)
        .unwrap()
        .into_iter()
        .map(|p| {
            (
                p.name().as_str().to_owned(),
                generate_state_tag_for_package(&package_set, p.package_id(), &resolve),
            )
        })
        .collect()
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
    let dict = generate_state_tag_dict();
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
