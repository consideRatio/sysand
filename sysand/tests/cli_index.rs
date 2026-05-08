// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>

use assert_cmd::prelude::*;
use camino::{Utf8Path, Utf8PathBuf};
use predicates::prelude::*;
use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use sysand_core::{
    commands::lock::DEFAULT_LOCKFILE_NAME,
    env::local_directory::{DEFAULT_ENV_NAME, METADATA_PATH},
};

mod common;
pub use common::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct HttpServer {
    child: Child,
    url: String,
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn index_init_creates_index_json() -> TestResult {
    let (_temp_dir, cwd, out) = run_sysand(["index", "init"], None)?;

    out.assert().success();
    assert!(cwd.join("index.json").is_file());

    Ok(())
}

#[test]
fn index_add_accepts_explicit_iri_for_existing_kpar() -> TestResult {
    let (_temp_dir, cwd, out) = run_sysand(["index", "init"], None)?;
    out.assert().success();

    let fixture = fixture_path("test_lib.kpar");
    let out = run_sysand_in(
        &cwd,
        [
            "index",
            "add",
            fixture.as_str(),
            "--iri",
            "urn:kpar:test-lib",
        ],
        None,
    )?;

    out.assert()
        .success()
        .stderr(predicate::str::contains("Added"));
    assert!(cwd.join("_iri").is_dir());

    Ok(())
}

#[test]
fn index_tree_served_over_http_can_lock_and_sync_project() -> TestResult {
    let (_temp_dir, cwd) = new_temp_cwd()?;
    let index_root = cwd.join("index");
    std::fs::create_dir(&index_root)?;

    let dep_kpar = build_project_kpar(&cwd, "dep", "Acme", "Dep", "1.0.0", &[])?;
    let app_kpar = build_project_kpar(
        &cwd,
        "app",
        "Acme",
        "App",
        "1.0.0",
        &["pkg:sysand/acme/dep"],
    )?;

    run_sysand_in(&index_root, ["index", "init"], None)?
        .assert()
        .success();
    run_sysand_in(&index_root, ["index", "add", dep_kpar.as_str()], None)?
        .assert()
        .success();
    run_sysand_in(&index_root, ["index", "add", app_kpar.as_str()], None)?
        .assert()
        .success();

    let server = serve_dir(&index_root)?;

    let consumer = cwd.join("consumer");
    run_sysand_in(
        &cwd,
        [
            "init",
            "--publisher",
            "Acme",
            "--name",
            "Consumer",
            "--version",
            "1.0.0",
            consumer.as_str(),
        ],
        None,
    )?
    .assert()
    .success();

    run_sysand_in(
        &consumer,
        [
            "add",
            "pkg:sysand/acme/app",
            "--no-lock",
            "--no-sync",
            "--index",
            server.url.as_str(),
        ],
        None,
    )?
    .assert()
    .success();

    run_sysand_in(&consumer, ["lock", "--index", server.url.as_str()], None)?
        .assert()
        .success();
    run_sysand_in(&consumer, ["sync"], None)?.assert().success();

    let lockfile = std::fs::read_to_string(consumer.join(DEFAULT_LOCKFILE_NAME))?;
    assert!(lockfile.contains("pkg:sysand/acme/app"));
    assert!(lockfile.contains("pkg:sysand/acme/dep"));
    assert!(lockfile.contains("index_kpar"));

    let env_metadata =
        std::fs::read_to_string(consumer.join(DEFAULT_ENV_NAME).join(METADATA_PATH))?;
    assert!(env_metadata.contains("name = \"App\""));
    assert!(env_metadata.contains("name = \"Dep\""));

    Ok(())
}

fn build_project_kpar(
    workspace: &Utf8Path,
    dir_name: &str,
    publisher: &str,
    name: &str,
    version: &str,
    usages: &[&str],
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let project_dir = workspace.join(dir_name);
    run_sysand_in(
        workspace,
        [
            "init",
            "--publisher",
            publisher,
            "--name",
            name,
            "--version",
            version,
            "--license",
            "MIT",
            project_dir.as_str(),
        ],
        None,
    )?
    .assert()
    .success();

    std::fs::write(
        project_dir.join("model.kerml"),
        format!("package {name};\n"),
    )?;
    run_sysand_in(
        &project_dir,
        ["include", "--no-index-symbols", "model.kerml"],
        None,
    )?
    .assert()
    .success();

    for usage in usages {
        run_sysand_in(
            &project_dir,
            ["add", usage, "--no-lock", "--no-sync", "1.0.0"],
            None,
        )?
        .assert()
        .success();
    }

    let kpar_path = workspace.join(format!("{dir_name}.kpar"));
    run_sysand_in(&project_dir, ["build", kpar_path.as_str()], None)?
        .assert()
        .success();
    Ok(kpar_path)
}

fn serve_dir(root: &Utf8Path) -> Result<HttpServer, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);

    let child = Command::new("python3")
        .args([
            "-m",
            "http.server",
            port.to_string().as_str(),
            "--bind",
            "127.0.0.1",
            "-d",
            root.as_str(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let server = HttpServer {
        child,
        url: format!("http://127.0.0.1:{port}/"),
    };
    wait_for_http(&server.url)?;
    Ok(server)
}

fn wait_for_http(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let client = reqwest::blocking::Client::new();
    loop {
        match client.get(format!("{url}index.json")).send() {
            Ok(response) if response.status().is_success() => return Ok(()),
            _ if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            _ => return Err(format!("HTTP server at `{url}` did not become ready").into()),
        }
    }
}
