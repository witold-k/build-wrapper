// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Real end-to-end build-system integration tests using checked-in fixtures.
//! Fixtures live under tests/fixtures; working copies and build outputs live in Cargo's target directory.
//! Missing optional toolchains skip their respective test unless
//! BUILD_WRAPPER_E2E_REQUIRED=1 is set.

use build_wrapper::{BltRunner, BuildSystem, SetupMode, Stage, StageStatus};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Project(pub PathBuf);

impl Project {
    pub fn new(fixture: &str) -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let suite = std::env::current_exe()
            .ok()
            .and_then(|path| path.file_stem().map(|name| name.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "e2e".to_owned());
        let dir = cargo_target_dir()
            .join("build-wrapper-e2e")
            .join(format!("{suite}-{}-{id}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests").join("fixtures").join(fixture);
        copy_fixture(&source, &dir, true);
        Self(dir)
    }

    pub fn use_broken_sources(&self, fixture: &str) {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests").join("fixtures").join(fixture).join("broken");
        copy_fixture(&source, &self.0, false);
    }
}

fn cargo_target_dir() -> PathBuf {
    // Cargo metadata respects workspace roots, CARGO_TARGET_DIR and
    // target-dir configuration (including a shared external target).
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("invalid cargo metadata JSON");
    PathBuf::from(
        metadata["target_directory"]
            .as_str()
            .expect("cargo metadata missing target_directory"),
    )
}

fn copy_fixture(source: &Path, dest: &Path, skip_broken: bool) {
    for entry in fs::read_dir(source).expect("fixture directory missing") {
        let entry = entry.expect("invalid fixture entry");
        let name = entry.file_name();
        if skip_broken && name == "broken" { continue; }
        let target = dest.join(&name);
        if entry.file_type().unwrap().is_dir() {
            fs::create_dir_all(&target).unwrap();
            copy_fixture(&entry.path(), &target, false);
        } else {
            fs::copy(entry.path(), target).expect("failed to copy fixture source");
        }
    }

}

fn available(program: &str) -> bool {
    Command::new(program).arg("--version").output().is_ok()
}

pub fn toolchain(name: &str, tools: &[&str]) -> bool {
    let missing: Vec<_> = tools.iter().copied().filter(|tool| !available(tool)).collect();
    if missing.is_empty() {
        return true;
    }
    if std::env::var_os("BUILD_WRAPPER_E2E_REQUIRED").is_some() {
        panic!("{name}: missing required tools: {missing:?}");
    }
    eprintln!("SKIP {name}: missing tools: {missing:?}");
    false
}

pub fn run(project: &Path, system: BuildSystem) -> build_wrapper::RunReport {
    assert_eq!(BuildSystem::detect(project), Some(system));
    let target = system.default_target_dir(project);
    let commands = system.commands(project, &target);
    let mut runner = BltRunner::new(project, commands);
    runner.setup_mode = SetupMode::Auto;
    runner.setup_marker = system.setup_marker(project, &target);
    runner.run().expect("runner failed before executing stages")
}

pub fn assert_pass(report: &build_wrapper::RunReport) {
    assert!(report.succeeded(), "unexpected build failure: {report:#?}");
    assert_eq!(report.stages.iter().find(|s| s.stage == Stage::Build).unwrap().status,
        StageStatus::Succeeded);
}

pub fn assert_build_fails(report: &build_wrapper::RunReport) {
    assert!(!report.succeeded(), "broken source unexpectedly built: {report:#?}");
    assert_eq!(report.first_failure().unwrap().stage, Stage::Build);
    assert_eq!(report.stages.iter().find(|s| s.stage == Stage::Test).unwrap().status,
        StageStatus::NotRun);
}

