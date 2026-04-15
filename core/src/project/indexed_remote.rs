// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>

//! HTTP-backed project seeded with metadata advertised in `versions.json`.
//!
//! `IndexedRemoteProject` is the `ProjectRead`/`ProjectReadAsync` leaf
//! returned by [`crate::env::reqwest_index_http::HTTPIndexEnvironmentAsync`]
//! once a concrete version has been selected. Its shape follows directly
//! from the index protocol's split of responsibility:
//!
//! - **Advertised** (from `versions.json`, carried in
//!   [`crate::env::reqwest_index_http::AdvertisedVersion`]): `version`,
//!   `usage`, `project_digest`, `kpar_digest`, `kpar_size`. These are
//!   authoritative for candidate enumeration and lockfile population — the
//!   solver reads them via [`ProjectReadAsync::version_async`],
//!   [`ProjectReadAsync::usage_async`], and
//!   [`ProjectReadAsync::checksum_canonical_hex_async`] without triggering
//!   any additional network activity.
//!
//! - **Lazily fetched** (per-version `.project.json` / `.meta.json`, guarded
//!   by `fetched_info_meta`'s `OnceCell`): the real info/meta records used
//!   for lockfile materialization. Fetched once on first call to
//!   [`ProjectReadAsync::get_project_async`],
//!   [`ProjectReadAsync::get_info_async`], or
//!   [`ProjectReadAsync::get_meta_async`].
//!
//! - **Lazily fetched and verified** (the kpar archive, delegated to
//!   [`crate::project::reqwest_kpar_download::ReqwestKparDownloadedProject`]):
//!   fetched only on the first call to
//!   [`ProjectReadAsync::read_source_async`], which verifies the streamed
//!   body against the advertised `kpar_digest` before renaming into the
//!   verified path. Concurrent callers fan in to a single download via an
//!   internal lock.
//!
//! Reconciliation (drift detection):
//!
//! - [`IndexedRemoteProjectError::AdvertisedMetadataDrift`] surfaces when the
//!   lazily-fetched `.project.json` disagrees with `versions.json` on
//!   `version` or `usage` — versions.json is authoritative for solving, so
//!   such drift must be rejected rather than silently overriding the
//!   advertised view.
//! - [`IndexedRemoteProjectError::AdvertisedDigestDrift`] surfaces when the
//!   canonical digest computed from the fetched (info, meta) pair (and,
//!   post-download, from the canonical meta that may require reading
//!   sources) disagrees with the advertised `project_digest`. The check
//!   runs both pre- and post-download: pre-download is best-effort (skipped
//!   when canonicalizing would require a source read; see
//!   [`crate::project::canonical_project_digest_inline`]) and post-download
//!   is authoritative.

use std::sync::Arc;

use thiserror::Error;
use tokio::sync::OnceCell;

use crate::{
    auth::HTTPAuthentication,
    context::ProjectContext,
    env::reqwest_index_http::{AdvertisedVersion, HttpFetchError, MissingPolicy, fetch_json},
    lock::Source,
    model::{InterchangeProjectInfoRaw, InterchangeProjectMetadataRaw, InterchangeProjectUsageRaw},
    project::{
        CanonicalizationError, ProjectReadAsync, canonical_project_digest_inline,
        reqwest_kpar_download::{ReqwestKparDownloadedError, ReqwestKparDownloadedProject},
    },
};

#[derive(Debug)]
pub struct IndexedRemoteProject<Policy> {
    /// The kpar archive backend — field name tracks its role in this struct,
    /// type name tracks the transport.
    pub(crate) archive: ReqwestKparDownloadedProject<Policy>,
    /// Single source of truth for the protocol-advertised per-version
    /// metadata. All `version_async`/`usage_async`/`checksum_canonical_hex_async`
    /// accesses return slices/clones of these fields without any I/O.
    pub(crate) advertised: AdvertisedVersion,
    pub(crate) project_json_url: reqwest::Url,
    pub(crate) meta_json_url: reqwest::Url,
    pub(crate) fetched_info_meta:
        OnceCell<(InterchangeProjectInfoRaw, InterchangeProjectMetadataRaw)>,
}

#[derive(Error, Debug)]
pub enum IndexedRemoteProjectError {
    #[error(transparent)]
    Fetch(#[from] HttpFetchError),
    #[error(
        "project selection metadata at `{url}` disagrees with versions.json for `{field}`: \
         versions.json advertised `{advertised}` but `.project.json` had `{fetched}`"
    )]
    AdvertisedMetadataDrift {
        url: Box<str>,
        field: &'static str,
        advertised: String,
        fetched: String,
    },
    #[error(
        "project at `{url}` has locally-computed canonical digest `{computed}` \
         but the expected digest was `{expected}`"
    )]
    AdvertisedDigestDrift {
        url: Box<str>,
        expected: String,
        computed: String,
    },
    #[error(transparent)]
    Downloaded(#[from] ReqwestKparDownloadedError),
}

impl<Policy: HTTPAuthentication> IndexedRemoteProject<Policy> {
    /// Construct a project wrapper for a version that has already been
    /// selected out of `versions.json`. URL arguments are in
    /// archive → manifest → meta order (`kpar_url`, `project_json_url`,
    /// `meta_json_url`) so transposition across the three is hard to do
    /// silently.
    pub(crate) fn new(
        kpar_url: reqwest::Url,
        project_json_url: reqwest::Url,
        meta_json_url: reqwest::Url,
        advertised: AdvertisedVersion,
        client: reqwest_middleware::ClientWithMiddleware,
        auth_policy: Arc<Policy>,
    ) -> Result<Self, IndexedRemoteProjectError> {
        Ok(Self {
            archive: ReqwestKparDownloadedProject::new(kpar_url, client, auth_policy)?,
            advertised,
            project_json_url,
            meta_json_url,
            fetched_info_meta: OnceCell::new(),
        })
    }

    async fn fetch_required_json<T: serde::de::DeserializeOwned>(
        &self,
        url: reqwest::Url,
    ) -> Result<T, IndexedRemoteProjectError> {
        // RequirePresent: once a version has been selected, a 404 on the
        // per-version document is a hard error (unlike the `versions.json`
        // 404 at the env layer, which turns into `VersionNotInIndex`). The
        // `.expect` is valid because RequirePresent only returns `Ok(None)`
        // from `fetch_json` when the policy permits 404s — which it does not
        // here.
        Ok(fetch_json(
            &self.archive.client,
            &*self.archive.auth_policy,
            &url,
            MissingPolicy::RequirePresent,
        )
        .await?
        .expect("RequirePresent never returns Ok(None)"))
    }

    async fn fetched_project_async(
        &self,
    ) -> Result<
        &(InterchangeProjectInfoRaw, InterchangeProjectMetadataRaw),
        IndexedRemoteProjectError,
    > {
        self.fetched_info_meta
            .get_or_try_init(|| async {
                let (info, meta): (
                    InterchangeProjectInfoRaw,
                    InterchangeProjectMetadataRaw,
                ) = futures::try_join!(
                    self.fetch_required_json(self.project_json_url.clone()),
                    self.fetch_required_json(self.meta_json_url.clone()),
                )?;

                if info.version != self.advertised.version.to_string() {
                    return Err(IndexedRemoteProjectError::AdvertisedMetadataDrift {
                        url: self.project_json_url.as_str().into(),
                        field: "version",
                        advertised: self.advertised.version.to_string(),
                        fetched: info.version.clone(),
                    });
                }

                if info.usage != self.advertised.usage {
                    let to_json = |value: &Vec<InterchangeProjectUsageRaw>| -> String {
                        serde_json::to_string(value)
                            .expect("InterchangeProjectUsageRaw is always serializable")
                    };
                    return Err(IndexedRemoteProjectError::AdvertisedMetadataDrift {
                        url: self.project_json_url.as_str().into(),
                        field: "usage",
                        advertised: to_json(&self.advertised.usage),
                        fetched: to_json(&info.usage),
                    });
                }

                // Verify the advertised `project_digest` matches the
                // fetched `(info, meta)` pair, using the same canonical
                // digest definition as `checksum_canonical_hex_async` so
                // that the two reconciliations never disagree.
                //
                // Canonicalization of `meta.checksum` entries with
                // non-SHA256 algorithms requires reading source files from
                // the kpar, which we haven't downloaded yet. In that case
                // the inline helper returns `None` and we defer verification
                // to `checksum_canonical_hex_async` after the archive has
                // been fetched — that path still catches drift, just later.
                if let Some(hash) = canonical_project_digest_inline(&info, &meta) {
                    let computed = format!("{:x}", hash);
                    if computed != self.advertised.project_digest.as_hex() {
                        return Err(IndexedRemoteProjectError::AdvertisedDigestDrift {
                            url: self.project_json_url.as_str().into(),
                            expected: self.advertised.project_digest.as_hex().to_string(),
                            computed,
                        });
                    }
                }

                Ok((info, meta))
            })
            .await
    }
}

impl<Policy: HTTPAuthentication> ProjectReadAsync for IndexedRemoteProject<Policy> {
    type Error = IndexedRemoteProjectError;

    async fn get_project_async(
        &self,
    ) -> Result<
        (
            Option<InterchangeProjectInfoRaw>,
            Option<InterchangeProjectMetadataRaw>,
        ),
        Self::Error,
    > {
        let (info, meta) = self.fetched_project_async().await?;
        Ok((Some(info.clone()), Some(meta.clone())))
    }

    type SourceReader<'a>
        = <ReqwestKparDownloadedProject<Policy> as ProjectReadAsync>::SourceReader<'a>
    where
        Self: 'a;

    async fn read_source_async<P: AsRef<typed_path::Utf8UnixPath>>(
        &self,
        path: P,
    ) -> Result<Self::SourceReader<'_>, Self::Error> {
        self.archive
            .ensure_downloaded_verified(self.advertised.kpar_digest.as_hex())
            .await?;

        self.archive
            .read_source_async(path)
            .await
            .map_err(Into::into)
    }

    async fn sources_async(&self, _ctx: &ProjectContext) -> Result<Vec<Source>, Self::Error> {
        Ok(vec![Source::RemoteKpar {
            remote_kpar: self.archive.url.to_string(),
            remote_kpar_size: Some(self.advertised.kpar_size),
        }])
    }

    async fn get_info_async(&self) -> Result<Option<InterchangeProjectInfoRaw>, Self::Error> {
        Ok(Some(self.fetched_project_async().await?.0.clone()))
    }

    async fn get_meta_async(&self) -> Result<Option<InterchangeProjectMetadataRaw>, Self::Error> {
        Ok(Some(self.fetched_project_async().await?.1.clone()))
    }

    async fn version_async(&self) -> Result<Option<String>, Self::Error> {
        Ok(Some(self.advertised.version.to_string()))
    }

    async fn usage_async(&self) -> Result<Option<Vec<InterchangeProjectUsageRaw>>, Self::Error> {
        Ok(Some(self.advertised.usage.clone()))
    }

    async fn checksum_canonical_hex_async(
        &self,
    ) -> Result<Option<String>, CanonicalizationError<Self::Error>> {
        if !self.archive.is_downloaded() {
            return Ok(Some(self.advertised.project_digest.as_hex().to_string()));
        }

        let computed = self
            .archive
            .checksum_canonical_hex_async()
            .await
            .map_err(|e| e.map_project_read(IndexedRemoteProjectError::Downloaded))?;

        if let Some(computed_hex) = computed.as_ref()
            && computed_hex.as_str() != self.advertised.project_digest.as_hex()
        {
            return Err(CanonicalizationError::ProjectRead(
                IndexedRemoteProjectError::AdvertisedDigestDrift {
                    url: self.project_json_url.as_str().into(),
                    expected: self.advertised.project_digest.as_hex().to_string(),
                    computed: computed_hex.clone(),
                },
            ));
        }

        Ok(computed)
    }
}
