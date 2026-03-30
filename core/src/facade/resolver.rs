// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Resolver assembly helpers for commands that need dependency resolution.
//!
//! Builds the standard 3-level priority resolver:
//! 1. Config overrides (from sysand.toml)
//! 2. Stdlib memory resolver
//! 3. Standard resolver (file, env, HTTP, git, index)

#[cfg(all(feature = "filesystem", feature = "networking"))]
pub use imp::*;

#[cfg(all(feature = "filesystem", feature = "networking"))]
mod imp {
    use std::collections::HashMap;
    use camino::Utf8Path;
    use fluent_uri::Iri;

    use crate::auth::HTTPAuthentication;
    use crate::config::Config;
    use crate::env::local_directory::DEFAULT_ENV_NAME;
    use crate::error::{ErrorCode, SysandError};
    use crate::project::any::{AnyProject, OverrideProject};
    use crate::project::memory::InMemoryProject;
    use crate::project::reference::ProjectReference;
    use crate::project::utils::wrapfs;
    use crate::resolve::memory::{AcceptAll, MemoryResolver};
    use crate::resolve::priority::PriorityResolver;
    use crate::resolve::standard::{StandardResolver, standard_resolver};
    use crate::types::network::NetworkContext;

    /// The concrete resolver type returned by [`build_resolver`].
    pub type DefaultResolver<Policy> = PriorityResolver<
        PriorityResolver<
            MemoryResolver<AcceptAll, ProjectReference<AnyProject<Policy>>>,
            MemoryResolver<AcceptAll, InMemoryProject>,
        >,
        StandardResolver<Policy>,
    >;

    /// Build the standard 3-level priority resolver from a `NetworkContext`.
    ///
    /// `project_path` is used to locate `sysand_env/` for local resolution.
    /// `index_urls` are the resolved index URLs (or None to disable indexes).
    /// `provided_iris` are IRIs to treat as pre-resolved (stdlib exclusions).
    pub fn build_resolver<Policy: HTTPAuthentication>(
        project_path: &Utf8Path,
        net: &NetworkContext<Policy>,
        index_urls: Option<Vec<url::Url>>,
        provided_iris: HashMap<String, Vec<InMemoryProject>>,
    ) -> Result<DefaultResolver<Policy>, SysandError> {
        let local_env_path = project_path.join(DEFAULT_ENV_NAME);

        let overrides = build_overrides(&net.config, project_path, net)?;

        let mut memory_projects = HashMap::default();
        for (k, v) in provided_iris {
            let iri = Iri::parse(k)
                .map_err(|(e, _)| SysandError::new(ErrorCode::IriInvalid, e.to_string()))?;
            memory_projects.insert(iri, v);
        }

        let override_resolver = PriorityResolver::new(
            MemoryResolver::from(overrides),
            MemoryResolver {
                iri_predicate: AcceptAll {},
                projects: memory_projects,
            },
        );

        let local_env = if wrapfs::is_dir(&local_env_path).map_err(SysandError::from)? {
            Some(local_env_path)
        } else {
            None
        };

        let wrapped = PriorityResolver::new(
            override_resolver,
            standard_resolver(
                None,
                local_env,
                Some(net.client.clone()),
                index_urls,
                net.runtime.clone(),
                net.auth.clone(),
            ),
        );

        Ok(wrapped)
    }

    /// Resolve index URLs from config + overrides.
    pub fn resolve_index_urls(
        config: &Config,
        extra_indexes: Vec<String>,
        default_indexes: Vec<String>,
        index_mode: &str,
        default_url: &str,
    ) -> Result<Option<Vec<url::Url>>, SysandError> {
        if index_mode == "none" {
            return Ok(None);
        }
        let urls = config
            .index_urls(extra_indexes, vec![default_url.to_string()], default_indexes)
            .map_err(|e| SysandError::new(ErrorCode::ConfigInvalid, e.to_string()))?;
        Ok(Some(urls))
    }

    fn build_overrides<Policy: HTTPAuthentication>(
        config: &Config,
        project_root: &Utf8Path,
        net: &NetworkContext<Policy>,
    ) -> Result<Vec<(Iri<String>, Vec<OverrideProject<Policy>>)>, SysandError> {
        let mut overrides = Vec::new();
        for config_project in &config.projects {
            for identifier in &config_project.identifiers {
                let mut projects = Vec::new();
                for source in &config_project.sources {
                    projects.push(ProjectReference::new(
                        AnyProject::try_from_source(
                            source.clone(),
                            project_root,
                            net.auth.clone(),
                            net.client.clone(),
                            net.runtime.clone(),
                        )
                        .map_err(|e| {
                            SysandError::new(ErrorCode::ConfigInvalid, e.to_string())
                        })?,
                    ));
                }
                let iri: Iri<String> = Iri::parse(identifier.clone())
                    .map_err(|(e, _)| SysandError::new(ErrorCode::IriInvalid, e.to_string()))?;
                overrides.push((iri.into(), projects));
            }
        }
        Ok(overrides)
    }
}
