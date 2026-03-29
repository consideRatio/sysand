// SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use camino::Utf8PathBuf;
use jni::{
    JNIEnv,
    objects::{JClass, JObject, JObjectArray, JString},
};
use sysand_core::{
    SysandError,
    env::local_directory,
    project::local_src::LocalSrcProject,
    types::{
        enums::Compression,
        options::{BuildOptions, InitOptions},
    },
    workspace::Workspace,
};

use crate::{
    conversion::{ToJObject, ToJObjectArray, java_map_to_index_map},
    exceptions::JniExt,
};

mod conversion;
mod exceptions;

// ---------------------------------------------------------------------------
// Unified error handling
// ---------------------------------------------------------------------------

fn throw_sysand_error(env: &mut JNIEnv<'_>, err: SysandError) {
    let message = format!("[{}] {}", err.code, err.message);
    env.throw_new(
        "com/sensmetry/sysand/exceptions/SysandException",
        &message,
    )
    .expect("failed to throw SysandException");
}

// ---------------------------------------------------------------------------
// JNI functions
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sensmetry_sysand_Sysand_init<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    name: JString<'local>,
    publisher: JString<'local>,
    version: JString<'local>,
    license: JString<'local>,
    path: JString<'local>,
) {
    let Some(name) = env.get_str(&name, "name") else {
        return;
    };
    let publisher = env.get_nullable_str(&publisher);
    let Some(version) = env.get_str(&version, "version") else {
        return;
    };
    let Some(path) = env.get_str(&path, "path") else {
        return;
    };
    let license = env.get_nullable_str(&license);

    let mut project = LocalSrcProject {
        nominal_path: None,
        project_path: Utf8PathBuf::from(&path),
    };
    match sysand_core::facade::init::init(
        &mut project,
        InitOptions {
            name: Some(name),
            publisher,
            version: Some(version),
            license,
            allow_non_spdx: false,
        },
    ) {
        Ok(()) => {}
        Err(e) => throw_sysand_error(&mut env, e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sensmetry_sysand_Sysand_defaultEnvName<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> JString<'local> {
    match env.new_string(local_directory::DEFAULT_ENV_NAME) {
        Ok(s) => s,
        Err(e) => {
            env.throw_runtime_exception(format!("Failed to create String: {e}"));
            JString::default()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sensmetry_sysand_Sysand_env<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    path: JString<'local>,
) {
    let Some(path) = env.get_str(&path, "path") else {
        return;
    };
    match sysand_core::facade::env::create(camino::Utf8Path::new(&path)) {
        Ok(()) => {}
        Err(e) => throw_sysand_error(&mut env, e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sensmetry_sysand_Sysand_workspaceProjectPaths<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    workspace_path: JString<'local>,
) -> JObjectArray<'local> {
    let Some(workspace_path) = env.get_str(&workspace_path, "workspacePath") else {
        return JObjectArray::default();
    };
    let workspace = match Workspace::new(workspace_path.into()) {
        Ok(w) => w,
        Err(e) => {
            throw_sysand_error(&mut env, SysandError::from(e));
            return JObjectArray::default();
        }
    };
    let paths: Vec<String> = workspace
        .absolute_project_paths()
        .into_iter()
        .map(|p| p.into_string())
        .collect();
    paths.to_jobject_array(&mut env).unwrap_or_default()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sensmetry_sysand_Sysand_setProjectIndex<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    project_path: JString<'local>,
    index: JObject<'local>,
) {
    let Some(project_path) = env.get_str(&project_path, "projectPath") else {
        return;
    };
    let rust_index = match java_map_to_index_map(&mut env, &index) {
        Ok(index) => index,
        Err(jni::errors::Error::JavaException) => return,
        Err(e) => {
            env.throw_runtime_exception(format!("Failed to convert index map: {e}"));
            return;
        }
    };
    let mut project = LocalSrcProject {
        nominal_path: None,
        project_path: Utf8PathBuf::from(project_path),
    };
    if let Err(e) = project.set_index(rust_index) {
        throw_sysand_error(
            &mut env,
            SysandError::new(sysand_core::ErrorCode::Internal, e.to_string()),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sensmetry_sysand_Sysand_buildProject<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    output_path: JString<'local>,
    project_path: JString<'local>,
    compression: JString<'local>,
) {
    let Some(output_path) = env.get_str(&output_path, "outputPath") else {
        return;
    };
    let Some(project_path) = env.get_str(&project_path, "projectPath") else {
        return;
    };
    let project = LocalSrcProject {
        nominal_path: None,
        project_path: Utf8PathBuf::from(&project_path),
    };
    let Some(compression_str) = env.get_str(&compression, "compression") else {
        return;
    };
    let compression = match parse_compression(&compression_str) {
        Ok(c) => c,
        Err(e) => {
            throw_sysand_error(&mut env, e);
            return;
        }
    };
    match sysand_core::facade::build::build(
        &project,
        camino::Utf8Path::new(&output_path),
        BuildOptions {
            compression,
            ..Default::default()
        },
    ) {
        Ok(_) => {}
        Err(e) => throw_sysand_error(&mut env, e),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sensmetry_sysand_Sysand_buildWorkspace<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    output_path: JString<'local>,
    workspace_path: JString<'local>,
    compression: JString<'local>,
) {
    let Some(output_path) = env.get_str(&output_path, "outputPath") else {
        return;
    };
    let Some(workspace_path) = env.get_str(&workspace_path, "workspacePath") else {
        return;
    };
    let workspace = match Workspace::new(workspace_path.into()) {
        Ok(w) => w,
        Err(e) => {
            throw_sysand_error(&mut env, SysandError::from(e));
            return;
        }
    };
    let Some(compression_str) = env.get_str(&compression, "compression") else {
        return;
    };
    let compression = match parse_compression(&compression_str) {
        Ok(c) => c,
        Err(e) => {
            throw_sysand_error(&mut env, e);
            return;
        }
    };

    if let Err(e) = sysand_core::project::utils::wrapfs::create_dir_all(&output_path) {
        throw_sysand_error(&mut env, SysandError::from(e));
        return;
    }

    match sysand_core::facade::workspace::build(
        &workspace,
        camino::Utf8Path::new(&output_path),
        BuildOptions {
            compression,
            ..Default::default()
        },
    ) {
        Ok(_) => {}
        Err(e) => throw_sysand_error(&mut env, e),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_compression(s: &str) -> Result<Compression, SysandError> {
    match s.to_ascii_uppercase().as_str() {
        "STORED" => Ok(Compression::Stored),
        "DEFLATED" => Ok(Compression::Deflated),
        "BZIP2" => Ok(Compression::Bzip2),
        "ZSTD" => Ok(Compression::Zstd),
        "XZ" => Ok(Compression::Xz),
        "PPMD" => Ok(Compression::Ppmd),
        other => Err(SysandError::new(
            sysand_core::ErrorCode::FieldInvalid,
            format!("unknown compression: {other}"),
        )),
    }
}
