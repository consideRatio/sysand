use anyhow::{Result, anyhow, bail};
use camino::{Utf8Path, Utf8PathBuf};
use fluent_uri::Iri;
use semver::Version;

use std::{collections::HashMap, fs, io::ErrorKind, sync::Arc};

use sysand_core::{
    auth::HTTPAuthentication,
    config::Config,
    context::ProjectContext,
    env::utils::clone_project,
    project::{ProjectRead, local_src::LocalSrcProject, utils::wrapfs},
    resolve::{ResolutionOutcome, ResolveRead, standard::standard_resolver},
    types::network::NetworkContext,
};

use crate::{
    CliError, DEFAULT_INDEX_URL,
    cli::{CloneProjectLocatorArgs, ResolutionOptions},
};

pub enum ProjectLocator {
    Iri(Iri<String>),
    Path(Utf8PathBuf),
}

/// Clones project from `locator` to `target` directory.
#[allow(clippy::too_many_arguments)]
pub fn command_clone<Policy: HTTPAuthentication>(
    locator: CloneProjectLocatorArgs,
    version: Option<String>,
    target: Option<Utf8PathBuf>,
    ctx: ProjectContext,
    no_deps: bool,
    resolution_opts: ResolutionOptions,
    config: &Config,
    client: reqwest_middleware::ClientWithMiddleware,
    runtime: Arc<tokio::runtime::Runtime>,
    auth_policy: Arc<Policy>,
) -> Result<()> {
    let target: Utf8PathBuf = target.unwrap_or_else(|| ".".into());
    let project_path = {
        let canonical = wrapfs::absolute(&target)?;
        match fs::read_dir(&target) {
            Ok(mut dir_it) => {
                if dir_it.next().is_some() {
                    bail!("target directory not empty: `{}`", canonical)
                }
            }
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    wrapfs::create_dir_all(&canonical)?;
                }
                ErrorKind::NotADirectory => {
                    bail!("target path `{}` is not a directory", canonical)
                }
                e => {
                    bail!("failed to get metadata for `{}`: {}", canonical, e);
                }
            },
        }
        canonical
    };

    let include_std = resolution_opts.include_std;
    let index_urls = if resolution_opts.index_mode == "none" {
        None
    } else {
        Some(config.index_urls(
            resolution_opts.index,
            vec![DEFAULT_INDEX_URL.to_string()],
            resolution_opts.default_index,
        )?)
    };

    // Warn about existing project/workspace
    if let Some(existing_project) = &ctx.current_project {
        log::warn!(
            "found an existing project in one of target path's parent\n\
            {:>8} directories `{}`",
            ' ',
            existing_project.root_path()
        );
    }
    if let Some(existing_workspace) = &ctx.current_workspace {
        log::warn!(
            "found an existing workspace in one of target path's parent\n\
            {:>8} directories `{}`",
            ' ',
            existing_workspace.root_path()
        );
    }

    // Parse locator
    let CloneProjectLocatorArgs {
        auto_location,
        iri,
        path,
    } = locator;
    let locator = if let Some(auto_location) = auto_location {
        match fluent_uri::Iri::parse(auto_location) {
            Ok(iri) => ProjectLocator::Iri(iri),
            Err((_e, path)) => ProjectLocator::Path(path.into()),
        }
    } else if let Some(path) = path {
        ProjectLocator::Path(path)
    } else if let Some(iri) = iri {
        ProjectLocator::Iri(iri)
    } else {
        unreachable!()
    };

    let header = sysand_core::style::get_style_config().header;
    let cloning = "Cloning";
    let cloned = "Cloned";

    let mut local_project = LocalSrcProject {
        nominal_path: None,
        project_path: project_path.clone(),
    };

    let std_resolver = standard_resolver(
        None,
        None,
        Some(client.clone()),
        index_urls,
        runtime.clone(),
        auth_policy.clone(),
    );

    // Clone the project files
    let fetch_result: Result<(ProjectLocator, _)> = match &locator {
        ProjectLocator::Iri(iri) => {
            log::info!(
                "{header}{cloning:>12}{header:#} project with IRI `{}` to\n\
                {:>12} `{}`",
                iri, ' ', local_project.project_path,
            );
            let (_version, storage) = get_project_version(iri, version, &std_resolver)?;
            let (info, _meta) = clone_project(&storage, &mut local_project, true)?;
            log::info!("{header}{cloned:>12}{header:#} `{}` {}", info.name, info.version);
            Ok((locator, std_resolver))
        }
        ProjectLocator::Path(path) => {
            let remote_project = LocalSrcProject {
                nominal_path: None,
                project_path: path.into(),
            };
            if let Some(version) = version {
                let project_version = remote_project
                    .get_info()?
                    .ok_or_else(|| anyhow!("missing project info"))?
                    .version;
                if version != project_version {
                    bail!("given version {version} does not match project version {project_version}")
                }
            }
            log::info!(
                "{header}{cloning:>12}{header:#} project from `{}` to\n\
                {:>12} `{}`",
                wrapfs::canonicalize(&remote_project.project_path)?,
                ' ',
                local_project.project_path,
            );
            let (info, _meta) = clone_project(&remote_project, &mut local_project, true)?;
            log::info!("{header}{cloned:>12}{header:#} `{}` {}", info.name, info.version);
            Ok((locator, std_resolver))
        }
    };

    let (locator, _std_resolver) = match fetch_result {
        Ok(r) => r,
        Err(e) => {
            clean_dir(&target);
            return Err(e);
        }
    };

    // Resolve and install deps via facade
    if !no_deps {
        let net = NetworkContext::with_client(config.clone(), auth_policy, client, runtime);
        let provided_iris = if !include_std {
            sysand_core::stdlib::known_std_libs()
        } else {
            HashMap::default()
        };

        let identifiers = match locator {
            ProjectLocator::Iri(iri) => Some(vec![iri]),
            _ => None,
        };

        sysand_core::facade::clone::clone_with_deps(
            &project_path,
            identifiers,
            &net,
            &provided_iris,
        )?;
    }

    Ok(())
}

/// Obtains a project identified by `iri` via `resolver`.
pub fn get_project_version<R: ResolveRead>(
    iri: &Iri<String>,
    version: Option<String>,
    resolver: &R,
) -> Result<(semver::Version, R::ProjectStorage), anyhow::Error> {
    match resolver.resolve_read(iri)? {
        ResolutionOutcome::Resolved(alternatives) => {
            let requested_version = version
                .as_ref()
                .map(|v| {
                    semver::Version::parse(v)
                        .map_err(|e| anyhow!("failed to parse given version {v} as SemVer: {e}"))
                })
                .transpose()?;
            let mut candidates = Vec::new();
            for alt in alternatives {
                let candidate_project = match alt {
                    Ok(cp) => cp,
                    Err(e) => {
                        log::debug!("skipping candidate project: {e}");
                        continue;
                    }
                };
                let maybe_info = match candidate_project.get_info() {
                    Ok(mi) => mi,
                    Err(e) => {
                        log::debug!("skipping candidate project, failed to get info: {e}");
                        continue;
                    }
                };
                let info = match maybe_info {
                    Some(info) => info,
                    None => {
                        log::debug!("skipping candidate project with missing info");
                        continue;
                    }
                };
                let candidate_version = match Version::parse(&info.version) {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!(
                            "skipping candidate project with invalid SemVer version {}: {e}",
                            &info.version
                        );
                        continue;
                    }
                };
                if let Some(version) = &requested_version
                    && &candidate_version != version
                {
                    continue;
                }
                candidates.push((candidate_version, candidate_project));
            }

            match candidates.len() {
                0 => match version {
                    Some(v) => bail!(CliError::MissingProjectVersion(iri.as_ref().to_string(), v)),
                    None => bail!(CliError::MissingProject(iri.as_ref().to_string())),
                },
                1 => Ok(candidates.pop().unwrap()),
                _ => {
                    let max_v = candidates
                        .into_iter()
                        .max_by(|(v1, _), (v2, _)| v1.cmp(v2))
                        .unwrap();
                    Ok(max_v)
                }
            }
        }
        ResolutionOutcome::UnsupportedIRIType(e) => bail!(
            "IRI scheme `{}` of `{}` is not supported: {e}",
            iri.scheme(),
            iri
        ),
        ResolutionOutcome::Unresolvable(e) => {
            bail!("failed to resolve project `{iri}`: {e}")
        }
    }
}

fn clean_dir<P: AsRef<Utf8Path>>(path: P) {
    let Ok(entries) = fs::read_dir(path.as_ref()) else {
        return;
    };
    log::debug!("clearing contents of dir `{}`", path.as_ref());
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let Ok(entry_type) = entry.file_type() else {
            continue;
        };
        if entry_type.is_dir() {
            let _ = fs::remove_dir_all(&path);
        } else {
            let _ = fs::remove_file(&path);
        }
    }
}
