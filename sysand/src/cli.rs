// SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt::Display;

use camino::Utf8PathBuf;
use clap::{ValueEnum, crate_authors};
use fluent_uri::Iri;
use sysand_core::build::KparCompressionMethod;

use crate::env_vars;

/// A package manager for SysML v2 and KerML
///
/// Documentation:
/// <https://docs.sysand.org/>
/// Package index and more information:
/// <https://beta.sysand.org/>
/// Project repository:
/// <https://github.com/sensmetry/sysand/>
#[derive(clap::Parser, Debug)]
#[command(
    version,
    long_about,
    verbatim_doc_comment,
    arg_required_else_help = true,
    disable_help_flag = true,
    disable_version_flag = true,
    styles=crate::style::STYLING,
    author = crate_authors!(",\n"),
    help_template = "\
{before-help}{about-with-newline}
{usage-heading} {usage}

{all-args}

{name} v{version}
Developed by: {author-with-newline}
{after-help}"
)]
pub struct Args {
    #[command(flatten)]
    pub global_opts: GlobalOptions,

    #[command(subcommand)]
    pub command: Command,

    /// Display the sysand version.
    #[arg(short = 'V', long, action = clap::ArgAction::Version)]
    version: Option<bool>,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Command {
    /// Create a new project
    Init {
        /// The path to use for the project. Defaults to current directory
        path: Option<String>,
        /// The name of the project. Defaults to the directory name
        #[arg(long)]
        name: Option<String>,
        /// The publisher of the project. Defaults to `untitled`
        #[arg(long)]
        publisher: Option<String>,
        /// Set the version in SemVer 2.0 format. Defaults to `0.0.1`
        #[arg(long)]
        version: Option<String>,
        /// Set the license in the form of an SPDX license identifier.
        /// Defaults to omitting the license field
        #[arg(long, alias = "licence", verbatim_doc_comment)]
        license: Option<String>,
        /// Allow non-SPDX license expressions
        #[arg(long, requires = "license")]
        allow_non_spdx: bool,
    },
    /// Find the project root directory
    Locate,
    /// Clone a project to a specified directory.
    /// Equivalent to manually downloading, extracting the
    /// project to the directory and running `sysand env sync`
    #[clap(verbatim_doc_comment)]
    Clone {
        #[clap(flatten)]
        locator: CloneProjectLocatorArgs,
        /// Path to clone the project into. If already exists, must
        /// be an empty directory. Defaults to current directory
        #[arg(long, short, default_value = None, verbatim_doc_comment)]
        target: Option<Utf8PathBuf>,
        /// Version of the project to clone. Defaults to the latest
        /// version according to SemVer 2.0
        #[arg(long, short = 'V', verbatim_doc_comment)]
        version: Option<String>,
        /// Whether to resolve and install dependencies
        #[arg(long, default_value = "all", value_parser = ["all", "none"])]
        deps: String,
        #[command(flatten)]
        resolution_opts: ResolutionOptions,
    },
    /// Build a KerML Project Archive (KPAR). If executed in a workspace
    /// outside of a project, builds all projects in the workspace.
    #[clap(verbatim_doc_comment)]
    Build {
        /// Path for the finished KPAR or KPARs
        #[clap(verbatim_doc_comment)]
        path: Option<Utf8PathBuf>,
        /// Method to compress the files in the KPAR
        #[arg(short = 'c', long, default_value_t, value_enum)]
        compression: KparCompressionMethodCli,
        /// Allow usages of local paths (`file://`)
        #[arg(long, short, default_value_t = false, verbatim_doc_comment)]
        allow_path_usage: bool,
    },
    /// Manage project source files
    #[command(subcommand)]
    Source(SourceCommand),
    /// Manage project usages (dependencies)
    #[command(subcommand)]
    Usage(UsageCommand),
    /// Lockfile operations
    #[command(subcommand)]
    Lock(LockCommand),
    /// Environment management
    Env {
        #[command(subcommand)]
        command: Option<EnvCommand>,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SourceCommand {
    /// Add model interchange files to project metadata
    Add {
        /// File(s) to include in the project
        #[arg(num_args = 1..)]
        paths: Vec<Utf8PathBuf>,
        /// Compute and add each file's SHA256 checksum
        #[arg(long, default_value_t = false)]
        compute_checksum: bool,
        /// Control whether symbols are indexed when adding sources
        #[arg(long, default_value = "on", value_parser = ["on", "off"])]
        index_symbols: String,
    },
    /// Remove model interchange files from project metadata
    #[clap(alias = "rm")]
    Remove {
        /// File(s) to exclude from the project
        #[arg(num_args = 1..)]
        paths: Vec<String>,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum UsageCommand {
    /// Add a usage (dependency) to the project
    Add {
        #[clap(flatten)]
        locator: AddProjectLocatorArgs,
        /// Version constraint (semver syntax)
        #[clap(verbatim_doc_comment)]
        version_constraint: Option<String>,
        /// Controls side-effects: manifest (only edit .project.json),
        /// lock (also update lockfile), sync (also install deps)
        #[arg(long, default_value = "sync", value_parser = ["manifest", "lock", "sync"])]
        update: String,
        #[command(flatten)]
        resolution_opts: ResolutionOptions,
        #[command(flatten)]
        source_opts: Box<ProjectSourceOptions>,
    },
    /// Remove a usage (dependency) from the project
    #[clap(alias = "rm")]
    Remove {
        #[clap(flatten)]
        locator: RemoveProjectLocatorArgs,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum LockCommand {
    /// Create or update the lockfile
    Update {
        #[command(flatten)]
        resolution_opts: ResolutionOptions,
    },
}

#[derive(clap::Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct AddProjectLocatorArgs {
    /// IRI/URI/URL identifying the project to be used
    #[clap(default_value = None, value_parser = parse_iri_suggest_path)]
    pub iri: Option<fluent_uri::Iri<String>>,
    /// Path to the project to be added. Since every usage is identified
    /// by an IRI, `file://` URL will be used to refer to the project.
    /// Warning: using this makes the project not portable between different
    /// computers, as `file://` URL always contains an absolute path.
    /// For multiple related projects, consider using a workspace instead
    #[arg(
        long,
        short = 'p',
        default_value = None,
        verbatim_doc_comment
    )]
    pub path: Option<Utf8PathBuf>,
}

#[derive(clap::Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct RemoveProjectLocatorArgs {
    /// IRI identifying the project usage to be removed
    #[clap(default_value = None, value_parser = parse_iri_suggest_path)]
    pub iri: Option<fluent_uri::Iri<String>>,
    /// Path to the project to be removed from usages. Since every usage is
    /// identified by an IRI, the path will be transformed into a `file://` URL
    #[arg(
        long,
        short = 'p',
        default_value = None,
        verbatim_doc_comment
    )]
    pub path: Option<Utf8PathBuf>,
}

#[derive(clap::Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct CloneProjectLocatorArgs {
    /// Clone the project from a given locator, trying to parse it as an
    /// IRI/URI/URL and otherwise falling back to using it as a path
    #[clap(
        default_value = None,
        value_name = "LOCATOR", 
        verbatim_doc_comment
    )]
    pub auto_location: Option<String>,
    /// IRI/URI/URL identifying the project to be cloned
    #[arg(short = 'i', long, visible_alias = "uri", visible_alias = "url")]
    pub iri: Option<fluent_uri::Iri<String>>,
    /// Path to clone the project from. If version is also
    /// given, verifies that the project has the given version
    // TODO: allow somehow requiring to use git here
    #[arg(
        long,
        short = 's',
        default_value = None,
        verbatim_doc_comment
    )]
    pub path: Option<Utf8PathBuf>,
}

#[derive(clap::ValueEnum, Default, Copy, Clone, Debug)]
#[clap(rename_all = "lowercase")]
pub enum KparCompressionMethodCli {
    /// Store the files as is
    Stored,
    /// Compress the files using Deflate
    #[default]
    Deflated,
    /// Compress the files using BZIP2
    #[cfg(feature = "kpar-bzip2")]
    Bzip2,
    /// Compress the files using ZStandard
    #[cfg(feature = "kpar-zstd")]
    Zstd,
    /// Compress the files using XZ
    #[cfg(feature = "kpar-xz")]
    Xz,
    /// Compress the files using PPMd
    #[cfg(feature = "kpar-ppmd")]
    Ppmd,
}

impl From<KparCompressionMethodCli> for KparCompressionMethod {
    fn from(value: KparCompressionMethodCli) -> Self {
        match value {
            KparCompressionMethodCli::Stored => KparCompressionMethod::Stored,
            KparCompressionMethodCli::Deflated => KparCompressionMethod::Deflated,
            #[cfg(feature = "kpar-bzip2")]
            KparCompressionMethodCli::Bzip2 => KparCompressionMethod::Bzip2,
            #[cfg(feature = "kpar-zstd")]
            KparCompressionMethodCli::Zstd => KparCompressionMethod::Zstd,
            #[cfg(feature = "kpar-xz")]
            KparCompressionMethodCli::Xz => KparCompressionMethod::Xz,
            #[cfg(feature = "kpar-ppmd")]
            KparCompressionMethodCli::Ppmd => KparCompressionMethod::Ppmd,
        }
    }
}

// This is implemented mainly so that if KparCompressionMethod gets a new member
// and KparCompressionMethodCli isn't updated it would give a compilation error
impl From<KparCompressionMethod> for KparCompressionMethodCli {
    fn from(value: KparCompressionMethod) -> Self {
        match value {
            KparCompressionMethod::Stored => KparCompressionMethodCli::Stored,
            KparCompressionMethod::Deflated => KparCompressionMethodCli::Deflated,
            #[cfg(feature = "kpar-bzip2")]
            KparCompressionMethod::Bzip2 => KparCompressionMethodCli::Bzip2,
            #[cfg(feature = "kpar-zstd")]
            KparCompressionMethod::Zstd => KparCompressionMethodCli::Zstd,
            #[cfg(feature = "kpar-xz")]
            KparCompressionMethod::Xz => KparCompressionMethodCli::Xz,
            #[cfg(feature = "kpar-ppmd")]
            KparCompressionMethod::Ppmd => KparCompressionMethodCli::Ppmd,
        }
    }
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum EnvCommand {
    /// Sync `sysand_env` to lockfile
    Sync {
        #[command(flatten)]
        resolution_opts: ResolutionOptions,
    },
    /// Install project in `sysand_env`
    Install {
        /// IRI identifying the project to be installed
        iri: fluent_uri::Iri<String>,
        /// Version to be installed. Defaults to the latest
        /// version according to SemVer 2.0, ignoring pre-releases
        #[clap(verbatim_doc_comment)]
        version: Option<String>,
        /// Path to interchange project
        #[arg(long, default_value = None)]
        path: Option<Utf8PathBuf>,

        #[command(flatten)]
        install_opts: InstallOptions,
        #[command(flatten)]
        resolution_opts: ResolutionOptions,
    },
    /// Uninstall project in `sysand_env`
    Uninstall {
        /// IRI identifying the project to be uninstalled
        iri: fluent_uri::Iri<String>,
        /// Version to be uninstalled
        version: Option<String>,
    },
    /// List projects installed in `sysand_env`
    List,
}

#[derive(clap::Args, Debug, Clone)]
pub struct InstallOptions {
    /// Allow overwriting existing installation
    #[arg(long)]
    pub allow_overwrite: bool,
    /// Install even if another version is already installed
    #[arg(long)]
    pub allow_multiple: bool,
    /// Whether to install dependencies
    #[arg(long, default_value = "all", value_parser = ["all", "none"])]
    pub deps: String,
}

/// Control how packages and their dependencies are resolved.
/// `include_std` is here only for convenience, as it does not
/// affect package resolution, only installation
/// (in `sync`, `env install`, `lock`, etc.)
#[derive(clap::Args, Debug, Clone)]
pub struct ResolutionOptions {
    /// Comma-delimited list of index URLs to use when resolving
    /// project(s) and/or their dependencies, in addition to the default indexes.
    #[arg(
        long,
        num_args = 0..,
        global = true,
        help_heading = "Resolution options",
        env = env_vars::SYSAND_INDEX,
        value_delimiter = ',',
        verbatim_doc_comment
    )]
    pub index: Vec<String>,
    /// Comma-delimited list of URLs to use as default index
    /// URLs. Default indexes are tried before other indexes
    /// (default `https://beta.sysand.org`)
    // TODO: verify index use order
    #[arg(
        long,
        num_args = 0..,
        global = true,
        help_heading = "Resolution options",
        env = env_vars::SYSAND_DEFAULT_INDEX,
        value_delimiter = ',',
        verbatim_doc_comment
    )]
    pub default_index: Vec<String>,
    /// Whether to use default indexes when resolving
    #[arg(
        long,
        default_value = "default",
        value_parser = ["default", "none"],
        conflicts_with_all = ["index", "default_index"],
        global = true,
        help_heading = "Resolution options",
    )]
    pub index_mode: String,
    /// Don't ignore KerML/SysML v2 standard libraries if specified as dependencies
    #[arg(
        long,
        default_value_t = false,
        global = true,
        help_heading = "Resolution options"
    )]
    pub include_std: bool,
}

#[derive(clap::Args, Debug, Clone)]
pub struct ProjectSourceOptions {
    /// Add usage as a local interchange project at PATH and
    /// update configuration file attempting to guess the
    /// source from the PATH
    #[arg(long, value_name = "PATH", group = "source")]
    pub from_path: Option<Utf8PathBuf>,
    /// Add usage as a remote interchange project at URL and
    /// update configuration file attempting to guess the
    /// source from the URL
    #[arg(long, value_name = "URL", group = "source")]
    pub from_url: Option<Iri<String>>,
    /// Add usage as an editable interchange project at PATH and
    /// update configuration file with appropriate source
    #[arg(long, value_name = "PATH", group = "source")]
    pub as_editable: Option<Utf8PathBuf>,
    /// Add usage as a local interchange project at PATH and
    /// update configuration file with appropriate source
    #[arg(long, value_name = "PATH", group = "source")]
    pub as_local_src: Option<Utf8PathBuf>,
    /// Add usage as a local interchange project archive at PATH
    /// and update configuration file with appropriate source
    #[arg(long, value_name = "PATH", group = "source")]
    pub as_local_kpar: Option<Utf8PathBuf>,
    /// Add usage as a remote interchange project at URL and
    /// update configuration file with appropriate source
    #[arg(long, value_name = "URL", group = "source")]
    pub as_remote_src: Option<Iri<String>>,
    /// Add usage as a remote interchange project archive at URL
    /// and update configuration file with appropriate source
    #[arg(long, value_name = "URL", group = "source")]
    pub as_remote_kpar: Option<Iri<String>>,
    /// Add usage as a remote git interchange project at URL and
    /// update configuration file with appropriate source
    #[arg(long, value_name = "URL", group = "source")]
    pub as_remote_git: Option<Iri<String>>,
}

#[derive(clap::Args, Debug)]
pub struct GlobalOptions {
    /// Use verbose output
    #[arg(
        long,
        short,
        group = "log-level",
        global = true,
        help_heading = "Global options"
    )]
    pub verbose: bool,
    /// Do not output log messages
    #[arg(
        long,
        short,
        group = "log-level",
        global = true,
        help_heading = "Global options"
    )]
    pub quiet: bool,
    /// Config mode: auto (discover), none (disable), or path to sysand.toml
    #[arg(long, default_value = "auto", global = true, help_heading = "Global options")]
    pub config: String,
    /// Print help
    #[arg(long, short, global = true, action = clap::ArgAction::HelpLong, help_heading = "Global options")]
    pub help: Option<bool>,
}


// Default metamodel for .kpar archives is KerML according to spec.
// But for non-packaged projects there is no default.
// Therefore, we don't provide a default here.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[clap(rename_all = "lowercase")]
pub enum MetamodelKind {
    /// SysML v2 metamodel. Identifier: `https://www.omg.org/spec/SysML/<release>`
    SysML,
    /// KerML metamodel. Identifier: `https://www.omg.org/spec/KerML/<release>`
    KerML,
}

impl MetamodelKind {
    pub const SYSML: &str = "https://www.omg.org/spec/SysML/";
    pub const KERML: &str = "https://www.omg.org/spec/KerML/";
}

impl From<&MetamodelKind> for &'static str {
    fn from(value: &MetamodelKind) -> Self {
        match value {
            MetamodelKind::SysML => MetamodelKind::SYSML,
            MetamodelKind::KerML => MetamodelKind::KERML,
        }
    }
}

impl From<MetamodelKind> for &'static str {
    fn from(value: MetamodelKind) -> Self {
        Self::from(&value)
    }
}

impl Display for MetamodelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metamodel(pub MetamodelKind, pub MetamodelVersion);

impl From<&Metamodel> for String {
    fn from(value: &Metamodel) -> Self {
        let mut s = String::new();
        s.push_str(value.0.into());
        s.push_str(value.1.into());
        s
    }
}

impl Display for Metamodel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.into())?;
        f.write_str(self.1.into())
    }
}

impl From<Metamodel> for String {
    fn from(value: Metamodel) -> Self {
        Self::from(&value)
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetamodelVersion {
    Release_20250201 = 20250201,
}

impl From<&MetamodelVersion> for &'static str {
    fn from(value: &MetamodelVersion) -> Self {
        match value {
            MetamodelVersion::Release_20250201 => MetamodelVersion::RELEASE,
        }
    }
}

impl From<MetamodelVersion> for &'static str {
    fn from(value: MetamodelVersion) -> Self {
        Self::from(&value)
    }
}

impl MetamodelVersion {
    pub const RELEASE: &str = "20250201";
}

impl ValueEnum for MetamodelVersion {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Release_20250201]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        use clap::builder::PossibleValue;
        Some(match self {
            MetamodelVersion::Release_20250201 => {
                PossibleValue::new(MetamodelVersion::RELEASE).help("SysMLv2/KerML Release or Beta4")
            }
        })
    }
}

fn parse_iri_suggest_path(s: &str) -> Result<Iri<String>, String> {
    use crate::style::USAGE;
    Iri::parse(s.to_owned()).map_err(|(err, _val)| {
        format!("{err}\n{USAGE}hint:{USAGE:#} if you wanted to use a path, use `--path` instead")
    })
}
