// SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::process::ExitCode;

use camino::Utf8PathBuf;
use pyo3::prelude::*;
use sysand_core::{
    SysandError,
    env::local_directory::{DEFAULT_ENV_NAME, LocalDirectoryEnvironment},
    project::{
        ProjectRead as _,
        local_kpar::LocalKParProject,
        local_src::LocalSrcProject,
        utils::wrapfs,
    },
    env::{WriteEnvironment, utils::clone_project},
    types::{
        enums::{ChecksumMode, Compression, IndexSymbols, Language},
        options::{BuildOptions, InitOptions, SourceAddOptions},
    },
};

// ---------------------------------------------------------------------------
// Unified error conversion
// ---------------------------------------------------------------------------

/// Custom Python exception class for all sysand errors.
/// Has `code`, `message`, and `context` attributes.
pyo3::create_exception!(sysand, SysandPyError, pyo3::exceptions::PyException);

fn sysand_err(err: SysandError) -> PyErr {
    let msg = format!("[{}] {}", err.code, err.message);
    let py_err = SysandPyError::new_err(msg);
    // TODO: set .code, .message, .context attributes on the exception
    // once we register the exception class properly
    py_err
}

// ---------------------------------------------------------------------------
// CLI escape hatch
// ---------------------------------------------------------------------------

#[pyfunction(name = "_run_cli")]
fn run_cli(args: Vec<String>) -> PyResult<bool> {
    let exit_code = sysand::lib_main(args);
    Ok(exit_code == ExitCode::SUCCESS)
}

// ---------------------------------------------------------------------------
// Root commands
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(signature = (path, *, name=None, publisher=None, version=None, license=None, allow_non_spdx=false))]
fn init(
    path: String,
    name: Option<String>,
    publisher: Option<String>,
    version: Option<String>,
    license: Option<String>,
    allow_non_spdx: bool,
) -> PyResult<()> {
    let _ = pyo3_log::try_init();
    let mut project = LocalSrcProject {
        nominal_path: None,
        project_path: Utf8PathBuf::from(&path),
    };
    sysand_core::facade::init::init(
        &mut project,
        InitOptions {
            name,
            publisher,
            version,
            license,
            allow_non_spdx,
        },
    )
    .map_err(sysand_err)
}

#[pyfunction]
fn locate(path: String) -> PyResult<String> {
    let _ = pyo3_log::try_init();
    sysand_core::facade::locate::locate(camino::Utf8Path::new(&path))
        .map(|p| p.to_string())
        .map_err(sysand_err)
}

#[pyfunction]
#[pyo3(signature = (project_path, output_path, *, compression=None))]
fn build(
    project_path: String,
    output_path: String,
    compression: Option<String>,
) -> PyResult<()> {
    let _ = pyo3_log::try_init();
    let project = LocalSrcProject {
        nominal_path: None,
        project_path: project_path.into(),
    };
    let compression = match compression.as_deref() {
        Some("stored") => Compression::Stored,
        Some("deflated") | None => Compression::Deflated,
        Some("bzip2") => Compression::Bzip2,
        Some("zstd") => Compression::Zstd,
        Some("xz") => Compression::Xz,
        Some("ppmd") => Compression::Ppmd,
        Some(other) => {
            return Err(SysandPyError::new_err(format!(
                "unknown compression: {other}"
            )));
        }
    };
    sysand_core::facade::build::build(
        &project,
        camino::Utf8Path::new(&output_path),
        BuildOptions {
            compression,
            ..Default::default()
        },
    )
    .map(|_| ())
    .map_err(sysand_err)
}

// ---------------------------------------------------------------------------
// source namespace
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(signature = (project_path, src_path, *, checksum=false, index_symbols=true, language=None))]
fn source_add(
    project_path: String,
    src_path: String,
    checksum: bool,
    index_symbols: bool,
    language: Option<String>,
) -> PyResult<()> {
    let _ = pyo3_log::try_init();
    let mut project = LocalSrcProject {
        nominal_path: None,
        project_path: project_path.into(),
    };
    let lang = match language.as_deref() {
        Some("sysml") => Language::Sysml,
        Some("kerml") => Language::Kerml,
        None | Some("auto") => Language::Auto,
        Some(other) => {
            return Err(SysandPyError::new_err(format!(
                "unknown language: {other}"
            )));
        }
    };
    sysand_core::facade::source::add(
        &mut project,
        typed_path::Utf8UnixPath::new(&src_path),
        SourceAddOptions {
            checksum: if checksum {
                ChecksumMode::Sha256
            } else {
                ChecksumMode::None
            },
            index_symbols: if index_symbols {
                IndexSymbols::On
            } else {
                IndexSymbols::Off
            },
            language: lang,
        },
    )
    .map_err(sysand_err)
}

#[pyfunction]
fn source_remove(project_path: String, src_path: String) -> PyResult<()> {
    let _ = pyo3_log::try_init();
    let mut project = LocalSrcProject {
        nominal_path: None,
        project_path: project_path.into(),
    };
    sysand_core::facade::source::remove(
        &mut project,
        typed_path::Utf8UnixPath::new(&src_path),
    )
    .map_err(sysand_err)
}

// ---------------------------------------------------------------------------
// usage namespace
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(signature = (project_path, iri, version_req=None))]
fn usage_add(
    project_path: String,
    iri: String,
    version_req: Option<String>,
) -> PyResult<()> {
    let _ = pyo3_log::try_init();
    let mut project = LocalSrcProject {
        nominal_path: None,
        project_path: project_path.into(),
    };
    sysand_core::facade::usage::add(
        &mut project,
        &iri,
        version_req.as_deref(),
    )
    .map(|_| ())
    .map_err(sysand_err)
}

#[pyfunction]
fn usage_remove(project_path: String, iri: String) -> PyResult<()> {
    let _ = pyo3_log::try_init();
    let mut project = LocalSrcProject {
        nominal_path: None,
        project_path: project_path.into(),
    };
    sysand_core::facade::usage::remove(&mut project, &iri).map_err(sysand_err)
}

// ---------------------------------------------------------------------------
// env namespace
// ---------------------------------------------------------------------------

#[pyfunction]
fn env_create(path: String) -> PyResult<()> {
    let _ = pyo3_log::try_init();
    sysand_core::facade::env::create(camino::Utf8Path::new(&path)).map_err(sysand_err)
}

#[pyfunction]
fn env_list(env_path: String) -> PyResult<Vec<(String, Option<String>)>> {
    let _ = pyo3_log::try_init();
    let env = LocalDirectoryEnvironment {
        environment_path: env_path.into(),
    };
    let entries = sysand_core::facade::env::list(env).map_err(sysand_err)?;
    Ok(entries.into_iter().map(|e| (e.iri, e.version)).collect())
}

#[pyfunction]
#[pyo3(signature = (env_path, iri, version=None))]
fn env_uninstall(env_path: String, iri: String, version: Option<String>) -> PyResult<()> {
    let _ = pyo3_log::try_init();
    let env = LocalDirectoryEnvironment {
        environment_path: env_path.into(),
    };
    sysand_core::facade::env::uninstall(env, &iri, version.as_deref()).map_err(sysand_err)
}

#[pyfunction]
fn env_install_path(env_path: String, iri: String, location: String) -> PyResult<()> {
    let _ = pyo3_log::try_init();
    let location: Utf8PathBuf = location.into();
    let mut env = LocalDirectoryEnvironment {
        environment_path: env_path.into(),
    };

    let metadata =
        wrapfs::metadata(&location).map_err(|e| SysandPyError::new_err(e.to_string()))?;

    if metadata.is_file() {
        let project = LocalKParProject::new_guess_root(&location)
            .map_err(|e| SysandPyError::new_err(e.to_string()))?;
        let version = project
            .version()
            .map_err(|e| SysandPyError::new_err(e.to_string()))?
            .ok_or_else(|| {
                SysandPyError::new_err(format!("project at `{location}` lacks version"))
            })?;
        env.put_project(iri, version, |to| {
            clone_project(&project, to, true).map(|_| ())
        })
        .map_err(|e| SysandPyError::new_err(e.to_string()))?;
    } else if metadata.is_dir() {
        let project = LocalSrcProject {
            nominal_path: None,
            project_path: location.clone(),
        };
        let version = project
            .version()
            .map_err(|e| SysandPyError::new_err(e.to_string()))?
            .ok_or_else(|| {
                SysandPyError::new_err(format!("project at `{location}` lacks version"))
            })?;
        env.put_project(iri, version, |to| {
            clone_project(&project, to, true).map(|_| ())
        })
        .map_err(|e| SysandPyError::new_err(e.to_string()))?;
    } else {
        return Err(SysandPyError::new_err(format!(
            "unable to find project at `{location}`"
        )));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Kept from old API for backward compatibility (info, sources)
// These are not part of the new facade but tests depend on them.
// ---------------------------------------------------------------------------

#[pyfunction]
fn info_path(
    path: String,
) -> PyResult<
    Option<(
        sysand_core::model::InterchangeProjectInfoRaw,
        sysand_core::model::InterchangeProjectMetadataRaw,
    )>,
> {
    let _ = pyo3_log::try_init();
    let project = LocalSrcProject {
        nominal_path: None,
        project_path: path.into(),
    };
    Ok(sysand_core::info::do_info_project(&project))
}

#[pyfunction]
#[pyo3(signature = (uri, relative_file_root, index_urls))]
fn info_remote(
    py: Python,
    uri: String,
    relative_file_root: String,
    index_urls: Option<Vec<String>>,
) -> PyResult<
    Vec<(
        sysand_core::model::InterchangeProjectInfoRaw,
        sysand_core::model::InterchangeProjectMetadataRaw,
    )>,
> {
    let _ = pyo3_log::try_init();
    py.detach(|| {
        let client = sysand_core::resolve::net_utils::create_reqwest_client()
            .map_err(|e| SysandPyError::new_err(e.to_string()))?;
        let runtime = std::sync::Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
        );
        let index_url = index_urls
            .map(|urls| {
                urls.iter()
                    .map(|u| url::Url::parse(u))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()
            .map_err(|e| SysandPyError::new_err(e.to_string()))?;
        let resolver = sysand_core::resolve::standard::standard_resolver(
            Some(relative_file_root.into()),
            None,
            Some(client),
            index_url,
            runtime,
            std::sync::Arc::new(sysand_core::auth::Unauthenticated {}),
        );
        let mut results = vec![];
        match sysand_core::info::do_info(&uri, &resolver) {
            Ok(matches) => results.extend(matches),
            Err(sysand_core::info::InfoError::NoResolve(..)) => {}
            Err(e) => return Err(SysandPyError::new_err(e.to_string())),
        };
        Ok(results)
    })
}

#[pyfunction]
#[pyo3(signature = (env_path, iri, version, include_deps, include_std))]
fn sources_env(
    env_path: String,
    iri: String,
    version: Option<String>,
    include_deps: bool,
    include_std: bool,
) -> PyResult<Vec<String>> {
    let _ = pyo3_log::try_init();
    use sysand_core::{
        sources::{do_sources_local_src_project_no_deps, find_project_dependencies},
        stdlib::known_std_libs,
    };

    let provided_iris = if !include_std {
        known_std_libs()
    } else {
        std::collections::HashMap::default()
    };
    let version = version
        .map(|v| semver::VersionReq::parse(&v))
        .transpose()
        .map_err(|e| SysandPyError::new_err(e.to_string()))?;

    let env = LocalDirectoryEnvironment {
        environment_path: env_path.into(),
    };

    use sysand_core::env::ReadEnvironment as _;
    let mut projects = env
        .candidate_projects(&iri)
        .map_err(|e| SysandPyError::new_err(e.to_string()))?
        .into_iter();

    let project = match &version {
        None => projects.next(),
        Some(vr) => loop {
            if let Some(candidate) = projects.next() {
                if let Some(v) = candidate
                    .version()
                    .map_err(|e| SysandPyError::new_err(e.to_string()))?
                    .and_then(|x| semver::Version::parse(&x).ok())
                    && vr.matches(&v)
                {
                    break Some(candidate);
                }
            } else {
                break None;
            }
        },
    }
    .ok_or_else(|| SysandPyError::new_err(format!("project `{iri}` not found in environment")))?;

    let mut result = vec![];
    for p in do_sources_local_src_project_no_deps(&project, true)
        .map_err(|e| SysandPyError::new_err(e.to_string()))?
    {
        result.push(p.into_string());
    }

    if include_deps {
        let info = project
            .get_info()
            .map_err(|e| SysandPyError::new_err(e.to_string()))?
            .ok_or_else(|| SysandPyError::new_err("missing project information"))?;
        for dep in find_project_dependencies(
            info.validate()
                .map_err(|e| SysandPyError::new_err(e.to_string()))?
                .usage,
            env,
            &provided_iris,
        )
        .map_err(|e| SysandPyError::new_err(e.to_string()))?
        {
            for p in do_sources_local_src_project_no_deps(&dep, true)
                .map_err(|e| SysandPyError::new_err(e.to_string()))?
            {
                result.push(p.into_string());
            }
        }
    }
    Ok(result)
}

#[pyfunction]
#[pyo3(signature = (path, include_deps, env_path, include_std))]
fn sources_project(
    path: String,
    include_deps: bool,
    env_path: Option<String>,
    include_std: bool,
) -> PyResult<Vec<String>> {
    let _ = pyo3_log::try_init();
    use sysand_core::{
        sources::{do_sources_local_src_project_no_deps, find_project_dependencies},
        stdlib::known_std_libs,
    };

    let project = LocalSrcProject {
        nominal_path: None,
        project_path: path.into(),
    };
    let mut result = vec![];
    for p in do_sources_local_src_project_no_deps(&project, true)
        .map_err(|e| SysandPyError::new_err(e.to_string()))?
    {
        result.push(p.into_string());
    }

    if include_deps {
        let info = project
            .get_info()
            .map_err(|e| SysandPyError::new_err(e.to_string()))?
            .ok_or_else(|| SysandPyError::new_err("missing project information"))?;
        let env_path = env_path.ok_or_else(|| {
            SysandPyError::new_err("unable to identify local environment")
        })?;
        let provided_iris = if !include_std {
            known_std_libs()
        } else {
            std::collections::HashMap::default()
        };
        let env = LocalDirectoryEnvironment {
            environment_path: env_path.into(),
        };
        for dep in find_project_dependencies(
            info.validate()
                .map_err(|e| SysandPyError::new_err(e.to_string()))?
                .usage,
            env,
            &provided_iris,
        )
        .map_err(|e| SysandPyError::new_err(e.to_string()))?
        {
            for p in do_sources_local_src_project_no_deps(&dep, true)
                .map_err(|e| SysandPyError::new_err(e.to_string()))?
            {
                result.push(p.into_string());
            }
        }
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

#[pymodule(name = "_sysand_core")]
pub fn sysand_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Exception class
    m.add("SysandError", m.py().get_type::<SysandPyError>())?;

    // CLI escape hatch
    m.add_function(wrap_pyfunction!(run_cli, m)?)?;

    // Root commands (new facade)
    m.add_function(wrap_pyfunction!(init, m)?)?;
    m.add_function(wrap_pyfunction!(locate, m)?)?;
    m.add_function(wrap_pyfunction!(build, m)?)?;

    // source namespace
    m.add_function(wrap_pyfunction!(source_add, m)?)?;
    m.add_function(wrap_pyfunction!(source_remove, m)?)?;

    // usage namespace
    m.add_function(wrap_pyfunction!(usage_add, m)?)?;
    m.add_function(wrap_pyfunction!(usage_remove, m)?)?;

    // env namespace
    m.add_function(wrap_pyfunction!(env_create, m)?)?;
    m.add_function(wrap_pyfunction!(env_list, m)?)?;
    m.add_function(wrap_pyfunction!(env_uninstall, m)?)?;
    m.add_function(wrap_pyfunction!(env_install_path, m)?)?;

    // Backward-compat (info, sources — not in facade)
    m.add_function(wrap_pyfunction!(info_path, m)?)?;
    m.add_function(wrap_pyfunction!(info_remote, m)?)?;
    m.add_function(wrap_pyfunction!(sources_env, m)?)?;
    m.add_function(wrap_pyfunction!(sources_project, m)?)?;

    // Constants
    m.add("DEFAULT_ENV_NAME", DEFAULT_ENV_NAME)?;

    Ok(())
}
