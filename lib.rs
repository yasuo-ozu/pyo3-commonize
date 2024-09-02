use core::marker::PhantomData;
pub use pyo3;
use pyo3::impl_::pyclass::PyClassImpl;
use pyo3::prelude::*;
use pyo3::pyclass::PyClass;
use pyo3::types::{PyDict, PyType};
pub use pyo3_commonize_macro::Commonized;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

/// Marker trait implemented by `#[derive(Commonized)] macro. See [`commonize()`]`
pub unsafe trait Commonized: PyClass {
    #[doc(hidden)]
    const __COMMONIZED_INTERNAL_TAG: usize;
    #[doc(hidden)]
    const __COMMONIZED_MODPATH: &'static str;
    #[doc(hidden)]
    const __COMMONIZED_MANIFEST_DIR: &'static str;
}

fn process_module_path(module_path: &str, hasher: &mut impl Hasher) -> Option<()> {
    let module_name = &module_path[..module_path.find("::").unwrap_or(module_path.len())];
    let mut base_dir = Path::new(std::env!("COMMONIZE_OUT_DIR"))
        .parent()?
        .parent()?
        .parent()?
        .to_owned();
    base_dir.push("deps");
    let entries = std::fs::read_dir(&base_dir)
        .ok()?
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let mut entries = entries
        .into_iter()
        .filter(|entry| {
            let path = entry.path();
            path.extension() == Some(&OsStr::new("d"))
                && path
                    .iter()
                    .last()
                    .and_then(|s| s.to_str())
                    .map(|s| s.starts_with(&format!("{}-", module_name)))
                    == Some(true)
        })
        .collect::<Vec<_>>();
    entries.sort_by(|lhs, rhs| {
        let ltime = lhs.metadata().unwrap().modified().unwrap();
        let rtime = rhs.metadata().unwrap().modified().unwrap();
        ltime.cmp(&rtime)
    });
    entries.iter().last()?.path().iter().last()?.hash(hasher);
    Some(())
}

fn get_modified_of_dir(path: &Path) -> Option<std::time::SystemTime> {
    let mut times: Vec<_> = walkdir::WalkDir::new(path)
        .max_depth(20)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            if entry.path().extension() == Some(OsStr::new("rs")) {
                Some(entry)
            } else {
                None
            }
        })
        .map(|entry| entry.metadata().unwrap().modified().unwrap())
        .collect();
    times.sort();
    if times.len() > 0 {
        Some(times[times.len() - 1])
    } else {
        None
    }
}

fn hash_env(hasher: &mut impl Hasher) {
    env!("COMMONIZE_ENV").hash(hasher);
}

fn hash_cargo_deps(manifest_path: &Path, module_name: &str, hasher: &mut impl Hasher) {
    let context = cargo::util::context::GlobalContext::default().unwrap();
    let workspace = cargo::core::Workspace::new(manifest_path, &context)
        .unwrap_or_else(|e| panic!("Cannot load manifest for {:?}: {}", &manifest_path, e));
    let (packages, resolve) = cargo::ops::resolve_ws(&workspace).unwrap();
    let mut unresolved_deps = packages
        .package_ids()
        .filter(|id| id.name().as_str() == module_name)
        .collect::<BTreeSet<_>>();
    if unresolved_deps.len() != 1 {
        panic!("Cannot find root package, count of exactly one.");
    }
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
    for d in resolved_deps {
        let package = packages.get_one(d).unwrap();
        let modified = get_modified_of_dir(package.root()).unwrap();
        d.name().as_str().hash(hasher);
        modified.hash(hasher);
    }
}

fn get_module_name(module_path: &str) -> &str {
    &module_path[..module_path.find("::").unwrap_or(module_path.len())]
}

fn generate_final_tag<T: Commonized>() -> usize {
    let mut hasher = DefaultHasher::new();
    let module_name = get_module_name(T::__COMMONIZED_MODPATH);
    let mut manifest_path = PathBuf::from(T::__COMMONIZED_MANIFEST_DIR);
    manifest_path.push("Cargo.toml");
    hash_cargo_deps(manifest_path.as_ref(), module_name, &mut hasher);
    hash_env(&mut hasher);
    T::__COMMONIZED_INTERNAL_TAG.hash(&mut hasher);
    hasher.finish() as usize
}

fn set_type_obj<T: PyClass>(py: Python<'_>, type_object: Py<PyType>) {
    type Getter = for<'py> unsafe fn(
        Python<'py>,
        *mut pyo3::ffi::PyObject,
    ) -> PyResult<*mut pyo3::ffi::PyObject>;
    type Setter = for<'py> unsafe fn(
        Python<'py>,
        *mut pyo3::ffi::PyObject,
        *mut pyo3::ffi::PyObject,
    ) -> PyResult<core::ffi::c_int>;
    #[allow(unused)]
    struct GetterAndSetter {
        getter: Getter,
        setter: Setter,
    }
    #[allow(unused)]
    enum GetSetDefType {
        Getter(Getter),
        Setter(Setter),
        GetterAndSetter(Box<GetterAndSetter>),
    }
    #[allow(unused)]
    struct GetSetDefDestructor {
        closure: GetSetDefType,
    }
    #[allow(unused)]
    struct PyClassTypeObject {
        type_object: Py<PyType>,
        getset_destructors: Vec<GetSetDefDestructor>,
    }
    struct LazyTypeObject<T>(LazyTypeObjectInner, PhantomData<T>);
    #[allow(unused)]
    struct LazyTypeObjectInner {
        value: pyo3::sync::GILOnceCell<PyClassTypeObject>,
        initializing_threads:
            pyo3::sync::GILProtected<std::cell::RefCell<Vec<std::thread::ThreadId>>>,
        tp_dict_filled: pyo3::sync::GILOnceCell<()>,
    }
    let type_obj: &'static LazyTypeObject<T> =
        unsafe { core::mem::transmute(<T as PyClassImpl>::lazy_type_object()) };
    type_obj
        .0
        .value
        .set(
            py,
            PyClassTypeObject {
                type_object,
                getset_destructors: Vec::new(),
            },
        )
        .unwrap_or_else(|_| panic!("commonize() should be called at the head of #[pymodule]"));
}

const COMMONIZE_DICT_NAME: &'static str = "__commonize_type_dict";

fn find_type_object(
    py: Python<'_>,
    tag: usize,
    cb: impl FnOnce() -> Py<PyType>,
) -> PyResult<Option<Py<PyType>>> {
    let sys_mod = py.import_bound("sys")?;
    if let Ok(dict_attr) = sys_mod.getattr(COMMONIZE_DICT_NAME) {
        if let Ok(dict) = dict_attr.downcast::<PyDict>() {
            if let Some(res) = dict.get_item(tag)? {
                return Ok(Some(res.downcast::<PyType>()?.clone().unbind()));
            } else {
                dict.set_item(tag, cb())?;
                return Ok(None);
            }
        }
    }
    let new_dict = PyDict::new_bound(py);
    new_dict.set_item(tag, cb())?;
    sys_mod.setattr(COMMONIZE_DICT_NAME, new_dict)?;
    Ok(None)
}

/// ```
/// use pyo3::prelude::*;
/// use pyo3_commonize::{Commonized, commonize};
/// #[derive(Commonized)]
/// #[pyclass]
/// struct MyClass;
///
/// #[pymodule]
/// fn my_module(py: Python<'_>, m: Bound<'_, PyModule>) -> PyResult<()> {
///     commonize::<MyClass>(py)?;  //< should be called at first
///     m.add_class::<MyClass>()
/// }
/// ```
pub fn commonize<T: Commonized>(py: Python<'_>) -> PyResult<()> {
    let tag = generate_final_tag::<T>();
    if let Some(type_object) = find_type_object(py, tag, || {
        <T as PyClassImpl>::lazy_type_object()
            .get_or_init(py)
            .clone()
            .unbind()
    })? {
        set_type_obj::<T>(py, type_object);
    }
    Ok(())
}
